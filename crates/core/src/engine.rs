//! Mobile engine orchestration.
//!
//! The engine owns one agent turn: it emits timeline events, stores turn state,
//! parses tool calls from the model response, saves pending approvals and can
//! later continue a stored approval after the mobile UI sends a decision.

use crate::agent::DeepSeekAgent;
use crate::approval_card::{approval_cards_from_records, ApprovalCardView};
use crate::approval::ReviewDecision;
use crate::approval_session::ApprovalSessionPolicy;
use crate::config::Config;
use crate::events::AgentEvent;
use crate::runtime_store::{RuntimeThreadStore, TurnRecord};
use crate::tool_loop::{
    continue_pending_tool_approval_with_session, process_model_text_with_tools_and_session,
    ToolLoopExecutionRecord, ToolLoopOutcome,
};
use crate::tools::{default_mobile_tool_registry, ToolContext, ToolRegistry};
use crate::turn::{TurnContext, TurnStatus};
use crate::workspace::{ExecutorKind, Workspace};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub struct EngineTurnResult {
    pub events: Vec<AgentEvent>,
    pub final_text: Option<String>,
    pub approval_cards: Vec<ApprovalCardView>,
    pub executed: Vec<ToolLoopExecutionRecord>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EngineApprovalContinuationResult {
    pub events: Vec<AgentEvent>,
    pub executed: Vec<ToolLoopExecutionRecord>,
    pub session_grants_created: Vec<crate::approval_session::ApprovalSessionGrant>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnginePendingApprovalSnapshot {
    pub cards: Vec<ApprovalCardView>,
}

pub struct MobileEngine {
    agent: DeepSeekAgent,
    registry: ToolRegistry,
    runtime_store: Option<RuntimeThreadStore>,
    thread_id: String,
    workspace: Workspace,
    approval_session: ApprovalSessionPolicy,
    event_observer: Option<Arc<dyn Fn(AgentEvent)>>,
}

impl MobileEngine {
    pub fn new(config: Config) -> Self {
        Self {
            agent: DeepSeekAgent::new(config),
            registry: default_mobile_tool_registry(),
            runtime_store: None,
            thread_id: "mobile-default-thread".to_string(),
            workspace: Workspace::new(
                "mobile-workspace",
                "Mobile Workspace",
                PathBuf::from("."),
                ExecutorKind::LocalAndroid,
            ),
            approval_session: ApprovalSessionPolicy::new(),
            event_observer: None,
        }
    }

    pub fn with_runtime_store(mut self, store: RuntimeThreadStore) -> Self {
        self.runtime_store = Some(store);
        self
    }

    pub fn with_thread_id(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = thread_id.into();
        self
    }

    pub fn with_workspace(mut self, workspace_root: impl Into<PathBuf>) -> Self {
        self.workspace.root = workspace_root.into();
        self
    }

    pub fn with_event_observer<F>(mut self, observer: F) -> Self
    where
        F: Fn(AgentEvent) + 'static,
    {
        self.event_observer = Some(Arc::new(observer));
        self
    }

    pub fn supports_streaming(&self) -> bool {
        true
    }

    pub async fn run_turn_with_streaming(&self, user_input: String) -> Result<EngineTurnResult> {
        self.run_turn(user_input).await
    }

    pub async fn run_turn(&self, user_input: String) -> Result<EngineTurnResult> {
        let mut turn = TurnContext::new(100);
        turn.start();
        self.persist_turn(&turn)?;

        let mut events = Vec::new();
        self.push_event(&mut events, AgentEvent::Started)?;
        self.push_event(&mut events, AgentEvent::TurnStarted { turn_id: turn.id.clone() })?;
        self.push_event(&mut events, AgentEvent::Status("MobileEngine turn started".to_string()))?;

        let answer = match self.agent.run(user_input).await {
            Ok(answer) => answer,
            Err(error) => {
                turn.fail(error.to_string());
                self.persist_turn(&turn)?;
                self.push_event(&mut events, AgentEvent::Error(error.to_string()))?;
                self.push_event(
                    &mut events,
                    AgentEvent::TurnFinished {
                        turn_id: turn.id.clone(),
                        status: turn.status.clone(),
                        usage: turn.usage.clone(),
                        error: turn.error.clone(),
                    },
                )?;
                return Ok(EngineTurnResult {
                    events,
                    final_text: turn.error.clone(),
                    approval_cards: Vec::new(),
                    executed: Vec::new(),
                });
            }
        };

        self.push_event(&mut events, AgentEvent::TextDelta(answer.clone()))?;
        let context = ToolContext::new(self.workspace.clone());
        let mut session = self.approval_session.clone();
        let outcome = process_model_text_with_tools_and_session(
            answer.clone(),
            &self.registry,
            &context,
            &mut turn,
            &mut session,
        )
        .await?;

        self.store_pending_approvals(&turn, &outcome)?;
        for event in outcome.events.iter().cloned() {
            self.push_event(&mut events, event)?;
        }

        if outcome.has_pending_approvals() {
            turn.status = TurnStatus::WaitingForApproval;
        } else {
            turn.complete();
        }
        self.persist_turn(&turn)?;
        self.push_event(
            &mut events,
            AgentEvent::TurnFinished {
                turn_id: turn.id.clone(),
                status: turn.status.clone(),
                usage: turn.usage.clone(),
                error: turn.error.clone(),
            },
        )?;
        self.push_event(&mut events, AgentEvent::Finished)?;

        Ok(EngineTurnResult {
            events,
            final_text: outcome.final_text.or(Some(answer)),
            approval_cards: outcome.approval_cards,
            executed: outcome.executed,
        })
    }

    pub async fn continue_stored_approval(
        &self,
        approval_id: &str,
        decision: ReviewDecision,
        mut turn: TurnContext,
    ) -> Result<EngineApprovalContinuationResult> {
        let Some(store) = &self.runtime_store else {
            anyhow::bail!("runtime store is required to continue stored approval");
        };
        let pending_record = store.load_pending_approval(approval_id)?;
        let context = ToolContext::new(self.workspace.clone());
        let mut session = self.approval_session.clone();
        let outcome = continue_pending_tool_approval_with_session(
            pending_record.pending,
            decision.clone(),
            &self.registry,
            &context,
            &mut turn,
            &mut session,
        )
        .await?;

        store.save_decision(&crate::tool_loop::decision_record(
            pending_record.thread_id.clone(),
            pending_record.turn_id.clone(),
            approval_id.to_string(),
            &decision,
        ))?;
        store.remove_pending_approval(approval_id)?;
        self.persist_turn(&turn)?;

        Ok(EngineApprovalContinuationResult {
            events: outcome.events,
            executed: outcome.executed,
            session_grants_created: outcome.session_grants_created,
        })
    }

    pub fn pending_approval_cards_for_current_thread(&self) -> Result<Vec<ApprovalCardView>> {
        let Some(store) = &self.runtime_store else {
            return Ok(Vec::new());
        };
        let records = store.list_pending_approvals_for_thread(&self.thread_id)?;
        Ok(approval_cards_from_records(&records))
    }

    pub fn pending_approval_snapshot(&self) -> Result<EnginePendingApprovalSnapshot> {
        Ok(EnginePendingApprovalSnapshot {
            cards: self.pending_approval_cards_for_current_thread()?,
        })
    }

    fn store_pending_approvals(&self, turn: &TurnContext, outcome: &ToolLoopOutcome) -> Result<()> {
        if let Some(store) = &self.runtime_store {
            for pending in outcome.pending_approvals.iter().cloned() {
                store.save_pending_approval(self.thread_id.clone(), turn.id.clone(), pending)?;
            }
        }
        Ok(())
    }

    fn persist_turn(&self, turn: &TurnContext) -> Result<()> {
        if let Some(store) = &self.runtime_store {
            store.save_turn(&TurnRecord::from_turn(self.thread_id.clone(), turn))?;
        }
        Ok(())
    }

    fn push_event(&self, events: &mut Vec<AgentEvent>, event: AgentEvent) -> Result<()> {
        if let Some(observer) = &self.event_observer {
            observer(event.clone());
        }
        if let Some(store) = &self.runtime_store {
            let turn_id = match &event {
                AgentEvent::TurnStarted { turn_id } => turn_id.clone(),
                AgentEvent::TurnFinished { turn_id, .. } => turn_id.clone(),
                _ => "unknown-turn".to_string(),
            };
            let _ = store.save_event(self.thread_id.clone(), turn_id, &event);
        }
        events.push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MobileEngine;
    use crate::config::Config;

    #[test]
    fn engine_reports_streaming_support() {
        let engine = MobileEngine::new(Config::default());
        assert!(engine.supports_streaming());
    }
}
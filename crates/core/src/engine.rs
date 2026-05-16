//! Mobile engine orchestration.
//!
//! The engine owns one agent turn: it emits timeline events, stores turn state,
//! parses tool calls from the model response, saves pending approvals and can
//! later continue a stored approval after the mobile UI sends a decision.

use crate::agent::DeepSeekAgent;
use crate::approval::ReviewDecision;
use crate::approval_card::{approval_cards_from_records, ApprovalCardView};
use crate::approval_session::ApprovalSessionPolicy;
use crate::config::Config;
use crate::events::AgentEvent;
use crate::pc_gateway_client::PcGatewayClient;
use crate::runtime_store::{RuntimeThreadStore, TurnRecord};
use crate::tool_loop::{
    continue_pending_tool_approval_with_session_and_pc_gateway,
    process_model_text_with_tools_and_session_and_pc_gateway, ToolLoopExecutionRecord,
    ToolLoopOutcome,
};
use crate::tools::{default_mobile_tool_registry, ToolContext, ToolRegistry};
use crate::turn::{TurnContext, TurnStatus};
use crate::workspace::{ExecutorKind, Workspace};
use crate::workspace_connection::WorkspaceConnection;
use anyhow::{anyhow, Result};
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
    pc_gateway: Option<PcGatewayClient>,
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
            pc_gateway: None,
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

    pub fn with_workspace_model(mut self, workspace: Workspace) -> Self {
        self.workspace = workspace;
        self
    }

    pub fn with_pc_gateway(mut self, client: PcGatewayClient) -> Self {
        self.pc_gateway = Some(client);
        self
    }

    pub fn with_workspace_connection(mut self, connection: &WorkspaceConnection) -> Result<Self> {
        self.workspace = connection.to_workspace();
        if let Some(gateway_config) = connection.pc_gateway.clone() {
            self.pc_gateway = Some(PcGatewayClient::new(gateway_config));
        }
        if matches!(self.workspace.executor, ExecutorKind::PcGateway) && self.pc_gateway.is_none() {
            return Err(anyhow!(
                "PC gateway workspace selected but connection has no PcGatewayConfig"
            ));
        }
        Ok(self)
    }

    pub fn has_pc_gateway(&self) -> bool {
        self.pc_gateway.is_some()
    }

    pub fn with_approval_session(mut self, approval_session: ApprovalSessionPolicy) -> Self {
        self.approval_session = approval_session;
        self
    }

    pub fn approval_session(&self) -> &ApprovalSessionPolicy {
        &self.approval_session
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

    pub fn approval_session_grant_count(&self) -> usize {
        self.approval_session.grant_count()
    }

    pub async fn run_turn_with_streaming(&mut self, user_input: String) -> Result<EngineTurnResult> {
        self.run_turn_internal(user_input, true).await
    }

    pub async fn run_turn(&mut self, user_input: String) -> Result<EngineTurnResult> {
        self.run_turn_internal(user_input, false).await
    }

    async fn run_turn_internal(&mut self, user_input: String, streaming: bool) -> Result<EngineTurnResult> {
        let mut turn = TurnContext::new(100);
        turn.start();
        self.persist_turn(&turn)?;

        let mut events = Vec::new();
        self.push_event(&mut events, AgentEvent::Started)?;
        self.push_event(&mut events, AgentEvent::TurnStarted { turn_id: turn.id.clone() })?;
        self.push_event(
            &mut events,
            AgentEvent::Status(if streaming {
                "MobileEngine streaming turn started".to_string()
            } else {
                "MobileEngine turn started".to_string()
            }),
        )?;

        let answer = match self.collect_model_answer(user_input, &mut events, streaming).await {
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

        let context = ToolContext::new(self.workspace.clone());
        let outcome = process_model_text_with_tools_and_session_and_pc_gateway(
            answer.clone(),
            &self.registry,
            &context,
            &mut turn,
            &mut self.approval_session,
            self.pc_gateway.as_ref(),
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

    async fn collect_model_answer(
        &self,
        user_input: String,
        events: &mut Vec<AgentEvent>,
        streaming: bool,
    ) -> Result<String> {
        if !streaming {
            let answer = self.agent.run(user_input).await?;
            self.push_event(events, AgentEvent::TextDelta(answer.clone()))?;
            return Ok(answer);
        }

        self.push_event(
            events,
            AgentEvent::MessageStarted {
                index: 0,
                role: "assistant".to_string(),
            },
        )?;
        self.push_event(events, AgentEvent::Status("DeepSeek streaming response opened".to_string()))?;

        let mut receiver = self.agent.run_stream(user_input).await?;
        let mut answer = String::new();
        while let Some(delta) = receiver.recv().await {
            if delta == "[DONE]" {
                break;
            }
            if delta.is_empty() {
                continue;
            }
            answer.push_str(&delta);
            self.push_event(events, AgentEvent::TextDelta(delta))?;
        }

        self.push_event(events, AgentEvent::MessageFinished { index: 0 })?;
        self.push_event(events, AgentEvent::Status("DeepSeek streaming response completed".to_string()))?;
        Ok(answer)
    }

    pub async fn continue_stored_approval(
        &mut self,
        approval_id: &str,
        decision: ReviewDecision,
        mut turn: TurnContext,
    ) -> Result<EngineApprovalContinuationResult> {
        let Some(store) = &self.runtime_store else {
            anyhow::bail!("runtime store is required to continue stored approval");
        };
        let pending_record = store.load_pending_approval(approval_id)?;
        let context = ToolContext::new(self.workspace.clone());
        let outcome = continue_pending_tool_approval_with_session_and_pc_gateway(
            pending_record.pending,
            decision.clone(),
            &self.registry,
            &context,
            &mut turn,
            &mut self.approval_session,
            self.pc_gateway.as_ref(),
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
    use crate::pc_gateway::PcGatewayConfig;
    use crate::workspace::ExecutorKind;
    use crate::workspace_connection::WorkspaceConnection;

    #[test]
    fn engine_reports_streaming_support() {
        let engine = MobileEngine::new(Config::default());
        assert!(engine.supports_streaming());
    }

    #[test]
    fn engine_can_be_configured_from_pc_workspace_connection() {
        let mut gateway = PcGatewayConfig::new("pc-1", "Laptop", "http://127.0.0.1:8787", "phone-1");
        gateway.allow_http_on_local_network = true;
        let connection = WorkspaceConnection::pc_gateway(
            "pc",
            "Laptop",
            "w1",
            "Project",
            "/pc/project",
            gateway,
        );
        let engine = MobileEngine::new(Config::default())
            .with_workspace_connection(&connection)
            .expect("configure pc gateway");
        assert!(engine.has_pc_gateway());
        assert_eq!(engine.workspace.executor, ExecutorKind::PcGateway);
    }
}
//! Mobile engine skeleton.
//!
//! This is the bridge between the Android UI and the reusable agent core. It
//! models turn lifecycle, event emission, routing, tool-call parsing, approval
//! handoff and backend-aware tool execution.

use crate::agent::DeepSeekAgent;
use crate::api_client::Message;
use crate::approval::{ApprovalMode, ReviewDecision};
use crate::config::Config;
use crate::context::ContextManager;
use crate::events::AgentEvent;
use crate::model_router::ModelRouter;
use crate::pc_gateway_client::PcGatewayClient;
use crate::runtime_store::{RuntimeThreadStore, ThreadRecord, TurnRecord};
use crate::tool_execution::ToolExecutionCoordinator;
use crate::tool_loop::{continue_pending_tool_approval, process_model_text_with_tools, PendingToolCallApproval};
use crate::tools::{default_mobile_tool_registry, ToolContext};
use crate::turn::{TurnContext, TurnStatus};
use crate::workspace::{ExecutorKind, Workspace};
use crate::workspace_connection::WorkspaceConnectionManager;
use anyhow::Result;
use std::path::PathBuf;

pub struct MobileEngine {
    config: Config,
    agent: DeepSeekAgent,
    router: ModelRouter,
    context_manager: ContextManager,
    max_steps: u32,
    runtime_store: Option<RuntimeThreadStore>,
    thread_id: String,
    workspace: PathBuf,
    active_workspace: Option<Workspace>,
    connection_manager: Option<WorkspaceConnectionManager>,
    pc_gateway_client: Option<PcGatewayClient>,
    approval_mode: ApprovalMode,
}

impl MobileEngine {
    pub fn new(config: Config) -> Self {
        let approval_mode = if config.auto_mode {
            ApprovalMode::Auto
        } else {
            ApprovalMode::Suggest
        };
        Self {
            agent: DeepSeekAgent::new(config.clone()),
            router: ModelRouter::new(config.clone()),
            context_manager: ContextManager::default(),
            config,
            max_steps: 100,
            runtime_store: None,
            thread_id: "default".to_string(),
            workspace: PathBuf::from("."),
            active_workspace: None,
            connection_manager: None,
            pc_gateway_client: None,
            approval_mode,
        }
    }

    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = max_steps;
        self
    }

    pub fn with_runtime_store(mut self, store: RuntimeThreadStore) -> Self {
        self.runtime_store = Some(store);
        self
    }

    pub fn with_thread_id(mut self, thread_id: impl Into<String>) -> Self {
        self.thread_id = thread_id.into();
        self
    }

    pub fn with_workspace(mut self, workspace: impl Into<PathBuf>) -> Self {
        self.workspace = workspace.into();
        self
    }

    pub fn with_active_workspace(mut self, workspace: Workspace) -> Self {
        self.workspace = workspace.root.clone();
        self.active_workspace = Some(workspace);
        self
    }

    pub fn with_connection_manager(mut self, manager: WorkspaceConnectionManager) -> Self {
        if let Some(workspace) = manager.active_workspace() {
            self.workspace = workspace.root.clone();
            self.active_workspace = Some(workspace);
        }
        self.connection_manager = Some(manager);
        self
    }

    pub fn with_pc_gateway_client(mut self, client: PcGatewayClient) -> Self {
        self.pc_gateway_client = Some(client);
        self
    }

    pub fn with_approval_mode(mut self, mode: ApprovalMode) -> Self {
        self.approval_mode = mode;
        self
    }

    pub fn active_workspace(&self) -> Option<&Workspace> {
        self.active_workspace.as_ref()
    }

    pub fn connection_manager(&self) -> Option<&WorkspaceConnectionManager> {
        self.connection_manager.as_ref()
    }

    pub fn pc_gateway_client(&self) -> Option<&PcGatewayClient> {
        self.pc_gateway_client.as_ref()
    }

    pub fn approval_mode(&self) -> &ApprovalMode {
        &self.approval_mode
    }

    pub async fn run_turn(&self, user_input: String) -> Result<EngineTurnResult> {
        let mut events = Vec::new();
        let mut turn = TurnContext::new(self.max_steps);
        turn.start();

        let mut thread = ThreadRecord::new(
            self.thread_id.clone(),
            title_from_input(&user_input),
            self.config.model.clone(),
            self.workspace.clone(),
        );
        thread.latest_turn_id = Some(turn.id.clone());

        if let Some(store) = self.runtime_store.as_ref() {
            store.save_thread(&thread)?;
            store.save_turn(&TurnRecord::from_context(&thread.id, &user_input, &turn))?;
        }

        self.push_event(
            &mut events,
            Some(&turn.id),
            AgentEvent::TurnStarted {
                turn_id: turn.id.clone(),
            },
        )?;

        if let Some(workspace) = self.active_workspace.as_ref() {
            self.push_event(
                &mut events,
                Some(&turn.id),
                AgentEvent::Status(format!(
                    "Workspace backend: {:?} at {}",
                    workspace.executor,
                    workspace.root.display()
                )),
            )?;
        }

        let route = self.router.route_prompt(&user_input, 0);
        self.push_event(
            &mut events,
            Some(&turn.id),
            AgentEvent::Status(format!("Model route: {} ({})", route.model, route.reason)),
        )?;

        let messages = vec![Message {
            role: "user".to_string(),
            content: user_input.clone(),
        }];
        let compression_plan = self.context_manager.plan_for_messages(&messages);
        if compression_plan.should_compress {
            self.push_event(
                &mut events,
                Some(&turn.id),
                AgentEvent::Status(format!(
                    "Context compression planned: {:?}",
                    compression_plan.strategy
                )),
            )?;
        }

        let response = self.agent.run_with_messages(messages.clone()).await;
        match response {
            Ok(text) => {
                self.push_event(
                    &mut events,
                    Some(&turn.id),
                    AgentEvent::MessageStarted {
                        index: 0,
                        role: "assistant".to_string(),
                    },
                )?;
                self.push_event(&mut events, Some(&turn.id), AgentEvent::TextDelta(text.clone()))?;
                self.push_event(
                    &mut events,
                    Some(&turn.id),
                    AgentEvent::MessageFinished { index: 0 },
                )?;

                let tool_loop = self.process_tools_if_requested(&text, &mut turn).await?;
                for event in tool_loop.events {
                    self.push_event(&mut events, Some(&turn.id), event)?;
                }

                if tool_loop.requires_user_input {
                    if let Some(store) = self.runtime_store.as_ref() {
                        for pending in &tool_loop.pending_tool_approvals {
                            store.save_pending_approval(&thread.id, &turn.id, pending)?;
                        }
                    }
                    self.push_event(
                        &mut events,
                        Some(&turn.id),
                        AgentEvent::TurnFinished {
                            turn_id: turn.id.clone(),
                            status: TurnStatus::WaitingForApproval,
                            usage: turn.usage.clone(),
                            error: None,
                        },
                    )?;
                    if let Some(store) = self.runtime_store.as_ref() {
                        store.save_turn(&TurnRecord::from_context(&thread.id, &user_input, &turn))?;
                    }
                    return Ok(EngineTurnResult {
                        turn,
                        events,
                        final_text: Some(tool_loop.final_text),
                    });
                }

                turn.complete();

                if let Some(store) = self.runtime_store.as_ref() {
                    store.save_turn(&TurnRecord::from_context(&thread.id, &user_input, &turn))?;
                }

                self.push_event(
                    &mut events,
                    Some(&turn.id),
                    AgentEvent::TurnFinished {
                        turn_id: turn.id.clone(),
                        status: TurnStatus::Completed,
                        usage: turn.usage.clone(),
                        error: None,
                    },
                )?;
                self.push_event(&mut events, Some(&turn.id), AgentEvent::Finished)?;
                Ok(EngineTurnResult {
                    turn,
                    events,
                    final_text: Some(tool_loop.final_text),
                })
            }
            Err(error) => {
                let error_text = error.to_string();
                turn.fail(error_text.clone());

                if let Some(store) = self.runtime_store.as_ref() {
                    store.save_turn(&TurnRecord::from_context(&thread.id, &user_input, &turn))?;
                }

                self.push_event(&mut events, Some(&turn.id), AgentEvent::Error(error_text.clone()))?;
                self.push_event(
                    &mut events,
                    Some(&turn.id),
                    AgentEvent::TurnFinished {
                        turn_id: turn.id.clone(),
                        status: TurnStatus::Failed,
                        usage: turn.usage.clone(),
                        error: Some(error_text.clone()),
                    },
                )?;
                Ok(EngineTurnResult {
                    turn,
                    events,
                    final_text: None,
                })
            }
        }
    }

    pub async fn continue_after_approval(
        &self,
        pending: PendingToolCallApproval,
        decision: ReviewDecision,
        mut turn: TurnContext,
    ) -> Result<EngineApprovalContinuationResult> {
        let mut events = Vec::new();
        let approval_id = pending.approval.id.clone();
        let registry = default_mobile_tool_registry();
        let mut coordinator = ToolExecutionCoordinator::new(&registry);
        if let Some(client) = self.pc_gateway_client.as_ref() {
            coordinator = coordinator.with_pc_gateway(client);
        }
        let context = self.tool_context();
        let outcome = continue_pending_tool_approval(
            &pending,
            &decision,
            &coordinator,
            &context,
            &mut turn,
        )
        .await?;

        for event in outcome.events {
            self.push_event(&mut events, Some(&turn.id), event)?;
        }

        if let Some(store) = self.runtime_store.as_ref() {
            store.delete_pending_approval(&approval_id)?;
        }

        let completed = !matches!(turn.status, TurnStatus::Cancelled) && !outcome.requires_user_input;
        if completed {
            turn.complete();
            self.push_event(
                &mut events,
                Some(&turn.id),
                AgentEvent::TurnFinished {
                    turn_id: turn.id.clone(),
                    status: TurnStatus::Completed,
                    usage: turn.usage.clone(),
                    error: None,
                },
            )?;
            self.push_event(&mut events, Some(&turn.id), AgentEvent::Finished)?;
        } else if matches!(turn.status, TurnStatus::Cancelled) {
            self.push_event(
                &mut events,
                Some(&turn.id),
                AgentEvent::TurnFinished {
                    turn_id: turn.id.clone(),
                    status: TurnStatus::Cancelled,
                    usage: turn.usage.clone(),
                    error: turn.error.clone(),
                },
            )?;
        }

        if let Some(store) = self.runtime_store.as_ref() {
            store.save_turn(&TurnRecord::from_context(&self.thread_id, "approval-continuation", &turn))?;
        }

        Ok(EngineApprovalContinuationResult {
            turn,
            events,
            executed: outcome.executed,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    async fn process_tools_if_requested(
        &self,
        model_text: &str,
        turn: &mut TurnContext,
    ) -> Result<crate::tool_loop::ToolLoopOutcome> {
        let registry = default_mobile_tool_registry();
        let mut coordinator = ToolExecutionCoordinator::new(&registry);
        if let Some(client) = self.pc_gateway_client.as_ref() {
            coordinator = coordinator.with_pc_gateway(client);
        }
        let context = self.tool_context();
        process_model_text_with_tools(
            model_text,
            &registry,
            &coordinator,
            &context,
            &self.approval_mode,
            turn,
        )
        .await
    }

    fn tool_context(&self) -> ToolContext {
        let workspace = self.active_workspace.clone().unwrap_or_else(|| {
            Workspace::new(
                "default",
                "Default workspace",
                self.workspace.clone(),
                ExecutorKind::LocalAndroid,
            )
        });
        ToolContext::new(workspace)
            .with_external_access(self.config.external_access.clone())
            .with_auto_approve(matches!(self.approval_mode, ApprovalMode::Never))
    }

    fn push_event(
        &self,
        events: &mut Vec<AgentEvent>,
        turn_id: Option<&str>,
        event: AgentEvent,
    ) -> Result<()> {
        if let Some(store) = self.runtime_store.as_ref() {
            store.append_event(
                self.thread_id.clone(),
                turn_id.map(std::string::ToString::to_string),
                event.clone(),
            )?;
        }
        events.push(event);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct EngineTurnResult {
    pub turn: TurnContext,
    pub events: Vec<AgentEvent>,
    pub final_text: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EngineApprovalContinuationResult {
    pub turn: TurnContext,
    pub events: Vec<AgentEvent>,
    pub executed: Vec<crate::tool_loop::ToolLoopExecutionRecord>,
}

fn title_from_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        "New mobile thread".to_string()
    } else {
        let mut title = trimmed.chars().take(80).collect::<String>();
        if trimmed.chars().count() > 80 {
            title.push('…');
        }
        title
    }
}
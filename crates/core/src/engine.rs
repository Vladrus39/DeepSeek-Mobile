//! Mobile engine orchestration.
//!
//! The engine owns one agent turn: it emits timeline events, stores turn state,
//! parses tool calls from the model response, saves pending approvals and can
//! later continue a stored approval after the mobile UI sends a decision.
//!
//! Sessions maintain full conversation history so the model has context across turns.

use crate::agent::DeepSeekAgent;
use crate::api_client::{format_http_transport_error, Message, StreamDelta};
use crate::approval::ReviewDecision;
use crate::approval_card::{approval_cards_from_records, ApprovalCardView};
use crate::approval_session::ApprovalSessionPolicy;
use crate::auto_commit::auto_commit_and_push;
use crate::config::{Config, ExternalAccessMode};
use crate::context::{estimate_messages_tokens, ContextBudget, ContextManager};
use crate::events::AgentEvent;
use crate::large_output::{
    format_tool_results_message, route_tool_result_for_model, DEFAULT_MAX_TOOL_RESULT_CHARS,
};
use crate::model_router::ModelRouter;
use crate::pc_gateway_client::PcGatewayClient;
use crate::runtime_store::{RuntimeThreadStore, TurnRecord};
use crate::session::{Session, SessionDiagnosticsContext};
use crate::tool_loop::{
    continue_pending_tool_approval_with_session_and_pc_gateway,
    process_model_text_with_tools_and_session_and_pc_gateway, ToolLoopExecutionRecord,
    ToolLoopOutcome,
};

/// Maximum model↔tool round-trips per user turn (TUI-style agent loop).
const MAX_TOOL_FOLLOWUP_ITERATIONS: usize = 8;
use crate::mcp::McpToolDescriptor;
use crate::tools::{default_mobile_tool_registry_with_mcp, ToolContext, ToolRegistry};
use crate::turn::{TurnContext, TurnStatus};
use crate::workspace::{ExecutorKind, Workspace};
use crate::workspace_connection::WorkspaceConnection;
use anyhow::{anyhow, Result};
use serde_json::Value;
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
    pub final_text: Option<String>,
    pub approval_cards: Vec<ApprovalCardView>,
    pub executed: Vec<ToolLoopExecutionRecord>,
    pub session_grants_created: Vec<crate::approval_session::ApprovalSessionGrant>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnginePendingApprovalSnapshot {
    pub cards: Vec<ApprovalCardView>,
}

pub struct MobileEngine {
    agent: DeepSeekAgent,
    config: Config,
    registry: ToolRegistry,
    execution_mode: crate::config::ExecutionMode,
    external_access: ExternalAccessMode,
    trusted_external_paths: Vec<PathBuf>,
    github_token: Option<String>,
    runtime_store: Option<RuntimeThreadStore>,
    thread_id: String,
    workspace: Workspace,
    pc_gateway: Option<PcGatewayClient>,
    approval_session: ApprovalSessionPolicy,
    event_observer: Option<Arc<dyn Fn(AgentEvent)>>,
    /// Active session with full conversation history
    session: Session,
    /// Auto-create workspace snapshot after each successful turn with changes
    auto_snapshot: bool,
    /// Optional skills context injected into the system prompt.
    skills_context: Option<String>,
}

impl MobileEngine {
    pub fn new(config: Config) -> Self {
        let execution_mode = config.execution_mode.clone();
        let external_access = config.external_access.clone();
        let trusted_external_paths = config
            .trusted_external_paths
            .iter()
            .map(PathBuf::from)
            .collect();
        let github_token = config.github_token.clone();
        Self {
            agent: DeepSeekAgent::new(config.clone()),
            config,
            registry: default_mobile_tool_registry_with_mcp(&[]),
            execution_mode,
            external_access,
            trusted_external_paths,
            github_token,
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
            session: Session::new("default"),
            auto_snapshot: true,
            skills_context: None,
        }
    }

    pub fn with_skills_context(mut self, skills_context: Option<String>) -> Self {
        self.skills_context = skills_context.filter(|text| !text.trim().is_empty());
        self
    }

    pub fn with_mcp_tools(mut self, descriptors: &[McpToolDescriptor]) -> Self {
        self.registry = default_mobile_tool_registry_with_mcp(descriptors);
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

    pub fn with_session(mut self, session: Session) -> Self {
        self.session = session;
        self
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
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

    pub fn with_auto_snapshot(mut self, enabled: bool) -> Self {
        self.auto_snapshot = enabled;
        self
    }

    pub fn auto_snapshot_enabled(&self) -> bool {
        self.auto_snapshot
    }

    pub fn approval_session_grant_count(&self) -> usize {
        self.approval_session.grant_count()
    }

    pub async fn run_turn_with_streaming(
        &mut self,
        user_input: String,
    ) -> Result<EngineTurnResult> {
        self.run_turn_internal(user_input, true).await
    }

    pub async fn run_turn(&mut self, user_input: String) -> Result<EngineTurnResult> {
        self.run_turn_internal(user_input, false).await
    }

    async fn run_turn_internal(
        &mut self,
        user_input: String,
        streaming: bool,
    ) -> Result<EngineTurnResult> {
        let mut turn = TurnContext::new(100);
        turn.start();
        self.persist_turn(&turn)?;

        // --- Model routing ---
        // Select the best model for this task before building the prompt.
        let estimated_tokens =
            estimate_messages_tokens(&self.session.messages) + user_input.chars().count() / 4 + 500;
        let router = ModelRouter::new(self.config.clone());
        let route = router.route_prompt(user_input.clone(), estimated_tokens);
        let model_name = route.model.clone();

        tracing::info!(
            "ModelRouter: selected {} (thinking={:?}) for ~{} tokens — {}",
            model_name,
            route.thinking_level,
            estimated_tokens,
            route.reason,
        );

        // Add user message to session history
        self.session.push_message("user", &user_input);

        let mut events = Vec::new();
        self.push_event(&mut events, AgentEvent::Started)?;
        self.push_event(
            &mut events,
            AgentEvent::TurnStarted {
                turn_id: turn.id.clone(),
            },
        )?;
        self.push_event(
            &mut events,
            AgentEvent::Status(format!(
                "MobileEngine {} turn started (model: {})",
                if streaming {
                    "streaming"
                } else {
                    "non-streaming"
                },
                model_name,
            )),
        )?;

        let answer = match self
            .collect_model_answer(&user_input, &mut events, streaming, &model_name)
            .await
        {
            Ok(answer) => answer,
            Err(error) => {
                let formatted_error = format_http_transport_error(&error);
                turn.fail(formatted_error.clone());
                self.persist_turn(&turn)?;
                self.push_event(&mut events, AgentEvent::Error(formatted_error.clone()))?;
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
                    final_text: Some(formatted_error),
                    approval_cards: Vec::new(),
                    executed: Vec::new(),
                });
            }
        };

        // Add assistant answer to session history
        self.session.push_message("assistant", &answer);

        let context = self.tool_context();
        let outcome = process_model_text_with_tools_and_session_and_pc_gateway(
            answer.clone(),
            &self.registry,
            &context,
            &mut turn,
            &mut self.approval_session,
            self.pc_gateway.as_ref(),
            self.execution_mode.clone(),
        )
        .await?;

        for event in outcome.events.iter().cloned() {
            self.push_event(&mut events, event)?;
        }

        let (outcome, answer) = self
            .run_tool_followup_loop(
                answer,
                outcome,
                &mut turn,
                &mut events,
                streaming,
                &model_name,
            )
            .await?;

        // Detect pending Termux requests before capturing diagnostics
        let has_pending_termux = !outcome.pending_termux_requests.is_empty();
        if has_pending_termux {
            self.store_pending_termux(&turn, &outcome)?;
        }

        self.capture_latest_diagnostics(&outcome.executed);

        self.store_pending_approvals(&turn, &outcome)?;

        if outcome.has_pending_approvals() {
            turn.status = TurnStatus::WaitingForApproval;
        } else if has_pending_termux {
            turn.wait_for_termux();
        } else {
            turn.complete();
        }

        // Auto-create post-turn snapshot if enabled and tools were executed
        if self.auto_snapshot
            && !outcome.executed.is_empty()
            && turn.status == TurnStatus::Completed
        {
            // PC gateway snapshot path
            if let Some(ref client) = self.pc_gateway {
                let _ = client
                    .create_snapshot(
                        &self.workspace.id,
                        &format!(
                            "post-turn auto snapshot after {} tools",
                            outcome.executed.len()
                        ),
                    )
                    .await;
            } else {
                let store_root = self
                    .workspace
                    .root
                    .join(".deepseek-mobile")
                    .join("snapshots");
                let service = crate::snapshots::WorkspaceSnapshotService::new(
                    self.workspace.clone(),
                    store_root,
                );
                match service.create_snapshot(format!(
                    "post-turn auto snapshot after {} tools",
                    outcome.executed.len()
                )) {
                    Ok(snapshot) => {
                        self.push_event(
                            &mut events,
                            AgentEvent::Status(format!(
                                "Auto-snapshot created: {} files, {} bytes",
                                snapshot.file_count, snapshot.total_bytes,
                            )),
                        )?;
                    }
                    Err(error) => {
                        // Non-fatal: log but don't fail the turn
                        self.push_event(
                            &mut events,
                            AgentEvent::Status(format!("Auto-snapshot skipped: {}", error)),
                        )?;
                    }
                }
            } // close pc_gateway else block
        }

        // --- Auto-commit/push ---
        if self.config.auto_commit_push
            && turn.status == TurnStatus::Completed
            && !outcome.executed.is_empty()
        {
            if let Some(repo) = self.config.github_repo.as_deref() {
                let branch = self.config.github_branch.as_deref().unwrap_or("main");
                let commit_msg = crate::auto_commit::commit_message_from_input(&user_input);
                match auto_commit_and_push(&self.workspace.root, repo, branch, &commit_msg) {
                    Ok(Some(sha)) => {
                        self.push_event(
                            &mut events,
                            AgentEvent::Status(format!(
                                "Auto-committed & pushed {}: {}",
                                &sha[..sha.len().min(8)],
                                commit_msg,
                            )),
                        )?;
                    }
                    Ok(None) => {
                        self.push_event(
                            &mut events,
                            AgentEvent::Status(
                                "Auto-commit skipped: no changes detected".to_string(),
                            ),
                        )?;
                    }
                    Err(error) => {
                        // Non-fatal: log warning but don't fail the turn
                        self.push_event(
                            &mut events,
                            AgentEvent::Status(format!("Auto-commit warning: {}", error)),
                        )?;
                    }
                }
            }
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
            final_text: if has_pending_termux {
                Some("Ожидаю реальный результат Termux…".to_string())
            } else {
                outcome.final_text.or(Some(answer))
            },
            approval_cards: outcome.approval_cards,
            executed: outcome.executed,
        })
    }

    async fn collect_model_answer(
        &self,
        user_input: &str,
        events: &mut Vec<AgentEvent>,
        streaming: bool,
        model_name: &str,
    ) -> Result<String> {
        if !streaming {
            // Use full session history for non-streaming
            let messages = self.build_messages_for_turn(user_input, model_name);
            let answer = self
                .agent
                .run_with_messages_and_model(model_name, messages)
                .await?;
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
        self.push_event(
            events,
            AgentEvent::Status("DeepSeek streaming response opened".to_string()),
        )?;

        // Use full session history for streaming
        let messages = self.build_messages_for_turn(user_input, model_name);
        let mut receiver = self
            .agent
            .run_stream_with_messages_and_model(model_name, messages)
            .await?;
        let mut answer = String::new();
        let mut reasoning_buffer = String::new();

        while let Some(delta) = receiver.recv().await {
            match delta {
                StreamDelta::Text(text) => {
                    answer.push_str(&text);
                    self.push_event(events, AgentEvent::TextDelta(text))?;
                }
                StreamDelta::Reasoning(reasoning) => {
                    reasoning_buffer.push_str(&reasoning);
                    self.push_event(events, AgentEvent::ReasoningDelta(reasoning))?;
                }
                StreamDelta::Done => {
                    break;
                }
            }
        }

        // Emit full reasoning as a status for the timeline
        if !reasoning_buffer.is_empty() {
            self.push_event(
                events,
                AgentEvent::Status(format!(
                    "Reasoning completed ({} chars)",
                    reasoning_buffer.len()
                )),
            )?;
        }

        self.push_event(events, AgentEvent::MessageFinished { index: 0 })?;
        self.push_event(
            events,
            AgentEvent::Status("DeepSeek streaming response completed".to_string()),
        )?;
        Ok(answer)
    }

    /// Build the messages array for a turn: system prompt + full conversation history.
    /// Applies ContextManager fitting based on the selected model's context budget.
    fn build_messages_for_turn(&self, _user_input: &str, model_name: &str) -> Vec<Message> {
        // Determine context budget from the selected model
        let max_tokens = if model_name.contains("pro") {
            1_000_000 // V4 Pro has 1M context
        } else {
            128_000 // V4 Flash has 128K context
        };

        let budget = ContextBudget::new(max_tokens);
        let usable_tokens = budget.usable_input_tokens();
        let context_manager = ContextManager::with_budget(budget);

        // Start with system prompt
        let mut messages = vec![Message {
            role: "system".to_string(),
            content: "You are a helpful coding assistant with access to tools for \
                      reading, writing, editing files, running shell commands, \
                      git, snapshots, MCP tools, workspace_overview, and file_summary. You run \
                      inside DeepSeek-Mobile on Android: the primary execution \
                      surface is the user's Termux workspace (full shell/build). \
                      The app sandbox is for light file edits when Termux is not \
                      configured. An optional PC Host can offload very large repos \
                      or desktop-only toolchains — use it only when the user has \
                      paired it. Prefer workspace_overview before wide refactors. \
                      Large command output may be spilled to `.deepseek-mobile/tool-output/`; \
                      use read_file to inspect the full file.\n\n\
                      Tool-call contract: when you need current filesystem, shell, git, \
                      workspace, MCP, or phone-native state, do not guess or describe a \
                      command in prose. Emit exactly one valid JSON object and no prose around \
                      it. Supported forms are \
                      {\"tool\":\"exec_shell\",\"args\":{\"command\":\"pwd\",\"timeout_secs\":30}} \
                      or {\"tool_calls\":[{\"tool\":\"read_file\",\"args\":{\"path\":\"README.md\"}}]}. \
                      For any user request to create, edit, delete, list, inspect, run, test, \
                      build, or verify files in the project, call a tool first. In the Termux \
                      workspace, file tools and exec_shell are real native Termux actions; do \
                      not claim a file was created until the real tool result is returned. \
                      After the app returns tool results, answer normally. Never fabricate \
                      tool output; if a real value is needed, call a tool first."
                .to_string(),
        }];

        // Inject model routing hint
        messages.push(Message {
            role: "system".to_string(),
            content: format!(
                "You are running on DeepSeek model `{}`. Adapt your response length \
                 and detail level accordingly.",
                model_name,
            ),
        });

        if let Some(diagnostics) = self.session.latest_diagnostics.as_ref() {
            messages.push(Message {
                role: "system".to_string(),
                content: format!(
                    "Latest post-edit diagnostics from the previous tool execution:\n{}",
                    format_session_diagnostics_context(diagnostics)
                ),
            });
        }

        if let Some(skills) = self.skills_context.as_ref() {
            messages.push(Message {
                role: "system".to_string(),
                content: skills.clone(),
            });
        }

        // Append full session history (already includes the latest user message
        // since it was pushed before collect_model_answer is called)
        messages.extend(self.session.messages.clone());

        // Apply context fitting to stay within budget
        let fitted = context_manager.fit_messages(&messages);
        tracing::info!(
            "ContextManager: {}/{} messages selected, ~{} tokens (budget: {} usable)",
            fitted.len(),
            messages.len(),
            estimate_messages_tokens(&fitted),
            usable_tokens,
        );

        fitted
    }

    pub async fn continue_stored_approval(
        &mut self,
        approval_id: &str,
        decision: ReviewDecision,
        turn: TurnContext,
    ) -> Result<EngineApprovalContinuationResult> {
        self.continue_stored_approval_with_streaming(approval_id, decision, turn, true)
            .await
    }

    pub async fn continue_stored_approval_with_streaming(
        &mut self,
        approval_id: &str,
        decision: ReviewDecision,
        mut turn: TurnContext,
        streaming: bool,
    ) -> Result<EngineApprovalContinuationResult> {
        let Some(store) = self.runtime_store.clone() else {
            anyhow::bail!("runtime store is required to continue stored approval");
        };
        let pending_record = store.load_pending_approval(approval_id)?;
        let tool_name = pending_record.pending.call.name.clone();
        let mut events = Vec::new();
        self.push_event(
            &mut events,
            AgentEvent::Status(format!(
                "Continuing turn after approval decision: {:?}",
                decision
            )),
        )?;

        let context = self.tool_context();
        let mut outcome = continue_pending_tool_approval_with_session_and_pc_gateway(
            pending_record.pending,
            decision.clone(),
            &self.registry,
            &context,
            &mut turn,
            &mut self.approval_session,
            self.pc_gateway.as_ref(),
        )
        .await?;

        for event in outcome.events.iter().cloned() {
            self.push_event(&mut events, event)?;
        }

        let model_name = self.resolve_model_for_hint("approval continuation");
        let mut final_answer = self.last_assistant_message_from_session();

        match &decision {
            ReviewDecision::Denied => {
                self.session.push_message(
                    "user",
                    format!(
                        "[Approval denied for tool `{tool_name}`. Respond to the user without running it.]"
                    ),
                );
                final_answer = self
                    .collect_model_answer("approval denied", &mut events, streaming, &model_name)
                    .await?;
                self.session.push_message("assistant", &final_answer);
                turn.complete();
            }
            ReviewDecision::Abort => {
                turn.cancel();
            }
            ReviewDecision::Approved | ReviewDecision::ApprovedForSession => {
                if !outcome.pending_termux_requests.is_empty() {
                    self.store_pending_termux(&turn, &outcome)?;
                    turn.wait_for_termux();
                } else if !outcome.executed.is_empty() {
                    let (followup_outcome, followup_answer) = self
                        .run_tool_followup_loop(
                            final_answer,
                            outcome,
                            &mut turn,
                            &mut events,
                            streaming,
                            &model_name,
                        )
                        .await?;
                    outcome = followup_outcome;
                    final_answer = followup_answer;
                } else {
                    self.session.push_message(
                        "user",
                        format!(
                            "[Tool `{tool_name}` was approved but produced no output. Continue the turn.]"
                        ),
                    );
                    final_answer = self
                        .collect_model_answer(
                            "approval continued",
                            &mut events,
                            streaming,
                            &model_name,
                        )
                        .await?;
                    self.session.push_message("assistant", &final_answer);
                }
            }
        }

        self.capture_latest_diagnostics(&outcome.executed);
        self.store_pending_approvals(&turn, &outcome)?;

        let has_pending_termux = !outcome.pending_termux_requests.is_empty();
        if outcome.has_pending_approvals() {
            turn.status = TurnStatus::WaitingForApproval;
        } else if has_pending_termux {
            turn.wait_for_termux();
        } else if turn.status != TurnStatus::Cancelled {
            turn.complete();
        }

        store.save_decision(&crate::tool_loop::decision_record(
            pending_record.thread_id.clone(),
            pending_record.turn_id.clone(),
            approval_id.to_string(),
            &decision,
        ))?;
        store.remove_pending_approval(approval_id)?;
        self.persist_turn(&turn)?;

        let approval_cards = self.pending_approval_cards_for_current_thread()?;

        self.push_event(
            &mut events,
            AgentEvent::TurnFinished {
                turn_id: turn.id.clone(),
                status: turn.status.clone(),
                usage: turn.usage.clone(),
                error: turn.error.clone(),
            },
        )?;

        Ok(EngineApprovalContinuationResult {
            events,
            final_text: if has_pending_termux {
                Some("Ожидаю реальный результат Termux…".to_string())
            } else {
                outcome.final_text.or(Some(final_answer))
            },
            approval_cards,
            executed: outcome.executed,
            session_grants_created: outcome.session_grants_created,
        })
    }

    fn resolve_model_for_hint(&self, hint: &str) -> String {
        let estimated_tokens =
            estimate_messages_tokens(&self.session.messages) + hint.chars().count() / 4 + 500;
        let router = ModelRouter::new(self.config.clone());
        router
            .route_prompt(hint.to_string(), estimated_tokens)
            .model
    }

    fn last_assistant_message_from_session(&self) -> String {
        self.session
            .messages
            .iter()
            .rev()
            .find(|message| message.role == "assistant")
            .map(|message| message.content.clone())
            .unwrap_or_default()
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

    fn store_pending_termux(&self, turn: &TurnContext, outcome: &ToolLoopOutcome) -> Result<()> {
        if let Some(store) = &self.runtime_store {
            for pending in outcome.pending_termux_requests.iter().cloned() {
                let record = crate::runtime_store::PendingTermuxRecord {
                    request_id: pending.request.request_id.clone(),
                    thread_id: self.thread_id.clone(),
                    turn_id: turn.id.clone(),
                    call_id: pending.call_id,
                    tool_name: pending.tool_name,
                    request: pending.request,
                    created_at_unix: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or_default(),
                };
                store.save_pending_termux(&record)?;
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

    /// Continue a turn that was paused waiting for a Termux command result.
    /// The real tool output is injected into the session and the model is
    /// re-queried so it can respond to the actual command output.
    pub async fn continue_termux_result(
        &mut self,
        termux_result: crate::executor::TermuxExecResult,
    ) -> Result<EngineTurnResult> {
        let Some(store) = self.runtime_store.clone() else {
            anyhow::bail!("runtime store is required to continue termux result");
        };

        // Load the pending Termux record
        let pending = store.load_pending_termux(&termux_result.request_id)?;
        let mut turn = TurnContext::new(100);
        let turn_record = store.load_turn(&pending.turn_id)?;
        turn.id = turn_record.id.clone();
        turn.status = TurnStatus::WaitingForTermuxResult;
        turn.created_at_unix = turn_record.created_at_unix;
        turn.updated_at_unix = turn_record.updated_at_unix;
        turn.usage = crate::turn::TokenUsage {
            input_tokens: turn_record.usage_input_tokens,
            output_tokens: turn_record.usage_output_tokens,
            reasoning_tokens: turn_record.usage_reasoning_tokens,
        };
        turn.error = turn_record.error.clone();

        let mut events = Vec::new();

        // Build tool result message and add to session
        let tool_result_content = format_termux_tool_result(&pending.tool_name, &termux_result);
        self.session.push_message("system", &tool_result_content);

        // Re-query the model with the full session including the real tool result
        let prompt = format!(
            "The {} command completed. Analyze the result above and respond.",
            pending.tool_name
        );
        self.session.push_message("user", &prompt);

        // Route model
        let estimated_tokens =
            crate::context::estimate_messages_tokens(&self.session.messages) + 500;
        let router = crate::model_router::ModelRouter::new(self.config.clone());
        let route = router.route_prompt(prompt.clone(), estimated_tokens);
        let model_name = route.model.clone();

        let answer = self
            .collect_model_answer(&prompt, &mut events, true, &model_name)
            .await?;

        // Add assistant response to session
        self.session.push_message("assistant", &answer);

        // Process any tool calls in the response
        let context = self.tool_context();
        let outcome = process_model_text_with_tools_and_session_and_pc_gateway(
            answer.clone(),
            &self.registry,
            &context,
            &mut turn,
            &mut self.approval_session,
            self.pc_gateway.as_ref(),
            self.execution_mode.clone(),
        )
        .await?;

        for event in outcome.events.iter().cloned() {
            self.push_event(&mut events, event)?;
        }

        let (outcome, answer) = self
            .run_tool_followup_loop(answer, outcome, &mut turn, &mut events, true, &model_name)
            .await?;

        self.capture_latest_diagnostics(&outcome.executed);
        self.store_pending_approvals(&turn, &outcome)?;

        // Check for nested pending termux
        let has_pending_termux = !outcome.pending_termux_requests.is_empty();
        if has_pending_termux {
            self.store_pending_termux(&turn, &outcome)?;
        }

        if outcome.has_pending_approvals() {
            turn.status = TurnStatus::WaitingForApproval;
        } else if has_pending_termux {
            turn.wait_for_termux();
        } else {
            turn.complete();
        }

        // Clean up the consumed Termux pending record
        store.remove_pending_termux(&termux_result.request_id)?;

        // Auto-snapshot if enabled and turn completed
        if self.auto_snapshot
            && !outcome.executed.is_empty()
            && turn.status == TurnStatus::Completed
        {
            let store_root = self
                .workspace
                .root
                .join(".deepseek-mobile")
                .join("snapshots");
            let service =
                crate::snapshots::WorkspaceSnapshotService::new(self.workspace.clone(), store_root);
            if let Ok(snapshot) = service.create_snapshot(format!(
                "post-termux-continuation snapshot after {} tools",
                outcome.executed.len()
            )) {
                let _ = self.push_event(
                    &mut events,
                    AgentEvent::Status(format!(
                        "Auto-snapshot created: {} files, {} bytes",
                        snapshot.file_count, snapshot.total_bytes,
                    )),
                );
            }
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

    async fn run_tool_followup_loop(
        &mut self,
        mut last_assistant_text: String,
        mut outcome: ToolLoopOutcome,
        turn: &mut TurnContext,
        events: &mut Vec<AgentEvent>,
        streaming: bool,
        model_name: &str,
    ) -> Result<(ToolLoopOutcome, String)> {
        let mut merged_executed = outcome.executed.clone();
        let mut followup_round = 0usize;

        loop {
            if outcome.executed.is_empty() {
                break;
            }

            self.inject_tool_results_into_session(&outcome.executed)?;

            if outcome.has_pending_approvals() || !outcome.pending_termux_requests.is_empty() {
                break;
            }

            if followup_round >= MAX_TOOL_FOLLOWUP_ITERATIONS {
                self.push_event(
                    events,
                    AgentEvent::Status(format!(
                        "Tool follow-up stopped after {MAX_TOOL_FOLLOWUP_ITERATIONS} rounds (safety cap)"
                    )),
                )?;
                break;
            }
            followup_round += 1;

            let prompt = "Continue based on the tool results above. Call more tools if needed; otherwise answer the user.".to_string();
            self.session.push_message("user", &prompt);

            let followup_answer = self
                .collect_model_answer(&prompt, events, streaming, model_name)
                .await?;
            self.session.push_message("assistant", &followup_answer);
            last_assistant_text = followup_answer.clone();

            let context = self.tool_context();
            outcome = process_model_text_with_tools_and_session_and_pc_gateway(
                followup_answer,
                &self.registry,
                &context,
                turn,
                &mut self.approval_session,
                self.pc_gateway.as_ref(),
                self.execution_mode.clone(),
            )
            .await?;

            merged_executed.extend(outcome.executed.clone());
            for event in outcome.events.iter().cloned() {
                self.push_event(events, event)?;
            }
        }

        outcome.executed = merged_executed;
        Ok((outcome, last_assistant_text))
    }

    fn inject_tool_results_into_session(
        &mut self,
        executed: &[ToolLoopExecutionRecord],
    ) -> Result<()> {
        for record in executed {
            let Some(result) = record.result.as_ref() else {
                continue;
            };
            let routed = route_tool_result_for_model(
                &record.tool_name,
                result,
                &self.workspace,
                DEFAULT_MAX_TOOL_RESULT_CHARS,
            )?;
            let message = format_tool_results_message(&record.tool_name, &routed);
            self.session.push_message("user", &message);
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

    fn tool_context(&self) -> ToolContext {
        let mcp_registry_path = std::env::var("DEEPSEEK_MOBILE_DATA_DIR")
            .ok()
            .map(|dir| std::path::PathBuf::from(dir).join("mcp.json"))
            .or(Some(crate::mcp_client::default_mcp_path()));
        ToolContext::new(self.workspace.clone())
            .with_external_access(self.external_access.clone())
            .with_trusted_external_paths(self.trusted_external_paths.clone())
            .with_github_token(self.github_token.clone())
            .with_mcp_registry_path(mcp_registry_path)
    }

    fn capture_latest_diagnostics(&mut self, records: &[ToolLoopExecutionRecord]) {
        let latest = records
            .iter()
            .rev()
            .filter_map(|record| record.result.as_ref())
            .filter_map(|result| result.metadata.as_ref())
            .find_map(diagnostics_context_from_metadata);
        if let Some(diagnostics) = latest {
            self.session.set_latest_diagnostics(diagnostics);
        }
    }
}

fn diagnostics_context_from_metadata(metadata: &Value) -> Option<SessionDiagnosticsContext> {
    let summary = metadata
        .get("post_edit_diagnostics_summary")
        .and_then(Value::as_str)?
        .to_string();
    Some(SessionDiagnosticsContext {
        summary,
        diagnostics: metadata
            .get("post_edit_diagnostics")
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or_default(),
        path: metadata
            .get("post_edit_diagnostics_path")
            .and_then(Value::as_str)
            .map(str::to_string),
        provider: metadata
            .get("post_edit_diagnostics_provider")
            .and_then(Value::as_str)
            .map(str::to_string),
        status: metadata
            .get("post_edit_diagnostics_status")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

fn format_session_diagnostics_context(diagnostics: &SessionDiagnosticsContext) -> String {
    let mut lines = vec![diagnostics.summary.clone()];
    if let Some(provider) = diagnostics.provider.as_deref() {
        lines.push(format!("provider: {}", provider));
    }
    if let Some(status) = diagnostics.status.as_deref() {
        lines.push(format!("status: {}", status));
    }
    if let Some(path) = diagnostics.path.as_deref() {
        lines.push(format!("path: {}", path));
    }
    for item in diagnostics.diagnostics.iter().take(8) {
        lines.push(format!(
            "- {}:{}:{} [{:?}] {}{}",
            item.path,
            item.line,
            item.column,
            item.severity,
            item.message,
            item.source
                .as_deref()
                .map(|source| format!(" ({})", source))
                .unwrap_or_default()
        ));
    }
    if diagnostics.diagnostics.len() > 8 {
        lines.push(format!(
            "- ... {} more diagnostic(s)",
            diagnostics.diagnostics.len() - 8
        ));
    }
    lines.join("\n")
}

/// Build a human-readable tool result message from a Termux execution result.
fn format_termux_tool_result(
    tool_name: &str,
    result: &crate::executor::TermuxExecResult,
) -> String {
    let mut content = format!(
        "The `{}` command completed with exit code {}.",
        tool_name,
        result
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    if result.timed_out {
        content.push_str("\nThe command timed out.");
    }
    if let Some(error) = result.error.as_ref() {
        content.push_str(&format!("\nError: {}", error));
    }
    content
}

#[cfg(test)]
mod tests {
    use super::{diagnostics_context_from_metadata, MobileEngine};
    use crate::config::Config;
    use crate::pc_gateway::{PcDiagnostic, PcDiagnosticSeverity, PcGatewayConfig};
    use crate::session::SessionDiagnosticsContext;
    use crate::workspace::ExecutorKind;
    use crate::workspace_connection::WorkspaceConnection;

    #[test]
    fn engine_reports_streaming_support() {
        let engine = MobileEngine::new(Config::default());
        assert!(engine.supports_streaming());
    }

    #[test]
    fn engine_can_be_configured_from_pc_workspace_connection() {
        let mut gateway =
            PcGatewayConfig::new("pc-1", "Laptop", "http://127.0.0.1:8787", "phone-1");
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

    #[test]
    fn engine_has_default_session() {
        let engine = MobileEngine::new(Config::default());
        assert_eq!(engine.session().id, "default");
        assert!(engine.session().messages.is_empty());
    }

    #[test]
    fn engine_session_persists_messages() {
        let mut engine = MobileEngine::new(Config::default());
        engine.session_mut().push_message("user", "hello");
        engine.session_mut().push_message("assistant", "hi there");
        assert_eq!(engine.session().messages.len(), 2);
    }

    #[test]
    fn engine_injects_latest_diagnostics_into_next_turn_context() {
        let mut engine = MobileEngine::new(Config::default());
        engine
            .session_mut()
            .set_latest_diagnostics(SessionDiagnosticsContext {
                summary: "1 diagnostic(s): 1 error(s), 0 warning(s)".to_string(),
                diagnostics: vec![PcDiagnostic {
                    path: "src/main.rs".to_string(),
                    line: 7,
                    column: 3,
                    severity: PcDiagnosticSeverity::Error,
                    message: "expected expression".to_string(),
                    source: Some("cargo check".to_string()),
                }],
                path: Some("src/main.rs".to_string()),
                provider: Some("cargo check".to_string()),
                status: Some("Completed".to_string()),
            });
        let messages = engine.build_messages_for_turn("fix it", "deepseek-v4-flash");
        assert!(messages
            .iter()
            .any(|message| message.content.contains("Latest post-edit diagnostics")));
        assert!(messages.iter().any(|message| message
            .content
            .contains("1 diagnostic(s): 1 error(s), 0 warning(s)")));
        assert!(messages
            .iter()
            .any(|message| message.content.contains("expected expression")));
    }

    #[test]
    fn metadata_becomes_session_diagnostics_context() {
        let context = diagnostics_context_from_metadata(&serde_json::json!({
            "post_edit_diagnostics_summary": "1 diagnostic(s): 1 error(s), 0 warning(s)",
            "post_edit_diagnostics_path": "src/main.rs",
            "post_edit_diagnostics_provider": "cargo check",
            "post_edit_diagnostics_status": "Completed",
            "post_edit_diagnostics": [{
                "path": "src/main.rs",
                "line": 7,
                "column": 3,
                "severity": "Error",
                "message": "expected expression",
                "source": "cargo check"
            }]
        }))
        .unwrap();

        assert_eq!(context.path.as_deref(), Some("src/main.rs"));
        assert_eq!(context.provider.as_deref(), Some("cargo check"));
        assert_eq!(context.diagnostics.len(), 1);
    }

    #[test]
    fn build_messages_includes_skills_context() {
        let mut engine = MobileEngine::new(Config::default());
        engine.skills_context = Some("## Active Skills\n\n- demo: test\n\n".to_string());
        let messages = engine.build_messages_for_turn("hello", "deepseek-v4-flash");
        assert!(messages
            .iter()
            .any(|message| message.content.contains("Active Skills")));
    }

    #[test]
    fn build_messages_includes_json_tool_call_contract() {
        let engine = MobileEngine::new(Config::default());
        let messages = engine.build_messages_for_turn("pwd", "deepseek-v4-flash");
        let system = messages
            .iter()
            .find(|message| message.role == "system")
            .expect("system prompt");
        assert!(system.content.contains("Tool-call contract"));
        assert!(system.content.contains(r#""tool":"exec_shell""#));
        assert!(system.content.contains(r#""tool_calls""#));
        assert!(system.content.contains("Never fabricate tool output"));
    }

    #[test]
    fn build_messages_applies_context_manager() {
        let mut engine = MobileEngine::new(Config::default());
        // Push many messages to trigger context fitting
        for i in 0..200 {
            engine
                .session_mut()
                .push_message("user", &format!("message {}", i));
            engine
                .session_mut()
                .push_message("assistant", &format!("reply {}", i));
        }
        let messages = engine.build_messages_for_turn("new question", "deepseek-v4-flash");
        // Should fit within Flash 128K budget
        assert!(!messages.is_empty());
        // Should be less than full session + system messages
        assert!(messages.len() < engine.session().messages.len() + 5);
    }
}

//! Tool loop and approval continuation contract.
//!
//! This is the glue between parsed model tool calls, approval policy and actual
//! execution. The implementation is intentionally compact but functional enough
//! for the mobile runner: it can collect pending approvals, execute approved
//! calls, and continue a stored approval after the user decision.

use crate::approval::{approval_request_for_call, ApprovalMode, ReviewDecision};

fn execution_mode_to_approval_mode(mode: &crate::config::ExecutionMode) -> ApprovalMode {
    match mode {
        crate::config::ExecutionMode::Yolo => ApprovalMode::Auto,
        crate::config::ExecutionMode::Plan => ApprovalMode::AskEveryTime,
        crate::config::ExecutionMode::Agent => ApprovalMode::ReviewWritesAndCommands,
    }
}
use crate::approval_card::ApprovalCardView;
use crate::approval_session::{ApprovalSessionGrant, ApprovalSessionPolicy};
use crate::events::{AgentEvent, ToolCallEvent, ToolResultEvent};
use crate::pc_gateway::{PcGatewayResponse};
use crate::pc_gateway_client::PcGatewayClient;
use crate::runtime_store::ApprovalDecisionRecord;
use crate::snapshots::{WorkspaceSnapshotRecord, WorkspaceSnapshotService};
use crate::tool_call::{parse_tool_calls_from_text, ToolCallRequest};
use crate::tool_execution::ToolExecutionCoordinator;
use crate::tools::{ToolContext, ToolRegistry, ToolResult};
use crate::turn::{TurnContext, TurnToolCall};
use crate::workspace::ExecutorKind;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PendingToolCallApproval {
    pub approval: crate::approval::MobileApprovalRequest,
    pub call: ToolCallRequest,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolLoopExecutionRecord {
    pub call_id: String,
    pub tool_name: String,
    pub result: Option<ToolResult>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolLoopOutcome {
    pub final_text: Option<String>,
    pub events: Vec<AgentEvent>,
    pub executed: Vec<ToolLoopExecutionRecord>,
    pub pending_approvals: Vec<PendingToolCallApproval>,
    pub approval_cards: Vec<ApprovalCardView>,
    pub session_grants_created: Vec<ApprovalSessionGrant>,
    pub pending_termux_requests: Vec<TermuxPendingRequest>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TermuxPendingRequest {
    pub call_id: String,
    pub tool_name: String,
    pub request: crate::executor::TermuxExecRequest,
}

impl ToolLoopOutcome {
    pub fn has_pending_approvals(&self) -> bool {
        !self.pending_approvals.is_empty()
    }
}

pub async fn process_model_text_with_tools(
    text: String,
    registry: &ToolRegistry,
    context: &ToolContext,
    turn: &mut TurnContext,
) -> Result<ToolLoopOutcome> {
    process_model_text_with_tools_and_session(
        text,
        registry,
        context,
        turn,
        &mut ApprovalSessionPolicy::new(),
    )
    .await
}

pub async fn process_model_text_with_tools_and_session(
    text: String,
    registry: &ToolRegistry,
    context: &ToolContext,
    turn: &mut TurnContext,
    session: &mut ApprovalSessionPolicy,
) -> Result<ToolLoopOutcome> {
    process_model_text_with_tools_and_session_and_pc_gateway(
        text, registry, context, turn, session, None, crate::config::ExecutionMode::Agent,
    )
    .await
}

pub async fn process_model_text_with_tools_and_session_and_pc_gateway(
    text: String,
    registry: &ToolRegistry,
    context: &ToolContext,
    turn: &mut TurnContext,
    session: &mut ApprovalSessionPolicy,
    pc_gateway: Option<&PcGatewayClient>,
    execution_mode: crate::config::ExecutionMode,
) -> Result<ToolLoopOutcome> {
    let approval_mode = execution_mode_to_approval_mode(&execution_mode);
    let parsed = parse_tool_calls_from_text(&text);
    let mut outcome = ToolLoopOutcome {
        final_text: if parsed.final_text.trim().is_empty() {
            None
        } else {
            Some(parsed.final_text)
        },
        ..ToolLoopOutcome::default()
    };

    if execution_mode == crate::config::ExecutionMode::Plan {
        let skipped = parsed.tool_calls.len();
        if skipped > 0 {
            outcome.events.push(AgentEvent::Status(format!(
                "Plan mode: {skipped} tool call(s) parsed but not executed. Switch to Agent mode in Settings to run tools."
            )));
        }
        return Ok(outcome);
    }

    for call in parsed.tool_calls {
        let approval = approval_request_for_call(&call);
        if session.is_call_allowed_by_session(&approval, &call) || context.auto_approve {
            let result =
                execute_approved_call_with_pc_gateway(registry, context, turn, &call, pc_gateway)
                    .await;
            push_execution_result(&mut outcome, &call, result);
        } else if crate::approval::should_request_approval(
            &approval_mode,
            &approval,
        ) {
            let pending = PendingToolCallApproval { approval, call };
            outcome.events.push(AgentEvent::ApprovalRequired(
                crate::events::ApprovalRequest {
                    id: pending.approval.id.clone(),
                    title: pending.approval.tool_name.clone(),
                    description: pending.approval.description.clone(),
                    risk_level: match pending.approval.risk {
                        crate::approval::ApprovalRisk::Benign => crate::events::RiskLevel::Medium,
                        crate::approval::ApprovalRisk::Destructive => {
                            crate::events::RiskLevel::Dangerous
                        }
                    },
                },
            ));
            outcome
                .approval_cards
                .push(ApprovalCardView::from_approval_request(&pending.approval));
            outcome.pending_approvals.push(pending);
        } else {
            let result =
                execute_approved_call_with_pc_gateway(registry, context, turn, &call, pc_gateway)
                    .await;
            push_execution_result(&mut outcome, &call, result);
        }
    }

    Ok(outcome)
}

pub async fn continue_pending_tool_approval(
    pending: PendingToolCallApproval,
    decision: ReviewDecision,
    registry: &ToolRegistry,
    context: &ToolContext,
    turn: &mut TurnContext,
) -> Result<ToolLoopOutcome> {
    continue_pending_tool_approval_with_session(
        pending,
        decision,
        registry,
        context,
        turn,
        &mut ApprovalSessionPolicy::new(),
    )
    .await
}

pub async fn continue_pending_tool_approval_with_session(
    pending: PendingToolCallApproval,
    decision: ReviewDecision,
    registry: &ToolRegistry,
    context: &ToolContext,
    turn: &mut TurnContext,
    session: &mut ApprovalSessionPolicy,
) -> Result<ToolLoopOutcome> {
    continue_pending_tool_approval_with_session_and_pc_gateway(
        pending, decision, registry, context, turn, session, None,
    )
    .await
}

pub async fn continue_pending_tool_approval_with_session_and_pc_gateway(
    pending: PendingToolCallApproval,
    decision: ReviewDecision,
    registry: &ToolRegistry,
    context: &ToolContext,
    turn: &mut TurnContext,
    session: &mut ApprovalSessionPolicy,
    pc_gateway: Option<&PcGatewayClient>,
) -> Result<ToolLoopOutcome> {
    let mut outcome = ToolLoopOutcome::default();

    match decision {
        ReviewDecision::Approved => {
            let result = execute_approved_call_with_pc_gateway(
                registry,
                context,
                turn,
                &pending.call,
                pc_gateway,
            )
            .await;
            push_execution_result(&mut outcome, &pending.call, result);
        }
        ReviewDecision::ApprovedForSession => {
            if let Some(grant) = session.grant_for_approved_call(&pending.approval, &pending.call) {
                outcome.session_grants_created.push(grant);
            }
            let result = execute_approved_call_with_pc_gateway(
                registry,
                context,
                turn,
                &pending.call,
                pc_gateway,
            )
            .await;
            push_execution_result(&mut outcome, &pending.call, result);
        }
        ReviewDecision::Denied => {
            outcome.events.push(AgentEvent::Status(format!(
                "Approval denied for tool '{}'",
                pending.call.name
            )));
        }
        ReviewDecision::Abort => {
            turn.cancel();
            outcome.events.push(AgentEvent::Error(format!(
                "Turn aborted before running tool '{}'",
                pending.call.name
            )));
        }
    }

    Ok(outcome)
}

pub async fn execute_approved_call(
    registry: &ToolRegistry,
    context: &ToolContext,
    turn: &mut TurnContext,
    call: &ToolCallRequest,
) -> Result<ToolResult> {
    execute_approved_call_with_pc_gateway(registry, context, turn, call, None).await
}

pub async fn execute_approved_call_with_pc_gateway(
    registry: &ToolRegistry,
    context: &ToolContext,
    turn: &mut TurnContext,
    call: &ToolCallRequest,
    pc_gateway: Option<&PcGatewayClient>,
) -> Result<ToolResult> {
    turn.record_tool_call(TurnToolCall::new(
        &call.id,
        &call.name,
        call.arguments.clone(),
    ));
    let pre_snapshot = create_pre_tool_snapshot_if_needed(context, call, pc_gateway).await?;
    let mut coordinator = ToolExecutionCoordinator::new(registry);
    if let Some(client) = pc_gateway {
        coordinator = coordinator.with_pc_gateway(client);
    }
    let mut result = coordinator.execute(call, context).await?;
    if let Some(snapshot) = pre_snapshot {
        attach_pre_snapshot_metadata(&mut result, snapshot);
    }
    Ok(result)
}

pub fn decision_record(
    thread_id: impl Into<String>,
    turn_id: impl Into<String>,
    approval_id: impl Into<String>,
    decision: &ReviewDecision,
) -> ApprovalDecisionRecord {
    ApprovalDecisionRecord {
        approval_id: approval_id.into(),
        thread_id: thread_id.into(),
        turn_id: turn_id.into(),
        decision: format!("{:?}", decision),
        created_at_unix: current_unix_time(),
    }
}

async fn create_pre_tool_snapshot_if_needed(
    context: &ToolContext,
    call: &ToolCallRequest,
    pc_gateway: Option<&crate::pc_gateway_client::PcGatewayClient>,
) -> Result<Option<WorkspaceSnapshotRecord>> {
    if !should_create_pre_tool_snapshot(call) {
        return Ok(None);
    }

    // PC gateway snapshot path
    if context.workspace.executor == ExecutorKind::PcGateway {
        if let Some(client) = pc_gateway {
            let reason = format!("pre-tool snapshot before {} ({})", call.name, call.id);
            match client.create_snapshot(&context.workspace.id, &reason).await {
                Ok(PcGatewayResponse::SnapshotRecord(record)) => return Ok(Some(record)),
                Ok(_other) => return Ok(None), // unexpected response, skip
                Err(_) => return Ok(None), // gateway error, skip snapshot
            }
        }
        return Ok(None);
    }

    // Local snapshot path
    if !supports_local_snapshots(context) {
        return Ok(None);
    }

    let store_root = context
        .workspace
        .root
        .join(".deepseek-mobile")
        .join("snapshots");
    let service = WorkspaceSnapshotService::new(context.workspace.clone(), store_root);
    let snapshot = service.create_snapshot(format!(
        "pre-tool snapshot before {} ({})",
        call.name, call.id
    ))?;
    Ok(Some(snapshot))
}

fn supports_local_snapshots(context: &ToolContext) -> bool {
    matches!(
        context.workspace.executor,
        ExecutorKind::LocalAndroid | ExecutorKind::Termux
    )
}

fn should_create_pre_tool_snapshot(call: &ToolCallRequest) -> bool {
    match call.name.as_str() {
        "write_file" | "edit_file" | "apply_patch" | "delete_file" | "snapshot_restore" => true,
        "file_ops" => file_ops_may_modify(&call.arguments),
        "exec_shell" | "shell" | "run_command" | "terminal" => true,
        "git" => git_operation_may_modify(&call.arguments),
        "git_commit" | "git_push" | "git_pull" | "git_checkout" | "git_reset" => true,
        _ => false,
    }
}

fn file_ops_may_modify(arguments: &Value) -> bool {
    arguments
        .get("operation")
        .and_then(Value::as_str)
        .map(|operation| {
            matches!(
                operation,
                "write" | "write_file" | "edit" | "edit_file" | "delete" | "remove" | "rm"
            )
        })
        .unwrap_or(true)
}

fn git_operation_may_modify(arguments: &Value) -> bool {
    let operation = arguments
        .get("operation")
        .and_then(Value::as_str)
        .unwrap_or_default();
    !matches!(operation, "status" | "diff" | "log" | "show")
}

fn attach_pre_snapshot_metadata(result: &mut ToolResult, snapshot: WorkspaceSnapshotRecord) {
    let mut metadata = result.metadata.take().unwrap_or_else(|| json!({}));
    if let Value::Object(object) = &mut metadata {
        object.insert(
            "pre_snapshot".to_string(),
            serde_json::to_value(snapshot).unwrap_or_else(|_| json!({})),
        );
    }
    result.metadata = Some(metadata);
}

fn push_execution_result(
    outcome: &mut ToolLoopOutcome,
    call: &ToolCallRequest,
    result: Result<ToolResult>,
) {
    // Emit ToolCallStarted first
    outcome
        .events
        .push(AgentEvent::ToolCallStarted(ToolCallEvent {
            id: call.id.clone(),
            name: call.name.clone(),
            args: call.arguments.to_string(),
        }));

    match result {
        Ok(result) => {
            // Check if this is a Termux-pending result (queued for native bridge execution)
            let is_termux_pending = result
                .metadata
                .as_ref()
                .and_then(|m| m.get("termux_execution_pending"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if is_termux_pending {
                if let Some(request) = result
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("termux_exec_request"))
                    .and_then(|v| serde_json::from_value::<crate::executor::TermuxExecRequest>(v.clone()).ok())
                {
                    outcome.events.push(AgentEvent::TermuxExecutionPending {
                        call_id: call.id.clone(),
                        request: request.clone(),
                    });
                    outcome.pending_termux_requests.push(TermuxPendingRequest {
                        call_id: call.id.clone(),
                        tool_name: call.name.clone(),
                        request,
                    });
                    // Still emit ToolCallFinished so timeline shows the pending state
                }
            }

            // Always emit ToolCallFinished for the record
            outcome
                .events
                .push(AgentEvent::ToolCallFinished(ToolResultEvent {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    success: result.success,
                    output: result.content.clone(),
                    metadata: result.metadata.clone(),
                }));
            outcome.executed.push(ToolLoopExecutionRecord {
                call_id: call.id.clone(),
                tool_name: call.name.clone(),
                result: Some(result),
                error: None,
            });
        }
        Err(error) => {
            outcome
                .events
                .push(AgentEvent::ToolCallFinished(ToolResultEvent {
                    id: call.id.clone(),
                    name: call.name.clone(),
                    success: false,
                    output: error.to_string(),
                    metadata: None,
                }));
            outcome.executed.push(ToolLoopExecutionRecord {
                call_id: call.id.clone(),
                tool_name: call.name.clone(),
                result: None,
                error: Some(error.to_string()),
            });
        }
    }
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        should_create_pre_tool_snapshot, supports_local_snapshots, PendingToolCallApproval,
    };
    use crate::approval::{ApprovalRisk, MobileApprovalRequest, ToolCategory};
    use crate::tool_call::{ToolCallRequest, ToolCallSource};
    use crate::tools::ToolContext;
    use crate::workspace::{ExecutorKind, Workspace};
    use serde_json::json;

    #[test]
    fn pending_approval_is_serializable() {
        let pending = PendingToolCallApproval {
            approval: MobileApprovalRequest::new(
                "write_file",
                ToolCategory::FileWrite,
                ApprovalRisk::Benign,
                json!({"path":"README.md"}),
            ),
            call: ToolCallRequest::new(
                "write_file",
                json!({"path":"README.md"}),
                ToolCallSource::Manual,
            ),
        };
        let encoded = serde_json::to_string(&pending).expect("serialize pending approval");
        assert!(encoded.contains("write_file"));
    }

    #[test]
    fn destructive_tools_request_pre_snapshot() {
        assert!(should_create_pre_tool_snapshot(&ToolCallRequest::new(
            "write_file",
            json!({"path":"README.md","content":"x"}),
            ToolCallSource::Manual,
        )));
        assert!(should_create_pre_tool_snapshot(&ToolCallRequest::new(
            "apply_patch",
            json!({"operations":[]}),
            ToolCallSource::Manual,
        )));
        assert!(should_create_pre_tool_snapshot(&ToolCallRequest::new(
            "exec_shell",
            json!({"command":"cargo test"}),
            ToolCallSource::Manual,
        )));
    }

    #[test]
    fn read_only_tools_do_not_request_pre_snapshot() {
        assert!(!should_create_pre_tool_snapshot(&ToolCallRequest::new(
            "read_file",
            json!({"path":"README.md"}),
            ToolCallSource::Manual,
        )));
        assert!(!should_create_pre_tool_snapshot(&ToolCallRequest::new(
            "git",
            json!({"operation":"status"}),
            ToolCallSource::Manual,
        )));
        assert!(!should_create_pre_tool_snapshot(&ToolCallRequest::new(
            "git",
            json!({"operation":"diff"}),
            ToolCallSource::Manual,
        )));
    }

    #[tokio::test]
    async fn plan_mode_skips_tool_execution() {
        use super::process_model_text_with_tools_and_session_and_pc_gateway;
        use crate::approval_session::ApprovalSessionPolicy;
        use crate::events::AgentEvent;
        use crate::turn::TurnContext;

        let registry = crate::tools::default_mobile_tool_registry();
        let workspace = Workspace::new("w1", "Test", "/tmp/w", ExecutorKind::LocalAndroid);
        let context = ToolContext::new(workspace);
        let mut turn = TurnContext::new(8);
        let mut session = ApprovalSessionPolicy::new();
        let text = r#"{"tool":"read_file","args":{"path":"main.rs"}}"#;
        let outcome = process_model_text_with_tools_and_session_and_pc_gateway(
            text.to_string(),
            &registry,
            &context,
            &mut turn,
            &mut session,
            None,
            crate::config::ExecutionMode::Plan,
        )
        .await
        .unwrap();
        assert!(outcome.executed.is_empty());
        assert!(outcome.pending_approvals.is_empty());
        assert!(outcome
            .events
            .iter()
            .any(|event| matches!(event, AgentEvent::Status(_))));
    }

    #[test]
    fn snapshots_are_only_local_for_now() {
        let local = ToolContext::new(Workspace::new(
            "w1",
            "Local",
            "/tmp/local",
            ExecutorKind::LocalAndroid,
        ));
        let termux = ToolContext::new(Workspace::new(
            "w2",
            "Termux",
            "/tmp/termux",
            ExecutorKind::Termux,
        ));
        let pc = ToolContext::new(Workspace::new(
            "w3",
            "PC",
            "/tmp/pc",
            ExecutorKind::PcGateway,
        ));
        assert!(supports_local_snapshots(&local));
        assert!(supports_local_snapshots(&termux));
        assert!(!supports_local_snapshots(&pc));
    }
}

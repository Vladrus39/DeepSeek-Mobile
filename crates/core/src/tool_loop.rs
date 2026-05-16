//! Tool loop and approval continuation contract.
//!
//! This is the glue between parsed model tool calls, approval policy and actual
//! execution. The implementation is intentionally compact but functional enough
//! for the mobile runner: it can collect pending approvals, execute approved
//! calls, and continue a stored approval after the user decision.

use crate::approval::{approval_request_for_call, ReviewDecision};
use crate::approval_card::ApprovalCardView;
use crate::approval_session::{ApprovalSessionGrant, ApprovalSessionPolicy};
use crate::events::{AgentEvent, ToolCallEvent, ToolResultEvent};
use crate::runtime_store::ApprovalDecisionRecord;
use crate::tool_call::{parse_tool_calls_from_text, ToolCallRequest};
use crate::tool_execution::ToolExecutionCoordinator;
use crate::tools::{ToolContext, ToolRegistry, ToolResult};
use crate::turn::{TurnContext, TurnToolCall};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
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
    let parsed = parse_tool_calls_from_text(&text);
    let mut outcome = ToolLoopOutcome {
        final_text: if parsed.final_text.trim().is_empty() { None } else { Some(parsed.final_text) },
        ..ToolLoopOutcome::default()
    };

    for call in parsed.tool_calls {
        let approval = approval_request_for_call(&call);
        if session.is_call_allowed_by_session(&approval, &call) || context.auto_approve {
            let result = execute_approved_call(registry, context, turn, &call).await;
            push_execution_result(&mut outcome, &call, result);
        } else if crate::approval::should_request_approval(&crate::approval::ApprovalMode::ReviewWritesAndCommands, &approval) {
            let pending = PendingToolCallApproval { approval, call };
            outcome.events.push(AgentEvent::ApprovalRequired(crate::events::ApprovalRequest {
                id: pending.approval.id.clone(),
                title: pending.approval.tool_name.clone(),
                description: pending.approval.description.clone(),
                risk_level: match pending.approval.risk {
                    crate::approval::ApprovalRisk::Benign => crate::events::RiskLevel::Medium,
                    crate::approval::ApprovalRisk::Destructive => crate::events::RiskLevel::Dangerous,
                },
            }));
            outcome.approval_cards.push(ApprovalCardView::from_approval_request(&pending.approval));
            outcome.pending_approvals.push(pending);
        } else {
            let result = execute_approved_call(registry, context, turn, &call).await;
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
    let mut outcome = ToolLoopOutcome::default();

    match decision {
        ReviewDecision::Approved => {
            let result = execute_approved_call(registry, context, turn, &pending.call).await;
            push_execution_result(&mut outcome, &pending.call, result);
        }
        ReviewDecision::ApprovedForSession => {
            if let Some(grant) = session.grant_for_approved_call(&pending.approval, &pending.call) {
                outcome.session_grants_created.push(grant);
            }
            let result = execute_approved_call(registry, context, turn, &pending.call).await;
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
    turn.record_tool_call(TurnToolCall::new(&call.id, &call.name, call.arguments.clone()));
    let coordinator = ToolExecutionCoordinator::new(registry);
    coordinator.execute(call, context).await
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

fn push_execution_result(
    outcome: &mut ToolLoopOutcome,
    call: &ToolCallRequest,
    result: Result<ToolResult>,
) {
    outcome.events.push(AgentEvent::ToolCallStarted(ToolCallEvent {
        id: call.id.clone(),
        name: call.name.clone(),
        args: call.arguments.to_string(),
    }));

    match result {
        Ok(result) => {
            outcome.events.push(AgentEvent::ToolCallFinished(ToolResultEvent {
                id: call.id.clone(),
                name: call.name.clone(),
                success: result.success,
                output: result.content.clone(),
            }));
            outcome.executed.push(ToolLoopExecutionRecord {
                call_id: call.id.clone(),
                tool_name: call.name.clone(),
                result: Some(result),
                error: None,
            });
        }
        Err(error) => {
            outcome.events.push(AgentEvent::ToolCallFinished(ToolResultEvent {
                id: call.id.clone(),
                name: call.name.clone(),
                success: false,
                output: error.to_string(),
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
    use super::PendingToolCallApproval;
    use crate::approval::{MobileApprovalRequest, ApprovalRisk, ToolCategory};
    use crate::tool_call::{ToolCallRequest, ToolCallSource};
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
            call: ToolCallRequest::new("write_file", json!({"path":"README.md"}), ToolCallSource::Manual),
        };
        let encoded = serde_json::to_string(&pending).expect("serialize pending approval");
        assert!(encoded.contains("write_file"));
    }
}
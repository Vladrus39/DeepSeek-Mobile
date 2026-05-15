//! Tool loop orchestration.
//!
//! The loop connects model output to real actions. Android remains in charge of
//! parsing tool calls, applying approval policy, emitting timeline events and
//! recording turn state. The selected execution backend then decides where the
//! action actually runs: phone, Termux, PC gateway or remote runtime.

use crate::approval::{
    should_request_approval, ApprovalMode, ApprovalRisk, MobileApprovalRequest, ReviewDecision,
    ToolCategory,
};
use crate::events::{AgentEvent, ApprovalRequest, RiskLevel, ToolCallEvent, ToolResultEvent};
use crate::tool_call::{parse_tool_calls_from_text, ToolCallRequest};
use crate::tool_execution::ToolExecutionCoordinator;
use crate::tools::{ToolContext, ToolRegistry};
use crate::turn::{TurnContext, TurnStatus, TurnToolCall};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolLoopOutcome {
    pub final_text: String,
    pub events: Vec<AgentEvent>,
    pub pending_approvals: Vec<MobileApprovalRequest>,
    pub pending_tool_approvals: Vec<PendingToolCallApproval>,
    pub executed: Vec<ToolLoopExecutionRecord>,
    pub requires_user_input: bool,
}

impl ToolLoopOutcome {
    pub fn no_tools(final_text: impl Into<String>) -> Self {
        Self {
            final_text: final_text.into(),
            events: Vec::new(),
            pending_approvals: Vec::new(),
            pending_tool_approvals: Vec::new(),
            executed: Vec::new(),
            requires_user_input: false,
        }
    }

    pub fn empty() -> Self {
        Self::no_tools("")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PendingToolCallApproval {
    pub approval: MobileApprovalRequest,
    pub call: ToolCallRequest,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolLoopExecutionRecord {
    pub id: String,
    pub name: String,
    pub success: bool,
    pub output: String,
    pub metadata: Option<Value>,
}

pub async fn process_model_text_with_tools(
    model_text: &str,
    registry: &ToolRegistry,
    coordinator: &ToolExecutionCoordinator<'_>,
    context: &ToolContext,
    approval_mode: &ApprovalMode,
    turn: &mut TurnContext,
) -> Result<ToolLoopOutcome> {
    let parsed = parse_tool_calls_from_text(model_text);
    if !parsed.has_tool_calls() {
        return Ok(ToolLoopOutcome::no_tools(parsed.final_text));
    }

    let mut outcome = ToolLoopOutcome {
        final_text: parsed.final_text,
        events: Vec::new(),
        pending_approvals: Vec::new(),
        pending_tool_approvals: Vec::new(),
        executed: Vec::new(),
        requires_user_input: false,
    };

    for call in parsed.tool_calls {
        if !turn.next_step() {
            return Err(anyhow!("tool loop reached max step limit"));
        }

        let Some(tool) = registry.get(&call.name) else {
            let message = format!("tool '{}' is not registered", call.name);
            outcome.events.push(AgentEvent::Error(message.clone()));
            outcome.executed.push(ToolLoopExecutionRecord {
                id: call.id.clone(),
                name: call.name.clone(),
                success: false,
                output: message,
                metadata: None,
            });
            continue;
        };

        let approval = MobileApprovalRequest::new(call.id.clone(), tool, call.arguments.clone());
        if should_request_approval(approval_mode, &approval.requirement, &approval.risk) {
            turn.status = TurnStatus::WaitingForApproval;
            turn.record_tool_call(TurnToolCall::new(
                call.id.clone(),
                call.name.clone(),
                call.arguments.clone(),
            ));
            outcome.requires_user_input = true;
            outcome.events.push(AgentEvent::ApprovalRequired(to_agent_approval_request(&approval)));
            outcome.pending_approvals.push(approval.clone());
            outcome.pending_tool_approvals.push(PendingToolCallApproval { approval, call });
            continue;
        }

        execute_approved_call(&call, coordinator, context, turn, &mut outcome).await?;
    }

    Ok(outcome)
}

pub async fn continue_pending_tool_approval(
    pending: &PendingToolCallApproval,
    decision: &ReviewDecision,
    coordinator: &ToolExecutionCoordinator<'_>,
    context: &ToolContext,
    turn: &mut TurnContext,
) -> Result<ToolLoopOutcome> {
    let mut outcome = ToolLoopOutcome::empty();
    match decision {
        ReviewDecision::Approved | ReviewDecision::ApprovedForSession => {
            turn.status = TurnStatus::Running;
            outcome.events.push(AgentEvent::Status(format!(
                "Approval accepted for tool '{}'",
                pending.call.name
            )));
            execute_approved_call(&pending.call, coordinator, context, turn, &mut outcome).await?;
        }
        ReviewDecision::Denied => {
            let message = format!("Tool '{}' was denied by user", pending.call.name);
            let mut turn_call = TurnToolCall::new(
                pending.call.id.clone(),
                pending.call.name.clone(),
                pending.call.arguments.clone(),
            );
            turn_call.set_error(message.clone());
            turn.record_tool_call(turn_call);
            turn.status = TurnStatus::Running;
            outcome.events.push(AgentEvent::Status(message.clone()));
            outcome.executed.push(ToolLoopExecutionRecord {
                id: pending.call.id.clone(),
                name: pending.call.name.clone(),
                success: false,
                output: message,
                metadata: None,
            });
        }
        ReviewDecision::Abort => {
            let message = format!("Tool '{}' aborted by user", pending.call.name);
            let mut turn_call = TurnToolCall::new(
                pending.call.id.clone(),
                pending.call.name.clone(),
                pending.call.arguments.clone(),
            );
            turn_call.set_error(message.clone());
            turn.record_tool_call(turn_call);
            turn.cancel();
            outcome.events.push(AgentEvent::Error(message.clone()));
            outcome.executed.push(ToolLoopExecutionRecord {
                id: pending.call.id.clone(),
                name: pending.call.name.clone(),
                success: false,
                output: message,
                metadata: None,
            });
        }
    }
    Ok(outcome)
}

pub async fn execute_approved_call(
    call: &ToolCallRequest,
    coordinator: &ToolExecutionCoordinator<'_>,
    context: &ToolContext,
    turn: &mut TurnContext,
    outcome: &mut ToolLoopOutcome,
) -> Result<()> {
    let args_text = serde_json::to_string(&call.arguments)?;
    outcome.events.push(AgentEvent::ToolCallStarted(ToolCallEvent {
        id: call.id.clone(),
        name: call.name.clone(),
        args: args_text,
    }));

    let mut turn_call = TurnToolCall::new(call.id.clone(), call.name.clone(), call.arguments.clone());
    match coordinator.execute(call, context).await {
        Ok(result) => {
            turn_call.set_result(result.content.clone());
            turn.record_tool_call(turn_call);
            outcome.events.push(AgentEvent::ToolCallFinished(ToolResultEvent {
                id: call.id.clone(),
                name: call.name.clone(),
                success: result.success,
                output: result.content.clone(),
            }));
            outcome.executed.push(ToolLoopExecutionRecord {
                id: call.id.clone(),
                name: call.name.clone(),
                success: result.success,
                output: result.content,
                metadata: result.metadata,
            });
        }
        Err(error) => {
            let message = error.to_string();
            turn_call.set_error(message.clone());
            turn.record_tool_call(turn_call);
            outcome.events.push(AgentEvent::ToolCallFinished(ToolResultEvent {
                id: call.id.clone(),
                name: call.name.clone(),
                success: false,
                output: message.clone(),
            }));
            outcome.executed.push(ToolLoopExecutionRecord {
                id: call.id.clone(),
                name: call.name.clone(),
                success: false,
                output: message,
                metadata: None,
            });
        }
    }
    Ok(())
}

fn to_agent_approval_request(request: &MobileApprovalRequest) -> ApprovalRequest {
    ApprovalRequest {
        id: request.id.clone(),
        title: format!("Approve {}", request.tool_name),
        description: request.impacts.join("\n"),
        risk_level: risk_level_for(&request.category, &request.risk),
    }
}

fn risk_level_for(category: &ToolCategory, risk: &ApprovalRisk) -> RiskLevel {
    match (category, risk) {
        (_, ApprovalRisk::Benign) => RiskLevel::Low,
        (ToolCategory::Git, ApprovalRisk::Destructive) => RiskLevel::Medium,
        (ToolCategory::FileWrite, ApprovalRisk::Destructive)
        | (ToolCategory::Network, ApprovalRisk::Destructive)
        | (ToolCategory::Unknown, ApprovalRisk::Destructive) => RiskLevel::High,
        (ToolCategory::Shell, ApprovalRisk::Destructive) => RiskLevel::Dangerous,
        (ToolCategory::Safe, ApprovalRisk::Destructive) => RiskLevel::Medium,
    }
}

#[cfg(test)]
mod tests {
    use super::{continue_pending_tool_approval, process_model_text_with_tools};
    use crate::approval::{ApprovalMode, ReviewDecision};
    use crate::tool_execution::ToolExecutionCoordinator;
    use crate::tools::file_ops::ReadFileTool;
    use crate::tools::{ToolContext, ToolRegistry};
    use crate::turn::{TurnContext, TurnStatus};
    use crate::workspace::{ExecutorKind, Workspace};

    #[tokio::test]
    async fn plain_text_does_not_require_tools() {
        let registry = ToolRegistry::new();
        let coordinator = ToolExecutionCoordinator::new(&registry);
        let context = ToolContext::new(Workspace::new("w1", "Project", "/tmp", ExecutorKind::LocalAndroid));
        let mut turn = TurnContext::new(5);
        let outcome = process_model_text_with_tools(
            "normal answer",
            &registry,
            &coordinator,
            &context,
            &ApprovalMode::Suggest,
            &mut turn,
        )
        .await
        .unwrap();
        assert_eq!(outcome.final_text, "normal answer");
        assert!(!outcome.requires_user_input);
    }

    #[tokio::test]
    async fn write_file_requires_approval_in_suggest_mode() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(crate::tools::file_ops::WriteFileTool));
        let coordinator = ToolExecutionCoordinator::new(&registry);
        let context = ToolContext::new(Workspace::new("w1", "Project", "/tmp", ExecutorKind::LocalAndroid));
        let mut turn = TurnContext::new(5);
        let outcome = process_model_text_with_tools(
            r#"{"tool":"write_file","args":{"path":"README.md","content":"x"}}"#,
            &registry,
            &coordinator,
            &context,
            &ApprovalMode::Suggest,
            &mut turn,
        )
        .await
        .unwrap();
        assert!(outcome.requires_user_input);
        assert_eq!(outcome.pending_approvals.len(), 1);
        assert_eq!(outcome.pending_tool_approvals.len(), 1);
        assert_eq!(turn.status, TurnStatus::WaitingForApproval);
    }

    #[tokio::test]
    async fn approved_pending_tool_continues_same_turn() {
        let dir = std::env::temp_dir().join(format!(
            "deepseek_mobile_tool_approval_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut registry = ToolRegistry::new();
        registry.register(Box::new(crate::tools::file_ops::WriteFileTool));
        let coordinator = ToolExecutionCoordinator::new(&registry);
        let context = ToolContext::new(Workspace::new("w1", "Project", &dir, ExecutorKind::LocalAndroid));
        let mut turn = TurnContext::new(5);
        let outcome = process_model_text_with_tools(
            r#"{"tool":"write_file","args":{"path":"README.md","content":"approved"}}"#,
            &registry,
            &coordinator,
            &context,
            &ApprovalMode::Suggest,
            &mut turn,
        )
        .await
        .unwrap();
        let pending = outcome.pending_tool_approvals[0].clone();

        let continued = continue_pending_tool_approval(
            &pending,
            &ReviewDecision::Approved,
            &coordinator,
            &context,
            &mut turn,
        )
        .await
        .unwrap();

        assert_eq!(turn.status, TurnStatus::Running);
        assert_eq!(continued.executed.len(), 1);
        assert_eq!(std::fs::read_to_string(dir.join("README.md")).unwrap(), "approved");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn denied_pending_tool_records_error_without_execution() {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(crate::tools::file_ops::WriteFileTool));
        let coordinator = ToolExecutionCoordinator::new(&registry);
        let context = ToolContext::new(Workspace::new("w1", "Project", "/tmp", ExecutorKind::LocalAndroid));
        let mut turn = TurnContext::new(5);
        let outcome = process_model_text_with_tools(
            r#"{"tool":"write_file","args":{"path":"README.md","content":"x"}}"#,
            &registry,
            &coordinator,
            &context,
            &ApprovalMode::Suggest,
            &mut turn,
        )
        .await
        .unwrap();
        let pending = outcome.pending_tool_approvals[0].clone();
        let continued = continue_pending_tool_approval(
            &pending,
            &ReviewDecision::Denied,
            &coordinator,
            &context,
            &mut turn,
        )
        .await
        .unwrap();
        assert_eq!(turn.status, TurnStatus::Running);
        assert_eq!(continued.executed.len(), 1);
        assert!(!continued.executed[0].success);
    }

    #[tokio::test]
    async fn safe_read_tool_can_run_without_approval() {
        let dir = std::env::temp_dir().join(format!(
            "deepseek_mobile_tool_loop_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("README.md"), "ok").unwrap();

        let mut registry = ToolRegistry::new();
        registry.register(Box::new(ReadFileTool));
        let coordinator = ToolExecutionCoordinator::new(&registry);
        let context = ToolContext::new(Workspace::new("w1", "Project", &dir, ExecutorKind::LocalAndroid));
        let mut turn = TurnContext::new(5);
        let outcome = process_model_text_with_tools(
            r#"{"tool":"read_file","args":{"path":"README.md"}}"#,
            &registry,
            &coordinator,
            &context,
            &ApprovalMode::Suggest,
            &mut turn,
        )
        .await
        .unwrap();

        assert!(!outcome.requires_user_input);
        assert_eq!(outcome.executed.len(), 1);
        assert_eq!(outcome.executed[0].output, "ok");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

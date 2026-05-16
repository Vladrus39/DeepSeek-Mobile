use crate::agent_timeline::{
    MobileTimelineItemKind, MobileTimelineItemStatus, MobileTimelineState,
};
use deepseek_mobile_core::{AgentEvent, RiskLevel};

pub fn push_agent_event(timeline: &mut MobileTimelineState, event: &AgentEvent) -> Option<String> {
    match event {
        AgentEvent::Started => Some(timeline.push(
            MobileTimelineItemKind::Status,
            MobileTimelineItemStatus::Running,
            "Agent started",
            "DeepSeek agent request started",
        )),
        AgentEvent::Status(message) => Some(timeline.push(
            MobileTimelineItemKind::Status,
            MobileTimelineItemStatus::Running,
            "Agent status",
            message.clone(),
        )),
        AgentEvent::TurnStarted { turn_id } => Some(timeline.push(
            MobileTimelineItemKind::Status,
            MobileTimelineItemStatus::Running,
            "Turn started",
            turn_id.clone(),
        )),
        AgentEvent::TurnFinished {
            turn_id,
            status,
            usage,
            error,
        } => {
            if error.is_some() {
                timeline.fail_live_assistant_message();
            } else {
                timeline.finish_live_assistant_message();
            }
            Some(timeline.push(
                if error.is_some() {
                    MobileTimelineItemKind::Error
                } else {
                    MobileTimelineItemKind::Status
                },
                if error.is_some() {
                    MobileTimelineItemStatus::Failed
                } else {
                    MobileTimelineItemStatus::Done
                },
                "Turn finished",
                format!(
                    "turn_id={} status={:?} prompt_tokens={} completion_tokens={} total_tokens={}{}",
                    turn_id,
                    status,
                    usage.prompt_tokens,
                    usage.completion_tokens,
                    usage.total_tokens,
                    error
                        .as_ref()
                        .map(|message| format!(" error={}", message))
                        .unwrap_or_default()
                ),
            ))
        }
        AgentEvent::MessageStarted { index, role } => {
            if role == "assistant" {
                timeline.start_live_assistant_message();
            }
            Some(timeline.push(
                MobileTimelineItemKind::Status,
                MobileTimelineItemStatus::Running,
                "Message started",
                format!("#{} role={}", index, role),
            ))
        }
        AgentEvent::TextDelta(text) => {
            if text.is_empty() {
                None
            } else {
                Some(timeline.append_live_assistant_delta(text))
            }
        }
        AgentEvent::ReasoningDelta(text) => Some(timeline.push(
            MobileTimelineItemKind::Status,
            MobileTimelineItemStatus::Running,
            "Reasoning",
            text.clone(),
        )),
        AgentEvent::MessageFinished { index } => {
            timeline.finish_live_assistant_message();
            Some(timeline.push(
                MobileTimelineItemKind::Status,
                MobileTimelineItemStatus::Done,
                "Message finished",
                format!("message #{} completed", index),
            ))
        }
        AgentEvent::ToolCallStarted(tool) => Some(timeline.push(
            MobileTimelineItemKind::ToolCall,
            MobileTimelineItemStatus::Running,
            format!("Tool: {}", tool.name),
            tool.args.clone(),
        )),
        AgentEvent::ToolCallFinished(result) => Some(timeline.push(
            MobileTimelineItemKind::ToolCall,
            if result.success {
                MobileTimelineItemStatus::Done
            } else {
                MobileTimelineItemStatus::Failed
            },
            format!("Tool result: {}", result.name),
            result.output.clone(),
        )),
        AgentEvent::ApprovalRequired(request) => Some(timeline.push(
            MobileTimelineItemKind::Approval,
            MobileTimelineItemStatus::WaitingForApproval,
            request.title.clone(),
            format!(
                "{}\nrisk={}",
                request.description,
                risk_label(&request.risk_level)
            ),
        )),
        AgentEvent::PatchProposed(patch) => Some(timeline.push(
            MobileTimelineItemKind::ToolCall,
            MobileTimelineItemStatus::WaitingForApproval,
            format!("Patch proposed: {}", patch.file_path),
            patch.diff.clone(),
        )),
        AgentEvent::SessionUpdated {
            messages,
            model,
            workspace,
        } => Some(timeline.push(
            MobileTimelineItemKind::Status,
            MobileTimelineItemStatus::Done,
            "Session updated",
            format!(
                "messages={} model={} workspace={}",
                messages.len(), model, workspace
            ),
        )),
        AgentEvent::Error(message) => {
            timeline.fail_live_assistant_message();
            Some(timeline.push(
                MobileTimelineItemKind::Error,
                MobileTimelineItemStatus::Failed,
                "Agent error",
                message.clone(),
            ))
        }
        AgentEvent::Finished => {
            timeline.finish_live_assistant_message();
            Some(timeline.push(
                MobileTimelineItemKind::Status,
                MobileTimelineItemStatus::Done,
                "Agent finished",
                "DeepSeek agent request completed",
            ))
        }
    }
}

pub fn risk_label(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
        RiskLevel::Dangerous => "dangerous",
    }
}

#[cfg(test)]
mod tests {
    use super::push_agent_event;
    use crate::agent_timeline::{MobileTimelineItemKind, MobileTimelineItemStatus, MobileTimelineState};
    use deepseek_mobile_core::{
        AgentEvent, ApprovalRequest, RiskLevel, ToolCallEvent, ToolResultEvent,
    };

    #[test]
    fn status_event_becomes_timeline_status() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(&mut timeline, &AgentEvent::Status("Thinking".to_string()));
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline.items[0].kind, MobileTimelineItemKind::Status);
        assert_eq!(timeline.items[0].body, "Thinking");
    }

    #[test]
    fn streaming_text_deltas_merge_into_one_assistant_message() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(&mut timeline, &AgentEvent::MessageStarted { index: 0, role: "assistant".to_string() });
        push_agent_event(&mut timeline, &AgentEvent::TextDelta("hel".to_string()));
        push_agent_event(&mut timeline, &AgentEvent::TextDelta("lo".to_string()));
        push_agent_event(&mut timeline, &AgentEvent::MessageFinished { index: 0 });

        let assistant = timeline
            .items
            .iter()
            .find(|item| item.kind == MobileTimelineItemKind::AssistantMessage)
            .expect("assistant message");
        assert_eq!(assistant.body, "hello");
        assert_eq!(assistant.status, MobileTimelineItemStatus::Done);
    }

    #[test]
    fn tool_events_become_tool_cards() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(
            &mut timeline,
            &AgentEvent::ToolCallStarted(ToolCallEvent {
                id: "tool-1".to_string(),
                name: "read_file".to_string(),
                args: "{\"path\":\"Cargo.toml\"}".to_string(),
            }),
        );
        push_agent_event(
            &mut timeline,
            &AgentEvent::ToolCallFinished(ToolResultEvent {
                id: "tool-1".to_string(),
                name: "read_file".to_string(),
                success: true,
                output: "ok".to_string(),
            }),
        );
        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline.items[0].kind, MobileTimelineItemKind::ToolCall);
        assert_eq!(timeline.items[1].status, MobileTimelineItemStatus::Done);
    }

    #[test]
    fn approval_event_becomes_waiting_card() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(
            &mut timeline,
            &AgentEvent::ApprovalRequired(ApprovalRequest {
                id: "approval-1".to_string(),
                title: "Run command".to_string(),
                description: "cargo test".to_string(),
                risk_level: RiskLevel::Medium,
            }),
        );
        assert_eq!(timeline.items[0].kind, MobileTimelineItemKind::Approval);
        assert_eq!(timeline.items[0].status, MobileTimelineItemStatus::WaitingForApproval);
        assert!(timeline.items[0].body.contains("risk=medium"));
    }
}
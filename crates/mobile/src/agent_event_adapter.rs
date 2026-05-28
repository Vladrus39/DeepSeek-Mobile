use crate::agent_timeline::{
    MobileTimelineItemKind, MobileTimelineItemStatus, MobileTimelineState,
};
use deepseek_mobile_core::{AgentEvent, RiskLevel, WorkspaceSnapshotRecord};

fn skip_status_timeline_line(message: &str) -> bool {
    matches!(
        message,
        "DeepSeek streaming response opened" | "DeepSeek streaming response completed"
    ) || message.starts_with("ModelRouter:")
}

pub fn push_agent_event(timeline: &mut MobileTimelineState, event: &AgentEvent) -> Option<String> {
    match event {
        AgentEvent::Started => None,
        AgentEvent::Status(message) => {
            if skip_status_timeline_line(message) {
                return None;
            }
            timeline.seal_agent_status_items();
            let status = if MobileTimelineState::status_message_is_terminal(message) {
                MobileTimelineItemStatus::Done
            } else {
                MobileTimelineItemStatus::Running
            };
            Some(timeline.push(
                MobileTimelineItemKind::Status,
                status,
                "Agent status",
                message.clone(),
            ))
        }
        AgentEvent::TurnStarted { .. } => None,
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
            timeline.seal_open_work_items();
            Some(timeline.push(
                MobileTimelineItemKind::Status,
                if error.is_some() {
                    MobileTimelineItemStatus::Failed
                } else {
                    MobileTimelineItemStatus::Done
                },
                "Turn finished",
                format!(
                    "turn_id={} status={:?} input_tokens={} output_tokens={} reasoning_tokens={} total_tokens={}{}",
                    turn_id,
                    status,
                    usage.input_tokens,
                    usage.output_tokens,
                    usage.reasoning_tokens,
                    usage.input_tokens + usage.output_tokens + usage.reasoning_tokens,
                    error
                        .as_ref()
                        .map(|message| format!(" error={}", message))
                        .unwrap_or_default()
                ),
            ))
        }
        AgentEvent::MessageStarted { index: _, role } => {
            if role == "assistant" {
                timeline.start_live_assistant_message();
            }
            None
        }
        AgentEvent::TextDelta(text) => {
            if text.is_empty() {
                None
            } else {
                Some(timeline.append_live_assistant_delta(text))
            }
        }
        AgentEvent::ReasoningDelta(text) => {
            if text.is_empty() {
                None
            } else {
                Some(timeline.append_live_reasoning_delta(text))
            }
        }
        AgentEvent::MessageFinished { index: _ } => {
            timeline.finish_live_assistant_message();
            None
        }
        AgentEvent::ToolCallStarted(tool) => Some(timeline.push(
            MobileTimelineItemKind::ToolCall,
            MobileTimelineItemStatus::Running,
            format!("Tool: {}", tool.name),
            tool.args.clone(),
        )),
        AgentEvent::ToolCallFinished(result) => {
            timeline.seal_tool_call(&result.name);
            if let Some(snapshot) = result
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("pre_snapshot"))
                .and_then(|value| {
                    serde_json::from_value::<WorkspaceSnapshotRecord>(value.clone()).ok()
                })
            {
                timeline.push(
                    MobileTimelineItemKind::Status,
                    MobileTimelineItemStatus::Done,
                    "Safety snapshot",
                    format!(
                        "{} · {} file(s) · {} bytes",
                        snapshot.id, snapshot.file_count, snapshot.total_bytes
                    ),
                );
            }

            if let Some(request_id) = result
                .metadata
                .as_ref()
                .filter(|metadata| {
                    metadata
                        .get("termux_execution_pending")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
                })
                .and_then(|metadata| metadata.get("termux_request_id"))
                .and_then(|value| value.as_str())
            {
                timeline.push(
                    MobileTimelineItemKind::Status,
                    MobileTimelineItemStatus::Running,
                    "Termux native execution queued",
                    format!("request_id={}", request_id),
                );
            }

            if let Some(summary) = result
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("post_edit_diagnostics_summary"))
                .and_then(|value| value.as_str())
            {
                timeline.push(
                    MobileTimelineItemKind::Status,
                    MobileTimelineItemStatus::Done,
                    "Post-edit diagnostics",
                    summary.to_string(),
                );
            }

            Some(timeline.push(
                MobileTimelineItemKind::ToolCall,
                if result.success {
                    MobileTimelineItemStatus::Done
                } else {
                    MobileTimelineItemStatus::Failed
                },
                format!("Tool result: {}", result.name),
                result.output.clone(),
            ))
        }
        AgentEvent::ApprovalRequired(request) => Some(timeline.push_with_id(
            request.id.clone(),
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
                messages.len(),
                model,
                workspace
            ),
        )),
        AgentEvent::TermuxExecutionPending { call_id, .. } => Some(timeline.push(
            MobileTimelineItemKind::Status,
            MobileTimelineItemStatus::Running,
            "Termux execution pending",
            format!(
                "tool_call_id={} — waiting for Android Termux bridge to complete",
                call_id
            ),
        )),
        AgentEvent::Error(message) => {
            timeline.fail_live_assistant_message();
            timeline.seal_open_work_items();
            Some(timeline.push(
                MobileTimelineItemKind::Error,
                MobileTimelineItemStatus::Failed,
                "Agent error",
                message.clone(),
            ))
        }
        AgentEvent::Finished => {
            timeline.finish_live_assistant_message();
            timeline.seal_open_work_items();
            None
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
    use crate::agent_timeline::{
        MobileTimelineItemKind, MobileTimelineItemStatus, MobileTimelineState,
    };
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
        assert_eq!(timeline.items[0].status, MobileTimelineItemStatus::Running);
    }

    #[test]
    fn terminal_status_event_is_marked_done() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(
            &mut timeline,
            &AgentEvent::Status("Готово · workspace: /tmp · thread: t1".to_string()),
        );
        assert_eq!(timeline.items[0].status, MobileTimelineItemStatus::Done);
    }

    #[test]
    fn reasoning_deltas_merge_into_one_work_log_line() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(&mut timeline, &AgentEvent::ReasoningDelta("a".to_string()));
        push_agent_event(&mut timeline, &AgentEvent::ReasoningDelta("b".to_string()));
        push_agent_event(&mut timeline, &AgentEvent::MessageFinished { index: 0 });

        let reasoning: Vec<_> = timeline
            .items
            .iter()
            .filter(|item| item.title == "Reasoning")
            .collect();
        assert_eq!(reasoning.len(), 1);
        assert_eq!(reasoning[0].body, "ab");
        assert_eq!(reasoning[0].status, MobileTimelineItemStatus::Done);
    }

    #[test]
    fn streaming_text_deltas_merge_into_one_assistant_message() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(
            &mut timeline,
            &AgentEvent::MessageStarted {
                index: 0,
                role: "assistant".to_string(),
            },
        );
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
                metadata: None,
            }),
        );
        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline.items[0].kind, MobileTimelineItemKind::ToolCall);
        assert_eq!(timeline.items[1].status, MobileTimelineItemStatus::Done);
    }

    #[test]
    fn tool_metadata_surfaces_snapshot_and_diagnostics_status_cards() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(
            &mut timeline,
            &AgentEvent::ToolCallFinished(ToolResultEvent {
                id: "tool-2".to_string(),
                name: "write_file".to_string(),
                success: true,
                output: "ok".to_string(),
                metadata: Some(serde_json::json!({
                    "pre_snapshot": {
                        "schema_version": 1,
                        "id": "snapshot-1",
                        "workspace_id": "w1",
                        "workspace_name": "Workspace",
                        "workspace_root": ".",
                        "reason": "pre-tool",
                        "created_unix": 1,
                        "file_count": 2,
                        "total_bytes": 42,
                        "files": []
                    },
                    "post_edit_diagnostics_summary": "1 diagnostic(s): 1 error(s), 0 warning(s)"
                })),
            }),
        );

        assert_eq!(timeline.items.len(), 3);
        assert_eq!(timeline.items[0].title, "Safety snapshot");
        assert_eq!(timeline.items[1].title, "Post-edit diagnostics");
        assert_eq!(timeline.items[2].title, "Tool result: write_file");
    }

    #[test]
    fn termux_pending_metadata_surfaces_native_queue_status() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(
            &mut timeline,
            &AgentEvent::ToolCallFinished(ToolResultEvent {
                id: "tool-3".to_string(),
                name: "exec_shell".to_string(),
                success: true,
                output: "queued".to_string(),
                metadata: Some(serde_json::json!({
                    "termux_execution_pending": true,
                    "termux_request_id": "termux-tool-3"
                })),
            }),
        );

        assert_eq!(timeline.items.len(), 2);
        assert_eq!(timeline.items[0].title, "Termux native execution queued");
        assert!(timeline.items[0].body.contains("termux-tool-3"));
        assert_eq!(timeline.items[1].title, "Tool result: exec_shell");
    }

    #[test]
    fn turn_finished_seals_running_status_rows() {
        let mut timeline = MobileTimelineState::default();
        push_agent_event(&mut timeline, &AgentEvent::Status("Thinking".to_string()));
        push_agent_event(
            &mut timeline,
            &AgentEvent::TurnFinished {
                turn_id: "t1".to_string(),
                status: deepseek_mobile_core::TurnStatus::Completed,
                usage: deepseek_mobile_core::TokenUsage::default(),
                error: None,
            },
        );
        let running_status = timeline
            .items
            .iter()
            .filter(|item| {
                item.kind == MobileTimelineItemKind::Status
                    && item.title == "Agent status"
                    && item.status == MobileTimelineItemStatus::Running
            })
            .count();
        assert_eq!(running_status, 0);
        assert!(
            timeline
                .items
                .iter()
                .any(|item| item.title == "Turn finished")
        );
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
        assert_eq!(timeline.items[0].id, "approval-1");
        assert_eq!(timeline.items[0].kind, MobileTimelineItemKind::Approval);
        assert_eq!(
            timeline.items[0].status,
            MobileTimelineItemStatus::WaitingForApproval
        );
        assert!(timeline.items[0].body.contains("risk=medium"));
    }
}

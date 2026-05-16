//! UI-facing approval card contract.
//!
//! Internal approval objects contain full tool parameters because the core needs
//! them to continue execution after approval. Android UI should not have to
//! understand that internal shape directly. This module exposes compact,
//! user-facing cards with safe parameter previews, clear risk wording and the
//! exact decisions the UI can send back to the engine.

use crate::approval::{ApprovalRisk, MobileApprovalRequest, ReviewDecision, ToolCategory};
use crate::approval_session::can_grant_for_session;
use crate::runtime_store::PendingApprovalRecord;
use crate::tool_loop::PendingToolCallApproval;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const MAX_PREVIEW_STRING_CHARS: usize = 240;
const MAX_PREVIEW_ARRAY_ITEMS: usize = 8;
const MAX_PREVIEW_OBJECT_KEYS: usize = 16;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalCardStatus {
    Pending,
    Recovered,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalCardSeverity {
    Info,
    Warning,
    HighRisk,
    Dangerous,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApprovalCardAction {
    pub decision: ReviewDecision,
    pub label: String,
    pub destructive: bool,
    pub closes_turn: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ApprovalCardView {
    pub id: String,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub title: String,
    pub subtitle: String,
    pub tool_name: String,
    pub category: ToolCategory,
    pub risk: ApprovalRisk,
    pub severity: ApprovalCardSeverity,
    pub status: ApprovalCardStatus,
    pub description: String,
    pub impacts: Vec<String>,
    pub argument_preview: Value,
    pub actions: Vec<ApprovalCardAction>,
}

impl ApprovalCardView {
    pub fn from_pending_record(record: &PendingApprovalRecord) -> Self {
        Self::from_pending_tool_approval_with_context(
            &record.pending,
            Some(record.thread_id.clone()),
            Some(record.turn_id.clone()),
            ApprovalCardStatus::Recovered,
        )
    }

    pub fn from_pending_tool_approval(pending: &PendingToolCallApproval) -> Self {
        Self::from_pending_tool_approval_with_context(pending, None, None, ApprovalCardStatus::Pending)
    }

    pub fn from_approval_request(request: &MobileApprovalRequest) -> Self {
        Self::from_parts(
            request,
            None,
            None,
            ApprovalCardStatus::Pending,
            sanitize_value_for_preview(&request.params),
        )
    }

    fn from_pending_tool_approval_with_context(
        pending: &PendingToolCallApproval,
        thread_id: Option<String>,
        turn_id: Option<String>,
        status: ApprovalCardStatus,
    ) -> Self {
        Self::from_parts(
            &pending.approval,
            thread_id,
            turn_id,
            status,
            sanitize_value_for_preview(&pending.call.arguments),
        )
    }

    fn from_parts(
        approval: &MobileApprovalRequest,
        thread_id: Option<String>,
        turn_id: Option<String>,
        status: ApprovalCardStatus,
        argument_preview: Value,
    ) -> Self {
        let severity = severity_for(&approval.category, &approval.risk);
        Self {
            id: approval.id.clone(),
            thread_id,
            turn_id,
            title: title_for(&approval.category, &approval.tool_name),
            subtitle: subtitle_for(&approval.category, &approval.risk),
            tool_name: approval.tool_name.clone(),
            category: approval.category.clone(),
            risk: approval.risk.clone(),
            severity,
            status,
            description: approval.description.clone(),
            impacts: approval.impacts.clone(),
            argument_preview,
            actions: default_actions_for(approval),
        }
    }
}

pub fn approval_cards_from_records(records: &[PendingApprovalRecord]) -> Vec<ApprovalCardView> {
    records.iter().map(ApprovalCardView::from_pending_record).collect()
}

pub fn sanitize_value_for_preview(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = Map::new();
            for (idx, (key, value)) in map.iter().enumerate() {
                if idx >= MAX_PREVIEW_OBJECT_KEYS {
                    out.insert("_truncated".to_string(), Value::String("object preview truncated".to_string()));
                    break;
                }
                if is_sensitive_key(key) {
                    out.insert(key.clone(), Value::String("<redacted>".to_string()));
                } else {
                    out.insert(key.clone(), sanitize_value_for_preview(value));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .take(MAX_PREVIEW_ARRAY_ITEMS)
                .map(sanitize_value_for_preview)
                .collect(),
        ),
        Value::String(text) => Value::String(truncate_preview_string(text)),
        other => other.clone(),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("token")
        || key.contains("password")
        || key.contains("secret")
        || key.contains("api_key")
        || key.contains("apikey")
        || key.contains("authorization")
        || key.contains("credential")
}

fn truncate_preview_string(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= MAX_PREVIEW_STRING_CHARS {
        return text.to_string();
    }
    let mut preview = text.chars().take(MAX_PREVIEW_STRING_CHARS).collect::<String>();
    preview.push_str("... <truncated>");
    preview
}

fn title_for(category: &ToolCategory, tool_name: &str) -> String {
    match category {
        ToolCategory::Safe => format!("Allow read-only tool: {}", tool_name),
        ToolCategory::FileWrite => format!("Approve file change: {}", tool_name),
        ToolCategory::Shell => format!("Approve command execution: {}", tool_name),
        ToolCategory::Git => format!("Approve git operation: {}", tool_name),
        ToolCategory::Network => format!("Approve network access: {}", tool_name),
        ToolCategory::Unknown => format!("Approve tool action: {}", tool_name),
    }
}

fn subtitle_for(category: &ToolCategory, risk: &ApprovalRisk) -> String {
    match (category, risk) {
        (ToolCategory::Safe, ApprovalRisk::Benign) => "Read-only operation".to_string(),
        (ToolCategory::Safe, ApprovalRisk::Destructive) => "Unexpected high-risk read operation".to_string(),
        (ToolCategory::FileWrite, _) => "This may modify workspace files".to_string(),
        (ToolCategory::Shell, _) => "This may run a command through the selected executor".to_string(),
        (ToolCategory::Git, _) => "This may inspect or change repository state".to_string(),
        (ToolCategory::Network, _) => "This may access network resources".to_string(),
        (ToolCategory::Unknown, _) => "Tool impact is unknown".to_string(),
    }
}

fn severity_for(category: &ToolCategory, risk: &ApprovalRisk) -> ApprovalCardSeverity {
    match (category, risk) {
        (ToolCategory::Shell, ApprovalRisk::Destructive) => ApprovalCardSeverity::Dangerous,
        (ToolCategory::FileWrite, ApprovalRisk::Destructive)
        | (ToolCategory::Network, ApprovalRisk::Destructive)
        | (ToolCategory::Unknown, ApprovalRisk::Destructive) => ApprovalCardSeverity::HighRisk,
        (ToolCategory::Git, ApprovalRisk::Destructive) => ApprovalCardSeverity::Warning,
        (_, ApprovalRisk::Benign) => ApprovalCardSeverity::Info,
        _ => ApprovalCardSeverity::Warning,
    }
}

fn default_actions_for(approval: &MobileApprovalRequest) -> Vec<ApprovalCardAction> {
    let mut actions = vec![
        ApprovalCardAction {
            decision: ReviewDecision::Approved,
            label: "Approve once".to_string(),
            destructive: false,
            closes_turn: false,
        },
        ApprovalCardAction {
            decision: ReviewDecision::Denied,
            label: "Deny".to_string(),
            destructive: true,
            closes_turn: false,
        },
        ApprovalCardAction {
            decision: ReviewDecision::Abort,
            label: "Abort turn".to_string(),
            destructive: true,
            closes_turn: true,
        },
    ];

    if can_grant_for_session(approval) {
        actions.insert(
            1,
            ApprovalCardAction {
                decision: ReviewDecision::ApprovedForSession,
                label: "Approve for session".to_string(),
                destructive: false,
                closes_turn: false,
            },
        );
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::{sanitize_value_for_preview, ApprovalCardView};
    use crate::approval::{ApprovalRisk, MobileApprovalRequest, ReviewDecision, ToolCategory};
    use crate::tool_call::{ToolCallRequest, ToolCallSource};
    use crate::tool_loop::PendingToolCallApproval;
    use serde_json::json;

    #[test]
    fn redacts_sensitive_preview_values() {
        let preview = sanitize_value_for_preview(&json!({
            "api_key": "abc",
            "nested": { "password": "secret", "path": "README.md" }
        }));
        assert_eq!(preview["api_key"], "<redacted>");
        assert_eq!(preview["nested"]["password"], "<redacted>");
        assert_eq!(preview["nested"]["path"], "README.md");
    }

    #[test]
    fn truncates_large_content_preview() {
        let long = "x".repeat(400);
        let preview = sanitize_value_for_preview(&json!({"content": long}));
        let content = preview["content"].as_str().unwrap();
        assert!(content.ends_with("... <truncated>"));
        assert!(content.len() < 280);
    }

    #[test]
    fn builds_file_write_card_from_pending_tool_approval() {
        let call = ToolCallRequest::new(
            "write_file",
            json!({"path":"README.md","content":"x"}),
            ToolCallSource::Manual,
        );
        let approval = MobileApprovalRequest::new(
            "write_file",
            ToolCategory::FileWrite,
            ApprovalRisk::Benign,
            call.arguments.clone(),
        )
        .with_id("approval-1");
        let pending = PendingToolCallApproval { approval, call };
        let card = ApprovalCardView::from_pending_tool_approval(&pending);
        assert_eq!(card.id, "approval-1");
        assert_eq!(card.tool_name, "write_file");
        assert!(card.title.contains("file change"));
        assert!(card
            .actions
            .iter()
            .any(|action| action.decision == ReviewDecision::ApprovedForSession));
    }

    #[test]
    fn shell_card_does_not_offer_session_approval() {
        let call = ToolCallRequest::new(
            "exec_shell",
            json!({"command":"cargo check"}),
            ToolCallSource::Manual,
        );
        let approval = MobileApprovalRequest::new(
            "exec_shell",
            ToolCategory::Shell,
            ApprovalRisk::Destructive,
            call.arguments.clone(),
        )
        .with_id("approval-1");
        let pending = PendingToolCallApproval { approval, call };
        let card = ApprovalCardView::from_pending_tool_approval(&pending);
        assert!(!card
            .actions
            .iter()
            .any(|action| action.decision == ReviewDecision::ApprovedForSession));
    }
}

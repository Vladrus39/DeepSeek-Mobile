use crate::project_diff::{build_text_diff_preview, ProjectDiffPreview};
use deepseek_mobile_core::{ApprovalCardView, ToolCategory};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalDiffPreview {
    pub approval_id: String,
    pub tool_name: String,
    pub path: String,
    pub diff: ProjectDiffPreview,
}

pub fn build_approval_diff_preview(card: &ApprovalCardView) -> Option<ApprovalDiffPreview> {
    if card.category != ToolCategory::FileWrite {
        return None;
    }

    let path = first_string_arg(card, &["path", "file", "file_path", "relative_path", "target_path"])?;
    let after = first_string_arg(card, &["content", "new_content", "after", "replacement", "text"])?;
    let before = first_string_arg(card, &["before", "old_content", "current_content"]).unwrap_or_default();
    let diff = build_text_diff_preview(path.clone(), &before, &after);

    Some(ApprovalDiffPreview {
        approval_id: card.id.clone(),
        tool_name: card.tool_name.clone(),
        path,
        diff,
    })
}

fn first_string_arg(card: &ApprovalCardView, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = card.argument_preview.get(*key).and_then(|value| value.as_str()) {
            if !value.trim().is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub fn approval_diff_summary(preview: &ApprovalDiffPreview) -> String {
    format!(
        "{} · {} · +{} / -{}",
        preview.path, preview.tool_name, preview.diff.added_lines, preview.diff.removed_lines
    )
}

#[cfg(test)]
mod tests {
    use super::{approval_diff_summary, build_approval_diff_preview};
    use deepseek_mobile_core::{
        ApprovalCardSeverity, ApprovalCardStatus, ApprovalCardView, ApprovalRisk, ToolCategory,
    };

    fn file_write_card() -> ApprovalCardView {
        ApprovalCardView {
            id: "approval-1".to_string(),
            thread_id: None,
            turn_id: None,
            title: "Approve file change".to_string(),
            subtitle: "This may modify workspace files".to_string(),
            tool_name: "write_file".to_string(),
            category: ToolCategory::FileWrite,
            risk: ApprovalRisk::Benign,
            severity: ApprovalCardSeverity::Warning,
            status: ApprovalCardStatus::Pending,
            description: "Write project file".to_string(),
            impacts: vec![],
            argument_preview: serde_json::json!({
                "path": "src/main.rs",
                "before": "fn main() {}\n",
                "content": "fn main() { println!(\"ok\"); }\n"
            }),
            actions: vec![],
        }
    }

    #[test]
    fn builds_diff_from_file_write_approval_card() {
        let preview = build_approval_diff_preview(&file_write_card()).expect("diff preview");
        assert_eq!(preview.path, "src/main.rs");
        assert_eq!(preview.diff.added_lines, 1);
        assert_eq!(preview.diff.removed_lines, 1);
        assert!(approval_diff_summary(&preview).contains("write_file"));
    }

    #[test]
    fn skips_non_file_write_cards() {
        let mut card = file_write_card();
        card.category = ToolCategory::Safe;
        assert!(build_approval_diff_preview(&card).is_none());
    }
}
use crate::agent_timeline::{
    timeline_kind_label, timeline_status_label, MobileTimelineItem, MobileTimelineItemKind,
    MobileTimelineItemStatus, MobileTimelineState,
};
use crate::locale::{pick, AppLanguage};
use dioxus::prelude::*;

pub fn agent_timeline_panel(lang: AppLanguage, timeline: &MobileTimelineState) -> Element {
    if timeline.is_empty() {
        let empty_hint = pick(
            lang,
            "Попросите DeepSeek собрать, проверить, исправить или развернуть проект.\nЗдесь появятся действия агента, вызовы инструментов и одобрения.",
            "Ask DeepSeek to build, inspect, fix, test or deploy a project.\nAgent actions, tool calls and approvals will appear here.",
        );
        return rsx! {
            div {
                color: "#9ca3af",
                text_align: "center",
                margin_top: "32px",
                white_space: "pre-wrap",
                "{empty_hint}"
            }
        };
    }

    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            gap: "10px",

            for item in timeline.items.iter() {
                {timeline_item_card(item)}
            }
        }
    }
}

fn timeline_item_card(item: &MobileTimelineItem) -> Element {
    rsx! {
        div {
            background_color: item_background(&item.kind),
            border: item_border(&item.status),
            border_radius: "14px",
            padding: "10px 12px",
            display: "flex",
            flex_direction: "column",
            gap: "6px",
            max_width: if item.kind == MobileTimelineItemKind::UserMessage { "85%" } else { "100%" },
            align_self: if item.kind == MobileTimelineItemKind::UserMessage { "flex-end" } else { "stretch" },

            div {
                display: "flex",
                justify_content: "space-between",
                align_items: "center",
                gap: "8px",

                div {
                    display: "flex",
                    gap: "6px",
                    align_items: "center",

                    div {
                        background_color: item_badge_color(&item.kind),
                        color: "white",
                        border_radius: "999px",
                        padding: "3px 7px",
                        font_size: "10px",
                        font_weight: "bold",
                        "{timeline_kind_label(&item.kind)}"
                    }

                    div {
                        color: "#f9fafb",
                        font_size: "13px",
                        font_weight: "bold",
                        "{item.title}"
                    }
                }

                div {
                    color: status_color(&item.status),
                    font_size: "11px",
                    "{timeline_status_label(&item.status)}"
                }
            }

            div {
                color: "#d1d5db",
                font_size: "13px",
                white_space: "pre-wrap",
                "{item.body}"
            }
        }
    }
}

fn item_background(kind: &MobileTimelineItemKind) -> &'static str {
    match kind {
        MobileTimelineItemKind::UserMessage => "#2563eb",
        MobileTimelineItemKind::AssistantMessage => "#1f2937",
        MobileTimelineItemKind::Attachment => "#111827",
        MobileTimelineItemKind::NativeCommand => "#111827",
        MobileTimelineItemKind::ToolCall => "#111827",
        MobileTimelineItemKind::Approval => "#422006",
        MobileTimelineItemKind::Status => "#111827",
        MobileTimelineItemKind::Error => "#7f1d1d",
    }
}

fn item_border(status: &MobileTimelineItemStatus) -> &'static str {
    match status {
        MobileTimelineItemStatus::Pending => "1px solid #4b5563",
        MobileTimelineItemStatus::Running => "1px solid #3b82f6",
        MobileTimelineItemStatus::Done => "1px solid #374151",
        MobileTimelineItemStatus::Failed => "1px solid #dc2626",
        MobileTimelineItemStatus::WaitingForApproval => "1px solid #ca8a04",
    }
}

fn item_badge_color(kind: &MobileTimelineItemKind) -> &'static str {
    match kind {
        MobileTimelineItemKind::UserMessage => "#1d4ed8",
        MobileTimelineItemKind::AssistantMessage => "#374151",
        MobileTimelineItemKind::Attachment => "#059669",
        MobileTimelineItemKind::NativeCommand => "#7c3aed",
        MobileTimelineItemKind::ToolCall => "#7c3aed",
        MobileTimelineItemKind::Approval => "#ca8a04",
        MobileTimelineItemKind::Status => "#2563eb",
        MobileTimelineItemKind::Error => "#dc2626",
    }
}

fn status_color(status: &MobileTimelineItemStatus) -> &'static str {
    match status {
        MobileTimelineItemStatus::Pending => "#9ca3af",
        MobileTimelineItemStatus::Running => "#93c5fd",
        MobileTimelineItemStatus::Done => "#86efac",
        MobileTimelineItemStatus::Failed => "#fca5a5",
        MobileTimelineItemStatus::WaitingForApproval => "#fde68a",
    }
}

#[cfg(test)]
mod tests {
    use super::{item_background, item_badge_color, item_border};
    use crate::agent_timeline::{MobileTimelineItemKind, MobileTimelineItemStatus};

    #[test]
    fn timeline_styles_cover_core_kinds() {
        assert_eq!(
            item_badge_color(&MobileTimelineItemKind::ToolCall),
            "#7c3aed"
        );
        assert_eq!(item_background(&MobileTimelineItemKind::Error), "#7f1d1d");
        assert_eq!(
            item_border(&MobileTimelineItemStatus::WaitingForApproval),
            "1px solid #ca8a04"
        );
    }
}

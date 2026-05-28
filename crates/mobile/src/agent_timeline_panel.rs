use crate::agent_timeline::{
    MobileTimelineItem, MobileTimelineItemKind, MobileTimelineItemStatus, MobileTimelineState,
};
use crate::locale::{pick, timeline_kind_label, timeline_status_label, AppLanguage};
use crate::mobile_approval_panel::inline_approval_card;
use deepseek_mobile_core::{ApprovalCardStatus, ApprovalCardView, ReviewDecision};
use dioxus::prelude::*;

pub fn agent_timeline_panel(
    ui_lang: AppLanguage,
    timeline: &MobileTimelineState,
    approval_cards: &[ApprovalCardView],
    on_approval_decision: EventHandler<(String, ReviewDecision)>,
    activity_open: bool,
    on_activity_toggle: EventHandler<()>,
) -> Element {
    let conversation_items = timeline
        .items
        .iter()
        .filter(|item| is_conversation_item(item))
        .collect::<Vec<_>>();
    let activity_items = timeline
        .items
        .iter()
        .filter(|item| is_activity_item(item))
        .collect::<Vec<_>>();

    if timeline.is_empty() || conversation_items.is_empty() {
        let empty_title = pick(ui_lang, "Готов к задаче", "Ready for a task");
        let empty_hint = pick(
            ui_lang,
            "Напишите как в обычном чате: что проверить, исправить, собрать или запустить.",
            "Write naturally: what to inspect, fix, build or run.",
        );
        return rsx! {
            div {
                style: "display:flex;flex:1;min-height:40vh;align-items:center;justify-content:center;text-align:center;flex-direction:column;gap:10px;",
                div {
                    style: "max-width:22rem;padding:20px;border-radius:22px;background:#0b1220;border:1px solid #1f2937;",
                    div {
                        style: "font-size:18px;font-weight:800;color:#f9fafb;margin-bottom:8px;",
                        "{empty_title}"
                    }
                    div {
                        style: "font-size:14px;line-height:1.45;color:#9ca3af;",
                        "{empty_hint}"
                    }
                }
                if !activity_items.is_empty() {
                    {assistant_activity_panel(ui_lang, &activity_items, activity_open, on_activity_toggle)}
                }
            }
        };
    }

    let has_running_assistant = conversation_items.iter().any(|item| {
        item.kind == MobileTimelineItemKind::AssistantMessage
            && item.status == MobileTimelineItemStatus::Running
    });
    let has_running_activity = activity_has_active_item(&activity_items);

    // column-reverse: first DOM child sits at the visual bottom so reopening the app
    // shows the latest messages without a separate scrollTo call.
    rsx! {
        div {
            style: "display:flex;flex-direction:column-reverse;gap:10px;padding-bottom:4px;min-height:min-content;",

            if !activity_items.is_empty() {
                {assistant_activity_panel(ui_lang, &activity_items, activity_open, on_activity_toggle)}
            }

            if has_running_activity && !has_running_assistant {
                {assistant_thinking_bubble(ui_lang)}
            }

            for item in conversation_items.iter().rev() {
                {timeline_item_view(
                    ui_lang,
                    timeline,
                    approval_cards,
                    on_approval_decision,
                    item,
                )}
            }
        }
    }
}

fn is_conversation_item(item: &&MobileTimelineItem) -> bool {
    if item.kind == MobileTimelineItemKind::AssistantMessage && item.body.trim() == "PROBE_OK" {
        return false;
    }
    matches!(
        item.kind,
        MobileTimelineItemKind::UserMessage
            | MobileTimelineItemKind::AssistantMessage
            | MobileTimelineItemKind::Approval
            | MobileTimelineItemKind::Error
    ) || (item.kind == MobileTimelineItemKind::ToolCall
        && item.status == MobileTimelineItemStatus::WaitingForApproval)
}

fn is_activity_item(item: &&MobileTimelineItem) -> bool {
    matches!(
        item.kind,
        MobileTimelineItemKind::Status
            | MobileTimelineItemKind::Attachment
            | MobileTimelineItemKind::NativeCommand
            | MobileTimelineItemKind::ToolCall
    )
}

fn timeline_item_view(
    ui_lang: AppLanguage,
    timeline: &MobileTimelineState,
    approval_cards: &[ApprovalCardView],
    on_approval_decision: EventHandler<(String, ReviewDecision)>,
    item: &MobileTimelineItem,
) -> Element {
    match item.kind {
        MobileTimelineItemKind::UserMessage => user_message_bubble(item),
        MobileTimelineItemKind::AssistantMessage => assistant_message_bubble(ui_lang, item),
        MobileTimelineItemKind::Status => system_status_line(ui_lang, item),
        MobileTimelineItemKind::Attachment | MobileTimelineItemKind::NativeCommand => {
            compact_tool_card(ui_lang, item)
        }
        MobileTimelineItemKind::ToolCall => {
            tool_call_with_inline_approval(ui_lang, timeline, approval_cards, on_approval_decision, item)
        }
        MobileTimelineItemKind::Approval => {
            approval_with_linked_tool(ui_lang, timeline, approval_cards, on_approval_decision, item)
        }
        MobileTimelineItemKind::Error => assistant_error_bubble(ui_lang, item),
    }
}

fn pending_card_for_item<'a>(
    cards: &'a [ApprovalCardView],
    item: &MobileTimelineItem,
) -> Option<&'a ApprovalCardView> {
    if let Some(card) = cards
        .iter()
        .find(|card| card.id == item.id && card.status == ApprovalCardStatus::Pending)
    {
        return Some(card);
    }
    tool_name_from_timeline_item(item).and_then(|tool_name| {
        cards.iter().find(|card| {
            card.status == ApprovalCardStatus::Pending
                && card.tool_name == tool_name
                && item_matches_pending_tool(item, &card.tool_name)
        })
    })
}

fn tool_name_from_timeline_item(item: &MobileTimelineItem) -> Option<&str> {
    if let Some(name) = item.title.strip_prefix("Tool: ") {
        return Some(name);
    }
    item.title.strip_prefix("Patch proposed: ")
}

fn item_matches_pending_tool(item: &MobileTimelineItem, tool_name: &str) -> bool {
    item.title == format!("Tool: {tool_name}")
        || item.title == format!("Patch proposed: {tool_name}")
        || item.title.contains(tool_name)
}

fn find_linked_tool_item<'a>(
    timeline: &'a MobileTimelineState,
    card: &ApprovalCardView,
) -> Option<&'a MobileTimelineItem> {
    timeline.items.iter().rev().find(|row| {
        row.kind == MobileTimelineItemKind::ToolCall
            && matches!(
                row.status,
                MobileTimelineItemStatus::Running
                    | MobileTimelineItemStatus::WaitingForApproval
                    | MobileTimelineItemStatus::Pending
            )
            && item_matches_pending_tool(row, &card.tool_name)
    })
}

fn approval_with_linked_tool(
    ui_lang: AppLanguage,
    timeline: &MobileTimelineState,
    approval_cards: &[ApprovalCardView],
    on_approval_decision: EventHandler<(String, ReviewDecision)>,
    item: &MobileTimelineItem,
) -> Element {
    let card = pending_card_for_item(approval_cards, item);
    let linked_tool = card.and_then(|card| find_linked_tool_item(timeline, card));

    rsx! {
        div {
            style: "display:flex;justify-content:flex-start;width:100%;gap:8px;align-items:flex-start;",
            div {
                style: "width:28px;height:28px;border-radius:999px;background:#111827;border:1px solid #374151;color:#fde68a;display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:800;flex-shrink:0;margin-top:2px;",
                "!"
            }
            div {
                style: "max-width:92%;display:flex;flex-direction:column;gap:8px;min-width:0;",
                if let Some(card) = card {
                    {inline_approval_card(card, on_approval_decision)}
                } else {
                    {attention_card(ui_lang, item)}
                }
                if let Some(tool) = linked_tool {
                    {compact_tool_card(ui_lang, tool)}
                }
            }
        }
    }
}

fn tool_call_with_inline_approval(
    ui_lang: AppLanguage,
    _timeline: &MobileTimelineState,
    approval_cards: &[ApprovalCardView],
    on_approval_decision: EventHandler<(String, ReviewDecision)>,
    item: &MobileTimelineItem,
) -> Element {
    if item.status != MobileTimelineItemStatus::WaitingForApproval {
        return compact_tool_card(ui_lang, item);
    }
    let card = pending_card_for_item(approval_cards, item);
    rsx! {
        div {
            style: "display:flex;justify-content:flex-start;width:100%;",
            div {
                style: "max-width:94%;display:flex;flex-direction:column;gap:8px;min-width:0;",
                if let Some(card) = card {
                    {inline_approval_card(card, on_approval_decision)}
                }
                {compact_tool_card(ui_lang, item)}
            }
        }
    }
}

fn assistant_thinking_bubble(ui_lang: AppLanguage) -> Element {
    let body = pick(ui_lang, "Работаю…", "Working…");
    rsx! {
        div {
            style: "display:flex;justify-content:flex-start;width:100%;gap:8px;align-items:flex-end;",
            div {
                style: "width:28px;height:28px;border-radius:999px;background:#111827;border:1px solid #374151;color:#93c5fd;display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:800;flex-shrink:0;",
                "AI"
            }
            div {
                style: "background:#172033;color:#cbd5e1;border:1px solid #334155;border-radius:20px 20px 20px 6px;padding:10px 12px;font-size:15px;line-height:1.45;",
                "{body}"
            }
        }
    }
}

fn assistant_activity_panel(
    ui_lang: AppLanguage,
    items: &[&MobileTimelineItem],
    is_open: bool,
    on_toggle: EventHandler<()>,
) -> Element {
    let title = pick(ui_lang, "Ход работы", "Work log");
    let hint = if is_open {
        pick(
            ui_lang,
            "Нажмите, чтобы скрыть детали",
            "Tap to hide details",
        )
    } else {
        pick(
            ui_lang,
            "Нажмите, чтобы показать детали",
            "Tap to show details",
        )
    };
    let arrow = if is_open { "▾" } else { "▸" };
    let state_label = if is_open {
        pick(ui_lang, "открыто", "open")
    } else {
        pick(ui_lang, "скрыто", "hidden")
    };
    let hint_title = pick(
        ui_lang,
        "Нажмите, чтобы показать или скрыть детали",
        "Tap to show or hide details",
    );
    let has_active_item = activity_has_active_item(items);
    let step_count = activity_significant_step_count(items);
    let count_label = match ui_lang {
        AppLanguage::Ru if has_active_item => format!("{step_count} · выполняется"),
        AppLanguage::Ru => format!("{step_count} · {state_label}"),
        AppLanguage::En if has_active_item => format!("{step_count} · running"),
        AppLanguage::En => format!("{step_count} · {state_label}"),
    };

    rsx! {
        div {
            style: "display:flex;justify-content:flex-start;width:100%;gap:8px;align-items:flex-start;text-align:left;",
            div {
                style: "width:28px;height:28px;border-radius:999px;background:#111827;border:1px solid #374151;color:#93c5fd;display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:800;flex-shrink:0;margin-top:2px;",
                "AI"
            }
            div {
                style: "max-width:88%;background:#0b1220;border:1px solid #273244;border-radius:18px;padding:9px 11px;color:#94a3b8;",
                button {
                    style: "appearance:none;background:transparent;border:0;padding:0;margin:0;width:100%;display:flex;align-items:center;gap:9px;font-size:12px;font-weight:700;cursor:pointer;min-width:0;text-align:left;",
                    title: "{hint_title}",
                    onclick: move |_| on_toggle.call(()),
                    span {
                        style: "color:#64748b;white-space:nowrap;",
                        "{arrow}"
                    }
                    span {
                        style: "color:#93c5fd;white-space:nowrap;",
                        "{title}"
                    }
                    span {
                        style: "color:#64748b;font-weight:600;white-space:nowrap;",
                        "{count_label}"
                    }
                }
                div {
                    style: "color:#64748b;font-size:11px;margin-top:6px;margin-bottom:8px;",
                    "{hint}"
                }
                if is_open {
                    div {
                        style: "display:flex;flex-direction:column;gap:6px;max-height:220px;overflow:auto;",
                        for item in items {
                            {activity_row(ui_lang, item)}
                        }
                    }
                }
            }
        }
    }
}

/// Count tool/turn/reasoning steps — not every internal status line.
fn activity_significant_step_count(items: &[&MobileTimelineItem]) -> usize {
    items
        .iter()
        .filter(|item| {
            matches!(
                item.kind,
                MobileTimelineItemKind::ToolCall | MobileTimelineItemKind::Approval
            ) || item.title == "Reasoning"
                || item.title == "Turn finished"
                || item.kind == MobileTimelineItemKind::Error
        })
        .count()
}

fn activity_has_active_item(items: &[&MobileTimelineItem]) -> bool {
    items.iter().any(|item| activity_item_is_in_progress(item))
}

fn activity_item_is_in_progress(item: &MobileTimelineItem) -> bool {
    if !matches!(
        item.status,
        MobileTimelineItemStatus::Pending
            | MobileTimelineItemStatus::Running
            | MobileTimelineItemStatus::WaitingForApproval
    ) {
        return false;
    }
    if item.kind == MobileTimelineItemKind::Status
        && MobileTimelineState::status_message_is_terminal(&item.body)
    {
        return false;
    }
    true
}

fn activity_row(ui_lang: AppLanguage, item: &MobileTimelineItem) -> Element {
    let kind = timeline_kind_label(ui_lang, &item.kind);
    let status = timeline_status_label(ui_lang, &item.status);
    let body = activity_text(item);

    rsx! {
        div {
            style: "display:grid;grid-template-columns:auto 1fr auto;gap:7px;align-items:center;font-size:12px;min-width:0;",
            span {
                style: "background:{item_badge_color(&item.kind)};color:white;border-radius:999px;padding:2px 6px;font-size:10px;font-weight:800;",
                "{kind}"
            }
            span {
                style: "color:#cbd5e1;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;min-width:0;",
                "{body}"
            }
            span {
                style: "color:{status_color(&item.status)};font-weight:700;white-space:nowrap;",
                "{status}"
            }
        }
    }
}

fn activity_text(item: &MobileTimelineItem) -> String {
    let body = compact_status_text(&item.body);
    if body.trim().is_empty() || body == item.title {
        return item.title.clone();
    }
    format!("{} · {}", item.title, body)
}

fn user_message_bubble(item: &MobileTimelineItem) -> Element {
    rsx! {
        div {
            style: "display:flex;justify-content:flex-end;width:100%;",
            div {
                style: "max-width:84%;background:#2563eb;color:white;border-radius:20px 20px 6px 20px;padding:11px 13px;font-size:15px;line-height:1.42;white-space:pre-wrap;overflow-wrap:anywhere;word-break:break-word;box-shadow:0 10px 24px rgba(37,99,235,0.22);",
                "{item.body}"
            }
        }
    }
}

fn assistant_message_bubble(ui_lang: AppLanguage, item: &MobileTimelineItem) -> Element {
    let title = pick(ui_lang, "DeepSeek", "DeepSeek");
    let body = if item.body.trim().is_empty() && item.status == MobileTimelineItemStatus::Running {
        pick(ui_lang, "Печатает…", "Typing…")
    } else {
        item.body.as_str()
    };
    let border = if item.status == MobileTimelineItemStatus::Failed {
        "1px solid #ef4444"
    } else if item.status == MobileTimelineItemStatus::Running {
        "1px solid #3b82f6"
    } else {
        "1px solid #273244"
    };

    rsx! {
        div {
            style: "display:flex;justify-content:flex-start;width:100%;gap:8px;align-items:flex-end;",
            div {
                style: "width:28px;height:28px;border-radius:999px;background:#111827;border:1px solid #374151;color:#93c5fd;display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:800;flex-shrink:0;",
                "AI"
            }
            div {
                style: "max-width:88%;background:#172033;color:#e5e7eb;border:{border};border-radius:20px 20px 20px 6px;padding:10px 12px;box-shadow:0 10px 24px rgba(0,0,0,0.18);",
                div {
                    style: "font-size:11px;color:#93c5fd;font-weight:700;margin-bottom:4px;",
                    "{title}"
                }
                div {
                    style: "font-size:15px;line-height:1.45;white-space:pre-wrap;overflow-wrap:anywhere;word-break:break-word;",
                    "{body}"
                }
            }
        }
    }
}

fn assistant_error_bubble(ui_lang: AppLanguage, item: &MobileTimelineItem) -> Element {
    let title = pick(ui_lang, "Ошибка", "Error");
    let body = compact_status_text(&item.body);

    rsx! {
        div {
            style: "display:flex;justify-content:flex-start;width:100%;gap:8px;align-items:flex-end;",
            div {
                style: "width:28px;height:28px;border-radius:999px;background:#111827;border:1px solid #7f1d1d;color:#fca5a5;display:flex;align-items:center;justify-content:center;font-size:11px;font-weight:800;flex-shrink:0;",
                "AI"
            }
            div {
                style: "max-width:88%;background:#2a1114;color:#fee2e2;border:1px solid #dc2626;border-radius:20px 20px 20px 6px;padding:10px 12px;box-shadow:0 10px 24px rgba(127,29,29,0.18);",
                div {
                    style: "font-size:11px;color:#fca5a5;font-weight:800;margin-bottom:4px;text-transform:uppercase;letter-spacing:0.04em;",
                    "{title}"
                }
                div {
                    style: "font-size:14px;line-height:1.42;white-space:pre-wrap;overflow-wrap:anywhere;word-break:break-word;",
                    "{body}"
                }
            }
        }
    }
}

fn system_status_line(ui_lang: AppLanguage, item: &MobileTimelineItem) -> Element {
    let accent = status_color(&item.status);
    let label = timeline_status_label(ui_lang, &item.status);
    let body = compact_status_text(&item.body);

    rsx! {
        div {
            style: "display:flex;justify-content:center;width:100%;",
            div {
                style: "max-width:92%;background:#0b1220;border:1px solid #1f2937;color:#9ca3af;border-radius:999px;padding:6px 10px;font-size:12px;line-height:1.35;display:flex;gap:7px;align-items:center;",
                span {
                    style: "width:7px;height:7px;border-radius:999px;background:{accent};display:inline-block;flex-shrink:0;",
                    ""
                }
                span {
                    style: "color:{accent};font-weight:700;white-space:nowrap;",
                    "{label}"
                }
                span {
                    style: "white-space:nowrap;overflow:hidden;text-overflow:ellipsis;min-width:0;",
                    "{body}"
                }
            }
        }
    }
}

fn compact_tool_card(ui_lang: AppLanguage, item: &MobileTimelineItem) -> Element {
    let kind = timeline_kind_label(ui_lang, &item.kind);
    let status = timeline_status_label(ui_lang, &item.status);
    let accent = item_badge_color(&item.kind);
    let border = item_border(&item.status);

    rsx! {
        div {
            style: "display:flex;justify-content:flex-start;width:100%;",
            details {
                style: "max-width:94%;background:#0f172a;border:{border};border-radius:14px;padding:8px 10px;color:#cbd5e1;",
                summary {
                    style: "display:flex;align-items:center;gap:7px;font-size:12px;font-weight:700;list-style:none;",
                    span {
                        style: "background:{accent};color:white;border-radius:999px;padding:2px 7px;font-size:10px;",
                        "{kind}"
                    }
                    span {
                        style: "color:#e5e7eb;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;min-width:0;",
                        "{item.title}"
                    }
                    span {
                        style: "color:{status_color(&item.status)};font-weight:600;margin-left:auto;",
                        "{status}"
                    }
                }
                div {
                    style: "margin-top:8px;color:#9ca3af;font-size:12px;line-height:1.4;white-space:pre-wrap;max-height:150px;overflow:auto;",
                    "{item.body}"
                }
            }
        }
    }
}

fn attention_card(ui_lang: AppLanguage, item: &MobileTimelineItem) -> Element {
    let kind = timeline_kind_label(ui_lang, &item.kind);
    let status = timeline_status_label(ui_lang, &item.status);

    rsx! {
        div {
            style: "background:{item_background(&item.kind)};border:{item_border(&item.status)};border-radius:16px;padding:11px 12px;display:flex;flex-direction:column;gap:7px;",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;gap:8px;",
                div {
                    style: "display:flex;align-items:center;gap:7px;min-width:0;",
                    span {
                        style: "background:{item_badge_color(&item.kind)};color:white;border-radius:999px;padding:3px 7px;font-size:10px;font-weight:800;",
                        "{kind}"
                    }
                    span {
                        style: "color:#f9fafb;font-size:13px;font-weight:800;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                        "{item.title}"
                    }
                }
                span {
                    style: "color:{status_color(&item.status)};font-size:11px;font-weight:700;white-space:nowrap;",
                    "{status}"
                }
            }
            div {
                style: "color:#e5e7eb;font-size:13px;line-height:1.45;white-space:pre-wrap;",
                "{item.body}"
            }
        }
    }
}

fn compact_status_text(text: &str) -> String {
    let normalized = text.replace('\n', " · ");
    let max_chars = 120;
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut out = normalized.chars().take(max_chars).collect::<String>();
    out.push('…');
    out
}

fn item_background(kind: &MobileTimelineItemKind) -> &'static str {
    match kind {
        MobileTimelineItemKind::UserMessage => "#2563eb",
        MobileTimelineItemKind::AssistantMessage => "#172033",
        MobileTimelineItemKind::Attachment => "#0f172a",
        MobileTimelineItemKind::NativeCommand => "#0f172a",
        MobileTimelineItemKind::ToolCall => "#0f172a",
        MobileTimelineItemKind::Approval => "#422006",
        MobileTimelineItemKind::Status => "#0b1220",
        MobileTimelineItemKind::Error => "#7f1d1d",
    }
}

fn item_border(status: &MobileTimelineItemStatus) -> &'static str {
    match status {
        MobileTimelineItemStatus::Pending => "1px solid #4b5563",
        MobileTimelineItemStatus::Running => "1px solid #3b82f6",
        MobileTimelineItemStatus::Done => "1px solid #334155",
        MobileTimelineItemStatus::Failed => "1px solid #dc2626",
        MobileTimelineItemStatus::WaitingForApproval => "1px solid #ca8a04",
    }
}

fn item_badge_color(kind: &MobileTimelineItemKind) -> &'static str {
    match kind {
        MobileTimelineItemKind::UserMessage => "#1d4ed8",
        MobileTimelineItemKind::AssistantMessage => "#3b82f6",
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
        MobileTimelineItemStatus::Pending => "#94a3b8",
        MobileTimelineItemStatus::Running => "#93c5fd",
        MobileTimelineItemStatus::Done => "#86efac",
        MobileTimelineItemStatus::Failed => "#fca5a5",
        MobileTimelineItemStatus::WaitingForApproval => "#fde68a",
    }
}

#[cfg(test)]
mod tests {
    use super::{compact_status_text, item_background, item_badge_color, item_border};
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

    #[test]
    fn compact_status_text_limits_noise() {
        let text = "a".repeat(200);
        assert!(compact_status_text(&text).chars().count() <= 121);
    }
}

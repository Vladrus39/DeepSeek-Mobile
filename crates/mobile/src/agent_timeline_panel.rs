use crate::agent_timeline::{
    split_tool_step_body, tool_name_from_step_title, MobileTimelineItem, MobileTimelineItemKind,
    MobileTimelineItemStatus, MobileTimelineState,
};
use crate::locale::{pick, timeline_kind_label, timeline_status_label, AppLanguage};
use crate::mobile_approval_panel::inline_approval_card;
use deepseek_mobile_core::{ApprovalCardStatus, ApprovalCardView, ReviewDecision};
use dioxus::prelude::*;
use serde_json::Value;

/// Inline snapshot rollback from chat (confirm + restore wired in `lib.rs`).
#[derive(Clone, PartialEq)]
pub struct ChatSnapshotRollbackProps {
    pub latest_id: Option<String>,
    pub latest_summary: Option<String>,
    pub pending: Option<(String, usize, u64)>,
    pub restore_in_progress: bool,
    pub on_request_restore: EventHandler<String>,
    pub on_confirm_restore: EventHandler<()>,
    pub on_cancel_restore: EventHandler<()>,
}

pub fn agent_timeline_panel(
    ui_lang: AppLanguage,
    timeline: &MobileTimelineState,
    approval_cards: &[ApprovalCardView],
    on_approval_decision: EventHandler<(String, ReviewDecision)>,
    on_open_file_path: EventHandler<String>,
    on_open_project_folder: EventHandler<()>,
    activity_open: bool,
    on_activity_toggle: EventHandler<()>,
    snapshot_rollback: ChatSnapshotRollbackProps,
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
                    {assistant_activity_panel(
                        ui_lang,
                        &activity_items,
                        activity_open,
                        on_activity_toggle,
                        on_open_file_path,
                        on_open_project_folder,
                        snapshot_rollback.clone(),
                    )}
                }
                {chat_snapshot_restore_confirm(ui_lang, &snapshot_rollback)}
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

            {chat_snapshot_restore_confirm(ui_lang, &snapshot_rollback)}

            if !activity_items.is_empty() {
                {assistant_activity_panel(
                    ui_lang,
                    &activity_items,
                    activity_open,
                    on_activity_toggle,
                    on_open_file_path,
                    on_open_project_folder,
                    snapshot_rollback.clone(),
                )}
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
                    on_open_file_path,
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
    on_open_file_path: EventHandler<String>,
    item: &MobileTimelineItem,
) -> Element {
    match item.kind {
        MobileTimelineItemKind::UserMessage => user_message_bubble(item),
        MobileTimelineItemKind::AssistantMessage => assistant_message_bubble(ui_lang, item),
        MobileTimelineItemKind::Status => system_status_line(ui_lang, item),
        MobileTimelineItemKind::Attachment | MobileTimelineItemKind::NativeCommand => {
            compact_tool_card(ui_lang, item, on_open_file_path)
        }
        MobileTimelineItemKind::ToolCall => tool_call_with_inline_approval(
            ui_lang,
            timeline,
            approval_cards,
            on_approval_decision,
            on_open_file_path,
            item,
        ),
        MobileTimelineItemKind::Approval => approval_with_linked_tool(
            ui_lang,
            timeline,
            approval_cards,
            on_approval_decision,
            on_open_file_path,
            item,
        ),
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
    on_open_file_path: EventHandler<String>,
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
                    {compact_tool_card(ui_lang, tool, on_open_file_path)}
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
    on_open_file_path: EventHandler<String>,
    item: &MobileTimelineItem,
) -> Element {
    if item.status != MobileTimelineItemStatus::WaitingForApproval {
        return compact_tool_card(ui_lang, item, on_open_file_path);
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
                {compact_tool_card(ui_lang, item, on_open_file_path)}
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

fn activity_linked_paths(items: &[&MobileTimelineItem]) -> Vec<String> {
    let mut paths = Vec::new();
    for item in items {
        for path in &item.linked_file_paths {
            if !paths.iter().any(|existing| existing == path) {
                paths.push(path.clone());
            }
        }
    }
    paths
}

fn assistant_activity_panel(
    ui_lang: AppLanguage,
    items: &[&MobileTimelineItem],
    is_open: bool,
    on_toggle: EventHandler<()>,
    on_open_file_path: EventHandler<String>,
    on_open_project_folder: EventHandler<()>,
    snapshot_rollback: ChatSnapshotRollbackProps,
) -> Element {
    let has_linked_files = !activity_linked_paths(items).is_empty();
    let title = pick(ui_lang, "Ход работы", "Work log");
    let hint = if is_open {
        pick(
            ui_lang,
            "Нажмите на шаг ниже — вход/вывод инструмента. Свернуть список — по заголовку.",
            "Tap a step below for tool input/output. Collapse the list via the header.",
        )
    } else {
        pick(
            ui_lang,
            "Откройте список и нажмите на шаг — полные аргументы и результат инструмента.",
            "Open the list and tap a step to see full tool arguments and output.",
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
                if has_linked_files {
                    {open_project_folder_button(ui_lang, on_open_project_folder)}
                }
                if let Some(ref latest_id) = snapshot_rollback.latest_id {
                    if snapshot_rollback.pending.is_none() {
                        {snapshot_rollback_request_button(
                            ui_lang,
                            latest_id,
                            snapshot_rollback.latest_summary.as_deref(),
                            true,
                            snapshot_rollback.on_request_restore,
                            snapshot_rollback.restore_in_progress,
                        )}
                    }
                }
                if is_open {
                    div {
                        style: "display:flex;flex-direction:column;gap:8px;max-height:min(52vh, 480px);overflow:auto;padding-right:2px;",
                        for item in items {
                            {activity_step_card(ui_lang, item, on_open_file_path)}
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

fn activity_step_card(
    ui_lang: AppLanguage,
    item: &MobileTimelineItem,
    on_open_file_path: EventHandler<String>,
) -> Element {
    let kind = timeline_kind_label(ui_lang, &item.kind);
    let status = timeline_status_label(ui_lang, &item.status);
    let accent = item_badge_color(&item.kind);
    let border = item_border(&item.status);
    let headline = activity_step_headline(ui_lang, item);
    let expand_hint = pick(
        ui_lang,
        "Нажмите, чтобы раскрыть шаг",
        "Tap to expand this step",
    );

    rsx! {
        details {
            style: "background:#0f172a;border:{border};border-radius:12px;padding:7px 9px;color:#cbd5e1;min-width:0;",
            summary {
                style: "display:flex;align-items:center;gap:7px;font-size:12px;font-weight:700;list-style:none;cursor:pointer;min-width:0;",
                title: "{expand_hint}",
                span {
                    style: "background:{accent};color:white;border-radius:999px;padding:2px 7px;font-size:10px;flex-shrink:0;",
                    "{kind}"
                }
                span {
                    style: "color:#e5e7eb;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;min-width:0;flex:1;",
                    "{headline}"
                }
                span {
                    style: "color:{status_color(&item.status)};font-weight:600;flex-shrink:0;white-space:nowrap;",
                    "{status}"
                }
            }
            {activity_step_detail_body(ui_lang, item)}
            {file_path_link_buttons(ui_lang, &item.linked_file_paths, on_open_file_path)}
        }
    }
}

fn activity_step_headline(ui_lang: AppLanguage, item: &MobileTimelineItem) -> String {
    if item.kind == MobileTimelineItemKind::ToolCall {
        if let Some(name) = tool_name_from_step_title(&item.title) {
            if let Some((input, _)) = split_tool_step_body(&item.body) {
                if let Ok(value) = serde_json::from_str::<Value>(input) {
                    if let Some(path) = value.get("path").and_then(Value::as_str) {
                        return format!("{name} · {path}");
                    }
                    if let Some(command) = value.get("command").and_then(Value::as_str) {
                        let short = compact_status_text(command);
                        return format!("{name} · {short}");
                    }
                }
            }
            return name.to_string();
        }
    }
    if item.kind == MobileTimelineItemKind::Status && item.title == "Reasoning" {
        return pick(ui_lang, "Размышление", "Reasoning").to_string();
    }
    activity_text(item)
}

fn activity_step_detail_body(ui_lang: AppLanguage, item: &MobileTimelineItem) -> Element {
    if item.kind == MobileTimelineItemKind::ToolCall {
        return tool_step_detail_sections(ui_lang, item);
    }
    if item.kind == MobileTimelineItemKind::Status && item.title == "Reasoning" {
        let label = pick(ui_lang, "Ход мыслей", "Thought process");
        return rsx! {
            div {
                style: "margin-top:8px;display:flex;flex-direction:column;gap:6px;",
                div {
                    style: "font-size:10px;font-weight:800;color:#64748b;text-transform:uppercase;letter-spacing:0.05em;",
                    "{label}"
                }
                pre {
                    style: "margin:0;color:#cbd5e1;font-size:12px;line-height:1.45;white-space:pre-wrap;overflow-wrap:anywhere;word-break:break-word;max-height:min(40vh, 320px);overflow:auto;font-family:ui-monospace,monospace;",
                    "{item.body}"
                }
            }
        };
    }
    let label = pick(ui_lang, "Детали", "Details");
    let body = pretty_step_text(&item.body);
    rsx! {
        div {
            style: "margin-top:8px;display:flex;flex-direction:column;gap:6px;",
            div {
                style: "font-size:10px;font-weight:800;color:#64748b;text-transform:uppercase;letter-spacing:0.05em;",
                "{label}"
            }
            pre {
                style: "margin:0;color:#9ca3af;font-size:12px;line-height:1.4;white-space:pre-wrap;overflow-wrap:anywhere;word-break:break-word;max-height:min(36vh, 280px);overflow:auto;font-family:ui-monospace,monospace;",
                "{body}"
            }
        }
    }
}

fn tool_step_detail_sections(ui_lang: AppLanguage, item: &MobileTimelineItem) -> Element {
    let input_label = pick(ui_lang, "Вход (аргументы)", "Input (arguments)");
    let output_label = pick(ui_lang, "Выход (результат)", "Output (result)");
    let running_label = pick(ui_lang, "Выполняется…", "Running…");
    let running = item.status == MobileTimelineItemStatus::Running;

    if let Some((input, output)) = split_tool_step_body(&item.body) {
        let input_text = pretty_step_text(input);
        let output_text = pretty_step_text(output);
        return rsx! {
            div {
                style: "margin-top:8px;display:flex;flex-direction:column;gap:10px;",
                div {
                    style: "display:flex;flex-direction:column;gap:4px;",
                    div {
                        style: "font-size:10px;font-weight:800;color:#64748b;text-transform:uppercase;letter-spacing:0.05em;",
                        "{input_label}"
                    }
                    pre {
                        style: "margin:0;color:#cbd5e1;font-size:12px;line-height:1.4;white-space:pre-wrap;overflow-wrap:anywhere;word-break:break-word;max-height:min(28vh, 220px);overflow:auto;font-family:ui-monospace,monospace;",
                        "{input_text}"
                    }
                }
                if !output.trim().is_empty() {
                    div {
                        style: "display:flex;flex-direction:column;gap:4px;",
                        div {
                            style: "font-size:10px;font-weight:800;color:#64748b;text-transform:uppercase;letter-spacing:0.05em;",
                            "{output_label}"
                        }
                        pre {
                            style: "margin:0;color:#86efac;font-size:12px;line-height:1.4;white-space:pre-wrap;overflow-wrap:anywhere;word-break:break-word;max-height:min(36vh, 300px);overflow:auto;font-family:ui-monospace,monospace;",
                            "{output_text}"
                        }
                    }
                } else if running {
                    div {
                        style: "font-size:12px;color:#93c5fd;font-weight:600;",
                        "{running_label}"
                    }
                }
            }
        };
    }

    let body = pretty_step_text(&item.body);
    rsx! {
        div {
            style: "margin-top:8px;display:flex;flex-direction:column;gap:4px;",
            div {
                style: "font-size:10px;font-weight:800;color:#64748b;text-transform:uppercase;letter-spacing:0.05em;",
                "{input_label}"
            }
            pre {
                style: "margin:0;color:#cbd5e1;font-size:12px;line-height:1.4;white-space:pre-wrap;overflow-wrap:anywhere;word-break:break-word;max-height:min(40vh, 320px);overflow:auto;font-family:ui-monospace,monospace;",
                "{body}"
            }
            if running {
                div {
                    style: "font-size:12px;color:#93c5fd;font-weight:600;margin-top:4px;",
                    "{running_label}"
                }
            }
        }
    }
}

fn pretty_step_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if let Ok(pretty) = serde_json::to_string_pretty(&value) {
            return pretty;
        }
    }
    trimmed.to_string()
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

fn open_project_folder_button(ui_lang: AppLanguage, on_open: EventHandler<()>) -> Element {
    let label = pick(ui_lang, "Открыть в проводнике", "Open in file manager");
    rsx! {
        button {
            style: "align-self:flex-start;background:#1e3a5f;color:#93c5fd;border:1px solid #3b82f6;border-radius:10px;padding:6px 10px;font-size:12px;font-weight:600;margin-top:4px;",
            onclick: move |_| on_open.call(()),
            "{label}"
        }
    }
}

fn file_path_link_buttons(
    ui_lang: AppLanguage,
    paths: &[String],
    on_open_file_path: EventHandler<String>,
) -> Element {
    if paths.is_empty() {
        return rsx! {};
    }
    let open_label = pick(ui_lang, "Открыть в Файлах", "Open in Files");
    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:6px;margin-top:8px;",
            for path in paths.iter().cloned() {
                {
                    let label = format!("{open_label}: {path}");
                    rsx! {
                        button {
                            style: "align-self:flex-start;background:#1e3a5f;color:#93c5fd;border:1px solid #3b82f6;border-radius:10px;padding:6px 10px;font-size:12px;font-weight:600;max-width:100%;text-align:left;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;",
                            title: "{path}",
                            onclick: move |_| on_open_file_path.call(path.clone()),
                            "{label}"
                        }
                    }
                }
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

fn chat_snapshot_restore_confirm(
    ui_lang: AppLanguage,
    rollback: &ChatSnapshotRollbackProps,
) -> Element {
    let Some((snapshot_id, file_count, total_bytes)) = rollback.pending.clone() else {
        return rsx! {};
    };
    let title = pick(ui_lang, "Восстановить снимок?", "Restore snapshot?");
    let warning = pick(
        ui_lang,
        "Файлы workspace будут перезаписаны; лишние файлы удалятся.",
        "Workspace files will be overwritten; extra files will be removed.",
    );
    let files_label = pick(ui_lang, "Файлов в снимке", "Files in snapshot");
    let size_label = pick(ui_lang, "Размер", "Size");
    let confirm_label = pick(ui_lang, "Подтвердить откат", "Confirm restore");
    let cancel_label = pick(ui_lang, "Отмена", "Cancel");
    let running = rollback.restore_in_progress;
    let on_confirm = rollback.on_confirm_restore;
    let on_cancel = rollback.on_cancel_restore;

    rsx! {
        div {
            style: "display:flex;justify-content:center;width:100%;",
            div {
                style: "max-width:92%;background:#422006;border:2px solid #ca8a04;border-radius:16px;padding:12px 14px;color:#fef3c7;display:flex;flex-direction:column;gap:8px;",
                div {
                    style: "font-size:15px;font-weight:800;color:#fde68a;",
                    "{title}"
                }
                div { style: "font-size:13px;line-height:1.4;", "{warning}" }
                div { style: "font-size:12px;color:#fde68a;", "ID: {snapshot_id}" }
                div { style: "font-size:12px;color:#d1d5db;", "{files_label}: {file_count}" }
                div { style: "font-size:12px;color:#d1d5db;", "{size_label}: {total_bytes} bytes" }
                div {
                    style: "display:flex;gap:10px;flex-wrap:wrap;margin-top:4px;",
                    button {
                        style: "background:#dc2626;color:white;border:none;border-radius:10px;padding:8px 14px;font-size:13px;font-weight:700;",
                        disabled: running,
                        onclick: move |_| on_confirm.call(()),
                        "{confirm_label}"
                    }
                    button {
                        style: "background:#374151;color:white;border:none;border-radius:10px;padding:8px 14px;font-size:13px;font-weight:600;",
                        disabled: running,
                        onclick: move |_| on_cancel.call(()),
                        "{cancel_label}"
                    }
                }
            }
        }
    }
}

fn snapshot_rollback_request_button(
    ui_lang: AppLanguage,
    snapshot_id: &str,
    summary: Option<&str>,
    prominent: bool,
    on_request: EventHandler<String>,
    disabled: bool,
) -> Element {
    let label = pick(
        ui_lang,
        "Откатить к safety snapshot",
        "Rollback to safety snapshot",
    );
    let hint = summary.unwrap_or(snapshot_id);
    let style = if prominent {
        "margin-top:8px;align-self:flex-start;background:#422006;color:#fde68a;border:1px solid #ca8a04;border-radius:10px;padding:7px 11px;font-size:12px;font-weight:700;max-width:100%;text-align:left;"
    } else {
        "margin-top:6px;align-self:flex-start;background:#1e293b;color:#fcd34d;border:1px solid #b45309;border-radius:10px;padding:5px 9px;font-size:11px;font-weight:600;max-width:100%;text-align:left;"
    };
    let snapshot_id = snapshot_id.to_string();
    rsx! {
        button {
            style: "{style}",
            title: "{hint}",
            disabled: disabled,
            onclick: move |_| on_request.call(snapshot_id.clone()),
            "{label}"
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

fn compact_tool_card(
    ui_lang: AppLanguage,
    item: &MobileTimelineItem,
    on_open_file_path: EventHandler<String>,
) -> Element {
    let kind = timeline_kind_label(ui_lang, &item.kind);
    let status = timeline_status_label(ui_lang, &item.status);
    let accent = item_badge_color(&item.kind);
    let border = item_border(&item.status);
    let headline = activity_step_headline(ui_lang, item);
    let expand_hint = pick(
        ui_lang,
        "Нажмите, чтобы раскрыть шаг",
        "Tap to expand this step",
    );

    rsx! {
        div {
            style: "display:flex;justify-content:flex-start;width:100%;",
            details {
                style: "max-width:94%;background:#0f172a;border:{border};border-radius:14px;padding:8px 10px;color:#cbd5e1;",
                summary {
                    style: "display:flex;align-items:center;gap:7px;font-size:12px;font-weight:700;list-style:none;cursor:pointer;min-width:0;",
                    title: "{expand_hint}",
                    span {
                        style: "background:{accent};color:white;border-radius:999px;padding:2px 7px;font-size:10px;flex-shrink:0;",
                        "{kind}"
                    }
                    span {
                        style: "color:#e5e7eb;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;min-width:0;flex:1;",
                        "{headline}"
                    }
                    span {
                        style: "color:{status_color(&item.status)};font-weight:600;margin-left:auto;flex-shrink:0;",
                        "{status}"
                    }
                }
                {tool_step_detail_sections(ui_lang, item)}
                {file_path_link_buttons(ui_lang, &item.linked_file_paths, on_open_file_path)}
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

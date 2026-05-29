use crate::locale::{pick, tr, AppLanguage, Tr};
use crate::runtime_health::RuntimeHealthSnapshot;
use crate::ui_layout::screen_layout;
use deepseek_mobile_core::config::ExecutionMode;
use dioxus::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HealthQuickAction {
    RunTermuxCheck,
    OpenSettings,
    OpenPcHost,
    OpenFiles,
}

pub fn health_panel(
    lang: AppLanguage,
    snapshot: RuntimeHealthSnapshot,
    on_quick_action: EventHandler<HealthQuickAction>,
) -> Element {
    let layout = screen_layout();
    let agent_mode_ok = snapshot.execution_mode == ExecutionMode::Agent;
    let data_prefix = pick(lang, "Данные:", "Data:");
    let next_steps_label = pick(lang, "Дальше", "Next steps");
    let offline_hint = pick(
        lang,
        "Офлайн-сборка Android: tools/android/DOWNLOAD_BUDGET.md",
        "Offline Android setup: tools/android/DOWNLOAD_BUDGET.md",
    );
    let mcp_status = if lang == AppLanguage::Ru {
        format!(
            "{}/{} подключено",
            snapshot.mcp_servers_connected, snapshot.mcp_servers_total
        )
    } else {
        format!(
            "{}/{} connected",
            snapshot.mcp_servers_connected, snapshot.mcp_servers_total
        )
    };
    let mode_label = match (lang, snapshot.execution_mode) {
        (AppLanguage::Ru, ExecutionMode::Plan) => "Plan (инструменты выкл.)",
        (AppLanguage::En, ExecutionMode::Plan) => "Plan (tools disabled)",
        (AppLanguage::Ru, ExecutionMode::Agent) => "Agent (рекомендуется)",
        (AppLanguage::En, ExecutionMode::Agent) => "Agent (recommended)",
        (AppLanguage::Ru, ExecutionMode::Yolo) => "YOLO (авто-одобрение)",
        (AppLanguage::En, ExecutionMode::Yolo) => "YOLO (auto-approve)",
    };

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:clamp(10px,2.5vw,14px);color:white;max-width:100%;",
            div {
                style: "font-size:{layout.header_title_font};font-weight:bold;",
                "{tr(lang, Tr::HealthTitle)}"
            }
            div {
                style: "color:#9ca3af;font-size:{layout.subtitle_font};line-height:1.4;",
                "{tr(lang, Tr::HealthSubtitle)}"
            }

            {health_row_action(
                pick(lang, "DeepSeek API", "DeepSeek API"),
                if snapshot.api_configured {
                    pick(lang, "Настроен", "Configured")
                } else {
                    pick(lang, "Не задан", "Missing")
                },
                snapshot.api_configured,
                pick(lang, "Настройки", "Settings"),
                true,
                EventHandler::new(move |_| on_quick_action.call(HealthQuickAction::OpenSettings)),
            )}
            div {
                style: "color:#6b7280;font-size:11px;word-break:break-all;",
                "{data_prefix} {snapshot.data_dir_display}"
            }
            {health_row(
                pick(lang, "Полный агент на телефоне", "Full agent on phone"),
                if snapshot.full_agent_on_phone_ready {
                    pick(lang, "Готов (API + Termux)", "Ready (API + Termux)")
                } else if snapshot.termux_valid {
                    pick(lang, "Termux OK — добавьте API", "Termux OK — add API key")
                } else {
                    pick(lang, "Укажите путь Termux", "Set up Termux path")
                },
                snapshot.full_agent_on_phone_ready,
            )}
            {health_row(
                pick(lang, "Режим выполнения", "Execution mode"),
                mode_label,
                agent_mode_ok,
            )}
            {health_row(
                pick(lang, "Рабочая область Termux", "Termux workspace"),
                if snapshot.termux_valid {
                    pick(lang, "OK — основной исполнитель", "Valid — primary executor")
                } else if snapshot.termux_configured {
                    pick(lang, "Неверный путь", "Path invalid")
                } else {
                    pick(lang, "Не настроено", "Not configured")
                },
                snapshot.termux_valid,
            )}
            {health_row_action(
                pick(lang, "PC Host (ping)", "PC Host (ping)"),
                &snapshot.pc_status_label,
                snapshot.pc_online,
                pick(lang, "Открыть PC", "Open PC"),
                snapshot.pc_online || snapshot.pc_status_label.contains("Waiting") || snapshot.pc_status_label.contains("Ожидание"),
                EventHandler::new(move |_| on_quick_action.call(HealthQuickAction::OpenPcHost)),
            )}
            {health_row_action(
                pick(lang, "Файлы PC (sync)", "PC files (sync)"),
                if snapshot.pc_files_sync_ready {
                    pick(lang, "Готово — агент видит PC", "Ready — agent sees PC")
                } else if snapshot.pc_online {
                    pick(lang, "Нужен pairing request", "Needs pairing request")
                } else {
                    pick(lang, "Не используется", "Not used")
                },
                snapshot.pc_files_sync_ready,
                pick(lang, "Файлы", "Files"),
                snapshot.pc_files_sync_ready || snapshot.pc_online,
                EventHandler::new(move |_| on_quick_action.call(HealthQuickAction::OpenFiles)),
            )}
            {health_row(
                pick(lang, "MCP-серверы", "MCP servers"),
                mcp_status.as_str(),
                snapshot.mcp_servers_connected > 0 || snapshot.mcp_servers_total == 0,
            )}
            {health_row(
                pick(lang, "Мост Android", "Native bridge"),
                if snapshot.native_pending {
                    pick(lang, "Ожидание callback", "Waiting for Android callback")
                } else if snapshot.native_last_error.is_some() {
                    pick(lang, "Ошибка (ниже)", "Error (see below)")
                } else {
                    pick(lang, "Простой", "Idle")
                },
                !snapshot.native_pending && snapshot.native_last_error.is_none(),
            )}

            if let Some(error) = snapshot.native_last_error {
                div {
                    background_color: "#7f1d1d",
                    border: "1px solid #dc2626",
                    border_radius: "12px",
                    padding: "10px",
                    font_size: "12px",
                    "{error}"
                }
            }

            if !snapshot.network_hints.is_empty() {
                div {
                    background_color: "#111827",
                    border: "1px solid #374151",
                    border_radius: "14px",
                    padding: "12px",
                    display: "flex",
                    flex_direction: "column",
                    gap: "6px",

                    for hint in snapshot.network_hints {
                        div { color: "#9ca3af", font_size: "11px", "{hint}" }
                    }
                }
            }

            if snapshot.termux_valid && snapshot.api_configured {
                button {
                    style: "background:#1d4ed8;color:white;border:none;border-radius:10px;padding:clamp(8px,2.5vw,12px);font-size:{layout.subtitle_font};font-weight:bold;min-height:44px;",
                    onclick: move |_| on_quick_action.call(HealthQuickAction::RunTermuxCheck),
                    if lang == AppLanguage::Ru { "Проверить Termux (pwd)" } else { "Test Termux (pwd)" }
                }
            }

            if !snapshot.recommendations.is_empty() {
                div {
                    background_color: "#111827",
                    border: "1px solid #374151",
                    border_radius: "14px",
                    padding: "12px",
                    display: "flex",
                    flex_direction: "column",
                    gap: "8px",

                    div {
                        font_weight: "bold",
                        font_size: "13px",
                        "{next_steps_label}"
                    }
                    for line in snapshot.recommendations {
                        div { color: "#d1d5db", font_size: "12px", "• {line}" }
                    }
                }
            }

            div {
                color: "#6b7280",
                font_size: "11px",
                "{offline_hint}"
            }
        }
    }
}

fn health_row_action(
    label: &str,
    value: &str,
    ok: bool,
    action_label: &str,
    show_action: bool,
    on_action: EventHandler<()>,
) -> Element {
    let (bg, border) = if ok {
        ("#064e3b", "#10b981")
    } else {
        ("#1f2937", "#4b5563")
    };
    let border_style = format!("1px solid {border}");
    rsx! {
        div {
            background_color: bg,
            border: "{border_style}",
            border_radius: "12px",
            padding: "10px 12px",
            display: "flex",
            flex_direction: "column",
            gap: "8px",
            font_size: "12px",
            div {
                display: "flex",
                justify_content: "space-between",
                gap: "12px",
                span { font_weight: "bold", "{label}" }
                span { color: "#d1d5db", text_align: "right", "{value}" }
            }
            if show_action {
                button {
                    style: "align-self:flex-end;background:#1d4ed8;color:white;border:none;border-radius:8px;padding:6px 10px;font-size:11px;font-weight:bold;",
                    onclick: move |_| on_action.call(()),
                    "{action_label}"
                }
            }
        }
    }
}

fn health_row(label: &str, value: &str, ok: bool) -> Element {
    let (bg, border) = if ok {
        ("#064e3b", "#10b981")
    } else {
        ("#1f2937", "#4b5563")
    };
    let border_style = format!("1px solid {border}");
    rsx! {
        div {
            background_color: bg,
            border: "{border_style}",
            border_radius: "12px",
            padding: "10px 12px",
            display: "flex",
            justify_content: "space-between",
            gap: "12px",
            font_size: "12px",
            span { font_weight: "bold", "{label}" }
            span { color: "#d1d5db", text_align: "right", "{value}" }
        }
    }
}

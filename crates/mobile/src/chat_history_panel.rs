use crate::chat_session::ChatSessionIndex;
use crate::locale::{pick, AppLanguage};
use dioxus::prelude::*;

pub fn chat_history_panel(
    lang: AppLanguage,
    index: &ChatSessionIndex,
    active_project_title: &str,
    active_project_subtitle: &str,
    on_select_chat: EventHandler<String>,
    on_new_chat: EventHandler<()>,
    on_open_files: EventHandler<()>,
    on_delete_chat: EventHandler<String>,
) -> Element {
    let delete_label = pick(lang, "Удалить", "Delete");
    let title = pick(lang, "История", "History");
    let chats_title = pick(lang, "Чаты", "Chats");
    let project_title = pick(lang, "Активный проект", "Active project");
    let project_hint = pick(
        lang,
        "Старые проекты и файлы открываются через вкладку «Файлы» или подключение PC Host.",
        "Older projects and files are opened through Files or the PC Host connection.",
    );
    let open_files = pick(lang, "Открыть файлы", "Open files");
    let new_chat = pick(lang, "Новый чат", "New chat");
    let empty = pick(lang, "История чатов пока пустая.", "Chat history is empty.");

    let mut threads = index.threads.clone();
    threads.sort_by(|a, b| b.updated_at_unix.cmp(&a.updated_at_unix));

    rsx! {
        div {
            style: "background:#0b1220;border:1px solid #273244;border-radius:16px;padding:10px;display:flex;flex-direction:column;gap:10px;margin-bottom:6px;",

            div {
                style: "display:flex;align-items:center;justify-content:space-between;gap:8px;",
                div {
                    style: "color:#e5e7eb;font-size:14px;font-weight:800;",
                    "{title}"
                }
                button {
                    style: "background:#172033;color:#93c5fd;border:1px solid #334155;border-radius:999px;padding:6px 10px;font-size:12px;",
                    onclick: move |_| on_new_chat.call(()),
                    "{new_chat}"
                }
            }

            div {
                style: "background:#101827;border:1px solid #1f2937;border-radius:12px;padding:9px;display:flex;flex-direction:column;gap:5px;",
                div {
                    style: "color:#60a5fa;font-size:12px;font-weight:800;text-transform:uppercase;letter-spacing:0.04em;",
                    "{project_title}"
                }
                div {
                    style: "color:#f9fafb;font-size:14px;font-weight:700;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                    "{active_project_title}"
                }
                div {
                    style: "color:#9ca3af;font-size:12px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                    "{active_project_subtitle}"
                }
                div {
                    style: "color:#64748b;font-size:11px;line-height:1.35;",
                    "{project_hint}"
                }
                button {
                    style: "align-self:flex-start;background:#1d4ed8;color:white;border:0;border-radius:999px;padding:7px 11px;font-size:12px;",
                    onclick: move |_| on_open_files.call(()),
                    "{open_files}"
                }
            }

            div {
                style: "display:flex;align-items:center;justify-content:space-between;",
                div {
                    style: "color:#e5e7eb;font-size:13px;font-weight:800;",
                    "{chats_title}"
                }
                div {
                    style: "color:#64748b;font-size:12px;",
                    "{threads.len()}"
                }
            }

            if threads.is_empty() {
                div {
                    style: "color:#9ca3af;font-size:12px;padding:8px 0;",
                    "{empty}"
                }
            } else {
                div {
                    style: "display:flex;gap:7px;overflow-x:auto;padding-bottom:2px;",
                    for thread in threads {
                        {
                            let thread_id = thread.id.clone();
                            let thread_id_delete = thread_id.clone();
                            let active = thread.id == index.active_thread_id;
                            rsx! {
                                div {
                                    key: "{thread.id}",
                                    style: "display:flex;gap:4px;align-items:stretch;flex-shrink:0;",
                                    button {
                                        style: if active {
                                            "min-width:150px;max-width:190px;background:#1e3a8a;color:white;border:1px solid #3b82f6;border-radius:14px;padding:9px;text-align:left;"
                                        } else {
                                            "min-width:150px;max-width:190px;background:#111827;color:#e5e7eb;border:1px solid #273244;border-radius:14px;padding:9px;text-align:left;"
                                        },
                                        onclick: move |_| on_select_chat.call(thread_id.clone()),
                                        div {
                                            style: "font-size:13px;font-weight:800;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                                            "{thread.title}"
                                        }
                                        div {
                                            style: "color:#94a3b8;font-size:10px;margin-top:3px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                                            "{thread.id}"
                                        }
                                    }
                                    button {
                                        style: "background:#7f1d1d;color:#fecaca;border:1px solid #dc2626;border-radius:14px;padding:0 8px;font-size:11px;font-weight:bold;",
                                        title: "{delete_label}",
                                        onclick: move |_| on_delete_chat.call(thread_id_delete.clone()),
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

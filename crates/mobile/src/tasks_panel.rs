use crate::tasks_state::TasksUiState;
use deepseek_mobile_core::DurableTaskStatus;
use dioxus::prelude::*;

fn status_color(status: &DurableTaskStatus) -> &'static str {
    match status {
        DurableTaskStatus::Queued => "#6b7280",
        DurableTaskStatus::Running => "#2563eb",
        DurableTaskStatus::Completed => "#16a34a",
        DurableTaskStatus::Failed => "#dc2626",
        DurableTaskStatus::Canceled => "#ca8a04",
    }
}

fn status_label(status: &DurableTaskStatus) -> &'static str {
    match status {
        DurableTaskStatus::Queued => "Queued",
        DurableTaskStatus::Running => "Running",
        DurableTaskStatus::Completed => "Done",
        DurableTaskStatus::Failed => "Failed",
        DurableTaskStatus::Canceled => "Canceled",
    }
}

fn format_unix(ts: u64) -> String {
    if let Some(d) = std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(ts)) {
        // Simple UTC → local time formatting
        let secs_since_epoch = d.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
        let secs_in_day = 86400u64;
        let secs_in_hour = 3600u64;
        let secs_in_min = 60u64;
        let days = secs_since_epoch / secs_in_day;
        let remainder = secs_since_epoch % secs_in_day;
        let hours = remainder / secs_in_hour;
        let remainder = remainder % secs_in_hour;
        let mins = remainder / secs_in_min;
        format!("d{days} {hours:02}:{mins:02}")
    } else {
        "—".to_string()
    }
}

pub fn tasks_panel(mut state: Signal<TasksUiState>) -> Element {
    // Trigger initial load on first render
    let mut loaded = use_signal(|| false);
    if !*loaded.peek() {
        state.write().refresh();
        loaded.set(true);
    }

    let tasks = state.read().tasks.clone();
    let error = state.read().last_error.clone();
    let filter = state.read().filter_status.clone();

    let task_cards: Vec<Element> = tasks.iter().map(|t| {
        let tid = t.id.clone();
        let label = t.label.clone();
        let kind = t.kind.clone();
        let status = t.status.clone();
        let color = status_color(&status);
        let slabel = status_label(&status);
        let created = format_unix(t.created_at_unix);
        let summary = t.result_summary.clone();
        let err_msg = t.error_message.clone();
        let can_cancel = !status.is_terminal();
        let has_artifacts = t.has_artifacts();
        let artifact_count = t.artifact_count();
        let first_artifact = t.artifact_paths.first().cloned();

        rsx! {
            div {
                key: "{tid}",
                background_color: "#111827",
                border: "1px solid #1f2937",
                border_radius: "12px",
                padding: "10px",
                display: "flex",
                flex_direction: "column",
                gap: "4px",

                // Header row: label + status badge
                div {
                    display: "flex",
                    justify_content: "space-between",
                    align_items: "center",

                    div {
                        font_size: "13px",
                        font_weight: "bold",
                        color: "white",
                        "{label}"
                    }
                    div {
                        background_color: color,
                        color: "white",
                        border_radius: "6px",
                        padding: "2px 8px",
                        font_size: "11px",
                        font_weight: "bold",
                        "{slabel}"
                    }
                }

                // Kind + age
                div {
                    display: "flex",
                    gap: "8px",
                    font_size: "11px",
                    color: "#6b7280",
                    div { "⌅ {kind}" }
                    div { "created {created}" }
                }

                // Summary or error
                if let Some(ref s) = summary {
                    div { color: "#d1d5db", font_size: "12px", "{s}" }
                }
                if let Some(ref e) = err_msg {
                    div { color: "#fca5a5", font_size: "12px", "{e}" }
                }

                // Artifacts section
                if has_artifacts {
                    div {
                        display: "flex",
                        align_items: "center",
                        gap: "4px",
                        margin_top: "4px",
                        font_size: "11px",
                        color: "#9ca3af",

                        div { "{artifact_count} artifact(s)" }
                        div { font_size: "9px", color: "#6b7280", "\u{00b7}" }
                        // Use a local String binding to avoid Dioxus rsx capture issues
                        {{
                            let artifact = first_artifact.as_deref().map(|s| s.to_string()).unwrap_or_else(|| String::from("—"));
                            rsx! {
                                div { "{artifact}" }
                            }
                        }}
                    }
                }

                // Cancel button
                if can_cancel {
                    div {
                        display: "flex",
                        justify_content: "flex-end",
                        margin_top: "4px",
                        button {
                            background_color: "#374151",
                            border: "none",
                            border_radius: "8px",
                            padding: "4px 12px",
                            color: "#fca5a5",
                            font_size: "12px",
                            font_weight: "bold",
                            onclick: move |_| {
                                state.write().cancel_task(&tid);
                            },
                            "Cancel"
                        }
                    }
                }
            }
        }
    }).collect();

    rsx! {
        div {
            background_color: "#111827",
            color: "white",
            border: "1px solid #374151",
            border_radius: "16px",
            padding: "16px",
            display: "flex",
            flex_direction: "column",
            gap: "12px",

            // Header
            div {
                display: "flex",
                justify_content: "space-between",
                align_items: "center",

                div { font_size: "20px", font_weight: "bold", "Tasks ({tasks.len()})" }

                // Action buttons
                div {
                    display: "flex",
                    gap: "6px",
                    button {
                        background_color: "#1e3a8a",
                        border: "1px solid #2563eb",
                        border_radius: "8px",
                        padding: "4px 10px",
                        color: "white",
                        font_size: "12px",
                        onclick: move |_| state.write().refresh(),
                        "Refresh"
                    }
                    button {
                        background_color: "#374151",
                        border: "1px solid #4b5563",
                        border_radius: "8px",
                        padding: "4px 10px",
                        color: "#fca5a5",
                        font_size: "12px",
                        onclick: move |_| state.write().prune_terminal(),
                        "Prune done"
                    }
                }
            }

            // Status filter chips
            div {
                display: "flex",
                gap: "6px",
                flex_wrap: "wrap",

                {
                    let all_active = filter.is_none();
                    rsx! {
                        button {
                            background_color: if all_active { "#1e3a8a" } else { "#1f2937" },
                            border: if all_active { "1px solid #3b82f6" } else { "1px solid #374151" },
                            border_radius: "8px",
                            padding: "3px 10px",
                            color: "white",
                            font_size: "12px",
                            onclick: move |_| state.write().set_filter(None),
                            "All"
                        }
                    }
                }
                {
                    let filters = vec![
                        DurableTaskStatus::Running,
                        DurableTaskStatus::Queued,
                        DurableTaskStatus::Completed,
                        DurableTaskStatus::Failed,
                        DurableTaskStatus::Canceled,
                    ];
                    filters.into_iter().map(|f| {
                        let label = status_label(&f);
                        let active = filter.as_ref() == Some(&f);
                        rsx! {
                            button {
                                key: "{label}",
                                background_color: if active { status_color(&f) } else { "#1f2937" },
                                border: if active { "1px solid #3b82f6" } else { "1px solid #374151" },
                                border_radius: "8px",
                                padding: "3px 10px",
                                color: "white",
                                font_size: "12px",
                                onclick: move |_| state.write().set_filter(Some(f.clone())),
                                "{label}"
                            }
                        }
                    })
                }
            }

            // Error
            if let Some(ref e) = error {
                div {
                    background_color: "#7f1d1d",
                    border: "1px solid #991b1b",
                    border_radius: "8px",
                    padding: "8px",
                    color: "#fca5a5",
                    font_size: "12px",
                    "{e}"
                }
            }

            // Empty state
            if tasks.is_empty() {
                div {
                    color: "#6b7280",
                    font_size: "13px",
                    text_align: "center",
                    padding: "16px 0",
                    if filter.is_some() {
                        "No tasks match the selected filter."
                    } else {
                        "No background tasks recorded.\nTasks from the agent will appear here."
                    }
                }
            }

            // Task list
            div {
                display: "flex",
                flex_direction: "column",
                gap: "8px",
                {task_cards.into_iter()}
            }
        }
    }
}
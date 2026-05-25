use crate::tasks_state::TasksUiState;
use deepseek_mobile_core::{DurableTaskStatus, PcGatewayClient, PcGatewayResponse};
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

pub fn tasks_panel(mut state: Signal<TasksUiState>, pc_client: Option<PcGatewayClient>) -> Element {
    // Trigger initial load on first render
    let mut loaded = use_signal(|| false);
    let mut synced_pc_key = use_signal(|| None::<String>);
    if !*loaded.peek() {
        state.write().refresh();
        loaded.set(true);
    }
    let pc_key = pc_client
        .as_ref()
        .map(|client| format!("{}:{}", client.config().id, client.config().base_url));
    if synced_pc_key() != pc_key {
        if let Some(key) = pc_key.clone() {
            sync_pc_tasks(state, pc_client.clone());
            synced_pc_key.set(Some(key));
        } else {
            state.write().clear_pc_running_tasks();
            synced_pc_key.set(None);
        }
    }

    let tasks = state.read().tasks.clone();
    let pc_tasks = state.read().pc_running_tasks.clone();
    let error = state.read().last_error.clone();
    let pc_error = state.read().pc_last_error.clone();
    let pc_synced = state.read().pc_last_synced_at_unix.map(format_unix);
    let filter = state.read().filter_status.clone();
    let has_pc = pc_client.is_some();
    let active_count = state.read().active_count();
    let refresh_pc_client = pc_client.clone();
    let sync_button_pc_client = pc_client.clone();

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

    let pc_task_cards: Vec<Element> = pc_tasks.iter().map(|t| {
        let tid = t.id.clone();
        let label = t.label.clone();
        let kind = t.kind.clone();
        let started = format_unix(t.started_at_unix);
        let pc_client_for_stop = pc_client.clone();

        rsx! {
            div {
                key: "pc-{tid}",
                background_color: "#0f172a",
                border: "1px solid #1d4ed8",
                border_radius: "12px",
                padding: "10px",
                display: "flex",
                flex_direction: "column",
                gap: "5px",

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
                        background_color: "#2563eb",
                        color: "white",
                        border_radius: "6px",
                        padding: "2px 8px",
                        font_size: "11px",
                        font_weight: "bold",
                        "PC running"
                    }
                }
                div {
                    display: "flex",
                    gap: "8px",
                    font_size: "11px",
                    color: "#9ca3af",
                    div { "⌅ {kind}" }
                    div { "started {started}" }
                    div { "id {tid}" }
                }
                if pc_client_for_stop.is_some() {
                    div {
                        display: "flex",
                        justify_content: "flex-end",
                        margin_top: "4px",
                        button {
                            background_color: "#374151",
                            border: "1px solid #4b5563",
                            border_radius: "8px",
                            padding: "4px 12px",
                            color: "#fca5a5",
                            font_size: "12px",
                            font_weight: "bold",
                            onclick: move |_| {
                                let task_id = tid.clone();
                                stop_pc_task(state, pc_client_for_stop.clone(), task_id);
                            },
                            "Stop on PC"
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

                div { font_size: "20px", font_weight: "bold", "Tasks ({active_count})" }

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
                        onclick: move |_| {
                            state.write().refresh();
                            sync_pc_tasks(state, refresh_pc_client.clone());
                        },
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

            // PC running task reconciliation
            div {
                background_color: "#0b1220",
                border: "1px solid #1e3a8a",
                border_radius: "12px",
                padding: "10px",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                div {
                    display: "flex",
                    justify_content: "space-between",
                    align_items: "center",
                    gap: "8px",
                    div {
                        div { font_size: "14px", font_weight: "bold", "PC running tasks ({pc_tasks.len()})" }
                        if let Some(ref synced) = pc_synced {
                            div { color: "#9ca3af", font_size: "11px", "last sync {synced}" }
                        } else if has_pc {
                            div { color: "#9ca3af", font_size: "11px", "not synced yet" }
                        } else {
                            div { color: "#6b7280", font_size: "11px", "connect PC Host to sync live running tasks" }
                        }
                    }
                    button {
                        background_color: if has_pc { "#1e3a8a" } else { "#1f2937" },
                        border: if has_pc { "1px solid #2563eb" } else { "1px solid #374151" },
                        border_radius: "8px",
                        padding: "4px 10px",
                        color: "white",
                        font_size: "12px",
                        disabled: !has_pc,
                        onclick: move |_| sync_pc_tasks(state, sync_button_pc_client.clone()),
                        "Sync PC"
                    }
                }

                if let Some(ref e) = pc_error {
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

                if pc_tasks.is_empty() {
                    div {
                        color: "#6b7280",
                        font_size: "12px",
                        if has_pc {
                            "No running PC tasks reported by PC Host."
                        } else {
                            "PC task sync is unavailable until a PC workspace is online."
                        }
                    }
                }

                div {
                    display: "flex",
                    flex_direction: "column",
                    gap: "8px",
                    {pc_task_cards.into_iter()}
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

fn sync_pc_tasks(mut state: Signal<TasksUiState>, pc_client: Option<PcGatewayClient>) {
    if let Some(client) = pc_client {
        spawn(async move {
            let mut next = state();
            next.refresh_pc_running_tasks(&client).await;
            state.set(next);
        });
    } else {
        state.write().clear_pc_running_tasks();
    }
}

fn stop_pc_task(
    mut state: Signal<TasksUiState>,
    pc_client: Option<PcGatewayClient>,
    task_id: String,
) {
    let Some(client) = pc_client else {
        state.write().pc_last_error = Some("PC Host is not connected".to_string());
        return;
    };

    spawn(async move {
        let mut next = state();
        match client.stop_task(task_id.clone()).await {
            Ok(PcGatewayResponse::TaskStopped { .. }) => {
                next.refresh_pc_running_tasks(&client).await;
            }
            Ok(other) => {
                next.pc_last_error = Some(format!("Unexpected PC stop response: {:?}", other));
            }
            Err(error) => {
                next.pc_last_error = Some(format!("Failed to stop PC task {}: {}", task_id, error));
            }
        }
        state.set(next);
    });
}

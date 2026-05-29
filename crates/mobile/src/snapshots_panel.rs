use crate::locale::{pick, AppLanguage};
use crate::mobile_runtime_config::MobileRuntimeConfig;
use crate::mobile_snapshot_runner::{list_workspace_snapshots, restore_snapshot_by_id};
use crate::settings_state::SettingsFormState;
use crate::snapshots_state::SnapshotsUiState;
use dioxus::prelude::*;

#[derive(Clone)]
struct SnapshotDisplayInfo {
    id: String,
    file_count: usize,
    total_bytes: u64,
    reason: String,
}

pub fn snapshots_panel(
    ui_lang: AppLanguage,
    mut state: Signal<SnapshotsUiState>,
    settings_state: Signal<SettingsFormState>,
) -> Element {
    let refresh_label = pick(ui_lang, "Обновить список", "Refresh list");
    let refresh_running = use_signal(|| false);

    use_effect(move || {
        let settings = settings_state;
        let mut snapshots = state;
        let mut running = refresh_running;
        spawn(async move {
            running.set(true);
            let config = settings().to_config();
            let runtime = MobileRuntimeConfig::default_mobile();
            match list_workspace_snapshots(config, runtime).await {
                Ok(records) => snapshots.write().replace_all(records),
                Err(error) => {
                    snapshots.write().last_error = Some(error.to_string());
                }
            }
            running.set(false);
        });
    });
    let latest = state.read().latest().cloned();
    let pending_info = state
        .read()
        .pending_restore_snapshot()
        .map(|s| (s.id.clone(), s.file_count, s.total_bytes));
    let error_text = state.read().last_error.clone();
    let report_text = state.read().last_restore_report.clone();
    let has_pending = state.read().pending_restore_snapshot_id.is_some();
    let restore_running = state.read().restore_in_progress;

    let snapshot_list: Vec<SnapshotDisplayInfo> = state
        .read()
        .snapshots
        .iter()
        .take(12)
        .map(|s| SnapshotDisplayInfo {
            id: s.id.clone(),
            file_count: s.file_count,
            total_bytes: s.total_bytes,
            reason: s.reason.clone(),
        })
        .collect();

    // Build the snapshot cards outside rsx to avoid let-bindings inside the macro
    let snapshot_cards: Vec<Element> = snapshot_list
        .iter()
        .map(|info| {
            let sid = info.id.clone();
            let fc = info.file_count;
            let tb = info.total_bytes;
            let reason = info.reason.clone();
            let disabled = has_pending || restore_running;

            rsx! {
                div {
                    key: "{sid}",
                    background_color: "#111827",
                    border: "1px solid #1f2937",
                    border_radius: "12px",
                    padding: "10px",
                    display: "flex",
                    flex_direction: "column",
                    gap: "4px",

                    div {
                        display: "flex",
                        justify_content: "space_between",
                        align_items: "center",

                        div { font_size: "13px", font_weight: "bold", "{sid}" }
                        button {
                            background_color: "#b45309",
                            border: "none",
                            border_radius: "8px",
                            padding: "4px 12px",
                            color: "white",
                            font_size: "12px",
                            font_weight: "bold",
                            onclick: move |_| {
                                let mut s = state.write();
                                s.request_restore(&sid);
                            },
                            disabled: if disabled { "true" } else { "false" },
                            "Restore"
                        }
                    }
                    div { color: "#9ca3af", font_size: "12px", "{fc} file(s) · {tb} bytes" }
                    div { color: "#d1d5db", font_size: "12px", "{reason}" }
                }
            }
        })
        .collect();

    rsx! {
        div {
            background_color: "#111827",
            color: "white",
            border: "1px solid #374151",
            border_radius: "16px",
            padding: "12px",
            display: "flex",
            flex_direction: "column",
            gap: "12px",

            // --- Header card ---
            div {
                background_color: "#0f172a",
                border: "1px solid #334155",
                border_radius: "14px",
                padding: "12px",
                display: "flex",
                flex_direction: "column",
                gap: "6px",

                div {
                    display: "flex",
                    justify_content: "space_between",
                    align_items: "center",
                    gap: "8px",
                    div { font_size: "18px", font_weight: "bold", "Snapshots" }
                    button {
                        background_color: "#1d4ed8",
                        border: "none",
                        border_radius: "8px",
                        padding: "6px 12px",
                        color: "white",
                        font_size: "12px",
                        font_weight: "bold",
                        disabled: refresh_running(),
                        onclick: move |_| {
                            let settings = settings_state;
                            let mut snapshots = state;
                            let mut running = refresh_running;
                            spawn(async move {
                                running.set(true);
                                let config = settings().to_config();
                                let runtime = MobileRuntimeConfig::default_mobile();
                                match list_workspace_snapshots(config, runtime).await {
                                    Ok(records) => snapshots.write().replace_all(records),
                                    Err(error) => {
                                        snapshots.write().last_error = Some(error.to_string());
                                    }
                                }
                                running.set(false);
                            });
                        },
                        "{refresh_label}"
                    }
                }
                if let Some(ref snapshot) = latest {
                    div { color: "#d1d5db", font_size: "13px", "Latest safety point: {snapshot.id}" }
                    div { color: "#9ca3af", font_size: "12px", "{snapshot.file_count} file(s) · {snapshot.total_bytes} bytes · {snapshot.reason}" }
                } else {
                    div { color: "#9ca3af", font_size: "13px", "No snapshots have been surfaced yet." }
                }
            }

            // --- Restore confirmation dialog ---
            if let Some((snapshot_id, file_count, total_bytes)) = pending_info {
                div {
                    background_color: "#422006",
                    border: "2px solid #ca8a04",
                    border_radius: "14px",
                    padding: "14px",
                    display: "flex",
                    flex_direction: "column",
                    gap: "10px",

                    div { font_size: "16px", font_weight: "bold", color: "#fde68a", "⚠ Restore snapshot?" }
                    div { color: "#fef3c7", font_size: "13px", "This will overwrite files and delete any files not in the snapshot." }
                    div { color: "#fde68a", font_size: "13px", "Snapshot: {snapshot_id}" }
                    div { color: "#d1d5db", font_size: "13px", "Files to restore: {file_count}" }
                    div { color: "#d1d5db", font_size: "13px", "Total size: {total_bytes} bytes" }
                    div { color: "#fca5a5", font_size: "12px", "Deleted files cannot be recovered unless another snapshot exists." }

                    div {
                        display: "flex",
                        gap: "10px",
                        margin_top: "4px",

                        button {
                            background_color: "#dc2626",
                            border: "none",
                            border_radius: "10px",
                            padding: "8px 18px",
                            color: "white",
                            font_size: "13px",
                            font_weight: "bold",
                            onclick: move |_| {
                                let snapshot_id_for_restore = snapshot_id.clone();
                                let settings_signal = settings_state;
                                let mut state_signal = state;
                                spawn(async move {
                                    state_signal.write().confirm_restore();
                                    let config = settings_signal().to_config();
                                    let runtime = MobileRuntimeConfig::default_mobile();
                                    match restore_snapshot_by_id(
                                        config,
                                        runtime,
                                        &snapshot_id_for_restore,
                                    )
                                    .await
                                    {
                                        Ok(report) => {
                                            let mut s = state_signal.write();
                                            s.restore_in_progress = false;
                                            s.pending_restore_snapshot_id = None;
                                            s.last_restore_report = Some(report);
                                            s.last_error = None;
                                        }
                                        Err(error) => {
                                            let mut s = state_signal.write();
                                            s.restore_in_progress = false;
                                            s.pending_restore_snapshot_id = None;
                                            s.last_error = Some(error.to_string());
                                        }
                                    }
                                });
                            },
                            disabled: restore_running,
                            "Confirm Restore"
                        }
                        button {
                            background_color: "#374151",
                            border: "none",
                            border_radius: "10px",
                            padding: "8px 18px",
                            color: "white",
                            font_size: "13px",
                            onclick: move |_| {
                                state.write().cancel_restore();
                            },
                            "Cancel"
                        }
                    }
                }
            }

            // --- Error display ---
            if let Some(ref err) = error_text {
                div {
                    background_color: "#7f1d1d",
                    border: "1px solid #dc2626",
                    border_radius: "12px",
                    padding: "10px",
                    color: "white",
                    font_size: "12px",
                    "{err}"
                }
            }

            // --- Restore report ---
            if let Some(ref report) = report_text {
                div {
                    background_color: "#14532d",
                    border: "1px solid #22c55e",
                    border_radius: "12px",
                    padding: "10px",
                    color: "#bbf7d0",
                    font_size: "12px",
                    "{report}"
                }
            }

            // --- Available snapshots list ---
            div {
                background_color: "#020617",
                border: "1px solid #1f2937",
                border_radius: "14px",
                padding: "10px",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                div { font_size: "14px", font_weight: "bold", "Available snapshots" }
                if snapshot_cards.is_empty() {
                    div { color: "#9ca3af", font_size: "12px", "Approved destructive tools will create safety snapshots automatically." }
                } else {
                    {snapshot_cards.into_iter()}
                }
            }
        }
    }
}

mod agent_event_adapter;
mod agent_timeline;
mod agent_timeline_panel;
mod approval_diff_preview;
mod attachment_ingestion;
mod chat_attachment;
mod cockpit_section_panel;
mod diagnostics_panel;
mod diagnostics_state;
mod document_picker;
mod git_panel;
mod git_state;
mod mcp_panel;
mod mcp_state;
mod mobile_approval_panel;
mod mobile_drawer;
mod mobile_engine_runner;
mod mobile_git_runner;
mod mobile_runtime_config;
mod native_bridge;
mod native_document_picker;
mod native_event_router;
mod native_pc_discovery;
mod native_termux;
mod onboarding_panel;
mod pc_pairing_manager;
mod pc_pairing_panel;
mod pc_pairing_state;
mod project_diff;
mod project_files;
mod project_files_panel;
mod project_files_state;
mod project_transfer_state;
mod saved_timeline_loader;
mod settings_panel;
mod settings_state;
mod skills_panel;
mod skills_state;
mod snapshots_panel;
mod snapshots_state;
mod tasks_panel;
mod tasks_state;
mod terminal_panel;
mod terminal_state;
mod termux_state;

use agent_event_adapter::push_agent_event;
use agent_timeline::MobileTimelineState;
use agent_timeline_panel::agent_timeline_panel;
use chat_attachment::ChatComposerState;
use cockpit_section_panel::cockpit_section_panel;
use deepseek_mobile_core::{AgentEvent, ApprovalCardView, DurableTaskStatus, ReviewDecision};
use diagnostics_state::DiagnosticsUiState;
use dioxus::prelude::*;
use document_picker::{DocumentPickerPurpose, DocumentPickerRequest, DocumentPickerState};
use git_state::GitUiState;
use mcp_state::McpUiState;
use mobile_approval_panel::mobile_approval_panel;
use mobile_drawer::{bottom_nav_bar, mobile_drawer, CockpitSection, MobileChromeSummary};
use mobile_engine_runner::{
    continue_mobile_approval, load_default_mobile_approval_cards, run_mobile_turn_streaming,
};
use native_bridge::{NativeBridgeState, NativeMobileCommand, NativeMobileEvent};
use onboarding_panel::onboarding_panel;
use pc_pairing_state::{PcPairingUiState, PcPairingUiStatus};
use project_files_state::ProjectFilesUiState;
use project_transfer_state::{default_phone_workspace_root, ProjectTransferState};
use saved_timeline_loader::load_default_saved_events;
use settings_state::{load_saved_config, SettingsFormState};
use skills_state::SkillsUiState;
use snapshots_state::SnapshotsUiState;
use tasks_state::TasksUiState;
use terminal_state::TerminalUiState;
use termux_state::TermuxWorkspaceState;

fn main() {
    dioxus::launch(app);
}

fn build_chrome_summary(
    settings: &SettingsFormState,
    pc: &PcPairingUiState,
    approvals: usize,
    native_bridge: &NativeBridgeState,
    files: &ProjectFilesUiState,
    termux: &TermuxWorkspaceState,
    diagnostics: &DiagnosticsUiState,
    git: &GitUiState,
    tasks: &TasksUiState,
) -> MobileChromeSummary {
    let (pc_label, pc_online) = match &pc.status {
        PcPairingUiStatus::NotConfigured => ("PC SETUP".to_string(), false),
        PcPairingUiStatus::ReadyToExport => ("PAIR".to_string(), false),
        PcPairingUiStatus::Exported => ("ZIP".to_string(), false),
        PcPairingUiStatus::WaitingForPc => ("PC WAIT".to_string(), false),
        PcPairingUiStatus::Online => ("PC ON".to_string(), true),
        PcPairingUiStatus::Offline => ("PC OFF".to_string(), false),
        PcPairingUiStatus::Error(_) => ("PC ERR".to_string(), false),
    };

    let (active_project_title, active_project_subtitle) =
        if let Some(connection) = pc.active_workspace_connection() {
            (
                connection.workspace_name,
                format!("PC: {}", connection.workspace_root.display()),
            )
        } else if let Some(request) = pc.request.as_ref() {
            (
                request.workspace_id.clone(),
                format!("PC target pending: {}", request.workspace_root),
            )
        } else if termux.is_valid() {
            (
                termux.display_label(),
                format!("Termux: {}", termux.workspace_path.trim()),
            )
        } else {
            (
                "Local workspace".to_string(),
                files.current_browsing_display(),
            )
        };

    let running_tasks = tasks
        .tasks
        .iter()
        .filter(|task| {
            matches!(
                task.status,
                DurableTaskStatus::Queued | DurableTaskStatus::Running
            )
        })
        .count();

    MobileChromeSummary {
        api_configured: !settings.api_key.trim().is_empty(),
        pc_label,
        pc_online,
        active_project_title,
        active_project_subtitle,
        pending_approvals: approvals,
        running_tasks,
        diagnostics_errors: diagnostics.error_count(),
        diagnostics_warnings: diagnostics.warning_count(),
        dirty_files: git.changed_files,
        native_waiting: native_bridge.has_pending_commands()
            || native_bridge.is_waiting_for_document_picker_callback()
            || native_bridge.is_waiting_for_termux_callback()
            || native_bridge.is_waiting_for_pc_discovery_callback(),
    }
}

fn status_chip(label: String, colors: (&'static str, &'static str, &'static str)) -> Element {
    let (background, border, color) = colors;
    let border_style = format!("1px solid {border}");
    rsx! {
        div {
            background_color: background,
            color,
            border: "{border_style}",
            border_radius: "999px",
            padding: "7px 9px",
            font_size: "11px",
            font_weight: "bold",
            white_space: "nowrap",
            "{label}"
        }
    }
}

fn app() -> Element {
    let mut messages = use_signal(Vec::<(String, String)>::new);
    let mut input = use_signal(String::new);
    let mut composer = use_signal(ChatComposerState::default);
    let mut timeline = use_signal(MobileTimelineState::default);
    let mut approval_cards = use_signal(Vec::<ApprovalCardView>::new);
    let mut did_load_saved_runtime = use_signal(|| false);
    let mut did_load_terminal_state = use_signal(|| false);
    let mut picker = use_signal(DocumentPickerState::default);
    let mut native_bridge = use_signal(NativeBridgeState::default);
    let mut is_loading = use_signal(|| false);
    let mut drawer_open = use_signal(|| false);
    let mut active_section = use_signal(|| CockpitSection::Chat);
    let pc_pairing_state = use_signal(PcPairingUiState::default);
    let project_files_state = use_signal(ProjectFilesUiState::default);
    let project_transfer_state = use_signal(ProjectTransferState::default);
    let mut snapshots_state = use_signal(SnapshotsUiState::default);
    let tasks_state = use_signal(TasksUiState::default);
    let mut diagnostics_state = use_signal(DiagnosticsUiState::default);
    let git_state = use_signal(GitUiState::default);
    let mut terminal_state = use_signal(TerminalUiState::default);
    let mut settings_state = use_signal(SettingsFormState::default);
    let termux_state = use_signal(TermuxWorkspaceState::default);
    let mcp_state = use_signal(McpUiState::default);
    let skills_state = use_signal(SkillsUiState::default);
    let mut onboarding_done = use_signal(|| {
        if let Some(config) = load_saved_config() {
            let key = config.api_key.trim();
            if !key.is_empty() && key.starts_with("sk-") {
                return true;
            }
        }
        false
    });

    // Route native bridge terminal events into terminal UI state
    let terminal_event_bridge = native_bridge;
    let mut terminal_event_state = terminal_state;

    // Route Android picker/share callbacks into either chat attachments or the
    // local phone project import/export flow, depending on the pending picker
    // purpose.
    let project_picker_bridge = native_bridge;
    let mut project_picker_last_event = use_signal(|| None::<String>);
    let mut project_picker_state = picker;
    let mut project_picker_composer = composer;
    let mut project_picker_timeline = timeline;
    let mut project_picker_files = project_files_state;
    let mut project_picker_transfer = project_transfer_state;
    use_effect(move || {
        let bridge_snapshot = project_picker_bridge();
        let event_id = bridge_snapshot.last_event_id;
        let Some(event) = bridge_snapshot.last_event.clone() else {
            return;
        };
        let event_key = event_id.to_string();
        if project_picker_last_event()
            .as_deref()
            .map(|handled| handled == event_key.as_str())
            .unwrap_or(false)
        {
            return;
        }

        match event {
            NativeMobileEvent::DocumentsPicked(documents) => {
                project_picker_last_event.set(Some(event_key));
                let purpose = project_picker_state()
                    .pending_request
                    .as_ref()
                    .map(|request| request.purpose.clone())
                    .unwrap_or(DocumentPickerPurpose::ChatAttachment);

                match purpose {
                    DocumentPickerPurpose::ProjectImport => {
                        let mut transfer = project_picker_transfer();
                        let workspace_root = default_phone_workspace_root();
                        match transfer.import_documents(&documents, &workspace_root) {
                            Ok(report) => {
                                project_picker_transfer.set(transfer);

                                let mut files = project_picker_files();
                                if !files.is_pc_backend() {
                                    files.workspace_root =
                                        report.workspace_root.display().to_string();
                                    files.loaded = false;
                                }
                                project_picker_files.set(files);

                                let mut next_timeline = project_picker_timeline();
                                push_agent_event(
                                    &mut next_timeline,
                                    &AgentEvent::Status(format!(
                                        "Project archive imported: {}",
                                        report.archive_name
                                    )),
                                );
                                project_picker_timeline.set(next_timeline);
                            }
                            Err(error) => {
                                transfer.mark_error(error.to_string());
                                project_picker_transfer.set(transfer);
                                let mut next_timeline = project_picker_timeline();
                                push_agent_event(
                                    &mut next_timeline,
                                    &AgentEvent::Error(format!("Project import failed: {}", error)),
                                );
                                project_picker_timeline.set(next_timeline);
                            }
                        }
                    }
                    DocumentPickerPurpose::ChatAttachment
                    | DocumentPickerPurpose::SettingsImport => {
                        let mut next_composer = project_picker_composer();
                        let mut next_timeline = project_picker_timeline();
                        if documents.is_empty() {
                            push_agent_event(
                                &mut next_timeline,
                                &AgentEvent::Status(
                                    "Document picker returned no files".to_string(),
                                ),
                            );
                        } else {
                            for document in documents {
                                next_timeline.push_attachment(format!("{}", document.display_name));
                                next_composer.add_picked_document(document);
                            }
                            push_agent_event(
                                &mut next_timeline,
                                &AgentEvent::Status(
                                    "Android document picker files attached to chat composer"
                                        .to_string(),
                                ),
                            );
                        }
                        project_picker_composer.set(next_composer);
                        project_picker_timeline.set(next_timeline);
                    }
                }

                project_picker_state.write().complete();
            }
            NativeMobileEvent::DocumentPickerCancelled => {
                project_picker_last_event.set(Some(event_key));
                if project_picker_state()
                    .pending_request
                    .as_ref()
                    .map(|request| request.purpose == DocumentPickerPurpose::ProjectImport)
                    .unwrap_or(false)
                {
                    project_picker_transfer.write().mark_import_cancelled();
                }
                project_picker_state.write().complete();
            }
            NativeMobileEvent::DocumentPickerFailed(error) => {
                project_picker_last_event.set(Some(event_key));
                if project_picker_state()
                    .pending_request
                    .as_ref()
                    .map(|request| request.purpose == DocumentPickerPurpose::ProjectImport)
                    .unwrap_or(false)
                {
                    project_picker_transfer.write().mark_error(error.clone());
                }
                project_picker_state.write().fail(error);
            }
            NativeMobileEvent::FileShared => {
                project_picker_last_event.set(Some(event_key));
                if project_picker_transfer().is_sharing() {
                    project_picker_transfer.write().mark_shared();
                }
            }
            NativeMobileEvent::ShareFailed(error) => {
                project_picker_last_event.set(Some(event_key));
                if project_picker_transfer().is_sharing() {
                    project_picker_transfer.write().mark_error(error);
                }
            }
            _ => {}
        }
    });

    // Termux result continuation: when a Termux command completes, feed the real
    // result back into the engine so the model can respond to the actual output.
    let termux_continuation_bridge = native_bridge;
    let termux_continuation_settings = settings_state;
    let termux_continuation_timeline = timeline;
    let termux_continuation_cards = approval_cards;
    let termux_continuation_loading = is_loading;
    use_effect(move || {
        let event = termux_continuation_bridge().last_event.clone();
        if let Some(NativeMobileEvent::TermuxCommandCompleted(result)) = event {
            let config = termux_continuation_settings().to_config();
            let mut event_timeline = termux_continuation_timeline;
            let mut event_cards = termux_continuation_cards;
            let mut loading_signal = termux_continuation_loading;
            spawn(async move {
                loading_signal.set(true);
                match crate::mobile_engine_runner::continue_mobile_termux_result(config, result)
                    .await
                {
                    Ok(result) => {
                        let mut next_timeline = event_timeline();
                        push_agent_event(
                            &mut next_timeline,
                            &AgentEvent::Status(
                                "Termux result injected — model continuing with real output"
                                    .to_string(),
                            ),
                        );
                        for event in &result.events {
                            push_agent_event(&mut next_timeline, event);
                        }
                        event_timeline.set(next_timeline);

                        if let Some(final_text) = result.final_text.clone() {
                            messages.push(("assistant".to_string(), final_text));
                        }

                        event_cards.set(result.approval_cards);
                    }
                    Err(error) => {
                        let mut next_timeline = event_timeline();
                        push_agent_event(
                            &mut next_timeline,
                            &AgentEvent::Error(format!("Termux continuation failed: {}", error)),
                        );
                        event_timeline.set(next_timeline);
                    }
                }
                loading_signal.set(false);
            });
        }
    });

    // Auto-save terminal state on changes
    let terminal_persist_signal = terminal_state;
    let terminal_persist_path =
        std::path::PathBuf::from(".deepseek-mobile").join("terminal_state.json");
    use_effect(move || {
        let state = terminal_persist_signal();
        if !state.sessions.is_empty() {
            let _ = state.save_to_file(&terminal_persist_path);
        }
    });

    use_effect(move || {
        let event = terminal_event_bridge().last_event.clone();
        match event {
            Some(NativeMobileEvent::TerminalOpened {
                session_id,
                title,
                cwd,
            }) => {
                let session = deepseek_mobile_core::PcTerminalSession {
                    id: session_id,
                    workspace_id: "default".to_string(),
                    title,
                    cwd,
                    environment_id: None,
                    created_at_unix: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                };
                let mut ts = terminal_event_state.write();
                ts.add_session(session);
            }
            Some(NativeMobileEvent::TerminalOutput { session_id, chunk }) => {
                let mut ts = terminal_event_state.write();
                ts.append_output(&session_id, &chunk);
            }
            Some(NativeMobileEvent::TerminalClosed {
                session_id,
                exit_code,
            }) => {
                let mut ts = terminal_event_state.write();
                ts.close_session(&session_id, exit_code);
            }
            _ => {}
        }
    });

    if !did_load_saved_runtime() {
        did_load_saved_runtime.set(true);
        match load_default_saved_events() {
            Ok(saved_events) => {
                if !saved_events.is_empty() {
                    let mut restored_timeline = MobileTimelineState::default();
                    let mut restored_snapshots = SnapshotsUiState::default();
                    let mut restored_diagnostics = DiagnosticsUiState::default();
                    for event in &saved_events {
                        push_agent_event(&mut restored_timeline, event);
                        restored_snapshots.apply_agent_event(event);
                        restored_diagnostics.apply_agent_event(event);
                    }
                    timeline.set(restored_timeline);
                    snapshots_state.set(restored_snapshots);
                    diagnostics_state.set(restored_diagnostics);
                }
            }
            Err(error) => {
                let mut next_timeline = timeline();
                push_agent_event(
                    &mut next_timeline,
                    &AgentEvent::Error(format!("Failed to restore saved timeline: {}", error)),
                );
                timeline.set(next_timeline);
            }
        }

        match load_default_mobile_approval_cards() {
            Ok(cards) => approval_cards.set(cards),
            Err(error) => {
                let mut next_timeline = timeline();
                push_agent_event(
                    &mut next_timeline,
                    &AgentEvent::Error(format!("Failed to restore pending approvals: {}", error)),
                );
                timeline.set(next_timeline);
            }
        }
    }

    if !did_load_terminal_state() {
        did_load_terminal_state.set(true);
        let terminal_persistence_path =
            std::path::PathBuf::from(".deepseek-mobile").join("terminal_state.json");
        if terminal_persistence_path.exists() {
            if let Ok(saved_terminal) = TerminalUiState::load_from_file(&terminal_persistence_path)
            {
                terminal_state.set(saved_terminal);
            }
        }
    }

    if !onboarding_done() && settings_state().api_key.trim().is_empty() {
        return rsx! {
            {onboarding_panel(
                EventHandler::new(move |api_key: String| {
                    settings_state.write().api_key = api_key;
                    onboarding_done.set(true);
                })
            )}
        };
    }

    let chrome_summary = build_chrome_summary(
        &settings_state(),
        &pc_pairing_state(),
        approval_cards().len(),
        &native_bridge(),
        &project_files_state(),
        &termux_state(),
        &diagnostics_state(),
        &git_state(),
        &tasks_state(),
    );
    let api_chip = status_chip(
        chrome_summary.api_chip_label().to_string(),
        chrome_summary.api_chip_colors(),
    );
    let pc_chip = status_chip(
        chrome_summary.pc_label.clone(),
        chrome_summary.pc_chip_colors(),
    );
    let drawer_summary = chrome_summary.clone();
    let bottom_nav_summary = chrome_summary.clone();

    rsx! {
        div {
            background_color: "#0f0f0f",
            color: "white",
            height: "100vh",
            padding: "16px",
            display: "flex",
            flex_direction: "column",
            position: "relative",
            overflow: "hidden",

            {mobile_drawer(
                drawer_open(),
                active_section(),
                drawer_summary,
                EventHandler::new(move |section| {
                    active_section.set(section);
                    drawer_open.set(false);
                })
            )}

            div {
                display: "flex",
                align_items: "center",
                justify_content: "space-between",
                margin_bottom: "12px",

                button {
                    background_color: "#1f2937",
                    color: "white",
                    width: "44px",
                    height: "44px",
                    border_radius: "999px",
                    border: "1px solid #374151",
                    onclick: move |_| drawer_open.set(!drawer_open()),
                    "☰"
                }

                div {
                    display: "flex",
                    flex_direction: "column",
                    align_items: "center",
                    min_width: "0",
                    div { font_size: "18px", font_weight: "bold", "DeepSeek Mobile" }
                    div {
                        color: "#9ca3af",
                        font_size: "12px",
                        text_align: "center",
                        white_space: "nowrap",
                        overflow: "hidden",
                        text_overflow: "ellipsis",
                        max_width: "190px",
                        "{active_section().subtitle()}"
                    }
                }

                div {
                    display: "flex",
                    align_items: "center",
                    gap: "6px",
                    {api_chip}
                    {pc_chip}
                }
            }

            div {
                background_color: "#101827",
                border: "1px solid #1f2937",
                border_radius: "16px",
                padding: "10px 12px",
                margin_bottom: "12px",
                display: "flex",
                align_items: "center",
                justify_content: "space-between",
                gap: "10px",

                div {
                    min_width: "0",
                    div {
                        color: "#6b7280",
                        font_size: "10px",
                        font_weight: "bold",
                        letter_spacing: "0.08em",
                        "ACTIVE WORKSPACE"
                    }
                    div {
                        color: "white",
                        font_size: "14px",
                        font_weight: "bold",
                        white_space: "nowrap",
                        overflow: "hidden",
                        text_overflow: "ellipsis",
                        "{chrome_summary.active_project_title}"
                    }
                    div {
                        color: "#9ca3af",
                        font_size: "11px",
                        white_space: "nowrap",
                        overflow: "hidden",
                        text_overflow: "ellipsis",
                        "{chrome_summary.active_project_subtitle}"
                    }
                }

                div {
                    color: "#60a5fa",
                    font_size: "12px",
                    font_weight: "bold",
                    white_space: "nowrap",
                    "{active_section().title()}"
                }
            }

            div {
                flex: "1",
                background_color: "#111827",
                padding: "12px",
                border_radius: "18px",
                overflow_y: "auto",
                display: "flex",
                flex_direction: "column",
                gap: "8px",

                if active_section() == CockpitSection::Chat {
                    {mobile_approval_panel(
                        &approval_cards(),
                        EventHandler::new(move |(approval_id, decision): (String, ReviewDecision)| {
                            let config = settings_state().to_config();
                            is_loading.set(true);
                            spawn(async move {
                                match continue_mobile_approval(config, approval_id.clone(), decision.clone()).await {
                                    Ok(result) => {
                                        let mut next_timeline = timeline();
                                        push_agent_event(&mut next_timeline, &AgentEvent::Status(format!("Approval decision applied: {:?}", decision)));
                                        let mut next_snapshots = snapshots_state();
                                        let mut next_diagnostics = diagnostics_state();
                                        let mut next_native_bridge = native_bridge();
                                        for event in &result.events {
                                            push_agent_event(&mut next_timeline, event);
                                            next_snapshots.apply_agent_event(event);
                                            next_diagnostics.apply_agent_event(event);
                                            next_native_bridge.enqueue_termux_command_from_agent_event(event);
                                        }
                                        push_agent_event(&mut next_timeline, &AgentEvent::Status(format!("Executed tools: {} | session grants: {}", result.executed_count, result.session_grant_count)));
                                        timeline.set(next_timeline);
                                        snapshots_state.set(next_snapshots);
                                        diagnostics_state.set(next_diagnostics);
                                        native_bridge.set(next_native_bridge);
                                        approval_cards.set(result.remaining_approval_cards);
                                    }
                                    Err(error) => {
                                        let mut next_timeline = timeline();
                                        push_agent_event(&mut next_timeline, &AgentEvent::Error(format!("Approval continuation failed: {}", error)));
                                        timeline.set(next_timeline);
                                    }
                                }
                                is_loading.set(false);
                            });
                        })
                    )}

                    {agent_timeline_panel(&timeline())}

                    if is_loading() {
                        div { color: "#9ca3af", "Thinking with DeepSeek..." }
                    }
                } else {
                    {cockpit_section_panel(
                        active_section(),
                        approval_cards,
                        pc_pairing_state,
                        native_bridge,
                        picker,
                        project_files_state,
                        project_transfer_state,
                        snapshots_state,
                        diagnostics_state,
                        git_state,
                        terminal_state,
                        mcp_state,
                        skills_state,
                        tasks_state,
                        settings_state,
                        termux_state,
                        EventHandler::new(move |(approval_id, decision): (String, ReviewDecision)| {
                            let approval_id = approval_id.clone();
                            let decision = decision.clone();
                            let config = settings_state().to_config();
                            is_loading.set(true);
                            spawn(async move {
                                match continue_mobile_approval(config, approval_id.clone(), decision.clone()).await {
                                    Ok(result) => {
                                        let mut next_timeline = timeline();
                                        push_agent_event(&mut next_timeline, &AgentEvent::Status(format!("Approval decision applied: {:?}", decision)));
                                        let mut next_snapshots = snapshots_state();
                                        let mut next_diagnostics = diagnostics_state();
                                        let mut next_native_bridge = native_bridge();
                                        for event in &result.events {
                                            push_agent_event(&mut next_timeline, event);
                                            next_snapshots.apply_agent_event(event);
                                            next_diagnostics.apply_agent_event(event);
                                            next_native_bridge.enqueue_termux_command_from_agent_event(event);
                                        }
                                        push_agent_event(&mut next_timeline, &AgentEvent::Status(format!("Executed tools: {} | session grants: {}", result.executed_count, result.session_grant_count)));
                                        timeline.set(next_timeline);
                                        snapshots_state.set(next_snapshots);
                                        diagnostics_state.set(next_diagnostics);
                                        native_bridge.set(next_native_bridge);
                                        approval_cards.set(result.remaining_approval_cards);
                                    }
                                    Err(error) => {
                                        let mut next_timeline = timeline();
                                        push_agent_event(&mut next_timeline, &AgentEvent::Error(format!("Approval continuation failed: {}", error)));
                                        timeline.set(next_timeline);
                                    }
                                }
                                is_loading.set(false);
                            });
                        })
                    )}
                }
            }

            if picker().is_waiting_for_native_picker()
                || native_bridge().has_pending_commands()
                || native_bridge().is_waiting_for_termux_callback()
            {
                div {
                    margin_top: "8px",
                    background_color: "#1e3a8a",
                    border: "1px solid #3b82f6",
                    border_radius: "14px",
                    padding: "8px 10px",
                    color: "white",
                    font_size: "12px",
                    "Waiting for Android native callback..."
                }
            }

            if native_bridge().is_waiting_for_pc_discovery_callback() {
                div {
                    margin_top: "8px",
                    background_color: "#064e3b",
                    border: "1px solid #10b981",
                    border_radius: "14px",
                    padding: "8px 10px",
                    color: "white",
                    font_size: "12px",
                    "Scanning local network for DeepSeek PC Host..."
                }
            }

            if let Some(error) = native_bridge().last_error {
                div {
                    margin_top: "8px",
                    background_color: "#7f1d1d",
                    border: "1px solid #dc2626",
                    border_radius: "14px",
                    padding: "8px 10px",
                    color: "white",
                    font_size: "12px",
                    "Native bridge error: {error}"
                }
            }

            if !composer().attachments.is_empty() {
                div {
                    margin_top: "8px",
                    background_color: "#111827",
                    border: "1px solid #374151",
                    border_radius: "14px",
                    padding: "8px 10px",
                    color: "#d1d5db",
                    font_size: "12px",
                    "{composer().attachment_summary()}"
                }
            }

            div {
                display: "flex",
                gap: "8px",
                margin_top: "12px",
                align_items: "center",

                button {
                    background_color: "#1f2937",
                    color: "white",
                    width: "44px",
                    height: "44px",
                    border_radius: "999px",
                    border: "1px solid #4b5563",
                    onclick: move |_| {
                        let request = DocumentPickerRequest::chat_attachment();

                        let mut picker_state = picker();
                        picker_state.request(request.clone());
                        picker.set(picker_state);

                        let mut bridge_state = native_bridge();
                        bridge_state.enqueue(NativeMobileCommand::OpenDocumentPicker(request));
                        native_bridge.set(bridge_state);

                        let mut next_timeline = timeline();
                        next_timeline.push_native_command("Request Android OPEN_DOCUMENT picker for chat attachment");
                        push_agent_event(&mut next_timeline, &AgentEvent::Status("Document picker request queued for native Android layer".to_string()));
                        timeline.set(next_timeline);
                    },
                    "+"
                }

                input {
                    flex: "1",
                    background_color: "#1f2937",
                    color: "white",
                    padding: "12px",
                    border: "1px solid #4b5563",
                    border_radius: "999px",
                    placeholder: "Message DeepSeek...",
                    value: "{input}",
                    oninput: move |e| {
                        let value = e.value();
                        input.set(value.clone());
                        let mut next = composer();
                        next.draft_text = value;
                        composer.set(next);
                    },
                }

                button {
                    background_color: "#3b82f6",
                    color: "white",
                    padding: "0 20px",
                    height: "44px",
                    border_radius: "999px",
                    disabled: is_loading() || !composer().has_content(),
                    onclick: move |_| {
                        let draft = composer();
                        if !draft.has_content() { return; }

                        let (user_input, ingestion_statuses) = draft.to_core_input_with_ingestion();
                        let user_message = user_input.clone().into_message();
                        let prompt = user_message.content.clone();

                        messages.push((user_message.role.clone(), prompt.clone()));
                        let mut next_timeline = timeline();
                        next_timeline.push_user_message(prompt);
                        for status in ingestion_statuses { push_agent_event(&mut next_timeline, &AgentEvent::Status(status)); }
                        push_agent_event(&mut next_timeline, &AgentEvent::Started);
                        push_agent_event(&mut next_timeline, &AgentEvent::Status("MobileEngine turn started".to_string()));
                        timeline.set(next_timeline);

                        input.set(String::new());
                        composer.set(ChatComposerState::default());
                        is_loading.set(true);

                        spawn(async move {
                            let config = settings_state().to_config();
                            let event_timeline = timeline;
                            let event_snapshots = snapshots_state;
                            let event_diagnostics = diagnostics_state;
                            let event_native_bridge = native_bridge;
                            match run_mobile_turn_streaming(config, user_input, move |event| {
                                let mut timeline_signal = event_timeline;
                                let mut next_timeline = timeline_signal();
                                push_agent_event(&mut next_timeline, &event);
                                timeline_signal.set(next_timeline);

                                let mut snapshots_signal = event_snapshots;
                                let mut next_snapshots = snapshots_signal();
                                next_snapshots.apply_agent_event(&event);
                                snapshots_signal.set(next_snapshots);

                                let mut diagnostics_signal = event_diagnostics;
                                let mut next_diagnostics = diagnostics_signal();
                                next_diagnostics.apply_agent_event(&event);
                                diagnostics_signal.set(next_diagnostics);

                                let mut native_bridge_signal = event_native_bridge;
                                let mut next_native_bridge = native_bridge_signal();
                                if next_native_bridge.enqueue_termux_command_from_agent_event(&event) {
                                    native_bridge_signal.set(next_native_bridge);
                                }
                            }).await {
                                Ok(result) => {
                                    if let Some(final_text) = result.final_text.clone() {
                                        messages.push(("assistant".to_string(), final_text));
                                    }

                                    let mut next_timeline = timeline();
                                    push_agent_event(&mut next_timeline, &AgentEvent::Status(format!("Runtime store: {} | workspace: {} | thread: {}", result.runtime_store_root, result.workspace_root, result.thread_id)));
                                    if result.has_pending_approvals() {
                                        push_agent_event(&mut next_timeline, &AgentEvent::Status("Waiting for user approval".to_string()));
                                    }
                                    timeline.set(next_timeline);
                                    approval_cards.set(result.approval_cards);
                                }
                                Err(err) => {
                                    let mut next_timeline = timeline();
                                    push_agent_event(&mut next_timeline, &AgentEvent::Error(format!("MobileEngine error: {}", err)));
                                    timeline.set(next_timeline);
                                }
                            }
                            is_loading.set(false);
                        });
                    },
                    "Send"
                }
            }

            {bottom_nav_bar(
                active_section(),
                bottom_nav_summary,
                EventHandler::new(move |section| {
                    active_section.set(section);
                })
            )}
        }
    }
}

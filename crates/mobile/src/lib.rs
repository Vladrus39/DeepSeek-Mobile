mod agent_event_adapter;
mod agent_mode_bar;
mod agent_timeline;
mod agent_timeline_panel;
mod agent_turn_probe;
mod android_host;
#[cfg(target_os = "android")]
mod android_plugin;
mod api_probe;
mod app_update_state;
mod approval_diff_preview;
mod attachment_ingestion;
mod chat_attachment;
mod chat_file_links;
mod chat_history_panel;
mod chat_quick_actions;
pub mod chat_scroll;
mod chat_session;
mod chat_toolbar;
mod cockpit_section_panel;
#[cfg(not(target_os = "android"))]
mod desktop_native_host;
mod dev_api_key;
mod dev_bootstrap;
mod device_calibration;
mod diagnostics_panel;
mod diagnostics_state;
mod document_picker;
mod git_panel;
mod git_state;
mod health_panel;
mod host_loop;
#[cfg(target_os = "android")]
mod jni_bridge;
mod locale;
mod mcp_panel;
mod mcp_state;
mod mobile_approval_panel;
mod mobile_data_dir;
mod mobile_drawer;
mod mobile_engine_runner;
mod mobile_git_runner;
mod mobile_runtime_config;
mod mobile_snapshot_runner;
mod native_bridge;
mod native_document_picker;
mod native_event_router;
mod native_host_runtime;
mod native_pc_discovery;
mod native_termux;
mod onboarding_panel;
mod pc_discovery_probe;
mod pc_pairing_bundle_probe;
mod pc_pairing_manager;
mod pc_pairing_panel;
mod pc_pairing_persist;
mod pc_pairing_state;
mod project_diff;
mod project_files;
mod project_files_panel;
mod project_files_state;
mod project_folder_open;
mod project_transfer_state;
mod readiness_strip;
mod runtime_health;
mod saved_timeline_loader;
mod settings_panel;
mod settings_state;
mod setup_panel;
mod setup_status;
mod skills_panel;
mod skills_state;
mod snapshots_list_probe;
mod snapshots_panel;
mod snapshots_state;
mod tasks_panel;
mod tasks_state;
mod terminal_panel;
mod terminal_state;
mod termux_provisioning;
mod termux_state;
mod tools_smoke_probe;
mod ui_layout;
mod zip_transfer_probe;

use agent_event_adapter::push_agent_event;
use agent_timeline::{MobileTimelineItemKind, MobileTimelineItemStatus, MobileTimelineState};
use agent_timeline_panel::{agent_timeline_panel, ChatSnapshotRollbackProps};
use chat_attachment::ChatComposerState;
use chat_history_panel::chat_history_panel;
use chat_quick_actions::chat_quick_actions_bar;
use chat_scroll::{scroll_chat_to_bottom_force, scroll_chat_to_bottom_sticky};
use chat_session::{clear_active_timeline_display, load_index, start_new_chat, switch_chat_thread};
use chat_toolbar::chat_toolbar;
use cockpit_section_panel::cockpit_section_panel;
use deepseek_mobile_core::config::{ExecutionMode, ModelMode, ThinkingLevel};
use deepseek_mobile_core::{
    format_http_transport_error, AgentEvent, ApprovalCardView, ReviewDecision,
};
use diagnostics_state::DiagnosticsUiState;
use dioxus::prelude::*;
use document_picker::{DocumentPickerPurpose, DocumentPickerRequest, DocumentPickerState};
use git_state::GitUiState;
use health_panel::HealthQuickAction;
#[cfg(target_os = "android")]
use host_loop::sync_bridge_from_runtime;
#[cfg(not(target_os = "android"))]
use host_loop::{run_host_tick, sync_bridge_from_runtime};
use locale::{load_ui_language, pick, tr, Tr};
use mcp_state::McpUiState;
use mobile_drawer::{bottom_nav_bar, mobile_drawer, CockpitSection, MobileChromeSummary};
use mobile_engine_runner::{
    continue_mobile_approval_with_runtime_and_observer, load_mobile_approval_cards,
    run_mobile_turn_streaming, MobileApprovalContinuationUiResult, MobileTurnUiResult,
};
use native_bridge::{NativeBridgeState, NativeMobileCommand, NativeMobileEvent};
#[cfg(target_os = "android")]
use native_event_router::route_native_mobile_event;
use pc_pairing_state::{PcPairingUiState, PcPairingUiStatus};
use project_files_state::ProjectFilesUiState;
use project_folder_open::{
    active_workspace_folder_path, clear_files_focus_tree_if_leaving, show_in_app_files_focus_tree,
    try_open_pc_workspace_folder,
};
use project_transfer_state::{default_phone_workspace_root, ProjectTransferState};
use readiness_strip::readiness_strip;
use runtime_health::RuntimeHealthSnapshot;
use saved_timeline_loader::load_active_saved_events;
use settings_state::{save_config as save_settings_config, SettingsFormState};
use setup_panel::setup_panel;
use setup_status::{
    complete_first_login, initial_api_key_draft, initial_termux_path_draft, SetupSnapshot,
};
use skills_state::SkillsUiState;
use snapshots_state::SnapshotsUiState;
use tasks_state::TasksUiState;
use terminal_state::TerminalUiState;
use termux_state::TermuxWorkspaceState;
use ui_layout::screen_layout;

/// Launch the Dioxus mobile cockpit (desktop and Android).
pub fn launch() {
    dioxus::launch(app);
}

fn apply_mobile_turn_ui_result(
    mut messages: Signal<Vec<(String, String)>>,
    mut timeline: Signal<MobileTimelineState>,
    mut approval_cards: Signal<Vec<ApprovalCardView>>,
    result: MobileTurnUiResult,
) {
    if let Some(final_text) = result.final_text.clone() {
        let mut msgs = messages();
        msgs.push(("assistant".to_string(), final_text));
        messages.set(msgs);
    }

    let mut next_timeline = timeline();
    next_timeline.finish_live_assistant_message();
    if let Some(text) = result.final_text.as_deref() {
        next_timeline.publish_assistant_reply(text);
    }
    next_timeline.seal_agent_status_items();
    next_timeline.push(
        MobileTimelineItemKind::Status,
        MobileTimelineItemStatus::Done,
        "Agent status",
        format!(
            "Готово · workspace: {} · thread: {}",
            result.workspace_root, result.thread_id
        ),
    );
    if result.has_pending_approvals() {
        push_agent_event(
            &mut next_timeline,
            &AgentEvent::Status(
                "Нужно подтверждение: нажмите карточку одобрения выше.".to_string(),
            ),
        );
    }
    next_timeline.seal_open_work_items();
    next_timeline.retain_recent(120);
    timeline.set(next_timeline);
    approval_cards.set(result.approval_cards);
}

fn apply_approval_continuation_ui(
    result: &MobileApprovalContinuationUiResult,
    timeline: &mut MobileTimelineState,
    _snapshots: &mut SnapshotsUiState,
    _diagnostics: &mut DiagnosticsUiState,
    _native_bridge: &mut NativeBridgeState,
    approval_cards: &mut Vec<ApprovalCardView>,
    messages: &mut Vec<(String, String)>,
) {
    // Streaming approval continuation events are applied by the observer while
    // the continuation is running. Do not replay `result.events` here, or the
    // timeline/snapshots/native queues will duplicate every event.
    push_agent_event(
        timeline,
        &AgentEvent::Status("Approval continuation finished".to_string()),
    );
    timeline.seal_open_work_items();
    if let Some(final_text) = result.final_text.clone() {
        messages.push(("assistant".to_string(), final_text));
    }
    *approval_cards = result.remaining_approval_cards.clone();
}

#[cfg(test)]
mod approval_continuation_ui_tests {
    use super::*;

    #[test]
    fn finalizer_does_not_replay_streamed_events() {
        let result = MobileApprovalContinuationUiResult {
            events: vec![AgentEvent::Status("already streamed".to_string())],
            final_text: Some("done".to_string()),
            executed_count: 0,
            session_grant_count: 0,
            remaining_approval_cards: Vec::new(),
        };
        let mut timeline = MobileTimelineState::default();
        let mut snapshots = SnapshotsUiState::default();
        let mut diagnostics = DiagnosticsUiState::default();
        let mut native_bridge = NativeBridgeState::default();
        let mut approval_cards = Vec::new();
        let mut messages = Vec::new();

        apply_approval_continuation_ui(
            &result,
            &mut timeline,
            &mut snapshots,
            &mut diagnostics,
            &mut native_bridge,
            &mut approval_cards,
            &mut messages,
        );

        assert_eq!(timeline.items.len(), 1);
        assert_eq!(timeline.items[0].body, "Approval continuation finished");
        assert_eq!(
            messages,
            vec![("assistant".to_string(), "done".to_string())]
        );
    }
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
    let pc_files_sync_ready = pc.active_workspace_connection().is_some();
    let (pc_label, pc_online) = if pc_files_sync_ready {
        ("PC OK".to_string(), true)
    } else {
        match &pc.status {
            PcPairingUiStatus::NotConfigured => ("PC SETUP".to_string(), false),
            PcPairingUiStatus::ReadyToExport => ("PAIR".to_string(), false),
            PcPairingUiStatus::Exported => ("ZIP".to_string(), false),
            PcPairingUiStatus::WaitingForPc => ("PC WAIT".to_string(), false),
            PcPairingUiStatus::Online => ("PC LINK".to_string(), true),
            PcPairingUiStatus::Offline => ("PC OFF".to_string(), false),
            PcPairingUiStatus::Error(_) => ("PC ERR".to_string(), false),
        }
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

    let running_tasks = tasks.active_count();

    MobileChromeSummary {
        api_configured: !settings.api_key.trim().is_empty(),
        pc_label,
        pc_online,
        pc_files_sync_ready,
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

fn status_chip(
    label: String,
    colors: (&'static str, &'static str, &'static str),
    title: &'static str,
    on_click: EventHandler<()>,
) -> Element {
    let (background, border, color) = colors;
    let border_style = format!("1px solid {border}");
    rsx! {
        button {
            background_color: background,
            color,
            border: "{border_style}",
            border_radius: "999px",
            padding: "4px 7px",
            font_size: "10px",
            font_weight: "bold",
            white_space: "nowrap",
            cursor: "pointer",
            title,
            onclick: move |_| on_click.call(()),
            "{label}"
        }
    }
}

fn app() -> Element {
    dev_bootstrap::startup();

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
    let mut templates_open = use_signal(|| false);
    let mut worklog_open = use_signal(|| false);
    let mut timeline_worklog_hint = timeline;
    use_effect(move || {
        if timeline_worklog_hint().open_worklog_hint {
            worklog_open.set(true);
            let mut next = timeline_worklog_hint();
            next.open_worklog_hint = false;
            timeline_worklog_hint.set(next);
        }
    });
    let mut chat_history_open = use_signal(|| false);
    let mut drawer_open = use_signal(|| false);
    let mut active_section = use_signal(|| CockpitSection::Chat);
    let pc_pairing_state =
        use_signal(|| crate::pc_pairing_persist::load_persisted_pairing().unwrap_or_default());
    let pc_pairing_persist_signal = pc_pairing_state;
    use_effect(move || {
        let snapshot = pc_pairing_persist_signal();
        crate::pc_pairing_persist::save_pairing(&snapshot);
    });
    let mut project_files_state = use_signal(ProjectFilesUiState::default);
    let project_transfer_state = use_signal(ProjectTransferState::default);
    let mut snapshots_state = use_signal(SnapshotsUiState::default);
    let tasks_state = use_signal(TasksUiState::default);
    let mut diagnostics_state = use_signal(DiagnosticsUiState::default);
    let git_state = use_signal(GitUiState::default);
    let mut terminal_state = use_signal(TerminalUiState::default);
    let mut settings_state = use_signal(SettingsFormState::default);
    let app_update_state = use_signal(crate::app_update_state::AppUpdateUiState::default);
    let termux_state = use_signal(TermuxWorkspaceState::default);
    let mcp_state = use_signal(McpUiState::default);
    let skills_state = use_signal(SkillsUiState::default);
    let mut setup_complete = use_signal(|| {
        let settings = SettingsFormState::default();
        let termux = TermuxWorkspaceState::default();
        SetupSnapshot::collect(&settings, &termux).full_agent_ready
    });
    let ui_lang = use_signal(load_ui_language);
    let setup_api_draft = use_signal(initial_api_key_draft);
    let setup_termux_draft =
        use_signal(|| initial_termux_path_draft(&TermuxWorkspaceState::default()));
    let mut setup_error = use_signal(|| None::<String>);
    // Wizard step for setup: 0 = API + mode, 1 = Termux guided setup (with visual steps), 2 = ready to continue
    let mut setup_wizard_step = use_signal(|| 0u8);

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
    let mut termux_continuation_timeline = timeline;
    let termux_continuation_cards = approval_cards;
    let mut termux_continuation_loading = is_loading;
    let mut termux_continuation_last_event = use_signal(|| 0u64);
    use_effect(move || {
        let event_id = termux_continuation_bridge().last_event_id;
        if event_id == 0 || event_id == termux_continuation_last_event() {
            return;
        }
        let bridge_snapshot = termux_continuation_bridge();
        let Some(event) = bridge_snapshot.last_event.clone() else {
            return;
        };

        if let NativeMobileEvent::TermuxCommandFailed {
            request_id,
            message,
        } = &event
        {
            termux_continuation_last_event.set(event_id);
            if device_calibration::is_calibration_request(request_id)
                || device_calibration::is_health_probe_request(request_id)
            {
                let mut next_timeline = termux_continuation_timeline();
                push_agent_event(
                    &mut next_timeline,
                    &AgentEvent::Error(format!("Termux: {message}")),
                );
                termux_continuation_timeline.set(next_timeline);
                return;
            }
            let mut next_timeline = termux_continuation_timeline();
            push_agent_event(
                &mut next_timeline,
                &AgentEvent::Error(format!("Termux command failed: {message}")),
            );
            termux_continuation_timeline.set(next_timeline);
            termux_continuation_loading.set(false);
            return;
        }

        if let NativeMobileEvent::TermuxCommandCompleted(result) = event {
            termux_continuation_last_event.set(event_id);
            if device_calibration::is_calibration_request(&result.request_id) {
                let ok = device_calibration::note_calibration_result(
                    &result.stdout,
                    &result.stderr,
                    result.error.as_deref(),
                    result.exit_code,
                );
                let mut next_timeline = termux_continuation_timeline();
                let msg = if ok {
                    "Калибровка Termux завершена: проект готов, shell работает."
                } else if device_calibration::needs_allow_external_apps() {
                    "Termux: откройте приложение Termux и выполните: echo allow-external-apps=true >> ~/.termux/termux.properties && termux-reload-settings"
                } else {
                    "Калибровка Termux: проверьте allow-external-apps и разрешение RUN_COMMAND."
                };
                push_agent_event(&mut next_timeline, &AgentEvent::Status(msg.to_string()));
                termux_continuation_timeline.set(next_timeline);
                return;
            }
            if device_calibration::is_health_probe_request(&result.request_id) {
                let mut next_timeline = termux_continuation_timeline();
                let body = format!(
                    "stdout:\n{}\nstderr:\n{}\nexit={:?}",
                    result.stdout, result.stderr, result.exit_code
                );
                if result.exit_code == Some(0) || result.stdout.contains("DEEPSEEK_TERMUX_PROBE_OK")
                {
                    push_agent_event(
                        &mut next_timeline,
                        &AgentEvent::Status("Termux OK (фоновая проверка).".to_string()),
                    );
                    // Auto-trigger configure + seed if we are still in the initial onboarding provisioning flow.
                    // This ensures that even if queue chaining was not used, after successful probe (permission granted)
                    // we auto-config properties and seed the workspace.
                    if termux_provisioning::is_onboarding_provision_pending() {
                        let mut b = termux_continuation_bridge();
                        termux_provisioning::enqueue_configure_termux_properties(&mut b);
                        termux_provisioning::enqueue_seed_default_workspace(
                            &mut b,
                            crate::setup_status::DEFAULT_TERMUX_PROJECT_PATH,
                        );
                        crate::native_host_runtime::replace(b.clone());
                        // Clear so we don't repeat
                        termux_provisioning::clear_onboarding_provision();
                    }
                } else {
                    push_agent_event(
                        &mut next_timeline,
                        &AgentEvent::Error(format!("Termux probe failed: {}", body)),
                    );
                }
                termux_continuation_timeline.set(next_timeline);
                return;
            }
            let config = termux_continuation_settings().to_config();
            let mut event_timeline = termux_continuation_timeline;
            let mut event_cards = termux_continuation_cards;
            let mut loading_signal = termux_continuation_loading;
            spawn(async move {
                loading_signal.set(true);
                match crate::mobile_engine_runner::continue_mobile_termux_result_for_saved_request(
                    config, result,
                )
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
                        if let Some(text) = result.final_text.as_deref() {
                            next_timeline.publish_assistant_reply(text);
                        }
                        next_timeline.seal_open_work_items();
                        next_timeline.retain_recent(120);
                        event_timeline.set(next_timeline);

                        if let Some(final_text) = result.final_text.clone() {
                            let mut event_messages = messages;
                            event_messages
                                .write()
                                .push(("assistant".to_string(), final_text));
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
        crate::mobile_runtime_config::default_data_dir().join("terminal_state.json");
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

    let mut did_sync_setup_gate = use_signal(|| false);
    #[cfg_attr(not(target_os = "android"), allow(unused_mut))]
    let mut setup_gate_settings = settings_state;
    #[cfg_attr(not(target_os = "android"), allow(unused_mut))]
    let mut setup_gate_termux = termux_state;
    let mut setup_gate_complete = setup_complete;
    use_effect(move || {
        if did_sync_setup_gate() {
            return;
        }
        did_sync_setup_gate.set(true);
        #[cfg(target_os = "android")]
        {
            mobile_data_dir::ensure_android_storage_initialized();
            setup_gate_settings.set(SettingsFormState::default());
            setup_gate_termux.set(TermuxWorkspaceState::default());
        }
        let snapshot = SetupSnapshot::collect(&setup_gate_settings(), &setup_gate_termux());
        if snapshot.full_agent_ready {
            setup_gate_complete.set(true);
        }
    });

    if !did_load_saved_runtime() {
        did_load_saved_runtime.set(true);
        match load_active_saved_events() {
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
                    restored_timeline.compact_for_display();
                    restored_timeline.seal_open_work_items();
                    restored_timeline.soften_stale_errors();
                    restored_timeline.retain_recent(100);
                    timeline.set(restored_timeline);
                    snapshots_state.set(restored_snapshots);
                    diagnostics_state.set(restored_diagnostics);
                }
            }
            Err(error) => {
                if saved_timeline_loader::is_benign_restore_error(&error) {
                    // Empty/corrupt runtime store: start fresh without alarming the user.
                } else {
                    let mut next_timeline = timeline();
                    push_agent_event(
                        &mut next_timeline,
                        &AgentEvent::Error(format!("Failed to restore saved timeline: {}", error)),
                    );
                    timeline.set(next_timeline);
                }
            }
        }

        match load_mobile_approval_cards(chat_session::runtime_for_active_thread()) {
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

    let active_section_scroll = active_section;
    let mut chat_scroll_item_count = use_signal(|| 0usize);
    use_effect(move || {
        let len = timeline().len();
        if active_section_scroll() != CockpitSection::Chat || len == 0 {
            return;
        }
        let force = len > chat_scroll_item_count();
        if force {
            chat_scroll_item_count.set(len);
        }
        scroll_chat_to_bottom_sticky(force);
    });

    #[cfg(target_os = "android")]
    let did_schedule_calibration = use_signal(|| false);

    // Mirror JNI/native runtime into the UI bridge on a timer — NOT in a bare use_effect
    // (that re-ran every render and caused UI freeze / «перезагрузку» on Android).
    let mut host_bridge_sync = native_bridge;
    #[cfg(not(target_os = "android"))]
    let mut host_timeline_sync = timeline;
    use_effect(move || {
        spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                let mut bridge = host_bridge_sync.write();
                sync_bridge_from_runtime(&mut bridge);
                #[cfg(not(target_os = "android"))]
                {
                    let notes = run_host_tick(&mut bridge);
                    if !notes.is_empty() {
                        let mut next_timeline = host_timeline_sync();
                        for note in notes {
                            push_agent_event(&mut next_timeline, &AgentEvent::Status(note));
                        }
                        host_timeline_sync.set(next_timeline);
                    }
                }
            }
        });
    });

    #[cfg(target_os = "android")]
    {
        let mut android_bridge_poll = native_bridge;
        let mut android_timeline_poll = timeline;
        let mut android_cal_announced = use_signal(|| device_calibration::is_calibrated());
        let android_setup = setup_complete;
        let android_termux = termux_state;
        let android_settings = settings_state;
        let mut android_cal_scheduled = did_schedule_calibration;
        let android_warmup_done = use_signal(|| false);
        let mut android_poll_started = use_signal(|| false);
        let mut android_is_loading = is_loading;
        let mut android_loading_since = use_signal(|| None::<u64>);
        let mut android_route_composer = composer;
        let mut android_route_picker = picker;
        let mut android_route_pc = pc_pairing_state;
        let mut android_route_timeline = timeline;
        let mut android_last_routed = use_signal(|| 0u64);
        use_effect(move || {
            if android_poll_started() {
                return;
            }
            android_poll_started.set(true);
            spawn(async move {
                // Let WebView/Dioxus paint before any native Termux work.
                tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                    let now_unix = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    if android_is_loading() {
                        if android_loading_since().is_none() {
                            android_loading_since.set(Some(now_unix));
                        } else if now_unix
                            .saturating_sub(android_loading_since().unwrap_or(now_unix))
                            >= 125
                        {
                            android_is_loading.set(false);
                            android_loading_since.set(None);
                            let mut next_timeline = android_timeline_poll();
                            push_agent_event(
                                &mut next_timeline,
                                &AgentEvent::Error(
                                    "Таймаут ответа API (125 с). Проверьте сеть и ключ DeepSeek."
                                        .to_string(),
                                ),
                            );
                            next_timeline.fail_live_assistant_message();
                            next_timeline.seal_open_work_items();
                            android_timeline_poll.set(next_timeline);
                        }
                    } else {
                        android_loading_since.set(None);
                    }
                    // ADB E2E probes only — never run in normal use (avoids surprise API turns / reload feel).
                    if api_probe::is_probe_requested() {
                        api_probe::run_if_requested().await;
                    }
                    if agent_turn_probe::is_probe_requested() {
                        agent_turn_probe::run_if_requested().await;
                    }
                    if zip_transfer_probe::is_probe_requested() {
                        zip_transfer_probe::run_if_requested().await;
                    }
                    if tools_smoke_probe::is_probe_requested() {
                        tools_smoke_probe::run_if_requested();
                    }
                    if snapshots_list_probe::is_probe_requested() {
                        snapshots_list_probe::run_if_requested();
                    }
                    if pc_pairing_bundle_probe::is_probe_requested() {
                        pc_pairing_bundle_probe::run_if_requested();
                    }
                    pc_discovery_probe::recover_stale_running_marker();
                    let manual_discovery_urls = {
                        let mut bridge = android_bridge_poll();
                        sync_bridge_from_runtime(&mut bridge);
                        if bridge.last_event_id > 0 && bridge.last_event_id != android_last_routed()
                        {
                            if let Some(event) = bridge.last_event.clone() {
                                android_last_routed.set(bridge.last_event_id);
                                let routed = route_native_mobile_event(
                                    android_route_composer(),
                                    android_route_picker(),
                                    bridge.clone(),
                                    android_route_pc(),
                                    android_route_timeline(),
                                    event,
                                );
                                android_route_composer.set(routed.composer);
                                android_route_picker.set(routed.picker);
                                android_route_pc.set(routed.pc_pairing);
                                android_route_timeline.set(routed.timeline);
                                bridge = routed.native_bridge;
                                crate::native_host_runtime::replace(bridge.clone());
                            }
                        }
                        let manual_urls = pc_discovery_probe::tick(&mut bridge);
                        let mut bridge_changed = false;
                        if let Some(message) = bridge.expire_stale_termux_wait(90) {
                            let mut next_timeline = android_timeline_poll();
                            push_agent_event(&mut next_timeline, &AgentEvent::Error(message));
                            android_timeline_poll.set(next_timeline);
                            bridge_changed = true;
                        }
                        if let Some(message) = bridge.expire_stale_pc_discovery(45) {
                            let mut next_timeline = android_timeline_poll();
                            push_agent_event(&mut next_timeline, &AgentEvent::Status(message));
                            android_timeline_poll.set(next_timeline);
                            bridge_changed = true;
                        }
                        if bridge_changed {
                            crate::native_host_runtime::replace(bridge.clone());
                        }
                        android_bridge_poll.set(bridge);
                        manual_urls
                    };
                    if let Some(urls) = manual_discovery_urls {
                        spawn(async move {
                            if !pc_discovery_probe::probe_manual_urls(&urls).await {
                                pc_discovery_probe::write_result(
                                    "FAIL manual URL probe: PC Host did not respond (firewall? wrong IP?)",
                                );
                            }
                        });
                    }
                    let termux_ready = android_termux().is_valid() && android_termux().saved;
                    if device_calibration::should_retry_calibration() {
                        android_cal_scheduled.set(false);
                        android_bridge_poll
                            .write()
                            .active_termux_request_ids
                            .clear();
                    }
                    if !android_setup()
                        || (!device_calibration::is_calibration_requested()
                            && !termux_provisioning::is_onboarding_provision_pending())
                    {
                        continue;
                    }
                    if termux_ready
                        && !device_calibration::is_calibrated()
                        && !android_cal_scheduled()
                    {
                        let mut bridge = android_bridge_poll.write();
                        if device_calibration::schedule_android_calibration(
                            &mut bridge,
                            &android_termux(),
                            &android_settings(),
                        ) {
                            android_cal_scheduled.set(true);
                            let mut next_timeline = android_timeline_poll();
                            push_agent_event(
                                &mut next_timeline,
                                &AgentEvent::Status(
                                    "Калибровка агента: настройка Termux и проверка shell…"
                                        .to_string(),
                                ),
                            );
                            android_timeline_poll.set(next_timeline);
                        }
                    }
                    if device_calibration::is_calibrated() && !android_cal_announced() {
                        android_cal_announced.set(true);
                        let mut next_timeline = android_timeline_poll();
                        push_agent_event(
                            &mut next_timeline,
                            &AgentEvent::Status(
                                "Калибровка Termux завершена: проект готов, shell работает."
                                    .to_string(),
                            ),
                        );
                        android_timeline_poll.set(next_timeline);
                    }
                    // Background Termux health probe removed from startup: it queued RUN_COMMAND
                    // on every launch and left the «Ожидание ответа Android» banner stuck when the
                    // UI bridge was not synced after JNI callbacks. Use Health panel or ADB probes.
                    let _ = android_warmup_done;
                }
            });
        });
    }

    if !did_load_terminal_state() {
        did_load_terminal_state.set(true);
        let terminal_persistence_path =
            crate::mobile_runtime_config::default_data_dir().join("terminal_state.json");
        if terminal_persistence_path.exists() {
            if let Ok(saved_terminal) = TerminalUiState::load_from_file(&terminal_persistence_path)
            {
                terminal_state.set(saved_terminal);
            }
        }
    }

    if !setup_complete() {
        let snapshot = SetupSnapshot::collect(&settings_state(), &termux_state());
        let mut settings_signal = settings_state;
        let mut termux_signal = termux_state;
        let lang_signal = ui_lang;
        let mut setup_native_bridge = native_bridge;
        return rsx! {
            {setup_panel(
                lang_signal,
                snapshot,
                setup_api_draft,
                setup_termux_draft,
                setup_error,
                EventHandler::new(move |_| {
                    let mut settings = settings_signal();
                    let mut termux = termux_signal();
                    let lang = lang_signal();
                    match complete_first_login(
                        &mut settings,
                        &mut termux,
                        &setup_api_draft(),
                        &setup_termux_draft(),
                        false,
                    ) {
                        Ok(()) => {
                            let termux_ready = termux.is_valid() && termux.saved;
                            settings_signal.set(settings);
                            termux_signal.set(termux);
                            setup_error.set(None);
                            if termux_ready {
                                termux_provisioning::on_setup_saved_termux_path();
                            }
                            setup_wizard_step.set(2);
                            setup_complete.set(true);
                        }
                        Err(code) => {
                            let msg = match code.as_str() {
                                "api_key_prefix" => tr(lang, Tr::SetupErrApiPrefix).to_string(),
                                "invalid_termux_path" => termux
                                    .validation_error
                                    .clone()
                                    .unwrap_or_else(|| tr(lang, Tr::CheckTermux).to_string()),
                                other => other.to_string(),
                            };
                            setup_error.set(Some(msg));
                        }
                    }
                }),
                EventHandler::new(move |_| {
                    let mut settings = settings_signal();
                    let mut termux = termux_signal();
                    let lang = lang_signal();
                    match complete_first_login(
                        &mut settings,
                        &mut termux,
                        &setup_api_draft(),
                        "",
                        true,
                    ) {
                        Ok(()) => {
                            settings_signal.set(settings);
                            termux_signal.set(termux);
                            setup_error.set(None);
                            setup_wizard_step.set(2);
                            setup_complete.set(true);
                        }
                        Err(code) => {
                            let msg = if code == "api_key_prefix" {
                                tr(lang, Tr::SetupErrApiPrefix).to_string()
                            } else {
                                code
                            };
                            setup_error.set(Some(msg));
                        }
                    }
                }),
                EventHandler::new(move |_| {
                    let mut bridge = setup_native_bridge.write();
                    termux_provisioning::enqueue_install_termux(&mut bridge);
                    crate::native_host_runtime::replace(bridge.clone());
                }),
                EventHandler::new(move |_| {
                    let mut bridge = setup_native_bridge.write();
                    termux_provisioning::enqueue_open_termux(&mut bridge);
                    crate::native_host_runtime::replace(bridge.clone());
                }),
                EventHandler::new(move |_| {
                    // "Grant & Auto-configure": set default path, trigger the permission dialog via probe,
                    // and (after success) we will auto-run configure + seed in the result handler.
                    let mut bridge = setup_native_bridge.write();
                    let mut termux = termux_signal();
                    if !termux.saved || termux.workspace_path.trim().is_empty() {
                        termux.set_path(crate::setup_status::DEFAULT_TERMUX_PROJECT_PATH);
                        termux.set_label("Termux Project");
                        if termux.is_valid() {
                            let _ = termux.save();
                            termux_signal.set(termux.clone());
                        }
                    }
                    termux_provisioning::enqueue_run_command_permission_probe(
                        &mut bridge,
                        &termux,
                    );
                    // Chain the auto-config and seed right after the probe command.
                    // The permission dialog will be shown for the probe; once granted,
                    // subsequent commands in the queue can succeed.
                    termux_provisioning::enqueue_configure_termux_properties(&mut bridge);
                    let path = setup_termux_draft();
                    termux_provisioning::enqueue_seed_default_workspace(&mut bridge, &path);
                    crate::native_host_runtime::replace(bridge.clone());
                }),
                // Auto-config Termux (writes allow-external-apps=true). Run after the probe succeeds.
                EventHandler::new(move |_| {
                    let mut bridge = setup_native_bridge.write();
                    termux_provisioning::enqueue_configure_termux_properties(&mut bridge);
                    crate::native_host_runtime::replace(bridge.clone());
                }),
                // Seed default workspace dir + welcome file (one-tap after Termux is ready).
                EventHandler::new(move |_| {
                    let path = setup_termux_draft();
                    let mut bridge = setup_native_bridge.write();
                    termux_provisioning::enqueue_seed_default_workspace(&mut bridge, &path);
                    crate::native_host_runtime::replace(bridge.clone());
                }),
                setup_wizard_step,
                EventHandler::new(move |step: u8| {
                    setup_wizard_step.set(step);
                }),
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
        chrome_summary.api_chip_label(ui_lang()).to_string(),
        chrome_summary.api_chip_colors(),
        "Open health",
        EventHandler::new(move |_| active_section.set(CockpitSection::Health)),
    );
    let pc_chip = status_chip(
        chrome_summary.pc_label.clone(),
        chrome_summary.pc_chip_colors(),
        "Open PC Host",
        EventHandler::new(move |_| active_section.set(CockpitSection::PcHost)),
    );
    let drawer_summary = chrome_summary.clone();
    let bottom_nav_summary = chrome_summary.clone();

    let shell_layout = screen_layout();
    let shell_style = format!(
        "{}position:relative;",
        ui_layout::stack_shell_style(&shell_layout)
    );
    let mut chat_sessions = use_signal(load_index);
    let chat_title = {
        let idx = chat_sessions();
        idx.threads
            .iter()
            .find(|t| t.id == idx.active_thread_id)
            .map(|t| t.title.clone())
            .unwrap_or_else(|| pick(ui_lang(), "Чат", "Chat").to_string())
    };
    let lang_now = ui_lang();
    let msg_wait_native = pick(
        lang_now,
        "Ожидание ответа Android…",
        "Waiting for Android native callback...",
    );
    let msg_scan_pc = pick(
        lang_now,
        "Поиск PC Host в сети…",
        "Scanning local network for DeepSeek PC Host...",
    );
    let msg_bridge_error = pick(lang_now, "Ошибка моста:", "Native bridge error:");
    let templates_button_style = if templates_open() {
        "background:#2563eb;color:white;width:38px;height:42px;border-radius:999px;border:1px solid #60a5fa;flex-shrink:0;font-size:17px;"
    } else {
        "background:#172033;color:#e5e7eb;width:38px;height:42px;border-radius:999px;border:1px solid #334155;flex-shrink:0;font-size:17px;"
    };

    rsx! {
        div {
            style: "{shell_style}",
            style {
                "html,body,#main{{margin:0;padding:0;width:100%;height:100%;background:#05070c;overflow:hidden;}}body{{overscroll-behavior:none;}}*{{box-sizing:border-box;}}button,textarea,input{{font-family:inherit;}}textarea::placeholder{{color:#6b7280;}}"
            }

            {mobile_drawer(
                drawer_open(),
                active_section(),
                drawer_summary,
                ui_lang(),
                EventHandler::new(move |section| {
                    clear_files_focus_tree_if_leaving(
                        active_section(),
                        section,
                        project_files_state,
                    );
                    active_section.set(section);
                    drawer_open.set(false);
                }),
                EventHandler::new(move |_| drawer_open.set(false))
            )}

            div {
                style: "{ui_layout::app_header_style()}",

                button {
                    style: "background:#1f2937;color:white;width:38px;height:38px;border-radius:999px;border:1px solid #374151;flex-shrink:0;font-size:18px;line-height:1;",
                    title: "{tr(ui_lang(), Tr::DrawerMenu)}",
                    onclick: move |_| drawer_open.set(!drawer_open()),
                    "☰"
                }

                div {
                    style: "display:flex;flex-direction:column;align-items:center;min-width:0;flex:1;",
                    div {
                        style: "font-size:{shell_layout.header_title_font};font-weight:bold;text-align:center;line-height:1.2;",
                        "{tr(ui_lang(), Tr::AppTitle)}"
                    }
                    if active_section() != CockpitSection::Chat {
                        div {
                            style: "color:#9ca3af;font-size:{shell_layout.caption_font};text-align:center;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:min(52vw,15rem);",
                            "{active_section().localized_subtitle(ui_lang())}"
                        }
                    }
                }

                div {
                    display: "flex",
                    align_items: "center",
                    gap: "4px",
                    flex_shrink: "0",
                    {api_chip}
                    {pc_chip}
                }
            }

            if active_section() != CockpitSection::Chat {
                div {
                    style: "{ui_layout::workspace_strip_compact_style(&shell_layout)}",
                    div {
                        style: "min-width:0;flex:1;",
                        div {
                            style: "color:#f1f5f9;font-size:{shell_layout.subtitle_font};font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                            "{chrome_summary.active_project_title}"
                        }
                        div {
                            style: "color:#64748b;font-size:{shell_layout.caption_font};white-space:nowrap;overflow:hidden;text-overflow:ellipsis;",
                            "{chrome_summary.active_project_subtitle}"
                        }
                    }
                    div {
                        style: "color:#60a5fa;font-size:{shell_layout.caption_font};font-weight:600;white-space:nowrap;flex-shrink:0;",
                        "{active_section().localized_title(ui_lang())}"
                    }
                }
            }

            if active_section() == CockpitSection::Chat {
                div {
                    style: "{ui_layout::chat_sticky_chrome_style()}",
                    {chat_toolbar(
                        ui_lang(),
                        &shell_layout,
                        &chrome_summary.active_project_title,
                        &chat_title,
                        chat_history_open(),
                        &settings_state(),
                        EventHandler::new(move |_| {
                            match start_new_chat(None) {
                                Ok((_id, empty_timeline)) => {
                                    timeline.set(empty_timeline);
                                    messages.set(Vec::new());
                                    approval_cards.set(Vec::new());
                                    worklog_open.set(false);
                                    chat_history_open.set(false);
                                    chat_sessions.set(load_index());
                                    let mut next = timeline();
                                    next.push(
                                        MobileTimelineItemKind::Status,
                                        MobileTimelineItemStatus::Done,
                                        "Agent status",
                                        pick(
                                            ui_lang(),
                                            "Новый чат. История предыдущего сохранена на устройстве.",
                                            "New chat. Previous history remains on device.",
                                        ),
                                    );
                                    timeline.set(next);
                                }
                                Err(err) => {
                                    let mut next = timeline();
                                    push_agent_event(
                                        &mut next,
                                        &AgentEvent::Error(format!("New chat failed: {}", err)),
                                    );
                                    timeline.set(next);
                                }
                            }
                        }),
                        EventHandler::new(move |_| {
                            timeline.set(clear_active_timeline_display());
                            messages.set(Vec::new());
                            worklog_open.set(false);
                            let mut next = timeline();
                            next.push(
                                MobileTimelineItemKind::Status,
                                MobileTimelineItemStatus::Done,
                                "Agent status",
                                pick(
                                    ui_lang(),
                                    "Экран очищен. События на диске не удалены.",
                                    "Screen cleared. Events on disk are kept.",
                                ),
                            );
                            timeline.set(next);
                        }),
                        EventHandler::new(move |_| chat_history_open.set(!chat_history_open())),
                        EventHandler::new(move |mode: ExecutionMode| {
                            let mut next = settings_state();
                            next.execution_mode = mode;
                            let config = next.to_config();
                            match save_settings_config(&config) {
                                Ok(()) => {
                                    next.saved = true;
                                    next.save_error = None;
                                }
                                Err(error) => {
                                    next.saved = false;
                                    next.save_error = Some(error);
                                }
                            }
                            settings_state.set(next);
                        }),
                        EventHandler::new(move |mode: ModelMode| {
                            let mut next = settings_state();
                            next.model_mode = mode;
                            let config = next.to_config();
                            match save_settings_config(&config) {
                                Ok(()) => {
                                    next.saved = true;
                                    next.save_error = None;
                                }
                                Err(error) => {
                                    next.saved = false;
                                    next.save_error = Some(error);
                                }
                            }
                            settings_state.set(next);
                        }),
                        EventHandler::new(move |level: ThinkingLevel| {
                            let mut next = settings_state();
                            next.thinking_level = level;
                            let config = next.to_config();
                            match save_settings_config(&config) {
                                Ok(()) => {
                                    next.saved = true;
                                    next.save_error = None;
                                }
                                Err(error) => {
                                    next.saved = false;
                                    next.save_error = Some(error);
                                }
                            }
                            settings_state.set(next);
                        }),
                    )}

                    {
                        let health_snapshot = RuntimeHealthSnapshot::collect(
                            &settings_state(),
                            &pc_pairing_state(),
                            &termux_state(),
                            &mcp_state(),
                            &native_bridge(),
                        );
                        readiness_strip(
                            ui_lang(),
                            &health_snapshot,
                            EventHandler::new(move |section: CockpitSection| {
                                active_section.set(section);
                                drawer_open.set(false);
                            }),
                        )
                    }

                    if chat_history_open() {
                        div {
                            style: "max-height:min(40vh,320px);overflow-y:auto;flex-shrink:0;",
                            {chat_history_panel(
                            ui_lang(),
                            &chat_sessions(),
                            &chrome_summary.active_project_title,
                            &chrome_summary.active_project_subtitle,
                            EventHandler::new(move |thread_id: String| {
                                match switch_chat_thread(&thread_id) {
                                    Ok(saved_timeline) => {
                                        timeline.set(saved_timeline);
                                        messages.set(Vec::new());
                                        worklog_open.set(false);
                                        chat_history_open.set(false);
                                        chat_sessions.set(load_index());
                                        match load_mobile_approval_cards(
                                            chat_session::runtime_for_active_thread(),
                                        ) {
                                            Ok(cards) => approval_cards.set(cards),
                                            Err(_) => approval_cards.set(Vec::new()),
                                        }
                                    }
                                    Err(error) => {
                                        let mut next = timeline();
                                        push_agent_event(
                                            &mut next,
                                            &AgentEvent::Error(format!("Open chat failed: {}", error)),
                                        );
                                        timeline.set(next);
                                    }
                                }
                            }),
                            EventHandler::new(move |_| {
                                match start_new_chat(None) {
                                    Ok((_id, empty_timeline)) => {
                                        timeline.set(empty_timeline);
                                        messages.set(Vec::new());
                                        approval_cards.set(Vec::new());
                                        worklog_open.set(false);
                                        chat_history_open.set(false);
                                        chat_sessions.set(load_index());
                                    }
                                    Err(error) => {
                                        let mut next = timeline();
                                        push_agent_event(
                                            &mut next,
                                            &AgentEvent::Error(format!("New chat failed: {}", error)),
                                        );
                                        timeline.set(next);
                                    }
                                }
                            }),
                            EventHandler::new(move |_| {
                                active_section.set(CockpitSection::Files);
                                chat_history_open.set(false);
                            }),
                            EventHandler::new(move |thread_id: String| {
                                match chat_session::delete_chat_thread(&thread_id) {
                                    Ok(active_id) => {
                                        chat_sessions.set(load_index());
                                        match switch_chat_thread(&active_id) {
                                            Ok(saved_timeline) => {
                                                timeline.set(saved_timeline);
                                                messages.set(Vec::new());
                                                approval_cards.set(Vec::new());
                                                match load_mobile_approval_cards(
                                                    chat_session::runtime_for_active_thread(),
                                                ) {
                                                    Ok(cards) => approval_cards.set(cards),
                                                    Err(_) => approval_cards.set(Vec::new()),
                                                }
                                            }
                                            Err(error) => {
                                                let mut next = timeline();
                                                push_agent_event(
                                                    &mut next,
                                                    &AgentEvent::Error(format!(
                                                        "Switch after delete failed: {}",
                                                        error
                                                    )),
                                                );
                                                timeline.set(next);
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        let mut next = timeline();
                                        push_agent_event(
                                            &mut next,
                                            &AgentEvent::Error(format!(
                                                "Delete chat failed: {}",
                                                error
                                            )),
                                        );
                                        timeline.set(next);
                                    }
                                }
                            }),
                            )}
                        }
                    }
                }
            }

            div {
                id: chat_scroll::CHAT_SCROLL_PANEL_ID,
                style: "{ui_layout::main_scroll_panel_style()}",
                if active_section() == CockpitSection::Chat {
                    {
                        let snap = snapshots_state();
                        let snapshot_rollback = ChatSnapshotRollbackProps {
                            latest_id: snap.latest().map(|s| s.id.clone()),
                            latest_summary: snap.latest().map(|s| {
                                format!(
                                    "{} · {} file(s) · {} bytes",
                                    s.id, s.file_count, s.total_bytes
                                )
                            }),
                            pending: snap.pending_restore_snapshot().map(|s| {
                                (s.id.clone(), s.file_count, s.total_bytes)
                            }),
                            restore_in_progress: snap.restore_in_progress,
                            on_request_restore: EventHandler::new(move |snapshot_id: String| {
                                snapshots_state.write().request_restore(&snapshot_id);
                            }),
                            on_confirm_restore: EventHandler::new(move |_| {
                                let snapshot_id = snapshots_state()
                                    .pending_restore_snapshot_id
                                    .clone();
                                let Some(snapshot_id) = snapshot_id else {
                                    return;
                                };
                                let settings_signal = settings_state;
                                let mut snapshots_signal = snapshots_state;
                                let mut timeline_signal = timeline;
                                is_loading.set(true);
                                spawn(async move {
                                    snapshots_signal.write().confirm_restore();
                                    let config = settings_signal().to_config();
                                    let runtime =
                                        crate::mobile_runtime_config::MobileRuntimeConfig::default_mobile();
                                    match crate::mobile_snapshot_runner::restore_snapshot_by_id(
                                        config,
                                        runtime,
                                        &snapshot_id,
                                    )
                                    .await
                                    {
                                        Ok(report) => {
                                            let mut s = snapshots_signal.write();
                                            s.restore_in_progress = false;
                                            s.pending_restore_snapshot_id = None;
                                            s.last_restore_report = Some(report.clone());
                                            s.last_error = None;
                                            let mut next_timeline = timeline_signal();
                                            push_agent_event(
                                                &mut next_timeline,
                                                &AgentEvent::Status(format!(
                                                    "Restored snapshot {}: {} file(s) restored",
                                                    snapshot_id, report
                                                )),
                                            );
                                            timeline_signal.set(next_timeline);
                                        }
                                        Err(error) => {
                                            let mut s = snapshots_signal.write();
                                            s.restore_in_progress = false;
                                            s.pending_restore_snapshot_id = None;
                                            s.last_error = Some(error.to_string());
                                            let mut next_timeline = timeline_signal();
                                            push_agent_event(
                                                &mut next_timeline,
                                                &AgentEvent::Error(format!(
                                                    "Snapshot restore failed: {}",
                                                    error
                                                )),
                                            );
                                            timeline_signal.set(next_timeline);
                                        }
                                    }
                                    is_loading.set(false);
                                });
                            }),
                            on_cancel_restore: EventHandler::new(move |_| {
                                snapshots_state.write().cancel_restore();
                            }),
                        };
                        agent_timeline_panel(
                        ui_lang(),
                        &timeline(),
                        &approval_cards(),
                        EventHandler::new(move |(approval_id, decision): (String, ReviewDecision)| {
                            let config = settings_state().to_config();
                            is_loading.set(true);
                            spawn(async move {
                                let event_timeline = timeline;
                                let event_snapshots = snapshots_state;
                                let event_diagnostics = diagnostics_state;
                                let event_native_bridge = native_bridge;
                                let event_approval_cards = approval_cards;
                                let event_messages = messages;
                                match continue_mobile_approval_with_runtime_and_observer(
                                    config,
                                    approval_id.clone(),
                                    decision.clone(),
                                    chat_session::runtime_for_active_thread(),
                                    move |event| {
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
                                        let queued_termux = next_native_bridge
                                            .enqueue_termux_command_from_agent_event(&event);
                                        let queued_phone = next_native_bridge
                                            .enqueue_phone_native_from_agent_event(&event);
                                        if queued_termux || queued_phone {
                                            native_bridge_signal.set(next_native_bridge);
                                        }
                                    },
                                )
                                .await
                                {
                                    Ok(result) => {
                                        let mut next_timeline = event_timeline();
                                        let mut next_snapshots = event_snapshots();
                                        let mut next_diagnostics = event_diagnostics();
                                        let mut next_native_bridge = event_native_bridge();
                                        let mut next_messages = event_messages();
                                        let mut next_cards = event_approval_cards();
                                        apply_approval_continuation_ui(
                                            &result,
                                            &mut next_timeline,
                                            &mut next_snapshots,
                                            &mut next_diagnostics,
                                            &mut next_native_bridge,
                                            &mut next_cards,
                                            &mut next_messages,
                                        );
                                        push_agent_event(
                                            &mut next_timeline,
                                            &AgentEvent::Status(format!(
                                                "Executed tools: {} | session grants: {}",
                                                result.executed_count, result.session_grant_count
                                            )),
                                        );
                                        timeline.set(next_timeline);
                                        snapshots_state.set(next_snapshots);
                                        diagnostics_state.set(next_diagnostics);
                                        native_bridge.set(next_native_bridge);
                                        messages.set(next_messages);
                                        approval_cards.set(next_cards);
                                    }
                                    Err(error) => {
                                        let mut next_timeline = event_timeline();
                                        push_agent_event(
                                            &mut next_timeline,
                                            &AgentEvent::Error(format!(
                                                "Approval continuation failed: {}",
                                                error
                                            )),
                                        );
                                        timeline.set(next_timeline);
                                    }
                                }
                                is_loading.set(false);
                            });
                        }),
                        EventHandler::new(move |path: String| {
                            active_section.set(CockpitSection::Files);
                            chat_history_open.set(false);
                            drawer_open.set(false);

                            if let Some(connection) =
                                pc_pairing_state().active_workspace_connection()
                            {
                                if let Some(gateway_config) = connection.pc_gateway.clone() {
                                    let client =
                                        deepseek_mobile_core::PcGatewayClient::new(gateway_config);
                                    let workspace_id = connection.workspace_id.clone();
                                    let workspace_root =
                                        connection.workspace_root.display().to_string();
                                    let mut files_signal = project_files_state;
                                    spawn(async move {
                                        let mut files = files_signal();
                                        files.workspace_root = workspace_root;
                                        files.reset_to_workspace_root();
                                        files.loaded = true;
                                        let _ = files
                                            .open_file_via_pc(&client, &path, &workspace_id)
                                            .await;
                                        files_signal.set(files);
                                    });
                                    return;
                                }
                            }

                            let runtime =
                                crate::mobile_runtime_config::MobileRuntimeConfig::default_mobile();
                            let mut files = project_files_state.write();
                            files.workspace_root = runtime.workspace_root_display();
                            files.reset_to_workspace_root();
                            files.refresh();
                            files.open_file(path);
                        }),
                        EventHandler::new(move |_| {
                            let folder_path =
                                active_workspace_folder_path(&pc_pairing_state());
                            if let Some(connection) =
                                pc_pairing_state().active_workspace_connection()
                            {
                                if let Some(gateway_config) = connection.pc_gateway.clone() {
                                    let client =
                                        deepseek_mobile_core::PcGatewayClient::new(gateway_config);
                                    let workspace_id = connection.workspace_id.clone();
                                    let mut active = active_section;
                                    let mut history = chat_history_open;
                                    let mut drawer = drawer_open;
                                    let files = project_files_state;
                                    let pairing = pc_pairing_state();
                                    spawn(async move {
                                        if !try_open_pc_workspace_folder(&client, &workspace_id)
                                            .await
                                        {
                                            show_in_app_files_focus_tree(
                                                active,
                                                history,
                                                drawer,
                                                files,
                                                &pairing,
                                            );
                                        }
                                    });
                                    return;
                                }
                            }

                            native_bridge
                                .write()
                                .enqueue_open_workspace_folder(folder_path);
                            #[cfg(not(target_os = "android"))]
                            {
                                let mut bridge = native_bridge.write();
                                run_host_tick(&mut bridge);
                                if matches!(
                                    bridge.last_event,
                                    Some(NativeMobileEvent::WorkspaceFolderOpenFailed { .. })
                                ) {
                                    show_in_app_files_focus_tree(
                                        active_section,
                                        chat_history_open,
                                        drawer_open,
                                        project_files_state,
                                        &pc_pairing_state(),
                                    );
                                }
                            }
                        }),
                        worklog_open(),
                        EventHandler::new(move |_| worklog_open.set(!worklog_open())),
                        snapshot_rollback,
                        )
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
                        app_update_state,
                        ui_lang,
                        EventHandler::new(move |(approval_id, decision): (String, ReviewDecision)| {
                            let approval_id = approval_id.clone();
                            let decision = decision.clone();
                            let config = settings_state().to_config();
                            is_loading.set(true);
                            spawn(async move {
                                let event_timeline = timeline;
                                let event_snapshots = snapshots_state;
                                let event_diagnostics = diagnostics_state;
                                let event_native_bridge = native_bridge;
                                let event_approval_cards = approval_cards;
                                let event_messages = messages;
                                match continue_mobile_approval_with_runtime_and_observer(
                                    config,
                                    approval_id.clone(),
                                    decision.clone(),
                                    chat_session::runtime_for_active_thread(),
                                    move |event| {
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
                                        let queued_termux = next_native_bridge
                                            .enqueue_termux_command_from_agent_event(&event);
                                        let queued_phone = next_native_bridge
                                            .enqueue_phone_native_from_agent_event(&event);
                                        if queued_termux || queued_phone {
                                            native_bridge_signal.set(next_native_bridge);
                                        }
                                    },
                                )
                                .await
                                {
                                    Ok(result) => {
                                        let mut next_timeline = event_timeline();
                                        let mut next_snapshots = event_snapshots();
                                        let mut next_diagnostics = event_diagnostics();
                                        let mut next_native_bridge = event_native_bridge();
                                        let mut next_messages = event_messages();
                                        let mut next_cards = event_approval_cards();
                                        apply_approval_continuation_ui(
                                            &result,
                                            &mut next_timeline,
                                            &mut next_snapshots,
                                            &mut next_diagnostics,
                                            &mut next_native_bridge,
                                            &mut next_cards,
                                            &mut next_messages,
                                        );
                                        push_agent_event(
                                            &mut next_timeline,
                                            &AgentEvent::Status(format!(
                                                "Executed tools: {} | session grants: {}",
                                                result.executed_count, result.session_grant_count
                                            )),
                                        );
                                        timeline.set(next_timeline);
                                        snapshots_state.set(next_snapshots);
                                        diagnostics_state.set(next_diagnostics);
                                        native_bridge.set(next_native_bridge);
                                        messages.set(next_messages);
                                        approval_cards.set(next_cards);
                                    }
                                    Err(error) => {
                                        let mut next_timeline = event_timeline();
                                        push_agent_event(
                                            &mut next_timeline,
                                            &AgentEvent::Error(format!(
                                                "Approval continuation failed: {}",
                                                error
                                            )),
                                        );
                                        timeline.set(next_timeline);
                                    }
                                }
                                is_loading.set(false);
                            });
                        }),
                        EventHandler::new(move |action: HealthQuickAction| {
                            match action {
                                HealthQuickAction::OpenSettings => {
                                    active_section.set(CockpitSection::Settings);
                                    drawer_open.set(false);
                                }
                                HealthQuickAction::OpenPcHost => {
                                    active_section.set(CockpitSection::PcHost);
                                    drawer_open.set(false);
                                }
                                HealthQuickAction::OpenFiles => {
                                    active_section.set(CockpitSection::Files);
                                    drawer_open.set(false);
                                }
                                HealthQuickAction::RunTermuxCheck => {
                                    let mut bridge = native_bridge.write();
                                    let workdir = termux_state().workspace_path.clone();
                                    bridge.enqueue_termux_command(
                                        deepseek_mobile_core::TermuxExecRequest {
                                            request_id: device_calibration::HEALTH_TERMUX_PROBE_ID
                                                .to_string(),
                                            command: "pwd && ls -la".to_string(),
                                            working_dir: std::path::PathBuf::from(workdir),
                                            timeout_secs: Some(90),
                                        },
                                    );
                                    drop(bridge);
                                    active_section.set(CockpitSection::Chat);
                                    drawer_open.set(false);
                                    let mut next_timeline = timeline();
                                    push_agent_event(
                                        &mut next_timeline,
                                        &AgentEvent::Status(
                                            pick(
                                                ui_lang(),
                                                "Проверка Termux: команда pwd отправлена в фоне…",
                                                "Termux check: pwd command queued in background…",
                                            )
                                            .to_string(),
                                        ),
                                    );
                                    timeline.set(next_timeline);
                                }
                            }
                        }),
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
                    "{msg_wait_native}"
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
                    "{msg_scan_pc}"
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
                    "{msg_bridge_error} {error}"
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

            if active_section() == CockpitSection::Chat && templates_open() {
                {chat_quick_actions_bar(
                    ui_lang(),
                    EventHandler::new(move |prompt: String| {
                        let mut next = composer();
                        next.draft_text = prompt.clone();
                        composer.set(next);
                        input.set(prompt);
                        templates_open.set(false);
                    }),
                    EventHandler::new(move |_| templates_open.set(false)),
                )}
            }

            if is_loading() {
                div {
                    style: "margin-top:8px;color:#93c5fd;font-size:12px;",
                    {pick(ui_lang(), "Думает…", "Thinking…")}
                }
            }

            div {
                style: "display:flex;gap:6px;margin-top:8px;align-items:center;max-width:100%;",

                button {
                    style: "background:#1f2937;color:white;width:38px;height:42px;border-radius:999px;border:1px solid #4b5563;flex-shrink:0;font-size:18px;",
                    onclick: move |_| {
                        let request = DocumentPickerRequest::chat_attachment();

                        let mut picker_state = picker();
                        picker_state.request(request.clone());
                        picker.set(picker_state);

                        let mut bridge_state = native_bridge();
                        bridge_state.enqueue(NativeMobileCommand::OpenDocumentPicker(request));
                        native_bridge.set(bridge_state);

                        let mut next_timeline = timeline();
                        next_timeline.push_native_command(
                            "Request Android OPEN_DOCUMENT picker for chat attachment",
                        );
                        push_agent_event(
                            &mut next_timeline,
                            &AgentEvent::Status(
                                "Document picker request queued for native Android layer"
                                    .to_string(),
                            ),
                        );
                        timeline.set(next_timeline);
                    },
                    "+"
                }

                if active_section() == CockpitSection::Chat {
                    button {
                        style: "{templates_button_style}",
                        onclick: move |_| templates_open.set(!templates_open()),
                        "⚡"
                    }
                }

                textarea {
                    style: "flex:1 1 auto;min-width:0;background:#172033;color:white;padding:10px 12px;border:1px solid #334155;border-radius:20px;font-size:{shell_layout.body_font};min-height:{shell_layout.chat_input_min_height};max-height:128px;resize:none;box-sizing:border-box;line-height:1.35;",
                    placeholder: "{tr(ui_lang(), Tr::ChatPlaceholder)}",
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
                    style: "background:#3b82f6;color:white;width:46px;min-height:42px;border-radius:999px;font-size:21px;line-height:1;flex-shrink:0;",
                    title: "{tr(ui_lang(), Tr::Send)}",
                    disabled: is_loading() || !composer().has_content(),
                    onclick: move |_| {
                        let draft = composer();
                        if !draft.has_content() { return; }

                        let config = crate::settings_state::load_config_for_agent_turn();
                        if config.execution_mode == deepseek_mobile_core::config::ExecutionMode::Plan {
                            let mut next_timeline = timeline();
                            push_agent_event(
                                &mut next_timeline,
                                &AgentEvent::Status(
                                    pick(
                                        ui_lang(),
                                        "Режим «план»: инструменты не выполняются. Нажмите A·агент для exec_shell, файлов, git…",
                                        "Plan mode: tools are not executed. Tap A·agent for exec_shell, files, git…",
                                    )
                                    .to_string(),
                                ),
                            );
                            timeline.set(next_timeline);
                        }
                        if !config.api_key.trim().starts_with("sk-") {
                            let mut next_timeline = timeline();
                            push_agent_event(
                                &mut next_timeline,
                                &AgentEvent::Error(
                                    pick(
                                        ui_lang(),
                                        "Нет API-ключа DeepSeek. ☰ → Настройки → вставьте sk-… и сохраните.",
                                        "DeepSeek API key missing. Open ☰ → Settings, paste sk-… and save.",
                                    )
                                    .to_string(),
                                ),
                            );
                            timeline.set(next_timeline);
                            return;
                        }

                        let (user_input, ingestion_statuses) = draft.to_core_input_with_ingestion();
                        let user_message = user_input.clone().into_message();
                        let prompt = user_message.content.clone();

                        messages.push((user_message.role.clone(), prompt.clone()));
                        let mut next_timeline = timeline();
                        next_timeline.push_user_message(prompt);
                        for status in ingestion_statuses { push_agent_event(&mut next_timeline, &AgentEvent::Status(status)); }
                        push_agent_event(&mut next_timeline, &AgentEvent::Started);
                        push_agent_event(
                            &mut next_timeline,
                            &AgentEvent::Status(
                                pick(
                                    ui_lang(),
                                    "Ход агента запущен…",
                                    "Agent turn started…",
                                )
                                .to_string(),
                            ),
                        );
                        chat_session::touch_active_thread();
                        timeline.set(next_timeline);
                        scroll_chat_to_bottom_force();

                        input.set(String::new());
                        composer.set(ChatComposerState::default());
                        worklog_open.set(false);
                        is_loading.set(true);

                        spawn(async move {
                            let config = config;
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
                                if next_native_bridge.enqueue_termux_command_from_agent_event(&event)
                                    || next_native_bridge.enqueue_phone_native_from_agent_event(&event)
                                {
                                    native_bridge_signal.set(next_native_bridge);
                                }
                            }).await {
                                Ok(result) => {
                                    apply_mobile_turn_ui_result(
                                        messages,
                                        timeline,
                                        approval_cards,
                                        result,
                                    );
                                }
                                Err(err) => {
                                    let mut next_timeline = timeline();
                                    next_timeline.fail_live_assistant_message();
                                    push_agent_event(
                                        &mut next_timeline,
                                        &AgentEvent::Error(format_http_transport_error(&err)),
                                    );
                                    timeline.set(next_timeline);
                                }
                            }
                            is_loading.set(false);
                        });
                    },
                    "➤"
                }
            }

            {bottom_nav_bar(
                active_section(),
                bottom_nav_summary,
                ui_lang(),
                EventHandler::new(move |section| {
                    clear_files_focus_tree_if_leaving(
                        active_section(),
                        section,
                        project_files_state,
                    );
                    active_section.set(section);
                })
            )}
        }
    }
}

mod agent_event_adapter;
mod agent_timeline;
mod agent_timeline_panel;
mod approval_diff_preview;
mod attachment_ingestion;
mod chat_attachment;
mod cockpit_section_panel;
mod diagnostics_panel;
mod git_panel;
mod git_state;
mod diagnostics_state;
mod document_picker;
mod mobile_approval_panel;
mod mobile_drawer;
mod mobile_engine_runner;
mod mobile_runtime_config;
mod native_bridge;
mod native_document_picker;
mod native_event_router;
mod native_pc_discovery;
mod pc_pairing_manager;
mod pc_pairing_panel;
mod pc_pairing_state;
mod project_diff;
mod project_files;
mod project_files_panel;
mod project_files_state;
mod saved_timeline_loader;
mod snapshots_panel;
mod snapshots_state;
mod terminal_panel;
mod terminal_state;

use agent_event_adapter::push_agent_event;
use agent_timeline::MobileTimelineState;
use agent_timeline_panel::agent_timeline_panel;
use chat_attachment::ChatComposerState;
use cockpit_section_panel::cockpit_section_panel;
use deepseek_mobile_core::{AgentEvent, ApprovalCardView, Config, ReviewDecision};
use diagnostics_state::DiagnosticsUiState;
use git_state::GitUiState;
use dioxus::prelude::*;
use document_picker::{DocumentPickerRequest, DocumentPickerState};
use mobile_approval_panel::mobile_approval_panel;
use mobile_drawer::{mobile_drawer, CockpitSection};
use mobile_engine_runner::{
    continue_mobile_approval, load_default_mobile_approval_cards, run_mobile_turn_streaming,
};
use native_bridge::{NativeBridgeState, NativeMobileCommand, NativeMobileEvent};
use pc_pairing_state::PcPairingUiState;
use project_files_state::ProjectFilesUiState;
use saved_timeline_loader::load_default_saved_events;
use snapshots_state::SnapshotsUiState;
use terminal_state::TerminalUiState;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut messages = use_signal(Vec::<(String, String)>::new);
    let mut input = use_signal(String::new);
    let mut composer = use_signal(ChatComposerState::default);
    let mut timeline = use_signal(MobileTimelineState::default);
    let mut approval_cards = use_signal(Vec::<ApprovalCardView>::new);
    let mut did_load_saved_runtime = use_signal(|| false);
    let mut picker = use_signal(DocumentPickerState::default);
    let mut native_bridge = use_signal(NativeBridgeState::default);
    let mut is_loading = use_signal(|| false);
    let mut drawer_open = use_signal(|| false);
    let mut active_section = use_signal(|| CockpitSection::Chat);
    let pc_pairing_state = use_signal(PcPairingUiState::default);
    let project_files_state = use_signal(ProjectFilesUiState::default);
    let mut snapshots_state = use_signal(SnapshotsUiState::default);
    let mut diagnostics_state = use_signal(DiagnosticsUiState::default);
    let git_state = use_signal(GitUiState::default);
    let terminal_state = use_signal(TerminalUiState::default);

    // Route native bridge terminal events into terminal UI state
    let terminal_event_bridge = native_bridge;
    let mut terminal_event_state = terminal_state;
    use_effect(move || {
        let event = terminal_event_bridge().last_event.clone();
        match event {
            Some(NativeMobileEvent::TerminalOpened { session_id, title, cwd }) => {
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
            Some(NativeMobileEvent::TerminalClosed { session_id, exit_code }) => {
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
                    div { font_size: "18px", font_weight: "bold", "DeepSeek Mobile" }
                    div { color: "#9ca3af", font_size: "12px", "{active_section().subtitle()}" }
                }

                div {
                    background_color: "#111827",
                    color: "#d1d5db",
                    border: "1px solid #374151",
                    border_radius: "999px",
                    padding: "8px 10px",
                    font_size: "12px",
                    "API"
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
                            is_loading.set(true);
                            spawn(async move {
                                match continue_mobile_approval(Config::default(), approval_id.clone(), decision.clone()).await {
                                    Ok(result) => {
                                        let mut next_timeline = timeline();
                                        push_agent_event(&mut next_timeline, &AgentEvent::Status(format!("Approval decision applied: {:?}", decision)));
                                        let mut next_snapshots = snapshots_state();
                                        let mut next_diagnostics = diagnostics_state();
                                        for event in &result.events {
                                            push_agent_event(&mut next_timeline, event);
                                            next_snapshots.apply_agent_event(event);
                                            next_diagnostics.apply_agent_event(event);
                                        }
                                        push_agent_event(&mut next_timeline, &AgentEvent::Status(format!("Executed tools: {} | session grants: {}", result.executed_count, result.session_grant_count)));
                                        timeline.set(next_timeline);
                                        snapshots_state.set(next_snapshots);
                                        diagnostics_state.set(next_diagnostics);
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
                        pc_pairing_state,
                        native_bridge,
                        project_files_state,
                        snapshots_state,
                        diagnostics_state,
                        git_state,
                        terminal_state,
                    )}
                }
            }

            if picker().is_waiting_for_native_picker() || native_bridge().has_pending_commands() {
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
                            let config = Config::default();
                            let event_timeline = timeline;
                            let event_snapshots = snapshots_state;
                            let event_diagnostics = diagnostics_state;
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
        }
    }
}

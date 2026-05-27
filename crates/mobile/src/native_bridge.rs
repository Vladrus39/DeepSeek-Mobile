use crate::document_picker::{DocumentPickerRequest, PickedDocument};
use crate::native_document_picker::{AndroidDocumentPickerCallback, AndroidDocumentPickerCommand};
use crate::native_pc_discovery::{
    AndroidPcGatewayDiscoveryCallback, AndroidPcGatewayDiscoveryCommand,
};
use crate::native_termux::{AndroidTermuxCallback, AndroidTermuxCommand};
use deepseek_mobile_core::tools::phone_bridge::PhoneNativeRequest;
use deepseek_mobile_core::{
    AgentEvent, PcGatewayDiscoveryReport, RuntimeThreadStore, TermuxExecRequest, TermuxExecResult,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeMobileCommand {
    OpenDocumentPicker(DocumentPickerRequest),
    StartPcGatewayDiscovery(AndroidPcGatewayDiscoveryCommand),
    ShareFile {
        path: String,
        mime_type: Option<String>,
    },
    OpenUrl {
        url: String,
    },
    LaunchApp {
        package: String,
    },
    OpenSystemSettings,
    OpenTerminal {
        workspace_id: String,
    },
    TerminalInput {
        session_id: String,
        input: String,
    },
    CloseTerminal {
        session_id: String,
    },
    RunTermuxCommand(TermuxExecRequest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeMobileEvent {
    DocumentsPicked(Vec<PickedDocument>),
    DocumentPickerCancelled,
    DocumentPickerFailed(String),
    PcGatewayDiscoveryStarted {
        request_id: String,
        service_type: String,
    },
    PcGatewayDiscoveryUpdated(PcGatewayDiscoveryReport),
    PcGatewayDiscoveryCompleted(PcGatewayDiscoveryReport),
    PcGatewayDiscoveryFailed(String),
    FileShared,
    ShareFailed(String),
    TerminalOpened {
        session_id: String,
        title: String,
        cwd: String,
    },
    TerminalOutput {
        session_id: String,
        chunk: String,
    },
    TerminalClosed {
        session_id: String,
        exit_code: Option<i32>,
    },
    TerminalFailed {
        session_id: Option<String>,
        message: String,
    },
    TermuxCommandCompleted(TermuxExecResult),
    TermuxCommandFailed {
        request_id: String,
        message: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeBridgeState {
    pub pending_commands: Vec<NativeMobileCommand>,
    pub last_event: Option<NativeMobileEvent>,
    pub last_error: Option<String>,
    pub active_document_picker_request_id: Option<String>,
    pub active_pc_discovery_request_id: Option<String>,
    pub active_termux_request_ids: Vec<String>,
    /// When the oldest in-flight Termux RUN_COMMAND was handed to Android.
    pub active_termux_since_unix: Option<u64>,
    pub active_pc_discovery_since_unix: Option<u64>,
    pub last_event_id: u64,
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn phone_native_request_from_agent_event(event: &AgentEvent) -> Option<PhoneNativeRequest> {
    let AgentEvent::ToolCallFinished(result) = event else {
        return None;
    };
    let metadata = result.metadata.as_ref()?;
    if metadata
        .get("phone_native_pending")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return None;
    }
    serde_json::from_value(metadata.get("phone_native_request")?.clone()).ok()
}

pub fn termux_request_from_agent_event(event: &AgentEvent) -> Option<TermuxExecRequest> {
    let AgentEvent::ToolCallFinished(result) = event else {
        return None;
    };
    let metadata = result.metadata.as_ref()?;
    if metadata
        .get("termux_execution_pending")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return None;
    }
    serde_json::from_value(metadata.get("termux_exec_request")?.clone()).ok()
}

fn has_saved_pending_termux_request(request_id: &str) -> bool {
    let root = crate::mobile_runtime_config::default_data_dir().join("runtime_store");
    RuntimeThreadStore::open(root)
        .and_then(|store| store.load_pending_termux(request_id))
        .is_ok()
}

impl NativeBridgeState {
    pub fn enqueue(&mut self, command: NativeMobileCommand) {
        self.pending_commands.push(command);
        crate::native_host_runtime::replace(self.clone());
    }

    pub fn enqueue_document_picker(&mut self, request: DocumentPickerRequest) {
        self.enqueue(NativeMobileCommand::OpenDocumentPicker(request));
    }

    pub fn enqueue_pc_gateway_discovery(&mut self, request_id: impl Into<String>) {
        self.enqueue(NativeMobileCommand::StartPcGatewayDiscovery(
            AndroidPcGatewayDiscoveryCommand::new(request_id),
        ));
    }

    pub fn enqueue_open_terminal(&mut self, workspace_id: impl Into<String>) {
        self.enqueue(NativeMobileCommand::OpenTerminal {
            workspace_id: workspace_id.into(),
        });
    }

    pub fn enqueue_terminal_input(
        &mut self,
        session_id: impl Into<String>,
        input: impl Into<String>,
    ) {
        self.enqueue(NativeMobileCommand::TerminalInput {
            session_id: session_id.into(),
            input: input.into(),
        });
    }

    pub fn enqueue_close_terminal(&mut self, session_id: impl Into<String>) {
        self.enqueue(NativeMobileCommand::CloseTerminal {
            session_id: session_id.into(),
        });
    }

    pub fn enqueue_share_file(&mut self, path: impl Into<String>) {
        self.enqueue(NativeMobileCommand::ShareFile {
            path: path.into(),
            mime_type: Some("application/zip".to_string()),
        });
    }

    pub fn enqueue_termux_command(&mut self, request: TermuxExecRequest) {
        self.enqueue(NativeMobileCommand::RunTermuxCommand(request));
    }

    pub fn enqueue_termux_command_from_agent_event(&mut self, event: &AgentEvent) -> bool {
        let Some(request) = termux_request_from_agent_event(event) else {
            return false;
        };
        self.enqueue_termux_command(request);
        true
    }

    pub fn enqueue_phone_native_from_agent_event(&mut self, event: &AgentEvent) -> bool {
        let Some(request) = phone_native_request_from_agent_event(event) else {
            return false;
        };
        match request.action.as_str() {
            "open_url" => {
                let Some(url) = request.url.filter(|url| !url.trim().is_empty()) else {
                    return false;
                };
                self.enqueue(NativeMobileCommand::OpenUrl { url });
            }
            "share_file" => {
                let Some(path) = request.path.filter(|path| !path.trim().is_empty()) else {
                    return false;
                };
                self.enqueue(NativeMobileCommand::ShareFile {
                    path,
                    mime_type: request.mime_type,
                });
            }
            "launch_app" => {
                let Some(package) = request.package.filter(|pkg| !pkg.trim().is_empty()) else {
                    return false;
                };
                self.enqueue(NativeMobileCommand::LaunchApp { package });
            }
            "open_settings" => {
                self.enqueue(NativeMobileCommand::OpenSystemSettings);
            }
            _ => return false,
        }
        true
    }

    pub fn pop_next_command(&mut self) -> Option<NativeMobileCommand> {
        if self.pending_commands.is_empty() {
            None
        } else {
            Some(self.pending_commands.remove(0))
        }
    }

    pub fn pop_next_android_document_picker_command(
        &mut self,
    ) -> Option<AndroidDocumentPickerCommand> {
        let command_index = self
            .pending_commands
            .iter()
            .position(|command| matches!(command, NativeMobileCommand::OpenDocumentPicker(_)))?;
        match self.pending_commands.remove(command_index) {
            NativeMobileCommand::OpenDocumentPicker(request) => {
                let command = AndroidDocumentPickerCommand::from_request(&request);
                self.active_document_picker_request_id = Some(command.request_id.clone());
                Some(command)
            }
            _ => None,
        }
    }

    pub fn pop_next_android_pc_discovery_command(
        &mut self,
    ) -> Option<AndroidPcGatewayDiscoveryCommand> {
        let command_index = self.pending_commands.iter().position(|command| {
            matches!(command, NativeMobileCommand::StartPcGatewayDiscovery(_))
        })?;
        match self.pending_commands.remove(command_index) {
            NativeMobileCommand::StartPcGatewayDiscovery(command) => {
                self.active_pc_discovery_request_id = Some(command.request_id.clone());
                self.active_pc_discovery_since_unix = Some(unix_now_secs());
                Some(command)
            }
            _ => None,
        }
    }

    pub fn pop_terminal_command(&mut self) -> Option<NativeMobileCommand> {
        let command_index = self.pending_commands.iter().position(|command| {
            matches!(
                command,
                NativeMobileCommand::OpenTerminal { .. }
                    | NativeMobileCommand::TerminalInput { .. }
                    | NativeMobileCommand::CloseTerminal { .. }
            )
        })?;
        Some(self.pending_commands.remove(command_index))
    }

    pub fn pop_next_android_termux_command(&mut self) -> Option<AndroidTermuxCommand> {
        let command_index = self
            .pending_commands
            .iter()
            .position(|command| matches!(command, NativeMobileCommand::RunTermuxCommand(_)))?;
        match self.pending_commands.remove(command_index) {
            NativeMobileCommand::RunTermuxCommand(request) => {
                let command = AndroidTermuxCommand::from_request(&request);
                if self.active_termux_request_ids.is_empty() {
                    self.active_termux_since_unix = Some(unix_now_secs());
                }
                self.active_termux_request_ids
                    .push(command.request_id.clone());
                Some(command)
            }
            _ => None,
        }
    }

    pub fn accept_android_document_picker_callback(
        &mut self,
        callback: AndroidDocumentPickerCallback,
    ) -> NativeMobileEvent {
        let callback_request_id = callback.request_id().to_string();
        let event = if self
            .active_document_picker_request_id
            .as_deref()
            .map(|active| active == callback_request_id)
            .unwrap_or(false)
        {
            self.active_document_picker_request_id = None;
            callback.into_native_event()
        } else {
            NativeMobileEvent::DocumentPickerFailed(format!(
                "stale Android document picker callback: expected {:?}, got {}",
                self.active_document_picker_request_id, callback_request_id
            ))
        };
        self.accept_event(event.clone());
        event
    }

    pub fn accept_android_pc_discovery_callback(
        &mut self,
        callback: AndroidPcGatewayDiscoveryCallback,
    ) -> NativeMobileEvent {
        let callback_request_id = callback.request_id().to_string();
        let is_active = self
            .active_pc_discovery_request_id
            .as_deref()
            .map(|active| active == callback_request_id)
            .unwrap_or(false);

        let event = if is_active {
            match callback {
                AndroidPcGatewayDiscoveryCallback::Started {
                    request_id,
                    service_type,
                } => NativeMobileEvent::PcGatewayDiscoveryStarted {
                    request_id,
                    service_type,
                },
                AndroidPcGatewayDiscoveryCallback::Candidate { .. } => {
                    let report = callback.into_discovery_report().unwrap_or_default();
                    NativeMobileEvent::PcGatewayDiscoveryUpdated(report)
                }
                AndroidPcGatewayDiscoveryCallback::Completed { .. } => {
                    self.active_pc_discovery_request_id = None;
                    self.active_pc_discovery_since_unix = None;
                    let report = callback.into_discovery_report().unwrap_or_default();
                    NativeMobileEvent::PcGatewayDiscoveryCompleted(report)
                }
                AndroidPcGatewayDiscoveryCallback::Failed { message, .. } => {
                    self.active_pc_discovery_request_id = None;
                    self.active_pc_discovery_since_unix = None;
                    NativeMobileEvent::PcGatewayDiscoveryFailed(message)
                }
            }
        } else {
            NativeMobileEvent::PcGatewayDiscoveryFailed(format!(
                "stale Android PC discovery callback: expected {:?}, got {}",
                self.active_pc_discovery_request_id, callback_request_id
            ))
        };
        self.accept_event(event.clone());
        event
    }

    pub fn accept_android_termux_callback(
        &mut self,
        callback: AndroidTermuxCallback,
    ) -> NativeMobileEvent {
        let request_id = callback.request_id().to_string();
        let is_active = self
            .active_termux_request_ids
            .iter()
            .any(|active| active == &request_id);
        let saved_pending = !is_active && has_saved_pending_termux_request(&request_id);
        let event = if is_active || saved_pending {
            self.active_termux_request_ids
                .retain(|active| active != &request_id);
            if self.active_termux_request_ids.is_empty() {
                self.active_termux_since_unix = None;
            }
            match callback {
                AndroidTermuxCallback::Completed(result) => {
                    NativeMobileEvent::TermuxCommandCompleted(result)
                }
                AndroidTermuxCallback::Failed {
                    request_id,
                    message,
                } => NativeMobileEvent::TermuxCommandFailed {
                    request_id,
                    message,
                },
            }
        } else {
            NativeMobileEvent::TermuxCommandFailed {
                request_id,
                message: "stale Android Termux callback".to_string(),
            }
        };
        self.accept_event(event.clone());
        event
    }

    pub fn accept_event(&mut self, event: NativeMobileEvent) {
        self.last_event_id = self.last_event_id.saturating_add(1);
        self.last_error = match &event {
            NativeMobileEvent::DocumentPickerFailed(message)
            | NativeMobileEvent::PcGatewayDiscoveryFailed(message)
            | NativeMobileEvent::ShareFailed(message)
            | NativeMobileEvent::TerminalFailed { message, .. }
            | NativeMobileEvent::TermuxCommandFailed { message, .. } => Some(message.clone()),
            _ => None,
        };
        self.last_event = Some(event);
        crate::native_host_runtime::replace(self.clone());
    }

    pub fn has_pending_commands(&self) -> bool {
        !self.pending_commands.is_empty()
    }

    pub fn is_waiting_for_document_picker_callback(&self) -> bool {
        self.active_document_picker_request_id.is_some()
    }

    pub fn is_waiting_for_pc_discovery_callback(&self) -> bool {
        self.active_pc_discovery_request_id.is_some()
    }

    pub fn is_waiting_for_termux_callback(&self) -> bool {
        !self.active_termux_request_ids.is_empty()
    }

    /// Clear stuck Termux waits so the UI does not show «Ожидание ответа Android» forever.
    pub fn expire_stale_termux_wait(&mut self, timeout_secs: u64) -> Option<String> {
        let since = self.active_termux_since_unix?;
        if self.active_termux_request_ids.is_empty() {
            self.active_termux_since_unix = None;
            return None;
        }
        let now = unix_now_secs();
        if now.saturating_sub(since) <= timeout_secs {
            return None;
        }
        let ids = self.active_termux_request_ids.join(", ");
        self.active_termux_request_ids.clear();
        self.active_termux_since_unix = None;
        let message = format!(
            "Termux не ответил за {timeout_secs} с (request_ids: {ids}). Проверьте Termux и allow-external-apps."
        );
        self.last_error = Some(message.clone());
        Some(message)
    }

    pub fn expire_stale_pc_discovery(&mut self, timeout_secs: u64) -> Option<String> {
        let request_id = match self.active_pc_discovery_request_id.clone() {
            Some(id) => id,
            None if timeout_secs == 0 && self.active_pc_discovery_since_unix.is_some() => {
                self.active_pc_discovery_since_unix = None;
                return None;
            }
            None => return None,
        };
        let since = self
            .active_pc_discovery_since_unix
            .unwrap_or_else(unix_now_secs);
        let now = unix_now_secs();
        if timeout_secs > 0 && now.saturating_sub(since) <= timeout_secs {
            if self.active_pc_discovery_since_unix.is_none() {
                self.active_pc_discovery_since_unix = Some(since);
            }
            return None;
        }
        self.active_pc_discovery_request_id = None;
        self.active_pc_discovery_since_unix = None;
        let message = format!(
            "Поиск PC Host прерван по таймауту ({timeout_secs} с, request_id={request_id})"
        );
        self.last_error = Some(message.clone());
        Some(message)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        termux_request_from_agent_event, NativeBridgeState, NativeMobileCommand, NativeMobileEvent,
    };
    use crate::document_picker::{DocumentPickerRequest, PickedDocument};
    use crate::native_document_picker::{
        AndroidDocumentPickerCallback, AndroidPickedDocumentPayload,
    };
    use crate::native_pc_discovery::{
        AndroidPcGatewayDiscoveryCallback, AndroidPcGatewayMdnsRecordPayload,
    };
    use crate::native_termux::AndroidTermuxCallback;
    use deepseek_mobile_core::{AgentEvent, TermuxExecRequest, TermuxExecResult, ToolResultEvent};
    use std::path::PathBuf;

    #[test]
    fn bridge_queues_document_picker_command() {
        let mut state = NativeBridgeState::default();
        state.enqueue_document_picker(DocumentPickerRequest::chat_attachment());
        assert!(state.has_pending_commands());
        assert!(matches!(
            state.pop_next_command(),
            Some(NativeMobileCommand::OpenDocumentPicker(_))
        ));
        assert!(!state.has_pending_commands());
    }

    #[test]
    fn bridge_exposes_android_document_picker_command() {
        let mut state = NativeBridgeState::default();
        state.enqueue_document_picker(DocumentPickerRequest::chat_attachment());
        let command = state
            .pop_next_android_document_picker_command()
            .expect("android picker command");
        assert_eq!(command.action, "android.intent.action.OPEN_DOCUMENT");
        assert!(command.allow_multiple);
        assert_eq!(
            state.active_document_picker_request_id.as_deref(),
            Some(command.request_id.as_str())
        );
        assert!(state.is_waiting_for_document_picker_callback());
        assert!(!state.has_pending_commands());
    }

    #[test]
    fn bridge_accepts_android_picker_callback_only_for_active_request() {
        let mut state = NativeBridgeState::default();
        state.enqueue_document_picker(DocumentPickerRequest::chat_attachment());
        let command = state.pop_next_android_document_picker_command().unwrap();
        let event =
            state.accept_android_document_picker_callback(AndroidDocumentPickerCallback::Picked {
                request_id: command.request_id,
                documents: vec![AndroidPickedDocumentPayload::new("1", "main.rs")
                    .with_local_path("/tmp/main.rs")],
            });
        assert!(matches!(event, NativeMobileEvent::DocumentsPicked(_)));
        assert!(!state.is_waiting_for_document_picker_callback());
        assert!(state.last_error.is_none());
    }

    #[test]
    fn bridge_rejects_stale_android_picker_callback() {
        let mut state = NativeBridgeState::default();
        let event = state.accept_android_document_picker_callback(
            AndroidDocumentPickerCallback::Cancelled {
                request_id: "old-request".to_string(),
            },
        );
        assert!(matches!(event, NativeMobileEvent::DocumentPickerFailed(_)));
        assert!(state
            .last_error
            .as_deref()
            .unwrap()
            .contains("stale Android document picker callback"));
    }

    #[test]
    fn bridge_exposes_android_pc_discovery_command() {
        let mut state = NativeBridgeState::default();
        state.enqueue_pc_gateway_discovery("scan-1");
        let command = state.pop_next_android_pc_discovery_command().unwrap();
        assert_eq!(command.request_id, "scan-1");
        assert_eq!(command.service_type, "_deepseek-pc-gateway._tcp.");
        assert!(state.is_waiting_for_pc_discovery_callback());
    }

    #[test]
    fn bridge_accepts_android_pc_discovery_callback_only_for_active_request() {
        let mut state = NativeBridgeState::default();
        state.enqueue_pc_gateway_discovery("scan-1");
        let command = state.pop_next_android_pc_discovery_command().unwrap();
        let event = state.accept_android_pc_discovery_callback(
            AndroidPcGatewayDiscoveryCallback::Candidate {
                request_id: command.request_id,
                record: AndroidPcGatewayMdnsRecordPayload::new("Laptop", "192.168.1.10", 8787),
            },
        );
        assert!(matches!(
            event,
            NativeMobileEvent::PcGatewayDiscoveryUpdated(_)
        ));
        assert!(state.is_waiting_for_pc_discovery_callback());
        assert!(state.last_error.is_none());
    }

    #[test]
    fn bridge_completes_android_pc_discovery_callback() {
        let mut state = NativeBridgeState::default();
        state.enqueue_pc_gateway_discovery("scan-1");
        let command = state.pop_next_android_pc_discovery_command().unwrap();
        let event = state.accept_android_pc_discovery_callback(
            AndroidPcGatewayDiscoveryCallback::Completed {
                request_id: command.request_id,
                records: vec![AndroidPcGatewayMdnsRecordPayload::new(
                    "Laptop",
                    "192.168.1.10",
                    8787,
                )],
            },
        );
        assert!(matches!(
            event,
            NativeMobileEvent::PcGatewayDiscoveryCompleted(_)
        ));
        assert!(!state.is_waiting_for_pc_discovery_callback());
    }

    #[test]
    fn bridge_accepts_documents_picked_event() {
        let mut state = NativeBridgeState::default();
        state.accept_event(NativeMobileEvent::DocumentsPicked(vec![
            PickedDocument::new("1", "spec.pdf"),
        ]));
        assert!(matches!(
            state.last_event,
            Some(NativeMobileEvent::DocumentsPicked(_))
        ));
        assert!(state.last_error.is_none());
    }

    #[test]
    fn bridge_records_native_errors() {
        let mut state = NativeBridgeState::default();
        assert_eq!(state.last_event_id, 0);
        state.accept_event(NativeMobileEvent::DocumentPickerFailed(
            "permission denied".to_string(),
        ));
        assert_eq!(state.last_event_id, 1);
        assert_eq!(state.last_error.as_deref(), Some("permission denied"));
        state.accept_event(NativeMobileEvent::FileShared);
        assert_eq!(state.last_event_id, 2);
        assert!(state.last_error.is_none());
    }

    #[test]
    fn bridge_queues_terminal_commands() {
        let mut state = NativeBridgeState::default();
        state.enqueue_open_terminal("w1");
        state.enqueue_terminal_input("term-1", "echo hello");
        state.enqueue_close_terminal("term-1");
        assert!(state.has_pending_commands());

        assert!(matches!(
            state.pop_terminal_command(),
            Some(NativeMobileCommand::OpenTerminal { .. })
        ));
        assert!(matches!(
            state.pop_terminal_command(),
            Some(NativeMobileCommand::TerminalInput { .. })
        ));
        assert!(matches!(
            state.pop_terminal_command(),
            Some(NativeMobileCommand::CloseTerminal { .. })
        ));
        assert!(state.pop_terminal_command().is_none());
    }

    #[test]
    fn bridge_accepts_terminal_events() {
        let mut state = NativeBridgeState::default();
        state.accept_event(NativeMobileEvent::TerminalOpened {
            session_id: "term-1".to_string(),
            title: "test".to_string(),
            cwd: "/workspace".to_string(),
        });
        assert!(matches!(
            state.last_event,
            Some(NativeMobileEvent::TerminalOpened { .. })
        ));
        assert!(state.last_error.is_none());

        state.accept_event(NativeMobileEvent::TerminalOutput {
            session_id: "term-1".to_string(),
            chunk: "output line".to_string(),
        });
        assert!(matches!(
            state.last_event,
            Some(NativeMobileEvent::TerminalOutput { .. })
        ));

        state.accept_event(NativeMobileEvent::TerminalFailed {
            session_id: Some("term-1".to_string()),
            message: "timeout".to_string(),
        });
        assert_eq!(state.last_error.as_deref(), Some("timeout"));
    }

    #[test]
    fn bridge_exposes_android_termux_commands_and_correlates_callbacks() {
        let mut state = NativeBridgeState::default();
        state.enqueue_termux_command(TermuxExecRequest {
            request_id: "termux-1".to_string(),
            command: "cargo test".to_string(),
            working_dir: PathBuf::from("/data/data/com.termux/files/home/project"),
            timeout_secs: Some(30),
        });

        let command = state.pop_next_android_termux_command().unwrap();
        assert_eq!(command.request_id, "termux-1");
        assert_eq!(
            state.active_termux_request_ids,
            vec!["termux-1".to_string()]
        );
        assert!(state.is_waiting_for_termux_callback());

        let event = state.accept_android_termux_callback(AndroidTermuxCallback::Completed(
            TermuxExecResult {
                request_id: "termux-1".to_string(),
                stdout: "ok".to_string(),
                stderr: String::new(),
                exit_code: Some(0),
                timed_out: false,
                error: None,
            },
        ));
        assert!(matches!(
            event,
            NativeMobileEvent::TermuxCommandCompleted(_)
        ));
        assert!(state.active_termux_request_ids.is_empty());
        assert!(!state.is_waiting_for_termux_callback());
    }

    #[test]
    fn bridge_rejects_stale_android_termux_callback() {
        let mut state = NativeBridgeState::default();
        let event = state.accept_android_termux_callback(AndroidTermuxCallback::Failed {
            request_id: "old".to_string(),
            message: "failed".to_string(),
        });
        assert!(matches!(
            event,
            NativeMobileEvent::TermuxCommandFailed { .. }
        ));
        assert!(state.last_error.as_deref().unwrap().contains("stale"));
    }

    #[test]
    fn bridge_extracts_termux_request_from_tool_result_metadata() {
        let event = AgentEvent::ToolCallFinished(ToolResultEvent {
            id: "tool-1".to_string(),
            name: "exec_shell".to_string(),
            success: true,
            output: "queued".to_string(),
            metadata: Some(serde_json::json!({
                "termux_execution_pending": true,
                "termux_exec_request": {
                    "request_id": "termux-tool-1",
                    "command": "pwd",
                    "working_dir": "/data/data/com.termux/files/home/project",
                    "timeout_secs": 5
                }
            })),
        });

        let request = termux_request_from_agent_event(&event).expect("termux request");
        assert_eq!(request.request_id, "termux-tool-1");
        assert_eq!(request.command, "pwd");

        let mut state = NativeBridgeState::default();
        assert!(state.enqueue_termux_command_from_agent_event(&event));
        assert!(matches!(
            state.pending_commands.first(),
            Some(NativeMobileCommand::RunTermuxCommand(request)) if request.request_id == "termux-tool-1"
        ));
    }
}

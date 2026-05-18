use crate::agent_timeline::MobileTimelineState;
use crate::chat_attachment::ChatComposerState;
use crate::document_picker::DocumentPickerState;
use crate::native_bridge::{NativeBridgeState, NativeMobileEvent};
use crate::pc_pairing_state::PcPairingUiState;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeEventRouteResult {
    pub composer: ChatComposerState,
    pub picker: DocumentPickerState,
    pub native_bridge: NativeBridgeState,
    pub pc_pairing: PcPairingUiState,
    pub timeline: MobileTimelineState,
}

pub fn route_native_mobile_event(
    mut composer: ChatComposerState,
    mut picker: DocumentPickerState,
    mut native_bridge: NativeBridgeState,
    mut pc_pairing: PcPairingUiState,
    mut timeline: MobileTimelineState,
    event: NativeMobileEvent,
) -> NativeEventRouteResult {
    native_bridge.accept_event(event.clone());

    match event {
        NativeMobileEvent::DocumentsPicked(documents) => {
            if documents.is_empty() {
                timeline.push_status("Document picker returned no files");
            } else {
                for document in documents {
                    timeline.push_attachment(format!("{}", document.display_name));
                    composer.add_picked_document(document);
                }
                timeline.push_status("Android document picker files attached to chat composer");
            }
            picker.complete();
        }
        NativeMobileEvent::DocumentPickerCancelled => {
            timeline.push_status("Android document picker cancelled");
            picker.complete();
        }
        NativeMobileEvent::DocumentPickerFailed(error) => {
            timeline.push_error(format!("Android document picker failed: {}", error));
            picker.fail(error);
        }
        NativeMobileEvent::PcGatewayDiscoveryStarted { service_type, .. } => {
            pc_pairing.mark_waiting_for_pc();
            timeline.push_status(format!("Android PC gateway discovery started: {}", service_type));
        }
        NativeMobileEvent::PcGatewayDiscoveryUpdated(report) => {
            let count = report.candidates.len();
            pc_pairing.apply_discovery_report(report);
            timeline.push_status(format!("Android PC gateway discovery candidate update: {} candidate(s)", count));
        }
        NativeMobileEvent::PcGatewayDiscoveryCompleted(report) => {
            let count = report.candidates.len();
            pc_pairing.apply_discovery_report(report);
            timeline.push_status(format!("Android PC gateway discovery completed: {} candidate(s)", count));
        }
        NativeMobileEvent::PcGatewayDiscoveryFailed(error) => {
            pc_pairing.set_error(error.clone());
            timeline.push_error(format!("Android PC gateway discovery failed: {}", error));
        }
        NativeMobileEvent::FileShared => {
            timeline.push_status("Native share completed");
        }
        NativeMobileEvent::ShareFailed(error) => {
            timeline.push_error(format!("Native share failed: {}", error));
        }
        NativeMobileEvent::TerminalOpened { session_id, title, cwd } => {
            timeline.push_status(format!("Terminal session opened: {} ({} - {})", session_id, title, cwd));
        }
        NativeMobileEvent::TerminalOutput { session_id, chunk } => {
            timeline.push_status(format!("Terminal {} output: {} char(s)", session_id, chunk.len()));
        }
        NativeMobileEvent::TerminalClosed { session_id, exit_code } => {
            let code = exit_code.map(|c| c.to_string()).unwrap_or_else(|| "unknown".to_string());
            timeline.push_status(format!("Terminal {} closed (exit code: {})", session_id, code));
        }
        NativeMobileEvent::TerminalFailed { session_id, message } => {
            let id = session_id.unwrap_or_else(|| "unknown".to_string());
            timeline.push_error(format!("Terminal {} failed: {}", id, message));
        }
    }

    NativeEventRouteResult {
        composer,
        picker,
        native_bridge,
        pc_pairing,
        timeline,
    }
}

#[cfg(test)]
mod tests {
    use super::route_native_mobile_event;
    use crate::agent_timeline::MobileTimelineState;
    use crate::chat_attachment::ChatComposerState;
    use crate::document_picker::{DocumentPickerRequest, DocumentPickerState, PickedDocument};
    use crate::native_bridge::{NativeBridgeState, NativeMobileEvent};
    use crate::native_pc_discovery::AndroidPcGatewayMdnsRecordPayload;
    use crate::pc_pairing_state::{PcPairingUiState, PcPairingUiStatus};
    use deepseek_mobile_core::PcGatewayDiscoveryService;

    #[test]
    fn routes_documents_picked_into_composer() {
        let mut picker = DocumentPickerState::default();
        picker.request(DocumentPickerRequest::chat_attachment());

        let result = route_native_mobile_event(
            ChatComposerState::default(),
            picker,
            NativeBridgeState::default(),
            PcPairingUiState::default(),
            MobileTimelineState::default(),
            NativeMobileEvent::DocumentsPicked(vec![
                PickedDocument::new("doc-1", "main.rs")
                    .with_uri("content://docs/main.rs")
                    .with_path("/tmp/main.rs")
                    .with_mime_type("text/plain"),
            ]),
        );

        assert_eq!(result.composer.attachments.len(), 1);
        assert_eq!(result.composer.attachments[0].display_name, "main.rs");
        assert!(!result.picker.is_waiting_for_native_picker());
        assert!(result.native_bridge.last_error.is_none());
        assert!(!result.timeline.items.is_empty());
    }

    #[test]
    fn routes_picker_cancel_to_complete_state() {
        let mut picker = DocumentPickerState::default();
        picker.request(DocumentPickerRequest::chat_attachment());

        let result = route_native_mobile_event(
            ChatComposerState::default(),
            picker,
            NativeBridgeState::default(),
            PcPairingUiState::default(),
            MobileTimelineState::default(),
            NativeMobileEvent::DocumentPickerCancelled,
        );

        assert!(!result.picker.is_waiting_for_native_picker());
        assert!(result.composer.attachments.is_empty());
    }

    #[test]
    fn routes_picker_error_to_picker_and_bridge_errors() {
        let mut picker = DocumentPickerState::default();
        picker.request(DocumentPickerRequest::chat_attachment());

        let result = route_native_mobile_event(
            ChatComposerState::default(),
            picker,
            NativeBridgeState::default(),
            PcPairingUiState::default(),
            MobileTimelineState::default(),
            NativeMobileEvent::DocumentPickerFailed("permission denied".to_string()),
        );

        assert_eq!(result.picker.last_error.as_deref(), Some("permission denied"));
        assert_eq!(result.native_bridge.last_error.as_deref(), Some("permission denied"));
    }

    #[test]
    fn routes_pc_discovery_report_into_pairing_state() {
        let report = PcGatewayDiscoveryService::default().from_mdns_records(vec![
            AndroidPcGatewayMdnsRecordPayload::new("Laptop", "192.168.1.10", 8787).into_core_record(),
        ]);
        let result = route_native_mobile_event(
            ChatComposerState::default(),
            DocumentPickerState::default(),
            NativeBridgeState::default(),
            PcPairingUiState::default(),
            MobileTimelineState::default(),
            NativeMobileEvent::PcGatewayDiscoveryCompleted(report),
        );
        assert_eq!(result.pc_pairing.status, PcPairingUiStatus::WaitingForPc);
        assert!(result.pc_pairing.discovery_rows()[0].contains("192.168.1.10"));
    }

    #[test]
    fn routes_terminal_events_to_timeline() {
        let result = route_native_mobile_event(
            ChatComposerState::default(),
            DocumentPickerState::default(),
            NativeBridgeState::default(),
            PcPairingUiState::default(),
            MobileTimelineState::default(),
            NativeMobileEvent::TerminalOpened {
                session_id: "term-1".to_string(),
                title: "test".to_string(),
                cwd: "/workspace".to_string(),
            },
        );
        assert!(result.timeline.items.iter().any(|i| i.body.contains("Terminal session opened")));

        let result = route_native_mobile_event(
            ChatComposerState::default(),
            DocumentPickerState::default(),
            NativeBridgeState::default(),
            PcPairingUiState::default(),
            MobileTimelineState::default(),
            NativeMobileEvent::TerminalOutput {
                session_id: "term-1".to_string(),
                chunk: "hello".to_string(),
            },
        );
        assert!(result.timeline.items.iter().any(|i| i.body.contains("Terminal term-1 output")));

        let result = route_native_mobile_event(
            ChatComposerState::default(),
            DocumentPickerState::default(),
            NativeBridgeState::default(),
            PcPairingUiState::default(),
            MobileTimelineState::default(),
            NativeMobileEvent::TerminalClosed {
                session_id: "term-1".to_string(),
                exit_code: Some(0),
            },
        );
        assert!(result.timeline.items.iter().any(|i| i.body.contains("Terminal term-1 closed")));

        let result = route_native_mobile_event(
            ChatComposerState::default(),
            DocumentPickerState::default(),
            NativeBridgeState::default(),
            PcPairingUiState::default(),
            MobileTimelineState::default(),
            NativeMobileEvent::TerminalFailed {
                session_id: Some("term-1".to_string()),
                message: "timeout".to_string(),
            },
        );
        assert!(result.timeline.items.iter().any(|i| i.body.contains("Terminal term-1 failed")));
    }
}

//! Android host command drain and callback delivery.
//!
//! The Dioxus/Android shell must call [`drain_next_host_action`] while the app is
//! foregrounded and forward results through the `accept_android_*` helpers on
//! [`NativeBridgeState`].

use crate::native_bridge::{NativeBridgeState, NativeMobileCommand, NativeMobileEvent};
use crate::native_document_picker::{
    AndroidDocumentPickerCallback, AndroidDocumentPickerCommand, AndroidPickedDocumentPayload,
};
use crate::native_pc_discovery::{
    AndroidPcGatewayDiscoveryCallback, AndroidPcGatewayDiscoveryCommand,
    AndroidPcGatewayMdnsRecordPayload,
};
use crate::native_termux::AndroidTermuxCommand;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Serializable command for the Kotlin host coordinator.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AndroidHostAction {
    OpenDocumentPicker {
        request_id: String,
        action: String,
        category: String,
        mime_types: Vec<String>,
        allow_multiple: bool,
    },
    StartPcGatewayDiscovery {
        request_id: String,
        service_type: String,
    },
    RunTermuxCommand {
        request_id: String,
        command: String,
        working_dir: String,
        timeout_secs: Option<u64>,
    },
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
}

impl AndroidHostAction {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    pub fn from_json(raw: &str) -> Option<Self> {
        serde_json::from_str(raw).ok()
    }
}

/// Drain the next native command and describe what the Android host should run.
pub fn drain_next_host_action(bridge: &mut NativeBridgeState) -> Option<AndroidHostAction> {
    if let Some(command) = bridge.pop_next_android_document_picker_command() {
        return Some(document_picker_action(command));
    }
    if let Some(command) = bridge.pop_next_android_pc_discovery_command() {
        return Some(pc_discovery_action(command));
    }
    if let Some(command) = bridge.pop_next_android_termux_command() {
        return Some(termux_action(command));
    }
    if let Some(command) = bridge.pop_terminal_command() {
        return Some(terminal_action(command));
    }
    match bridge.pop_next_command()? {
        NativeMobileCommand::ShareFile { path, mime_type } => Some(AndroidHostAction::ShareFile {
            path,
            mime_type,
        }),
        NativeMobileCommand::OpenUrl { url } => Some(AndroidHostAction::OpenUrl { url }),
        NativeMobileCommand::LaunchApp { package } => Some(AndroidHostAction::LaunchApp { package }),
        NativeMobileCommand::OpenDocumentPicker(_)
        | NativeMobileCommand::StartPcGatewayDiscovery(_)
        | NativeMobileCommand::RunTermuxCommand(_) => drain_next_host_action(bridge),
        NativeMobileCommand::OpenTerminal { workspace_id } => Some(AndroidHostAction::OpenTerminal {
            workspace_id,
        }),
        NativeMobileCommand::TerminalInput { session_id, input } => {
            Some(AndroidHostAction::TerminalInput { session_id, input })
        }
        NativeMobileCommand::CloseTerminal { session_id } => {
            Some(AndroidHostAction::CloseTerminal { session_id })
        }
    }
}

fn document_picker_action(command: AndroidDocumentPickerCommand) -> AndroidHostAction {
    AndroidHostAction::OpenDocumentPicker {
        request_id: command.request_id,
        action: command.action,
        category: command.category,
        mime_types: command.mime_types,
        allow_multiple: command.allow_multiple,
    }
}

fn pc_discovery_action(command: AndroidPcGatewayDiscoveryCommand) -> AndroidHostAction {
    AndroidHostAction::StartPcGatewayDiscovery {
        request_id: command.request_id,
        service_type: command.service_type,
    }
}

fn termux_action(command: AndroidTermuxCommand) -> AndroidHostAction {
    let shell_command = if command.arguments.len() >= 2 && command.arguments[0] == "-lc" {
        command.arguments[1].clone()
    } else {
        command.arguments.join(" ")
    };
    AndroidHostAction::RunTermuxCommand {
        request_id: command.request_id,
        command: shell_command,
        working_dir: command.working_dir,
        timeout_secs: command.timeout_secs,
    }
}

fn terminal_action(command: NativeMobileCommand) -> AndroidHostAction {
    match command {
        NativeMobileCommand::OpenTerminal { workspace_id } => AndroidHostAction::OpenTerminal {
            workspace_id,
        },
        NativeMobileCommand::TerminalInput { session_id, input } => AndroidHostAction::TerminalInput {
            session_id,
            input,
        },
        NativeMobileCommand::CloseTerminal { session_id } => AndroidHostAction::CloseTerminal {
            session_id,
        },
        other => AndroidHostAction::OpenUrl {
            url: format!("unsupported-terminal-command:{other:?}"),
        },
    }
}

fn parse_picked_document(value: &Value) -> Option<AndroidPickedDocumentPayload> {
    let id = value.get("id")?.as_str()?.to_string();
    let display_name = value.get("display_name")?.as_str()?.to_string();
    let mut document = AndroidPickedDocumentPayload::new(id, display_name);
    if let Some(uri) = value.get("uri").and_then(|v| v.as_str()) {
        document = document.with_uri(uri);
    }
    if let Some(local_path) = value.get("local_path").and_then(|v| v.as_str()) {
        document = document.with_local_path(local_path);
    }
    if let Some(mime_type) = value.get("mime_type").and_then(|v| v.as_str()) {
        document = document.with_mime_type(mime_type);
    }
    if let Some(size_bytes) = value.get("size_bytes").and_then(|v| v.as_u64()) {
        document = document.with_size_bytes(size_bytes);
    }
    Some(document)
}

fn parse_document_picker_callback(callback: &Value) -> Option<AndroidDocumentPickerCallback> {
    let callback_type = callback.get("type")?.as_str()?;
    let request_id = callback.get("request_id")?.as_str()?.to_string();
    match callback_type {
        "picked" => {
            let documents = callback
                .get("documents")?
                .as_array()?
                .iter()
                .filter_map(parse_picked_document)
                .collect();
            Some(AndroidDocumentPickerCallback::Picked {
                request_id,
                documents,
            })
        }
        "cancelled" => Some(AndroidDocumentPickerCallback::Cancelled { request_id }),
        "failed" => Some(AndroidDocumentPickerCallback::Failed {
            request_id,
            message: callback
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("document picker failed")
                .to_string(),
        }),
        _ => None,
    }
}

fn parse_mdns_record(value: &Value) -> Option<AndroidPcGatewayMdnsRecordPayload> {
    let instance_name = value.get("instance_name")?.as_str()?.to_string();
    let host = value.get("host")?.as_str()?.to_string();
    let port = value.get("port")?.as_u64()? as u16;
    let mut record = AndroidPcGatewayMdnsRecordPayload::new(instance_name, host, port);
    if let Some(txt) = value.get("txt").and_then(|v| v.as_object()) {
        for (key, value) in txt {
            if let Some(text) = value.as_str() {
                record.txt.insert(key.clone(), text.to_string());
            }
        }
    }
    Some(record)
}

fn parse_pc_discovery_callback(callback: &Value) -> Option<AndroidPcGatewayDiscoveryCallback> {
    let callback_type = callback.get("type")?.as_str()?;
    let request_id = callback.get("request_id")?.as_str()?.to_string();
    match callback_type {
        "started" => Some(AndroidPcGatewayDiscoveryCallback::Started {
            request_id,
            service_type: callback
                .get("service_type")
                .and_then(|v| v.as_str())
                .unwrap_or("_deepseek-pc-gateway._tcp.")
                .to_string(),
        }),
        "updated" => Some(AndroidPcGatewayDiscoveryCallback::Candidate {
            request_id,
            record: parse_mdns_record(callback.get("record")?)?,
        }),
        "completed" => {
            let records = callback
                .get("records")?
                .as_array()?
                .iter()
                .filter_map(parse_mdns_record)
                .collect();
            Some(AndroidPcGatewayDiscoveryCallback::Completed { request_id, records })
        }
        "failed" => Some(AndroidPcGatewayDiscoveryCallback::Failed {
            request_id,
            message: callback
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("pc gateway discovery failed")
                .to_string(),
        }),
        _ => None,
    }
}

/// Apply a host callback encoded as JSON into bridge state.
pub fn apply_host_callback_json(bridge: &mut NativeBridgeState, payload: &str) -> Option<NativeMobileEvent> {
    let value: Value = serde_json::from_str(payload).ok()?;
    let kind = value.get("kind")?.as_str()?;
    match kind {
        "document_picker_picked" => {
            let callback = parse_document_picker_callback(value.get("callback")?)?;
            Some(bridge.accept_android_document_picker_callback(callback))
        }
        "document_picker_cancelled" => {
            let request_id = value.get("request_id")?.as_str()?.to_string();
            Some(bridge.accept_android_document_picker_callback(
                AndroidDocumentPickerCallback::Cancelled { request_id },
            ))
        }
        "document_picker_failed" => {
            let request_id = value.get("request_id")?.as_str()?.to_string();
            let message = value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("document picker failed")
                .to_string();
            Some(bridge.accept_android_document_picker_callback(
                AndroidDocumentPickerCallback::Failed { request_id, message },
            ))
        }
        "pc_discovery" => {
            let callback = parse_pc_discovery_callback(value.get("callback")?)?;
            Some(bridge.accept_android_pc_discovery_callback(callback))
        }
        "termux_completed" => {
            let result: deepseek_mobile_core::TermuxExecResult =
                serde_json::from_value(value.get("result")?.clone()).ok()?;
            Some(bridge.accept_android_termux_callback(
                crate::native_termux::AndroidTermuxCallback::Completed(result),
            ))
        }
        "termux_failed" => {
            let request_id = value.get("request_id")?.as_str()?.to_string();
            let message = value
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("termux command failed")
                .to_string();
            Some(bridge.accept_android_termux_callback(
                crate::native_termux::AndroidTermuxCallback::Failed { request_id, message },
            ))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{drain_next_host_action, AndroidHostAction};
    use crate::document_picker::DocumentPickerRequest;
    use crate::native_bridge::NativeBridgeState;

    #[test]
    fn drains_document_picker_before_other_commands() {
        let mut bridge = NativeBridgeState::default();
        bridge.enqueue_share_file("/tmp/export.zip");
        bridge.enqueue_document_picker(DocumentPickerRequest::project_import());

        let action = drain_next_host_action(&mut bridge).expect("picker action");
        assert!(matches!(
            action,
            AndroidHostAction::OpenDocumentPicker { .. }
        ));
        assert!(bridge.has_pending_commands());
    }

    #[test]
    fn applies_document_picker_picked_callback() {
        let mut bridge = NativeBridgeState::default();
        bridge.enqueue_document_picker(DocumentPickerRequest::chat_attachment());
        let action = drain_next_host_action(&mut bridge).expect("picker");
        let request_id = match action {
            AndroidHostAction::OpenDocumentPicker { request_id, .. } => request_id,
            other => panic!("unexpected action: {other:?}"),
        };
        let payload = format!(
            r#"{{"kind":"document_picker_picked","callback":{{"type":"picked","request_id":"{request_id}","documents":[{{"id":"doc-1","display_name":"notes.txt","local_path":"/data/user/0/com.deepseek.mobile/files/notes.txt","mime_type":"text/plain","size_bytes":12}}]}}}}"#
        );
        let event = super::apply_host_callback_json(&mut bridge, &payload).expect("event");
        assert!(matches!(event, crate::native_bridge::NativeMobileEvent::DocumentsPicked(_)));
        assert!(!bridge.is_waiting_for_document_picker_callback());
    }

    #[test]
    fn applies_pc_discovery_updated_callback() {
        let mut bridge = NativeBridgeState::default();
        bridge.enqueue_pc_gateway_discovery("scan-test");
        let action = drain_next_host_action(&mut bridge).expect("discovery");
        let request_id = match action {
            AndroidHostAction::StartPcGatewayDiscovery { request_id, .. } => request_id,
            other => panic!("unexpected action: {other:?}"),
        };
        let payload = format!(
            r#"{{"kind":"pc_discovery","callback":{{"type":"updated","request_id":"{request_id}","record":{{"instance_name":"Laptop","host":"192.168.1.10","port":8787,"txt":{{}}}}}}}}"#
        );
        let event = super::apply_host_callback_json(&mut bridge, &payload).expect("event");
        assert!(matches!(
            event,
            crate::native_bridge::NativeMobileEvent::PcGatewayDiscoveryUpdated(_)
        ));
    }

    #[test]
    fn action_roundtrips_json() {
        let action = AndroidHostAction::RunTermuxCommand {
            request_id: "termux-1".to_string(),
            command: "pwd".to_string(),
            working_dir: "/data/data/com.termux/files/home".to_string(),
            timeout_secs: Some(30),
        };
        let json = action.to_json();
        let parsed = AndroidHostAction::from_json(&json).expect("parsed");
        assert_eq!(parsed, action);
    }
}

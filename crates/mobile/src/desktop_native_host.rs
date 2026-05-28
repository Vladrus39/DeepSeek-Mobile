//! Desktop development host: executes native bridge commands without Android.

use crate::android_host::AndroidHostAction;
use crate::native_bridge::{NativeBridgeState, NativeMobileEvent};
use crate::native_document_picker::{AndroidDocumentPickerCallback, AndroidPickedDocumentPayload};
use std::fs;
use std::path::PathBuf;

pub fn try_execute(action: &AndroidHostAction, bridge: &mut NativeBridgeState) -> Option<String> {
    match action {
        AndroidHostAction::OpenDocumentPicker {
            request_id,
            allow_multiple,
            ..
        } => {
            let paths = if *allow_multiple {
                rfd::FileDialog::new()
                    .set_title("Select files")
                    .pick_files()
            } else {
                rfd::FileDialog::new()
                    .set_title("Select file")
                    .pick_file()
                    .map(|path| vec![path])
            }?;
            let sandbox_dir = crate::mobile_runtime_config::default_data_dir()
                .join("attachments")
                .join(request_id);
            fs::create_dir_all(&sandbox_dir).ok()?;
            let mut documents = Vec::new();
            for path in paths {
                let display_name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "attachment".to_string());
                let target = unique_path(&sandbox_dir, &display_name);
                fs::copy(&path, &target).ok()?;
                let mut document = AndroidPickedDocumentPayload::new(
                    uuid::Uuid::new_v4().to_string(),
                    display_name,
                )
                .with_local_path(target.display().to_string());
                if let Some(mime_type) = guess_mime(&path) {
                    document = document.with_mime_type(mime_type);
                }
                if let Ok(metadata) = fs::metadata(&path) {
                    document = document.with_size_bytes(metadata.len());
                }
                documents.push(document);
            }
            bridge.accept_android_document_picker_callback(AndroidDocumentPickerCallback::Picked {
                request_id: request_id.clone(),
                documents,
            });
            Some("Desktop document picker completed".to_string())
        }
        AndroidHostAction::InstallApk { path } => Some(format!(
            "install_apk is Android-only (path={path})"
        )),
        AndroidHostAction::ShareFile { path, .. } => {
            if PathBuf::from(path).exists() {
                rfd::MessageDialog::new()
                    .set_title("Export ready")
                    .set_description(format!("Project archive created at:\n{path}"))
                    .show();
            }
            bridge.accept_event(NativeMobileEvent::FileShared);
            Some("Desktop share acknowledged".to_string())
        }
        AndroidHostAction::StartPcGatewayDiscovery { .. } => Some(
            "PC discovery requires Android NSD; use manual PC URL in PC Host panel".to_string(),
        ),
        AndroidHostAction::RunTermuxCommand { .. } => {
            Some("Termux execution is only available on Android with Termux installed".to_string())
        }
        AndroidHostAction::OpenUrl { url } => {
            open::that(url).ok()?;
            Some(format!("Opened URL: {url}"))
        }
        AndroidHostAction::LaunchApp { package } => {
            Some(format!("launch_app is Android-only (package={package})"))
        }
        AndroidHostAction::OpenSystemSettings => {
            Some("open_system_settings is Android-only".to_string())
        }
        AndroidHostAction::OpenTerminal { .. }
        | AndroidHostAction::TerminalInput { .. }
        | AndroidHostAction::CloseTerminal { .. } => {
            Some("Terminal sessions are routed through PC Host on desktop builds".to_string())
        }
    }
}

fn unique_path(dir: &PathBuf, name: &str) -> PathBuf {
    let mut candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    for index in 1..1000 {
        let stem = PathBuf::from(name)
            .file_stem()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        let ext = PathBuf::from(name)
            .extension()
            .map(|value| format!(".{}", value.to_string_lossy()))
            .unwrap_or_default();
        candidate = dir.join(format!("{stem}-{index}{ext}"));
        if !candidate.exists() {
            break;
        }
    }
    candidate
}

fn guess_mime(path: &PathBuf) -> Option<String> {
    let ext = path.extension()?.to_str()?;
    Some(
        match ext.to_ascii_lowercase().as_str() {
            "zip" => "application/zip",
            "pdf" => "application/pdf",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "rs" | "toml" | "json" | "md" | "txt" => "text/plain",
            _ => "application/octet-stream",
        }
        .to_string(),
    )
}

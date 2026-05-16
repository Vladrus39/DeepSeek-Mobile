//! Local/Termux workspace diagnostics.
//!
//! PC diagnostics are executed by `pc-host`. This module covers local Android
//! and Termux workspaces after file-changing tools. It is intentionally
//! best-effort: missing toolchains or non-Rust workspaces are reported in
//! metadata and must not fail the original edit.

use crate::pc_gateway::{PcDiagnostic, PcDiagnosticSeverity};
use crate::workspace::{ExecutorKind, Workspace};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceDiagnosticsStatus {
    Completed,
    NotApplicable,
    Unavailable,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceDiagnosticsReport {
    pub workspace_id: String,
    pub executor: ExecutorKind,
    pub status: WorkspaceDiagnosticsStatus,
    pub provider: Option<String>,
    pub diagnostics: Vec<PcDiagnostic>,
    pub message: Option<String>,
}

impl WorkspaceDiagnosticsReport {
    fn new(workspace: &Workspace, status: WorkspaceDiagnosticsStatus) -> Self {
        Self {
            workspace_id: workspace.id.clone(),
            executor: workspace.executor.clone(),
            status,
            provider: None,
            diagnostics: Vec::new(),
            message: None,
        }
    }

    pub fn not_applicable(workspace: &Workspace, message: impl Into<String>) -> Self {
        let mut report = Self::new(workspace, WorkspaceDiagnosticsStatus::NotApplicable);
        report.message = Some(message.into());
        report
    }

    pub fn unavailable(workspace: &Workspace, provider: impl Into<String>, message: impl Into<String>) -> Self {
        let mut report = Self::new(workspace, WorkspaceDiagnosticsStatus::Unavailable);
        report.provider = Some(provider.into());
        report.message = Some(message.into());
        report
    }

    pub fn failed(workspace: &Workspace, provider: impl Into<String>, message: impl Into<String>) -> Self {
        let mut report = Self::new(workspace, WorkspaceDiagnosticsStatus::Failed);
        report.provider = Some(provider.into());
        report.message = Some(message.into());
        report
    }

    pub fn completed(workspace: &Workspace, provider: impl Into<String>, diagnostics: Vec<PcDiagnostic>) -> Self {
        let mut report = Self::new(workspace, WorkspaceDiagnosticsStatus::Completed);
        report.provider = Some(provider.into());
        report.diagnostics = diagnostics;
        report
    }

    pub fn summary(&self) -> String {
        match self.status {
            WorkspaceDiagnosticsStatus::Completed => summarize_diagnostics(&self.diagnostics),
            _ => self.message.clone().unwrap_or_else(|| "Diagnostics were not produced.".to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct WorkspaceDiagnosticsService {
    workspace: Workspace,
    timeout_secs: u64,
}

impl WorkspaceDiagnosticsService {
    pub fn new(workspace: Workspace) -> Self {
        Self {
            workspace,
            timeout_secs: 60,
        }
    }

    pub fn with_timeout_secs(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    pub async fn run_post_edit_diagnostics(&self, path: Option<String>) -> WorkspaceDiagnosticsReport {
        match self.workspace.executor {
            ExecutorKind::LocalAndroid | ExecutorKind::Termux => self.run_local_rust_diagnostics(path.as_deref()).await,
            ExecutorKind::PcGateway => WorkspaceDiagnosticsReport::not_applicable(
                &self.workspace,
                "PC gateway diagnostics are produced by PcGatewayClient/pc-host.",
            ),
            ExecutorKind::RemoteYlit => WorkspaceDiagnosticsReport::not_applicable(
                &self.workspace,
                "Remote Y-lit diagnostics are not wired yet.",
            ),
        }
    }

    async fn run_local_rust_diagnostics(&self, path: Option<&str>) -> WorkspaceDiagnosticsReport {
        if !self.workspace.root.join("Cargo.toml").exists() {
            return WorkspaceDiagnosticsReport::not_applicable(
                &self.workspace,
                "No Cargo.toml found; Rust diagnostics are not applicable.",
            );
        }

        let requested_path = match path {
            Some(path) => match self.normalize_requested_path(path) {
                Some(path) => Some(path),
                None => {
                    return WorkspaceDiagnosticsReport::failed(
                        &self.workspace,
                        "cargo check",
                        format!("diagnostics path is outside workspace: {}", path),
                    );
                }
            },
            None => None,
        };

        self.run_cargo_check(requested_path.as_deref()).await
    }

    fn normalize_requested_path(&self, path: &str) -> Option<PathBuf> {
        let resolved = self.workspace.resolve_relative_path(path)?;
        resolved
            .strip_prefix(&self.workspace.root)
            .ok()
            .map(Path::to_path_buf)
    }

    async fn run_cargo_check(&self, requested_path: Option<&Path>) -> WorkspaceDiagnosticsReport {
        let run = Command::new("cargo")
            .args(["check", "--workspace", "--message-format=json"])
            .current_dir(&self.workspace.root)
            .output();

        let output = match timeout(Duration::from_secs(self.timeout_secs), run).await {
            Ok(Ok(output)) => output,
            Ok(Err(error)) => {
                return WorkspaceDiagnosticsReport::unavailable(
                    &self.workspace,
                    "cargo check",
                    format!("cargo check is unavailable: {}", error),
                );
            }
            Err(_) => {
                return WorkspaceDiagnosticsReport::unavailable(
                    &self.workspace,
                    "cargo check",
                    format!("cargo check timed out after {} seconds", self.timeout_secs),
                );
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut diagnostics = Vec::new();
        for line in stdout.lines() {
            let Ok(message) = serde_json::from_str::<CargoCheckMessage>(line) else {
                continue;
            };
            if message.reason.as_deref() != Some("compiler-message") {
                continue;
            }
            let Some(message) = message.message else {
                continue;
            };
            diagnostics.extend(cargo_message_to_diagnostics(&self.workspace, requested_path, message));
        }

        if diagnostics.is_empty() && !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return WorkspaceDiagnosticsReport::failed(
                &self.workspace,
                "cargo check",
                if stderr.is_empty() {
                    "cargo check failed without compiler diagnostics".to_string()
                } else {
                    stderr
                },
            );
        }

        WorkspaceDiagnosticsReport::completed(&self.workspace, "cargo check", diagnostics)
    }
}

#[derive(Clone, Debug, Deserialize)]
struct CargoCheckMessage {
    reason: Option<String>,
    message: Option<CargoDiagnosticMessage>,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoDiagnosticMessage {
    message: String,
    level: String,
    spans: Vec<CargoDiagnosticSpan>,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoDiagnosticSpan {
    file_name: String,
    line_start: u32,
    column_start: u32,
    is_primary: bool,
}

fn cargo_message_to_diagnostics(
    workspace: &Workspace,
    requested_path: Option<&Path>,
    message: CargoDiagnosticMessage,
) -> Vec<PcDiagnostic> {
    let severity = cargo_level_to_severity(&message.level);
    message
        .spans
        .into_iter()
        .filter(|span| span.is_primary)
        .filter_map(|span| {
            let path = Path::new(&span.file_name);
            let absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                workspace.root.join(path)
            };
            let Ok(relative) = absolute.strip_prefix(&workspace.root) else {
                return None;
            };
            if let Some(requested) = requested_path {
                if normalize_path(relative) != normalize_path(requested) {
                    return None;
                }
            }
            Some(PcDiagnostic {
                path: normalize_path(relative),
                line: span.line_start,
                column: span.column_start,
                severity: severity.clone(),
                message: message.message.clone(),
                source: Some("cargo check".to_string()),
            })
        })
        .collect()
}

fn cargo_level_to_severity(level: &str) -> PcDiagnosticSeverity {
    match level {
        "error" => PcDiagnosticSeverity::Error,
        "warning" => PcDiagnosticSeverity::Warning,
        "note" | "help" => PcDiagnosticSeverity::Hint,
        _ => PcDiagnosticSeverity::Info,
    }
}

fn summarize_diagnostics(diagnostics: &[PcDiagnostic]) -> String {
    if diagnostics.is_empty() {
        return "No diagnostics reported.".to_string();
    }
    let error_count = diagnostics
        .iter()
        .filter(|item| item.severity == PcDiagnosticSeverity::Error)
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|item| item.severity == PcDiagnosticSeverity::Warning)
        .count();
    let mut lines = vec![format!(
        "{} diagnostic(s): {} error(s), {} warning(s)",
        diagnostics.len(), error_count, warning_count
    )];
    for item in diagnostics.iter().take(8) {
        lines.push(format!(
            "- {:?} {}:{}:{} — {}",
            item.severity, item.path, item.line, item.column, item.message
        ));
    }
    if diagnostics.len() > 8 {
        lines.push(format!("- ... {} more diagnostic(s)", diagnostics.len() - 8));
    }
    lines.join("\n")
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::{WorkspaceDiagnosticsService, WorkspaceDiagnosticsStatus};
    use crate::workspace::{ExecutorKind, Workspace};
    use std::fs;

    #[tokio::test]
    async fn non_cargo_workspace_returns_not_applicable() {
        let root = std::env::temp_dir().join(format!(
            "deepseek_mobile_no_cargo_diagnostics_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::new("w1", "Workspace", root.clone(), ExecutorKind::LocalAndroid);
        let report = WorkspaceDiagnosticsService::new(workspace)
            .run_post_edit_diagnostics(Some("src/main.rs".to_string()))
            .await;
        assert_eq!(report.status, WorkspaceDiagnosticsStatus::NotApplicable);
        assert!(report.diagnostics.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn rejects_diagnostics_path_outside_workspace() {
        let root = std::env::temp_dir().join(format!(
            "deepseek_mobile_diagnostics_escape_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        let workspace = Workspace::new("w1", "Workspace", root.clone(), ExecutorKind::Termux);
        let report = WorkspaceDiagnosticsService::new(workspace)
            .run_post_edit_diagnostics(Some("../outside.rs".to_string()))
            .await;
        assert_eq!(report.status, WorkspaceDiagnosticsStatus::Failed);
        let _ = fs::remove_dir_all(root);
    }
}

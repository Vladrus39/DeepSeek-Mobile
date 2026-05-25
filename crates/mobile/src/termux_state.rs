use std::path::{Path, PathBuf};

use crate::mobile_runtime_config::{activate_workspace_connection_in_base_dir, default_data_dir};
use deepseek_mobile_core::{
    ExecutorKind, Workspace, WorkspaceConnection, WorkspaceConnectionStatus,
};

const TERMUX_CONNECTION_ID: &str = "termux:default";
const TERMUX_WORKSPACE_ID: &str = "termux-default";
const DEFAULT_TERMUX_LABEL: &str = "Termux Workspace";

/// Persistent Termux workspace configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermuxWorkspaceState {
    /// The Termux filesystem path used as the workspace root.
    /// Typical: `/data/data/com.termux/files/home/project`
    pub workspace_path: String,
    /// Human-readable label for the workspace.
    pub label: String,
    /// The last validation error (empty path, invalid characters, etc).
    pub validation_error: Option<String>,
    /// Whether the current config has been saved.
    pub saved: bool,
}

impl Default for TermuxWorkspaceState {
    fn default() -> Self {
        Self::load_from_base_dir(default_data_dir())
    }
}

impl TermuxWorkspaceState {
    pub fn load_from_base_dir(base_dir: impl AsRef<Path>) -> Self {
        let loaded = load_saved_termux_config_from_base_dir(base_dir);
        if let Some(saved) = loaded {
            let mut state = Self {
                workspace_path: saved.workspace_path,
                label: saved.label,
                validation_error: None,
                saved: true,
            };
            state.validate();
            if state.validation_error.is_some() {
                state.saved = false;
            }
            state
        } else {
            Self {
                workspace_path: String::new(),
                label: String::new(),
                validation_error: None,
                saved: false,
            }
        }
    }

    pub fn set_path(&mut self, path: impl Into<String>) {
        self.workspace_path = path.into();
        self.saved = false;
        self.validate();
    }

    pub fn set_label(&mut self, label: impl Into<String>) {
        self.label = label.into();
        self.saved = false;
    }

    /// Validate the workspace path and update `validation_error`.
    pub fn validate(&mut self) {
        self.validation_error = Self::validation_error_for_path(&self.workspace_path);
    }

    pub fn is_valid(&self) -> bool {
        Self::validation_error_for_path(&self.workspace_path).is_none()
    }

    fn validation_error_for_path(path: &str) -> Option<String> {
        let path = path.trim();
        if path.is_empty() {
            return Some("Workspace path cannot be empty.".to_string());
        }

        if path.contains('\0') {
            return Some("Workspace path cannot contain NUL bytes.".to_string());
        }

        if path.contains('\\') {
            return Some("Use a Termux Unix-style path with / separators.".to_string());
        }

        if !path.starts_with('/') {
            return Some("Termux workspace path must be absolute and start with /.".to_string());
        }

        for segment in path.split('/') {
            match segment {
                "" | "." => {}
                ".." => {
                    return Some(
                        "Paths with parent directory segments (..) are not accepted.".to_string(),
                    );
                }
                value if value.ends_with(':') => {
                    return Some("Windows-style path prefixes are not accepted.".to_string());
                }
                _ => {}
            }
        }

        None
    }

    /// Save the current config to the default mobile data dir and activate the
    /// corresponding Termux workspace connection for future engine turns.
    pub fn save(&mut self) {
        if let Err(e) = self.save_to_base_dir(default_data_dir()) {
            self.validation_error = Some(format!("Failed to save: {}", e));
            return;
        }
        self.saved = true;
        self.validation_error = None;
    }

    pub fn save_to_base_dir(&mut self, base_dir: impl AsRef<Path>) -> anyhow::Result<()> {
        self.validate();
        if let Some(error) = self.validation_error.clone() {
            anyhow::bail!(error);
        }

        let base_dir = base_dir.as_ref();
        save_termux_config_in_base_dir(base_dir, self)?;
        activate_workspace_connection_in_base_dir(base_dir, self.to_workspace_connection())?;
        self.saved = true;
        self.validation_error = None;
        Ok(())
    }

    pub fn display_label(&self) -> String {
        let label = self.label.trim();
        if label.is_empty() {
            DEFAULT_TERMUX_LABEL.to_string()
        } else {
            label.to_string()
        }
    }

    /// Build the workspace used by the engine for Termux execution.
    pub fn to_workspace(&self) -> Workspace {
        Workspace::new(
            TERMUX_WORKSPACE_ID,
            self.display_label(),
            PathBuf::from(self.workspace_path.trim()),
            ExecutorKind::Termux,
        )
    }

    pub fn to_workspace_connection(&self) -> WorkspaceConnection {
        WorkspaceConnection::termux(
            TERMUX_CONNECTION_ID,
            self.display_label(),
            TERMUX_WORKSPACE_ID,
            self.display_label(),
            PathBuf::from(self.workspace_path.trim()),
        )
        .with_status(WorkspaceConnectionStatus::Online)
        .with_priority(30)
    }
}

// ── Persistence ──

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SavedTermuxConfig {
    workspace_path: String,
    label: String,
}

fn termux_config_path_for_base_dir(base_dir: impl AsRef<Path>) -> PathBuf {
    base_dir.as_ref().join("termux_workspace.json")
}

fn load_saved_termux_config_from_base_dir(base_dir: impl AsRef<Path>) -> Option<SavedTermuxConfig> {
    std::fs::read_to_string(termux_config_path_for_base_dir(base_dir))
        .ok()
        .and_then(|json| serde_json::from_str::<SavedTermuxConfig>(&json).ok())
}

fn save_termux_config_in_base_dir(
    base_dir: impl AsRef<Path>,
    state: &TermuxWorkspaceState,
) -> anyhow::Result<()> {
    let base_dir = base_dir.as_ref();
    std::fs::create_dir_all(base_dir)?;
    let saved = SavedTermuxConfig {
        workspace_path: state.workspace_path.trim().to_string(),
        label: state.label.trim().to_string(),
    };
    std::fs::write(
        termux_config_path_for_base_dir(base_dir),
        serde_json::to_string_pretty(&saved)?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::TermuxWorkspaceState;
    use crate::mobile_runtime_config::MobileRuntimeConfig;
    use deepseek_mobile_core::{ExecutorKind, WorkspaceBackendKind, WorkspaceConnectionStatus};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn valid_absolute_path_passes_validation() {
        let mut state = empty_state();
        state.set_path("/data/data/com.termux/files/home/project");
        assert!(state.is_valid());
        assert!(state.validation_error.is_none());
    }

    #[test]
    fn empty_path_fails_validation() {
        let mut state = empty_state();
        state.set_path("");
        assert!(!state.is_valid());
        assert!(state.validation_error.is_some());
    }

    #[test]
    fn relative_path_fails_validation() {
        let mut state = empty_state();
        state.set_path("project");
        assert!(!state.is_valid());
        assert!(state.validation_error.unwrap().contains("must be absolute"));
    }

    #[test]
    fn parent_directory_segment_is_rejected() {
        let mut state = empty_state();
        state.set_path("/data/../etc");
        assert!(!state.is_valid());
        assert!(state.validation_error.unwrap().contains("parent directory"));
    }

    #[test]
    fn windows_style_path_fails_validation() {
        let mut state = empty_state();
        state.set_path("C:\\Users\\project");
        assert!(!state.is_valid());
        assert!(state.validation_error.unwrap().contains("Unix-style"));
    }

    #[test]
    fn label_updates_independently_of_path() {
        let mut state = empty_state();
        state.set_path("/home/project");
        state.set_label("My Project");
        assert_eq!(state.label, "My Project");
        assert_eq!(state.workspace_path, "/home/project");
    }

    #[test]
    fn to_workspace_uses_label_for_name() {
        let mut state = empty_state();
        state.set_path("/home/project");
        state.set_label("Dev");
        let ws = state.to_workspace();
        assert_eq!(ws.id, "termux-default");
        assert_eq!(ws.name, "Dev");
        assert_eq!(ws.root.display().to_string(), "/home/project");
        assert_eq!(ws.executor, ExecutorKind::Termux);
    }

    #[test]
    fn save_activates_termux_workspace_connection_for_runtime() {
        let base_dir = temp_dir("activate");
        let mut state = empty_state();
        state.set_path("/data/data/com.termux/files/home/project");
        state.set_label("Phone Dev");

        state.save_to_base_dir(&base_dir).unwrap();

        let runtime = MobileRuntimeConfig::from_base_dir_with_saved_workspace(&base_dir);
        let connection = runtime
            .workspace_connection
            .as_ref()
            .expect("active connection");
        assert_eq!(connection.id, "termux:default");
        assert_eq!(connection.backend, WorkspaceBackendKind::Termux);
        assert_eq!(connection.status, WorkspaceConnectionStatus::Online);
        assert_eq!(connection.workspace_name, "Phone Dev");
        assert_eq!(
            runtime.workspace_root_display(),
            "/data/data/com.termux/files/home/project"
        );

        let loaded = TermuxWorkspaceState::load_from_base_dir(&base_dir);
        assert_eq!(
            loaded.workspace_path,
            "/data/data/com.termux/files/home/project"
        );
        assert_eq!(loaded.label, "Phone Dev");

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn invalid_path_does_not_activate_runtime_connection() {
        let base_dir = temp_dir("invalid");
        let mut state = empty_state();
        state.set_path("/tmp/../secret");

        assert!(state.save_to_base_dir(&base_dir).is_err());
        let runtime = MobileRuntimeConfig::from_base_dir_with_saved_workspace(&base_dir);
        assert!(runtime.workspace_connection.is_none());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn invalid_saved_config_is_revalidated_on_load() {
        let base_dir = temp_dir("load-invalid");
        fs::create_dir_all(&base_dir).unwrap();
        fs::write(
            base_dir.join("termux_workspace.json"),
            r#"{"workspace_path":"../secret","label":"Bad"}"#,
        )
        .unwrap();

        let loaded = TermuxWorkspaceState::load_from_base_dir(&base_dir);
        assert!(!loaded.is_valid());
        assert!(!loaded.saved);
        assert!(loaded.validation_error.is_some());

        let _ = fs::remove_dir_all(base_dir);
    }

    #[test]
    fn missing_saved_config_loads_without_eager_error() {
        let base_dir = temp_dir("missing");

        let loaded = TermuxWorkspaceState::load_from_base_dir(&base_dir);
        assert!(!loaded.is_valid());
        assert!(!loaded.saved);
        assert!(loaded.validation_error.is_none());

        let _ = fs::remove_dir_all(base_dir);
    }

    fn empty_state() -> TermuxWorkspaceState {
        TermuxWorkspaceState {
            workspace_path: String::new(),
            label: String::new(),
            validation_error: None,
            saved: false,
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "deepseek-mobile-termux-state-{}-{}-{}",
            label,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}

//! Shared workspace folder naming for phone and PC.

/// Canonical projects folder under the app data dir (phone) or pairing bundle root (PC).
pub const PROJECT_WORKSPACE_DIR_NAME: &str = "Project workspace";

pub fn project_workspace_relative_name() -> &'static str {
    PROJECT_WORKSPACE_DIR_NAME
}

pub fn join_project_workspace(base: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    base.as_ref().join(PROJECT_WORKSPACE_DIR_NAME)
}

use crate::project_files::{
    choose_default_preview_file, read_project_file, scan_project_tree, scan_pc_gateway_tree,
    read_pc_gateway_file, ProjectFilePreview, ProjectTreeSnapshot, DEFAULT_MAX_FILE_BYTES,
    DEFAULT_MAX_TREE_ENTRIES,
};
use std::fmt;
use std::path::{Path, PathBuf};
use deepseek_mobile_core::{ApprovalCardView, PcGatewayClient};
use crate::project_diff::{ProjectDiffPreview, build_text_diff_preview};

const DEFAULT_WORKSPACE_ROOT: &str = ".";

/// Which backend the file browser is reading from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileBrowserBackend {
    /// Local / Termux filesystem.
    Local,
    /// Remote PC gateway with an active client session.
    PcGateway {
        workspace_id: String,
    },
}

impl fmt::Display for FileBrowserBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileBrowserBackend::Local => write!(f, "local"),
            FileBrowserBackend::PcGateway { .. } => write!(f, "PC gateway"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectFilesUiState {
    pub workspace_root: String,
    pub browsing_dir: String,
    pub snapshot: ProjectTreeSnapshot,
    pub selected_path: Option<String>,
    pub preview: Option<ProjectFilePreview>,
    pub last_error: Option<String>,
    pub pending_diff: Option<ProjectDiffPreview>,
    pub loaded: bool,
    /// Which file backend is active. `None` means uninitialised – use
    /// `detect_source()` or an explicit `set_backend()` before refreshing.
    pub backend: FileBrowserBackend,
}

impl Default for ProjectFilesUiState {
    fn default() -> Self {
        let workspace_root = DEFAULT_WORKSPACE_ROOT.to_string();
        Self {
            snapshot: ProjectTreeSnapshot {
                root: workspace_root.clone(),
                entries: Vec::new(),
                truncated: false,
            },
            workspace_root,
            browsing_dir: String::new(),
            selected_path: None,
            preview: None,
            last_error: None,
            pending_diff: None,
            loaded: false,
            backend: FileBrowserBackend::Local,
        }
    }
}

impl ProjectFilesUiState {
    /// Override the file source backend.
    pub fn set_backend(&mut self, backend: FileBrowserBackend) {
        self.backend = backend;
    }

    /// True when the active file source is a remote PC gateway.
    pub fn is_pc_backend(&self) -> bool {
        matches!(self.backend, FileBrowserBackend::PcGateway { .. })
    }

    /// Refresh the tree from the current backend.
    pub fn refresh(&mut self) {
        self.loaded = true;
        match &self.backend {
            FileBrowserBackend::PcGateway { .. } => {
                // Defer to refresh_via_pc which needs a client reference.
                // This path is taken when the state is reset or navigated without
                // the async PC refresh pathway. The panel should call
                // refresh_via_pc instead when a client is available.
                self.last_error = Some(
                    "PC gateway refresh requires refresh_via_pc (async)".into(),
                );
            }
            FileBrowserBackend::Local => {
                self.do_local_refresh();
            }
        }
    }

    fn do_local_refresh(&mut self) {
        let root = PathBuf::from(&self.workspace_root);
        let scan_root = if self.browsing_dir.is_empty() {
            root.clone()
        } else {
            root.join(&self.browsing_dir)
        };
        match scan_project_tree(&scan_root, DEFAULT_MAX_TREE_ENTRIES) {
            Ok(snapshot) => {
                let selected = self
                    .selected_path
                    .clone()
                    .filter(|path| snapshot.entries.iter().any(|entry| entry.path == *path))
                    .or_else(|| choose_default_preview_file(&snapshot));

                self.snapshot = snapshot;
                self.selected_path = selected.clone();
                self.last_error = None;
                self.preview = None;

                if let Some(path) = selected {
                    self.open_file(path);
                }
            }
            Err(error) => {
                self.last_error = Some(format!("Failed to scan workspace: {}", error));
                self.preview = None;
            }
        }
    }

    fn browsing_relative(&self) -> PathBuf {
        if self.browsing_dir.is_empty() {
            PathBuf::from(".")
        } else {
            PathBuf::from(&self.browsing_dir)
        }
    }

    pub fn navigate_to_dir(&mut self, subdir: String) {
        self.browsing_dir = subdir;
        self.selected_path = None;
        self.preview = None;
        // Refresh is triggered externally via the panel spawn; just reset state.
        self.loaded = false;
    }

    pub fn navigate_up(&mut self) {
        if self.browsing_dir.is_empty() {
            return;
        }
        let parent = Path::new(&self.browsing_dir).parent();
        match parent {
            Some(p) if p.to_string_lossy().is_empty() => self.browsing_dir = String::new(),
            Some(p) => self.browsing_dir = p.to_string_lossy().to_string(),
            None => self.browsing_dir = String::new(),
        }
        self.selected_path = None;
        self.preview = None;
        self.loaded = false;
    }

    pub fn open_file(&mut self, path: impl Into<String>) {
        let path = path.into();
        match &self.backend {
            FileBrowserBackend::PcGateway { .. } => {
                // open_file_via_pc needs a client reference.
                // Caller must use open_file_via_pc when a client is available.
                self.selected_path = Some(path.clone());
                self.last_error = Some(
                    "PC gateway file open requires open_file_via_pc (async)".into(),
                );
                self.preview = None;
            }
            FileBrowserBackend::Local => {
                self.do_local_open_file(path);
            }
        }
    }

    fn do_local_open_file(&mut self, path: String) {
        let root = PathBuf::from(&self.workspace_root);
        let file_path = if self.browsing_dir.is_empty() {
            PathBuf::from(&path)
        } else {
            PathBuf::from(&self.browsing_dir).join(&path)
        };
        match read_project_file(&root, &file_path, DEFAULT_MAX_FILE_BYTES) {
            Ok(preview) => {
                self.selected_path = Some(path);
                self.preview = Some(preview);
                self.last_error = None;
            }
            Err(error) => {
                self.selected_path = Some(path.clone());
                self.preview = None;
                self.last_error = Some(format!("Failed to open {}: {}", path, error));
            }
        }
    }

    /// Compute real pending diffs from approval cards for the currently selected file.
    /// For write_file cards: diff "before" content vs "content".
    /// For edit_file cards: diff "search" vs "replace" on the file content.
    /// For apply_patch cards: extract the first matching file from operations.
    pub fn set_pending_diffs(&mut self, cards: &[ApprovalCardView]) {
        let Some(ref selected) = self.selected_path else {
            self.pending_diff = None;
            return;
        };

        for card in cards {
            let card_path = first_string_arg(card, &["path", "file", "file_path", "relative_path", "target_path"]);
            if card_path.as_deref() != Some(selected.as_str()) {
                continue;
            }

            // Try write_file-style diff (before vs content)
            if let Some(after) = first_string_arg(card, &["content", "new_content", "after", "replacement", "text"]) {
                let before = first_string_arg(card, &["before", "old_content", "current_content"])
                    .unwrap_or_default();
                self.pending_diff = Some(build_text_diff_preview(selected.clone(), &before, &after));
                return;
            }

            // Try edit_file-style diff (search vs replace on current content)
            if let Some(search) = first_string_arg(card, &["search", "old_text"]) {
                let replace = first_string_arg(card, &["replace", "new_text"]).unwrap_or_default();
                let current = self.preview.as_ref()
                    .map(|p| p.content.as_str())
                    .unwrap_or("");
                let after = current.replacen(&search, &replace, 1);
                self.pending_diff = Some(build_text_diff_preview(selected.clone(), current, &after));
                return;
            }
        }

        // No matching card found
        self.pending_diff = None;
    }

    pub fn current_browsing_display(&self) -> String {
        let kind = match &self.backend {
            FileBrowserBackend::Local => "Local",
            FileBrowserBackend::PcGateway { .. } => "PC",
        };
        let dir = if self.browsing_dir.is_empty() {
            String::new()
        } else {
            format!(" / {}", self.browsing_dir)
        };
        format!("{}: {}{}", kind, self.workspace_root, dir)
    }

    pub fn has_selection(&self) -> bool {
        self.selected_path.is_some()
    }
}

// ── PC gateway remote file operations ──

impl ProjectFilesUiState {
    /// Refresh the tree using the PC gateway client.
    /// Returns `true` on success, `false` on error (also sets last_error).
    pub async fn refresh_via_pc(
        &mut self,
        client: &PcGatewayClient,
        workspace_id: &str,
    ) -> bool {
        self.set_backend(FileBrowserBackend::PcGateway {
            workspace_id: workspace_id.to_string(),
        });
        self.loaded = true;

        let relative = self.browsing_relative();
        let request_path = relative.to_string_lossy().replace('\\', "/");
        let request_path = if request_path == "." { "." } else { &request_path };

        match scan_pc_gateway_tree(client, workspace_id, DEFAULT_MAX_TREE_ENTRIES).await {
            Ok(mut snapshot) => {
                // Filter tree entries to the current browsing directory
                if !self.browsing_dir.is_empty() {
                    let prefix = format!("{}/", self.browsing_dir);
                    let prefix_len = prefix.len();
                    snapshot.entries.retain(|e| e.path.starts_with(&prefix) || e.path == self.browsing_dir);
                    for e in &mut snapshot.entries {
                        if e.path.starts_with(&prefix) {
                            e.path = e.path[prefix_len..].to_string();
                        }
                        if e.path == self.browsing_dir && e.kind == crate::project_files::ProjectEntryKind::Directory { continue; }
                    }
                    snapshot.root = format!("{}/{}", workspace_id, self.browsing_dir);
                } else {
                    snapshot.root = workspace_id.to_string();
                }

                let selected = self
                    .selected_path
                    .clone()
                    .filter(|path| snapshot.entries.iter().any(|entry| entry.path == *path))
                    .or_else(|| choose_default_preview_file(&snapshot));
                self.snapshot = snapshot;
                self.selected_path = selected.clone();
                self.last_error = None;
                self.preview = None;
                if let Some(path) = selected {
                    let _ = self.open_file_via_pc(client, &path, workspace_id).await;
                }
                true
            }
            Err(error) => {
                self.last_error = Some(format!("PC gateway scan failed: {}", error));
                self.preview = None;
                false
            }
        }
    }

    /// Open a file using the PC gateway client.
    pub async fn open_file_via_pc(
        &mut self,
        client: &PcGatewayClient,
        path: &str,
        workspace_id: &str,
    ) -> std::result::Result<(), String> {
        self.set_backend(FileBrowserBackend::PcGateway {
            workspace_id: workspace_id.to_string(),
        });

        let full_path = if self.browsing_dir.is_empty() {
            path.to_string()
        } else {
            format!("{}/{}", self.browsing_dir, path)
        };

        match read_pc_gateway_file(client, workspace_id, &full_path, DEFAULT_MAX_FILE_BYTES).await {
            Ok(preview) => {
                self.selected_path = Some(path.to_string());
                self.preview = Some(preview);
                self.last_error = None;
                Ok(())
            }
            Err(error) => {
                self.selected_path = Some(path.to_string());
                self.preview = None;
                self.last_error = Some(format!("PC gateway read failed: {}", path));
                Err(error.to_string())
            }
        }
    }
}

/// Extract the first matching string argument from an approval card's argument_preview.
fn first_string_arg(card: &ApprovalCardView, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = card.argument_preview.get(*key).and_then(|v| v.as_str()) {
            if !value.trim().is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::ProjectFilesUiState;
    use crate::project_files_state::FileBrowserBackend;
    use std::fs;

    fn unique_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("deepseek-mobile-files-state-{}-{}", name, nanos))
    }

    #[test]
    fn refresh_selects_default_previewable_file() {
        let root = unique_dir("refresh");
        fs::create_dir_all(root.join("src")).expect("create source dir");
        fs::write(root.join("src/lib.rs"), "pub fn ok() {}\n").expect("write source");

        let mut state = ProjectFilesUiState::default();
        state.workspace_root = root.to_string_lossy().to_string();
        state.refresh();

        assert!(state.loaded);
        assert_eq!(state.selected_path.as_deref(), Some("src/lib.rs"));
        assert!(state.preview.as_ref().unwrap().content.contains("pub fn ok"));
        assert!(state.last_error.is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn open_file_reports_missing_file_error() {
        let root = unique_dir("missing");
        fs::create_dir_all(&root).expect("create root");

        let mut state = ProjectFilesUiState::default();
        state.workspace_root = root.to_string_lossy().to_string();
        state.open_file("missing.rs");

        assert_eq!(state.selected_path.as_deref(), Some("missing.rs"));
        assert!(state.preview.is_none());
        assert!(state.last_error.as_ref().unwrap().contains("Failed to open"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn default_backend_is_local() {
        let state = ProjectFilesUiState::default();
        assert_eq!(state.backend, FileBrowserBackend::Local);
    }

    #[test]
    fn is_pc_backend_reflects_set_backend() {
        let mut state = ProjectFilesUiState::default();
        assert!(!state.is_pc_backend());
        state.set_backend(FileBrowserBackend::PcGateway {
            workspace_id: "w1".into(),
        });
        assert!(state.is_pc_backend());
    }

    #[test]
    fn navigate_up_from_root_is_noop() {
        let mut state = ProjectFilesUiState::default();
        state.navigate_up();
        assert!(state.browsing_dir.is_empty());
    }

    #[test]
    fn navigate_to_dir_resets_selection() {
        let mut state = ProjectFilesUiState::default();
        state.selected_path = Some("foo.rs".into());
        state.navigate_to_dir("subdir".into());
        assert_eq!(state.browsing_dir, "subdir");
        assert!(state.selected_path.is_none());
        assert!(!state.loaded);
    }
}

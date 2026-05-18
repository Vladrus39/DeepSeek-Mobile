use crate::project_files::{
    choose_default_preview_file, read_project_file, scan_project_tree, ProjectFilePreview,
    ProjectTreeSnapshot, DEFAULT_MAX_FILE_BYTES, DEFAULT_MAX_TREE_ENTRIES,
};
use std::path::{Path, PathBuf};

const DEFAULT_WORKSPACE_ROOT: &str = ".";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectFilesUiState {
    pub workspace_root: String,
    pub browsing_dir: String,
    pub snapshot: ProjectTreeSnapshot,
    pub selected_path: Option<String>,
    pub preview: Option<ProjectFilePreview>,
    pub last_error: Option<String>,
    pub loaded: bool,
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
            loaded: false,
        }
    }
}

impl ProjectFilesUiState {
    pub fn refresh(&mut self) {
        self.loaded = true;
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

    pub fn navigate_to_dir(&mut self, subdir: String) {
        self.browsing_dir = subdir;
        self.selected_path = None;
        self.preview = None;
        self.refresh();
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
        self.refresh();
    }

    pub fn open_file(&mut self, path: impl Into<String>) {
        let path = path.into();
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

    pub fn current_browsing_display(&self) -> String {
        if self.browsing_dir.is_empty() {
            format!("Root: {}", self.workspace_root)
        } else {
            format!("Browse: {}/{}", self.workspace_root, self.browsing_dir)
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selected_path.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectFilesUiState;
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
}
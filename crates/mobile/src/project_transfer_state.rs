use crate::document_picker::PickedDocument;
use crate::mobile_runtime_config::{default_data_dir, MobileRuntimeConfig};
use deepseek_mobile_core::workspace_io::{export_project, import_project};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProjectTransferState {
    pub status: ProjectTransferStatus,
    pub last_error: Option<String>,
    pub last_imported_archive: Option<String>,
    pub last_export_path: Option<PathBuf>,
    pub imported_count: u32,
    pub exported_count: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProjectTransferStatus {
    #[default]
    Idle,
    ImportPickerPending,
    Importing,
    Imported,
    Exporting,
    ExportReady,
    Sharing,
    Shared,
    Cancelled,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectImportReport {
    pub archive_name: String,
    pub archive_path: PathBuf,
    pub workspace_root: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectExportReport {
    pub archive_path: PathBuf,
    pub workspace_root: PathBuf,
}

impl ProjectTransferState {
    pub fn request_import(&mut self) {
        self.status = ProjectTransferStatus::ImportPickerPending;
        self.last_error = None;
    }

    pub fn mark_import_cancelled(&mut self) {
        self.status = ProjectTransferStatus::Cancelled;
        self.last_error = None;
    }

    pub fn import_documents(
        &mut self,
        documents: &[PickedDocument],
        workspace_root: impl AsRef<Path>,
    ) -> anyhow::Result<ProjectImportReport> {
        self.status = ProjectTransferStatus::Importing;
        self.last_error = None;

        let document = documents
            .first()
            .ok_or_else(|| anyhow::anyhow!("project import returned no selected archive"))?;
        let archive_path = document
            .path
            .clone()
            .ok_or_else(|| anyhow::anyhow!("selected archive was not copied into local storage"))?;
        let workspace_root = workspace_root.as_ref().to_path_buf();

        import_project(&archive_path, &workspace_root)?;

        let report = ProjectImportReport {
            archive_name: document.display_name.clone(),
            archive_path,
            workspace_root,
        };
        self.status = ProjectTransferStatus::Imported;
        self.last_imported_archive = Some(report.archive_name.clone());
        self.imported_count = self.imported_count.saturating_add(1);
        self.last_error = None;
        Ok(report)
    }

    pub fn export_workspace(
        &mut self,
        workspace_root: impl AsRef<Path>,
        export_dir: impl AsRef<Path>,
    ) -> anyhow::Result<ProjectExportReport> {
        self.status = ProjectTransferStatus::Exporting;
        self.last_error = None;

        let workspace_root = workspace_root.as_ref().to_path_buf();
        let export_dir = export_dir.as_ref();
        let archive_path = export_dir.join(export_archive_name());
        export_project(&workspace_root, &archive_path)?;

        let report = ProjectExportReport {
            archive_path,
            workspace_root,
        };
        self.status = ProjectTransferStatus::ExportReady;
        self.last_export_path = Some(report.archive_path.clone());
        self.exported_count = self.exported_count.saturating_add(1);
        self.last_error = None;
        Ok(report)
    }

    pub fn mark_share_queued(&mut self, archive_path: PathBuf) {
        self.status = ProjectTransferStatus::Sharing;
        self.last_export_path = Some(archive_path);
        self.last_error = None;
    }

    pub fn mark_shared(&mut self) {
        self.status = ProjectTransferStatus::Shared;
        self.last_error = None;
    }

    pub fn mark_error(&mut self, error: impl Into<String>) {
        self.status = ProjectTransferStatus::Error;
        self.last_error = Some(error.into());
    }

    pub fn status_text(&self) -> String {
        match self.status {
            ProjectTransferStatus::Idle => {
                "Ready to import/export local phone workspace".to_string()
            }
            ProjectTransferStatus::ImportPickerPending => {
                "Waiting for Android archive picker".to_string()
            }
            ProjectTransferStatus::Importing => "Importing project archive".to_string(),
            ProjectTransferStatus::Imported => self
                .last_imported_archive
                .as_ref()
                .map(|name| format!("Imported {name} into phone workspace"))
                .unwrap_or_else(|| "Project archive imported into phone workspace".to_string()),
            ProjectTransferStatus::Exporting => "Exporting phone workspace to ZIP".to_string(),
            ProjectTransferStatus::ExportReady => self
                .last_export_path
                .as_ref()
                .map(|path| format!("Export ready: {}", path.display()))
                .unwrap_or_else(|| "Project export ready".to_string()),
            ProjectTransferStatus::Sharing => self
                .last_export_path
                .as_ref()
                .map(|path| format!("Native share queued: {}", path.display()))
                .unwrap_or_else(|| "Native share queued".to_string()),
            ProjectTransferStatus::Shared => "Native share completed".to_string(),
            ProjectTransferStatus::Cancelled => "Project import cancelled".to_string(),
            ProjectTransferStatus::Error => self
                .last_error
                .as_ref()
                .map(|error| format!("Project transfer failed: {error}"))
                .unwrap_or_else(|| "Project transfer failed".to_string()),
        }
    }

    pub fn can_export(&self) -> bool {
        !matches!(
            self.status,
            ProjectTransferStatus::Importing | ProjectTransferStatus::Exporting
        )
    }

    pub fn is_sharing(&self) -> bool {
        self.status == ProjectTransferStatus::Sharing
    }
}

pub fn default_phone_workspace_root() -> PathBuf {
    MobileRuntimeConfig::from_base_dir(default_data_dir()).workspace_root
}

pub fn default_export_dir() -> PathBuf {
    default_data_dir().join("exports")
}

fn export_archive_name() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("deepseek-mobile-project-{seconds}.zip")
}

#[cfg(test)]
mod tests {
    use super::{
        default_export_dir, default_phone_workspace_root, ProjectTransferState,
        ProjectTransferStatus,
    };
    use crate::document_picker::PickedDocument;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn request_import_sets_pending_status() {
        let mut state = ProjectTransferState::default();
        state.request_import();
        assert_eq!(state.status, ProjectTransferStatus::ImportPickerPending);
        assert!(state.last_error.is_none());
    }

    #[test]
    fn import_documents_requires_local_archive_path() {
        let dir = temp_dir("missing-path");
        let mut state = ProjectTransferState::default();

        let err = state
            .import_documents(&[PickedDocument::new("zip", "project.zip")], &dir)
            .unwrap_err()
            .to_string();

        assert!(err.contains("local storage"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn import_documents_extracts_zip_into_workspace() {
        let dir = temp_dir("import");
        fs::create_dir_all(&dir).unwrap();
        let archive_path = dir.join("project.zip");
        write_test_zip(&dir, &archive_path);
        let workspace = dir.join("workspace");
        let mut state = ProjectTransferState::default();

        let report = state
            .import_documents(
                &[PickedDocument::new("zip", "project.zip").with_path(&archive_path)],
                &workspace,
            )
            .unwrap();

        assert_eq!(report.archive_name, "project.zip");
        assert_eq!(state.status, ProjectTransferStatus::Imported);
        assert_eq!(state.imported_count, 1);
        assert_eq!(
            fs::read_to_string(workspace.join("README.md")).unwrap(),
            "# Imported\n"
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn export_workspace_creates_shareable_zip() {
        let dir = temp_dir("export");
        let workspace = dir.join("workspace");
        fs::create_dir_all(workspace.join("src")).unwrap();
        fs::write(workspace.join("src/main.rs"), "fn main() {}\n").unwrap();
        let export_dir = dir.join("exports");
        let mut state = ProjectTransferState::default();

        let report = state.export_workspace(&workspace, &export_dir).unwrap();

        assert!(report.archive_path.exists());
        assert_eq!(state.status, ProjectTransferStatus::ExportReady);
        assert_eq!(state.exported_count, 1);
        state.mark_share_queued(report.archive_path.clone());
        assert_eq!(state.status, ProjectTransferStatus::Sharing);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn default_paths_stay_under_mobile_data_dir() {
        let root = default_phone_workspace_root();
        let export_dir = default_export_dir();
        assert!(
            root.ends_with(deepseek_mobile_core::PROJECT_WORKSPACE_DIR_NAME),
            "root={}",
            root.display()
        );
        assert!(export_dir.ends_with("exports"));
    }

    fn write_test_zip(dir: &PathBuf, path: &PathBuf) {
        let source = dir.join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("README.md"), "# Imported\n").unwrap();
        deepseek_mobile_core::workspace_io::export_project(&source, path).unwrap();
    }

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "deepseek-mobile-project-transfer-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}

use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentPickerMode {
    Single,
    Multiple,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentPickerPurpose {
    ChatAttachment,
    ProjectImport,
    SettingsImport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentPickerRequest {
    pub purpose: DocumentPickerPurpose,
    pub mode: DocumentPickerMode,
    pub mime_types: Vec<String>,
    pub allow_archives: bool,
    pub allow_images: bool,
    pub allow_source_files: bool,
}

impl DocumentPickerRequest {
    pub fn chat_attachment() -> Self {
        Self {
            purpose: DocumentPickerPurpose::ChatAttachment,
            mode: DocumentPickerMode::Multiple,
            mime_types: vec![
                "application/pdf".to_string(),
                "text/plain".to_string(),
                "text/markdown".to_string(),
                "application/json".to_string(),
                "application/zip".to_string(),
                "image/*".to_string(),
                "*/*".to_string(),
            ],
            allow_archives: true,
            allow_images: true,
            allow_source_files: true,
        }
    }

    pub fn project_import() -> Self {
        Self {
            purpose: DocumentPickerPurpose::ProjectImport,
            mode: DocumentPickerMode::Single,
            mime_types: vec![
                "application/zip".to_string(),
                "application/x-tar".to_string(),
                "application/gzip".to_string(),
            ],
            allow_archives: true,
            allow_images: false,
            allow_source_files: false,
        }
    }

    pub fn android_intent_action(&self) -> &'static str {
        "android.intent.action.OPEN_DOCUMENT"
    }

    pub fn android_intent_category(&self) -> &'static str {
        "android.intent.category.OPENABLE"
    }

    pub fn allows_multiple(&self) -> bool {
        self.mode == DocumentPickerMode::Multiple
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickedDocument {
    pub id: String,
    pub display_name: String,
    pub uri: Option<String>,
    pub path: Option<PathBuf>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
}

impl PickedDocument {
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            uri: None,
            path: None,
            mime_type: None,
            size_bytes: None,
        }
    }

    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn with_size_bytes(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DocumentPickerState {
    pub pending_request: Option<DocumentPickerRequest>,
    pub last_error: Option<String>,
}

impl DocumentPickerState {
    pub fn request(&mut self, request: DocumentPickerRequest) {
        self.pending_request = Some(request);
        self.last_error = None;
    }

    pub fn complete(&mut self) {
        self.pending_request = None;
        self.last_error = None;
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.pending_request = None;
        self.last_error = Some(error.into());
    }

    pub fn is_waiting_for_native_picker(&self) -> bool {
        self.pending_request.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{DocumentPickerRequest, DocumentPickerState};

    #[test]
    fn chat_attachment_picker_allows_multiple_documents() {
        let request = DocumentPickerRequest::chat_attachment();
        assert!(request.allows_multiple());
        assert!(request.allow_archives);
        assert!(request.allow_images);
        assert!(request.allow_source_files);
        assert!(request.mime_types.contains(&"application/pdf".to_string()));
    }

    #[test]
    fn project_import_picker_is_archive_focused() {
        let request = DocumentPickerRequest::project_import();
        assert!(!request.allows_multiple());
        assert!(request.allow_archives);
        assert!(!request.allow_images);
    }

    #[test]
    fn picker_state_tracks_pending_request() {
        let mut state = DocumentPickerState::default();
        assert!(!state.is_waiting_for_native_picker());
        state.request(DocumentPickerRequest::chat_attachment());
        assert!(state.is_waiting_for_native_picker());
        state.complete();
        assert!(!state.is_waiting_for_native_picker());
    }
}

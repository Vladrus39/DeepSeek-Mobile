use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatAttachmentKind {
    Document,
    Image,
    Archive,
    SourceFile,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatAttachmentDraft {
    pub id: String,
    pub display_name: String,
    pub path: Option<PathBuf>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub kind: ChatAttachmentKind,
}

impl ChatAttachmentDraft {
    pub fn new_document(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            path: None,
            mime_type: None,
            size_bytes: None,
            kind: ChatAttachmentKind::Document,
        }
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
pub struct ChatComposerState {
    pub draft_text: String,
    pub attachments: Vec<ChatAttachmentDraft>,
}

impl ChatComposerState {
    pub fn add_attachment(&mut self, attachment: ChatAttachmentDraft) {
        self.attachments.push(attachment);
    }

    pub fn clear(&mut self) {
        self.draft_text.clear();
        self.attachments.clear();
    }

    pub fn has_content(&self) -> bool {
        !self.draft_text.trim().is_empty() || !self.attachments.is_empty()
    }

    pub fn attachment_summary(&self) -> String {
        match self.attachments.len() {
            0 => "No attachments".to_string(),
            1 => format!("1 attachment: {}", self.attachments[0].display_name),
            count => format!("{} attachments", count),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ChatAttachmentDraft, ChatComposerState};

    #[test]
    fn composer_detects_text_content() {
        let mut state = ChatComposerState::default();
        assert!(!state.has_content());
        state.draft_text = "Build project".to_string();
        assert!(state.has_content());
    }

    #[test]
    fn composer_detects_attachment_content() {
        let mut state = ChatComposerState::default();
        state.add_attachment(ChatAttachmentDraft::new_document("1", "spec.pdf"));
        assert!(state.has_content());
        assert_eq!(state.attachment_summary(), "1 attachment: spec.pdf");
    }

    #[test]
    fn composer_clear_removes_text_and_attachments() {
        let mut state = ChatComposerState::default();
        state.draft_text = "Analyze".to_string();
        state.add_attachment(ChatAttachmentDraft::new_document("1", "project.zip"));
        state.clear();
        assert!(!state.has_content());
        assert_eq!(state.attachments.len(), 0);
    }
}

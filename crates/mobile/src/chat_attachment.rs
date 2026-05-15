use crate::attachment_ingestion::{ingest_attachment_texts, ingestion_status_message};
use crate::document_picker::PickedDocument;
use deepseek_mobile_core::{UserAttachmentKind, UserAttachmentRef, UserChatInput};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatAttachmentKind {
    Document,
    Image,
    Archive,
    SourceFile,
    Unknown,
}

impl ChatAttachmentKind {
    pub fn from_mime_or_name(mime_type: Option<&str>, display_name: &str) -> Self {
        if let Some(mime_type) = mime_type {
            if mime_type.starts_with("image/") {
                return Self::Image;
            }
            if mime_type == "application/zip" || mime_type == "application/x-tar" || mime_type == "application/gzip" {
                return Self::Archive;
            }
            if mime_type.starts_with("text/") || mime_type == "application/json" {
                return Self::SourceFile;
            }
            if mime_type == "application/pdf" {
                return Self::Document;
            }
        }

        let lower = display_name.to_ascii_lowercase();
        if lower.ends_with(".png") || lower.ends_with(".jpg") || lower.ends_with(".jpeg") || lower.ends_with(".webp") {
            Self::Image
        } else if lower.ends_with(".zip") || lower.ends_with(".tar") || lower.ends_with(".gz") || lower.ends_with(".tgz") {
            Self::Archive
        } else if lower.ends_with(".rs")
            || lower.ends_with(".py")
            || lower.ends_with(".js")
            || lower.ends_with(".ts")
            || lower.ends_with(".tsx")
            || lower.ends_with(".jsx")
            || lower.ends_with(".json")
            || lower.ends_with(".md")
            || lower.ends_with(".toml")
            || lower.ends_with(".yaml")
            || lower.ends_with(".yml")
            || lower.ends_with(".txt")
        {
            Self::SourceFile
        } else if lower.ends_with(".pdf") || lower.ends_with(".doc") || lower.ends_with(".docx") {
            Self::Document
        } else {
            Self::Unknown
        }
    }

    pub fn to_core_kind(&self) -> UserAttachmentKind {
        match self {
            Self::Document => UserAttachmentKind::Document,
            Self::Image => UserAttachmentKind::Image,
            Self::Archive => UserAttachmentKind::Archive,
            Self::SourceFile => UserAttachmentKind::SourceFile,
            Self::Unknown => UserAttachmentKind::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatAttachmentDraft {
    pub id: String,
    pub display_name: String,
    pub path: Option<PathBuf>,
    pub uri: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub kind: ChatAttachmentKind,
    pub extracted_text: Option<String>,
}

impl ChatAttachmentDraft {
    pub fn new_document(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            path: None,
            uri: None,
            mime_type: None,
            size_bytes: None,
            kind: ChatAttachmentKind::Document,
            extracted_text: None,
        }
    }

    pub fn from_picked_document(document: PickedDocument) -> Self {
        let kind = ChatAttachmentKind::from_mime_or_name(document.mime_type.as_deref(), &document.display_name);
        Self {
            id: document.id,
            display_name: document.display_name,
            path: document.path,
            uri: document.uri,
            mime_type: document.mime_type,
            size_bytes: document.size_bytes,
            kind,
            extracted_text: None,
        }
    }

    pub fn to_core_ref(&self) -> UserAttachmentRef {
        let mut attachment = UserAttachmentRef::new(
            self.id.clone(),
            self.display_name.clone(),
            self.kind.to_core_kind(),
        );
        if let Some(uri) = self.uri.as_ref() {
            attachment = attachment.with_uri(uri.clone());
        }
        if let Some(path) = self.path.as_ref() {
            attachment = attachment.with_path(path.to_string_lossy().to_string());
        }
        if let Some(mime_type) = self.mime_type.as_ref() {
            attachment = attachment.with_mime_type(mime_type.clone());
        }
        if let Some(size_bytes) = self.size_bytes {
            attachment = attachment.with_size_bytes(size_bytes);
        }
        if let Some(extracted_text) = self.extracted_text.as_ref() {
            attachment = attachment.with_extracted_text(extracted_text.clone());
        }
        attachment
    }

    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self.kind = ChatAttachmentKind::from_mime_or_name(self.mime_type.as_deref(), &self.display_name);
        self
    }

    pub fn with_size_bytes(mut self, size_bytes: u64) -> Self {
        self.size_bytes = Some(size_bytes);
        self
    }

    pub fn with_extracted_text(mut self, text: impl Into<String>) -> Self {
        self.extracted_text = Some(text.into());
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

    pub fn add_picked_document(&mut self, document: PickedDocument) {
        self.add_attachment(ChatAttachmentDraft::from_picked_document(document));
    }

    pub fn to_core_input(&self) -> UserChatInput {
        UserChatInput::new(self.draft_text.clone()).with_attachments(
            self.attachments
                .iter()
                .map(ChatAttachmentDraft::to_core_ref)
                .collect(),
        )
    }

    pub fn to_core_input_with_ingestion(&self) -> (UserChatInput, Vec<String>) {
        let ingestion_results = ingest_attachment_texts(self.attachments.clone());
        let status_messages = ingestion_results
            .iter()
            .filter_map(ingestion_status_message)
            .collect::<Vec<_>>();
        let attachments = ingestion_results
            .into_iter()
            .map(|result| result.attachment.to_core_ref())
            .collect();

        (
            UserChatInput::new(self.draft_text.clone()).with_attachments(attachments),
            status_messages,
        )
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
    use super::{ChatAttachmentDraft, ChatAttachmentKind, ChatComposerState};
    use crate::document_picker::PickedDocument;
    use deepseek_mobile_core::UserAttachmentKind;
    use std::fs;

    fn unique_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("deepseek-mobile-{}-{}", name, nanos))
    }

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

    #[test]
    fn picked_document_becomes_chat_attachment() {
        let document = PickedDocument::new("doc-1", "project.zip")
            .with_uri("content://docs/project.zip")
            .with_mime_type("application/zip")
            .with_size_bytes(1024);
        let attachment = ChatAttachmentDraft::from_picked_document(document);
        assert_eq!(attachment.kind, ChatAttachmentKind::Archive);
        assert_eq!(attachment.uri.as_deref(), Some("content://docs/project.zip"));
        assert_eq!(attachment.size_bytes, Some(1024));
    }

    #[test]
    fn attachment_kind_detects_source_files() {
        assert_eq!(ChatAttachmentKind::from_mime_or_name(None, "main.rs"), ChatAttachmentKind::SourceFile);
        assert_eq!(ChatAttachmentKind::from_mime_or_name(Some("image/png"), "photo"), ChatAttachmentKind::Image);
        assert_eq!(ChatAttachmentKind::from_mime_or_name(Some("application/pdf"), "manual"), ChatAttachmentKind::Document);
    }

    #[test]
    fn composer_exports_core_chat_input() {
        let mut state = ChatComposerState::default();
        state.draft_text = "Analyze".to_string();
        state.add_picked_document(
            PickedDocument::new("doc-1", "project.zip")
                .with_mime_type("application/zip")
                .with_uri("content://docs/project.zip"),
        );
        let input = state.to_core_input();
        assert_eq!(input.text, "Analyze");
        assert_eq!(input.attachments.len(), 1);
        assert_eq!(input.attachments[0].kind, UserAttachmentKind::Archive);
    }

    #[test]
    fn composer_ingests_local_text_attachment() {
        let path = unique_path("note.txt");
        fs::write(&path, "real attachment text").expect("write test attachment");

        let mut state = ChatComposerState::default();
        state.draft_text = "Analyze".to_string();
        state.add_attachment(
            ChatAttachmentDraft::new_document("doc-1", "note.txt")
                .with_path(&path)
                .with_mime_type("text/plain"),
        );

        let (input, statuses) = state.to_core_input_with_ingestion();
        assert_eq!(input.attachments.len(), 1);
        assert_eq!(input.attachments[0].extracted_text.as_deref(), Some("real attachment text"));
        assert!(statuses[0].contains("Read attachment text"));

        let _ = fs::remove_file(path);
    }
}
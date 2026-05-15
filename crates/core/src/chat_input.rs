//! User chat input contract shared by Android UI and the agent core.
//!
//! The mobile composer may contain plain text plus documents, source files,
//! archives or images. The model-facing API still receives text messages, so
//! this module owns the deterministic conversion from rich mobile input into a
//! prompt block that the runtime can store, audit and replay.

use crate::api_client::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserAttachmentKind {
    Document,
    Image,
    Archive,
    SourceFile,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserAttachmentRef {
    pub id: String,
    pub display_name: String,
    pub uri: Option<String>,
    pub path: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub kind: UserAttachmentKind,
}

impl UserAttachmentRef {
    pub fn new(
        id: impl Into<String>,
        display_name: impl Into<String>,
        kind: UserAttachmentKind,
    ) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            uri: None,
            path: None,
            mime_type: None,
            size_bytes: None,
            kind,
        }
    }

    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserChatInput {
    pub text: String,
    pub attachments: Vec<UserAttachmentRef>,
}

impl UserChatInput {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            attachments: Vec::new(),
        }
    }

    pub fn with_attachments(mut self, attachments: Vec<UserAttachmentRef>) -> Self {
        self.attachments = attachments;
        self
    }

    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty() && self.attachments.is_empty()
    }

    pub fn to_prompt_text(&self) -> String {
        let mut out = self.text.trim().to_string();
        if self.attachments.is_empty() {
            return out;
        }

        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str("Attached files:\n");
        for attachment in &self.attachments {
            out.push_str("- ");
            out.push_str(&attachment.display_name);
            out.push_str(" [");
            out.push_str(match attachment.kind {
                UserAttachmentKind::Document => "document",
                UserAttachmentKind::Image => "image",
                UserAttachmentKind::Archive => "archive",
                UserAttachmentKind::SourceFile => "source_file",
                UserAttachmentKind::Unknown => "unknown",
            });
            out.push(']');
            if let Some(mime_type) = attachment.mime_type.as_ref() {
                out.push_str(" mime=");
                out.push_str(mime_type);
            }
            if let Some(size_bytes) = attachment.size_bytes {
                out.push_str(" size_bytes=");
                out.push_str(&size_bytes.to_string());
            }
            if let Some(uri) = attachment.uri.as_ref() {
                out.push_str(" uri=");
                out.push_str(uri);
            }
            if let Some(path) = attachment.path.as_ref() {
                out.push_str(" path=");
                out.push_str(path);
            }
            out.push('\n');
        }
        out
    }

    pub fn into_message(self) -> Message {
        Message {
            role: "user".to_string(),
            content: self.to_prompt_text(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{UserAttachmentKind, UserAttachmentRef, UserChatInput};

    #[test]
    fn plain_text_input_becomes_user_message() {
        let message = UserChatInput::new("Build app").into_message();
        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Build app");
    }

    #[test]
    fn attachments_are_rendered_into_prompt_text() {
        let input = UserChatInput::new("Analyze this")
            .with_attachments(vec![
                UserAttachmentRef::new("a1", "project.zip", UserAttachmentKind::Archive)
                    .with_mime_type("application/zip")
                    .with_size_bytes(2048),
            ]);
        let prompt = input.to_prompt_text();
        assert!(prompt.contains("Analyze this"));
        assert!(prompt.contains("project.zip"));
        assert!(prompt.contains("archive"));
        assert!(prompt.contains("application/zip"));
    }

    #[test]
    fn attachment_only_input_is_not_empty() {
        let input = UserChatInput::new("")
            .with_attachments(vec![UserAttachmentRef::new("a1", "spec.pdf", UserAttachmentKind::Document)]);
        assert!(!input.is_empty());
        assert!(input.to_prompt_text().contains("Attached files"));
    }
}
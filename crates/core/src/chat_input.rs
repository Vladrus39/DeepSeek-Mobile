//! User chat input contract shared by Android UI and the agent core.
//!
//! The mobile composer may contain plain text plus documents, source files,
//! archives or images. The model-facing API still receives text messages, so
//! this module owns the deterministic conversion from rich mobile input into a
//! prompt block that the runtime can store, audit and replay.

use crate::api_client::Message;
use serde::{Deserialize, Serialize};

const MAX_ATTACHMENT_PROMPT_CHARS: usize = 24_000;

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
    pub extracted_text: Option<String>,
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
            extracted_text: None,
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

    pub fn with_extracted_text(mut self, text: impl Into<String>) -> Self {
        self.extracted_text = Some(text.into());
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
            if attachment.extracted_text.is_some() {
                out.push_str(" extracted_text=true");
            }
            out.push('\n');
        }

        for attachment in self.attachments.iter().filter(|item| item.extracted_text.is_some()) {
            let text = attachment.extracted_text.as_deref().unwrap_or_default();
            out.push_str("\n--- Extracted content for ");
            out.push_str(&attachment.display_name);
            out.push_str(" ---\n");
            out.push_str(&truncate_attachment_text(text));
            out.push_str("\n--- End extracted content for ");
            out.push_str(&attachment.display_name);
            out.push_str(" ---\n");
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

fn truncate_attachment_text(text: &str) -> String {
    if text.chars().count() <= MAX_ATTACHMENT_PROMPT_CHARS {
        return text.to_string();
    }

    let mut out = text.chars().take(MAX_ATTACHMENT_PROMPT_CHARS).collect::<String>();
    out.push_str("\n...[attachment content truncated]...");
    out
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
    fn extracted_attachment_text_is_rendered_into_prompt() {
        let input = UserChatInput::new("Review this")
            .with_attachments(vec![
                UserAttachmentRef::new("a1", "main.rs", UserAttachmentKind::SourceFile)
                    .with_extracted_text("fn main() {}"),
            ]);
        let prompt = input.to_prompt_text();
        assert!(prompt.contains("extracted_text=true"));
        assert!(prompt.contains("Extracted content for main.rs"));
        assert!(prompt.contains("fn main() {}"));
    }

    #[test]
    fn extracted_attachment_text_is_truncated() {
        let input = UserChatInput::new("Review this")
            .with_attachments(vec![
                UserAttachmentRef::new("a1", "large.txt", UserAttachmentKind::SourceFile)
                    .with_extracted_text("x".repeat(30_000)),
            ]);
        let prompt = input.to_prompt_text();
        assert!(prompt.contains("attachment content truncated"));
        assert!(prompt.len() < 26_000);
    }

    #[test]
    fn attachment_only_input_is_not_empty() {
        let input = UserChatInput::new("")
            .with_attachments(vec![UserAttachmentRef::new("a1", "spec.pdf", UserAttachmentKind::Document)]);
        assert!(!input.is_empty());
        assert!(input.to_prompt_text().contains("Attached files"));
    }
}
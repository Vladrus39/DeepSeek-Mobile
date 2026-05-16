use crate::chat_attachment::{ChatAttachmentDraft, ChatAttachmentKind};
use std::fs;
use std::io;

pub const MAX_INGEST_BYTES: u64 = 512 * 1024;
pub const MAX_INGEST_CHARS: usize = 120_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttachmentIngestionStatus {
    NotReadable,
    NoLocalPath,
    NeedsNativeSandboxCopy { uri: String },
    TooLarge { size_bytes: u64, max_bytes: u64 },
    UnsupportedKind,
    Read { char_count: usize, truncated: bool },
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentIngestionResult {
    pub attachment: ChatAttachmentDraft,
    pub status: AttachmentIngestionStatus,
}

pub fn ingest_attachment_text(mut attachment: ChatAttachmentDraft) -> AttachmentIngestionResult {
    if !is_text_ingestable(&attachment) {
        return AttachmentIngestionResult {
            attachment,
            status: AttachmentIngestionStatus::UnsupportedKind,
        };
    }

    let Some(path) = attachment.path.clone() else {
        if let Some(uri) = attachment.uri.clone() {
            return AttachmentIngestionResult {
                attachment,
                status: AttachmentIngestionStatus::NeedsNativeSandboxCopy { uri },
            };
        }
        return AttachmentIngestionResult {
            attachment,
            status: AttachmentIngestionStatus::NoLocalPath,
        };
    };

    match fs::metadata(&path) {
        Ok(metadata) => {
            let size_bytes = metadata.len();
            if size_bytes > MAX_INGEST_BYTES {
                return AttachmentIngestionResult {
                    attachment,
                    status: AttachmentIngestionStatus::TooLarge {
                        size_bytes,
                        max_bytes: MAX_INGEST_BYTES,
                    },
                };
            }
        }
        Err(error) => {
            return AttachmentIngestionResult {
                attachment,
                status: AttachmentIngestionStatus::Failed(error.to_string()),
            };
        }
    }

    match fs::read_to_string(&path) {
        Ok(text) => {
            let original_count = text.chars().count();
            let truncated = original_count > MAX_INGEST_CHARS;
            let extracted = if truncated {
                let mut out = text.chars().take(MAX_INGEST_CHARS).collect::<String>();
                out.push_str("\n...[mobile attachment ingestion truncated]...");
                out
            } else {
                text
            };
            attachment.extracted_text = Some(extracted);
            AttachmentIngestionResult {
                attachment,
                status: AttachmentIngestionStatus::Read {
                    char_count: original_count,
                    truncated,
                },
            }
        }
        Err(error) => AttachmentIngestionResult {
            attachment,
            status: AttachmentIngestionStatus::Failed(read_error_label(error)),
        },
    }
}

pub fn ingest_attachment_texts(
    attachments: Vec<ChatAttachmentDraft>,
) -> Vec<AttachmentIngestionResult> {
    attachments.into_iter().map(ingest_attachment_text).collect()
}

pub fn ingestion_status_message(result: &AttachmentIngestionResult) -> Option<String> {
    match &result.status {
        AttachmentIngestionStatus::Read {
            char_count,
            truncated,
        } => Some(format!(
            "Read attachment text: {} ({} chars{})",
            result.attachment.display_name,
            char_count,
            if *truncated { ", truncated" } else { "" }
        )),
        AttachmentIngestionStatus::NeedsNativeSandboxCopy { uri } => Some(format!(
            "Attachment has Android content URI but no sandbox copy yet: {} ({})",
            result.attachment.display_name, uri
        )),
        AttachmentIngestionStatus::TooLarge {
            size_bytes,
            max_bytes,
        } => Some(format!(
            "Attachment skipped as too large for mobile prompt ingestion: {} ({} > {} bytes)",
            result.attachment.display_name, size_bytes, max_bytes
        )),
        AttachmentIngestionStatus::Failed(error) => Some(format!(
            "Attachment text read failed: {} ({})",
            result.attachment.display_name, error
        )),
        AttachmentIngestionStatus::NoLocalPath
        | AttachmentIngestionStatus::NotReadable
        | AttachmentIngestionStatus::UnsupportedKind => None,
    }
}

fn is_text_ingestable(attachment: &ChatAttachmentDraft) -> bool {
    matches!(attachment.kind, ChatAttachmentKind::SourceFile)
        || attachment
            .mime_type
            .as_deref()
            .map(is_text_mime_type)
            .unwrap_or(false)
        || is_source_like_name(&attachment.display_name)
}

fn is_text_mime_type(mime: &str) -> bool {
    mime.starts_with("text/")
        || matches!(
            mime,
            "application/json"
                | "application/javascript"
                | "application/typescript"
                | "application/xml"
                | "application/x-yaml"
                | "application/toml"
                | "application/x-sh"
                | "application/x-python-code"
        )
}

fn is_source_like_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        ".rs", ".py", ".js", ".ts", ".tsx", ".jsx", ".json", ".md", ".toml", ".yaml",
        ".yml", ".txt", ".html", ".css", ".xml", ".sh", ".bash", ".zsh", ".sql",
        ".go", ".java", ".kt", ".swift", ".c", ".h", ".cpp", ".hpp", ".cs", ".php",
        ".rb", ".lua", ".dockerfile",
    ]
    .iter()
    .any(|suffix| lower.ends_with(suffix))
        || lower == "dockerfile"
        || lower == "makefile"
}

fn read_error_label(error: io::Error) -> String {
    match error.kind() {
        io::ErrorKind::InvalidData => "file is not valid UTF-8 text".to_string(),
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ingest_attachment_text, AttachmentIngestionStatus, MAX_INGEST_BYTES,
    };
    use crate::chat_attachment::ChatAttachmentDraft;
    use std::fs;

    fn unique_path(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("deepseek-mobile-{}-{}", name, nanos))
    }

    #[test]
    fn reads_text_attachment_from_local_path() {
        let path = unique_path("attachment.txt");
        fs::write(&path, "hello from attachment").expect("write test file");

        let attachment = ChatAttachmentDraft::new_document("a1", "attachment.txt")
            .with_path(&path)
            .with_mime_type("text/plain");
        let result = ingest_attachment_text(attachment);

        assert!(matches!(result.status, AttachmentIngestionStatus::Read { .. }));
        assert_eq!(result.attachment.extracted_text.as_deref(), Some("hello from attachment"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn reports_content_uri_without_sandbox_copy() {
        let attachment = ChatAttachmentDraft::new_document("a1", "main.rs")
            .with_uri("content://docs/main.rs")
            .with_mime_type("text/plain");
        let result = ingest_attachment_text(attachment);
        assert!(matches!(
            result.status,
            AttachmentIngestionStatus::NeedsNativeSandboxCopy { .. }
        ));
    }

    #[test]
    fn skips_attachment_without_local_path() {
        let attachment = ChatAttachmentDraft::new_document("a1", "attachment.txt")
            .with_mime_type("text/plain");
        let result = ingest_attachment_text(attachment);
        assert_eq!(result.status, AttachmentIngestionStatus::NoLocalPath);
    }

    #[test]
    fn rejects_large_text_attachment() {
        let path = unique_path("large.txt");
        fs::write(&path, vec![b'x'; MAX_INGEST_BYTES as usize + 1]).expect("write large test file");

        let attachment = ChatAttachmentDraft::new_document("a1", "large.txt")
            .with_path(&path)
            .with_mime_type("text/plain");
        let result = ingest_attachment_text(attachment);

        assert!(matches!(
            result.status,
            AttachmentIngestionStatus::TooLarge { .. }
        ));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn source_like_filename_is_ingestable_even_without_text_mime() {
        let path = unique_path("Cargo.toml");
        fs::write(&path, "[package]\nname = \"demo\"\n").expect("write test file");
        let attachment = ChatAttachmentDraft::new_document("a1", "Cargo.toml").with_path(&path);
        let result = ingest_attachment_text(attachment);
        assert!(matches!(result.status, AttachmentIngestionStatus::Read { .. }));
        let _ = fs::remove_file(path);
    }
}
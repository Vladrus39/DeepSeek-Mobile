use crate::document_picker::{DocumentPickerRequest, PickedDocument};
use crate::native_bridge::NativeMobileEvent;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_PICKER_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidDocumentPickerCommand {
    pub request_id: String,
    pub action: String,
    pub category: String,
    pub allow_multiple: bool,
    pub mime_types: Vec<String>,
}

impl AndroidDocumentPickerCommand {
    pub fn from_request(request: &DocumentPickerRequest) -> Self {
        Self {
            request_id: next_request_id(),
            action: request.android_intent_action().to_string(),
            category: request.android_intent_category().to_string(),
            allow_multiple: request.allows_multiple(),
            mime_types: request.mime_types.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidPickedDocumentPayload {
    pub id: String,
    pub display_name: String,
    pub uri: Option<String>,
    pub local_path: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
}

impl AndroidPickedDocumentPayload {
    pub fn new(id: impl Into<String>, display_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            display_name: display_name.into(),
            uri: None,
            local_path: None,
            mime_type: None,
            size_bytes: None,
        }
    }

    pub fn with_uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = Some(uri.into());
        self
    }

    pub fn with_local_path(mut self, path: impl Into<String>) -> Self {
        self.local_path = Some(path.into());
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

    pub fn into_picked_document(self) -> PickedDocument {
        let mut document = PickedDocument::new(self.id, self.display_name);
        if let Some(uri) = self.uri {
            document = document.with_uri(uri);
        }
        if let Some(local_path) = self.local_path {
            document = document.with_path(PathBuf::from(local_path));
        }
        if let Some(mime_type) = self.mime_type {
            document = document.with_mime_type(mime_type);
        }
        if let Some(size_bytes) = self.size_bytes {
            document = document.with_size_bytes(size_bytes);
        }
        document
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidDocumentPickerCallback {
    Picked {
        request_id: String,
        documents: Vec<AndroidPickedDocumentPayload>,
    },
    Cancelled {
        request_id: String,
    },
    Failed {
        request_id: String,
        message: String,
    },
}

impl AndroidDocumentPickerCallback {
    pub fn request_id(&self) -> &str {
        match self {
            Self::Picked { request_id, .. }
            | Self::Cancelled { request_id }
            | Self::Failed { request_id, .. } => request_id,
        }
    }

    pub fn into_native_event(self) -> NativeMobileEvent {
        match self {
            Self::Picked { documents, .. } => NativeMobileEvent::DocumentsPicked(
                documents
                    .into_iter()
                    .map(AndroidPickedDocumentPayload::into_picked_document)
                    .collect(),
            ),
            Self::Cancelled { .. } => NativeMobileEvent::DocumentPickerCancelled,
            Self::Failed { message, .. } => NativeMobileEvent::DocumentPickerFailed(message),
        }
    }
}

fn next_request_id() -> String {
    format!(
        "android-document-picker-{}",
        NEXT_PICKER_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
mod tests {
    use super::{
        AndroidDocumentPickerCallback, AndroidDocumentPickerCommand, AndroidPickedDocumentPayload,
    };
    use crate::document_picker::DocumentPickerRequest;
    use crate::native_bridge::NativeMobileEvent;

    #[test]
    fn command_uses_android_open_document_contract() {
        let command = AndroidDocumentPickerCommand::from_request(&DocumentPickerRequest::chat_attachment());
        assert_eq!(command.action, "android.intent.action.OPEN_DOCUMENT");
        assert_eq!(command.category, "android.intent.category.OPENABLE");
        assert!(command.allow_multiple);
        assert!(command.mime_types.contains(&"application/pdf".to_string()));
    }

    #[test]
    fn callback_exposes_request_id_for_bridge_correlation() {
        let callback = AndroidDocumentPickerCallback::Cancelled {
            request_id: "req-42".to_string(),
        };
        assert_eq!(callback.request_id(), "req-42");
    }

    #[test]
    fn picked_callback_becomes_native_event() {
        let callback = AndroidDocumentPickerCallback::Picked {
            request_id: "req-1".to_string(),
            documents: vec![AndroidPickedDocumentPayload::new("doc-1", "main.rs")
                .with_uri("content://docs/main.rs")
                .with_local_path("/tmp/main.rs")
                .with_mime_type("text/plain")
                .with_size_bytes(128)],
        };
        match callback.into_native_event() {
            NativeMobileEvent::DocumentsPicked(documents) => {
                assert_eq!(documents.len(), 1);
                assert_eq!(documents[0].display_name, "main.rs");
                assert_eq!(documents[0].uri.as_deref(), Some("content://docs/main.rs"));
                assert_eq!(documents[0].path.as_ref().unwrap().to_string_lossy(), "/tmp/main.rs");
            }
            other => panic!("unexpected native event: {:?}", other),
        }
    }

    #[test]
    fn failed_callback_becomes_failed_event() {
        let callback = AndroidDocumentPickerCallback::Failed {
            request_id: "req-1".to_string(),
            message: "permission denied".to_string(),
        };
        assert_eq!(
            callback.into_native_event(),
            NativeMobileEvent::DocumentPickerFailed("permission denied".to_string())
        );
    }
}
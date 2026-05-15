use crate::document_picker::{DocumentPickerRequest, PickedDocument};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeMobileCommand {
    OpenDocumentPicker(DocumentPickerRequest),
    ShareFile { path: String, mime_type: Option<String> },
    OpenUrl { url: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeMobileEvent {
    DocumentsPicked(Vec<PickedDocument>),
    DocumentPickerCancelled,
    DocumentPickerFailed(String),
    FileShared,
    ShareFailed(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NativeBridgeState {
    pub pending_commands: Vec<NativeMobileCommand>,
    pub last_event: Option<NativeMobileEvent>,
    pub last_error: Option<String>,
}

impl NativeBridgeState {
    pub fn enqueue(&mut self, command: NativeMobileCommand) {
        self.pending_commands.push(command);
    }

    pub fn enqueue_document_picker(&mut self, request: DocumentPickerRequest) {
        self.enqueue(NativeMobileCommand::OpenDocumentPicker(request));
    }

    pub fn pop_next_command(&mut self) -> Option<NativeMobileCommand> {
        if self.pending_commands.is_empty() {
            None
        } else {
            Some(self.pending_commands.remove(0))
        }
    }

    pub fn accept_event(&mut self, event: NativeMobileEvent) {
        self.last_error = match &event {
            NativeMobileEvent::DocumentPickerFailed(message) | NativeMobileEvent::ShareFailed(message) => {
                Some(message.clone())
            }
            _ => None,
        };
        self.last_event = Some(event);
    }

    pub fn has_pending_commands(&self) -> bool {
        !self.pending_commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{NativeBridgeState, NativeMobileCommand, NativeMobileEvent};
    use crate::document_picker::{DocumentPickerRequest, PickedDocument};

    #[test]
    fn bridge_queues_document_picker_command() {
        let mut state = NativeBridgeState::default();
        state.enqueue_document_picker(DocumentPickerRequest::chat_attachment());
        assert!(state.has_pending_commands());
        assert!(matches!(
            state.pop_next_command(),
            Some(NativeMobileCommand::OpenDocumentPicker(_))
        ));
        assert!(!state.has_pending_commands());
    }

    #[test]
    fn bridge_accepts_documents_picked_event() {
        let mut state = NativeBridgeState::default();
        state.accept_event(NativeMobileEvent::DocumentsPicked(vec![PickedDocument::new("1", "spec.pdf")]));
        assert!(matches!(state.last_event, Some(NativeMobileEvent::DocumentsPicked(_))));
        assert!(state.last_error.is_none());
    }

    #[test]
    fn bridge_records_native_errors() {
        let mut state = NativeBridgeState::default();
        state.accept_event(NativeMobileEvent::DocumentPickerFailed("permission denied".to_string()));
        assert_eq!(state.last_error.as_deref(), Some("permission denied"));
    }
}

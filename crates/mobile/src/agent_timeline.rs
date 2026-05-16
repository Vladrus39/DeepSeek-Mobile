#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MobileTimelineItemKind {
    UserMessage,
    AssistantMessage,
    Attachment,
    NativeCommand,
    ToolCall,
    Approval,
    Status,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MobileTimelineItemStatus {
    Pending,
    Running,
    Done,
    Failed,
    WaitingForApproval,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobileTimelineItem {
    pub id: String,
    pub kind: MobileTimelineItemKind,
    pub status: MobileTimelineItemStatus,
    pub title: String,
    pub body: String,
}

impl MobileTimelineItem {
    pub fn new(
        id: impl Into<String>,
        kind: MobileTimelineItemKind,
        status: MobileTimelineItemStatus,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            status,
            title: title.into(),
            body: body.into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MobileTimelineState {
    next_id: u64,
    pub items: Vec<MobileTimelineItem>,
    live_assistant_item_id: Option<String>,
}

impl MobileTimelineState {
    pub fn push(
        &mut self,
        kind: MobileTimelineItemKind,
        status: MobileTimelineItemStatus,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> String {
        self.next_id += 1;
        let id = format!("timeline-{}", self.next_id);
        self.items.push(MobileTimelineItem::new(
            id.clone(),
            kind,
            status,
            title,
            body,
        ));
        id
    }

    pub fn push_user_message(&mut self, body: impl Into<String>) -> String {
        self.finish_live_assistant_message();
        self.push(
            MobileTimelineItemKind::UserMessage,
            MobileTimelineItemStatus::Done,
            "User message",
            body,
        )
    }

    pub fn push_assistant_message(&mut self, body: impl Into<String>) -> String {
        let id = self.push(
            MobileTimelineItemKind::AssistantMessage,
            MobileTimelineItemStatus::Done,
            "DeepSeek response",
            body,
        );
        self.live_assistant_item_id = None;
        id
    }

    pub fn start_live_assistant_message(&mut self) -> String {
        if let Some(id) = self.live_assistant_item_id.clone() {
            return id;
        }
        let id = self.push(
            MobileTimelineItemKind::AssistantMessage,
            MobileTimelineItemStatus::Running,
            "DeepSeek response",
            "",
        );
        self.live_assistant_item_id = Some(id.clone());
        id
    }

    pub fn append_live_assistant_delta(&mut self, delta: &str) -> String {
        let id = self.start_live_assistant_message();
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.body.push_str(delta);
            item.status = MobileTimelineItemStatus::Running;
        }
        id
    }

    pub fn finish_live_assistant_message(&mut self) {
        if let Some(id) = self.live_assistant_item_id.take() {
            if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
                item.status = MobileTimelineItemStatus::Done;
            }
        }
    }

    pub fn fail_live_assistant_message(&mut self) {
        if let Some(id) = self.live_assistant_item_id.take() {
            if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
                item.status = MobileTimelineItemStatus::Failed;
            }
        }
    }

    pub fn push_attachment(&mut self, body: impl Into<String>) -> String {
        self.push(
            MobileTimelineItemKind::Attachment,
            MobileTimelineItemStatus::Done,
            "Attachment added",
            body,
        )
    }

    pub fn push_native_command(&mut self, body: impl Into<String>) -> String {
        self.push(
            MobileTimelineItemKind::NativeCommand,
            MobileTimelineItemStatus::Pending,
            "Native mobile command",
            body,
        )
    }

    pub fn push_status(&mut self, body: impl Into<String>) -> String {
        self.push(
            MobileTimelineItemKind::Status,
            MobileTimelineItemStatus::Running,
            "Agent status",
            body,
        )
    }

    pub fn push_error(&mut self, body: impl Into<String>) -> String {
        self.fail_live_assistant_message();
        self.push(
            MobileTimelineItemKind::Error,
            MobileTimelineItemStatus::Failed,
            "Error",
            body,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

pub fn timeline_kind_label(kind: &MobileTimelineItemKind) -> &'static str {
    match kind {
        MobileTimelineItemKind::UserMessage => "USER",
        MobileTimelineItemKind::AssistantMessage => "AI",
        MobileTimelineItemKind::Attachment => "FILE",
        MobileTimelineItemKind::NativeCommand => "NATIVE",
        MobileTimelineItemKind::ToolCall => "TOOL",
        MobileTimelineItemKind::Approval => "APPROVAL",
        MobileTimelineItemKind::Status => "STATUS",
        MobileTimelineItemKind::Error => "ERROR",
    }
}

pub fn timeline_status_label(status: &MobileTimelineItemStatus) -> &'static str {
    match status {
        MobileTimelineItemStatus::Pending => "pending",
        MobileTimelineItemStatus::Running => "running",
        MobileTimelineItemStatus::Done => "done",
        MobileTimelineItemStatus::Failed => "failed",
        MobileTimelineItemStatus::WaitingForApproval => "approval",
    }
}

#[cfg(test)]
mod tests {
    use super::{timeline_kind_label, MobileTimelineItemKind, MobileTimelineItemStatus, MobileTimelineState};

    #[test]
    fn timeline_pushes_items_in_order() {
        let mut timeline = MobileTimelineState::default();
        timeline.push_user_message("Build project");
        timeline.push_status("Thinking");
        timeline.push_assistant_message("Done");
        assert_eq!(timeline.len(), 3);
        assert_eq!(timeline.items[0].id, "timeline-1");
        assert_eq!(timeline.items[2].title, "DeepSeek response");
    }

    #[test]
    fn timeline_labels_are_stable() {
        assert_eq!(timeline_kind_label(&MobileTimelineItemKind::ToolCall), "TOOL");
        assert_eq!(timeline_kind_label(&MobileTimelineItemKind::Approval), "APPROVAL");
    }

    #[test]
    fn streaming_deltas_append_to_one_live_assistant_item() {
        let mut timeline = MobileTimelineState::default();
        timeline.append_live_assistant_delta("hel");
        timeline.append_live_assistant_delta("lo");
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline.items[0].body, "hello");
        assert_eq!(timeline.items[0].status, MobileTimelineItemStatus::Running);
        timeline.finish_live_assistant_message();
        assert_eq!(timeline.items[0].status, MobileTimelineItemStatus::Done);
    }
}
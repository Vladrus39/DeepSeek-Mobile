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
    live_reasoning_item_id: Option<String>,
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
        self.finish_live_reasoning();
    }

    pub fn append_live_reasoning_delta(&mut self, delta: &str) -> String {
        if delta.is_empty() {
            return self
                .live_reasoning_item_id
                .clone()
                .unwrap_or_else(|| self.start_live_reasoning());
        }
        let id = self.start_live_reasoning();
        if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
            item.body.push_str(delta);
            item.status = MobileTimelineItemStatus::Running;
        }
        id
    }

    fn start_live_reasoning(&mut self) -> String {
        if let Some(id) = self.live_reasoning_item_id.clone() {
            return id;
        }
        let id = self.push(
            MobileTimelineItemKind::Status,
            MobileTimelineItemStatus::Running,
            "Reasoning",
            "",
        );
        self.live_reasoning_item_id = Some(id.clone());
        id
    }

    pub fn finish_live_reasoning(&mut self) {
        if let Some(id) = self.live_reasoning_item_id.take() {
            if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
                if item.body.trim().is_empty() {
                    self.items.retain(|existing| existing.id != id);
                } else {
                    item.status = MobileTimelineItemStatus::Done;
                }
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
        self.seal_agent_status_items();
        self.push(
            MobileTimelineItemKind::Status,
            MobileTimelineItemStatus::Running,
            "Agent status",
            body,
        )
    }

    /// Mark in-flight agent status / tool rows as finished (end of turn or restore).
    pub fn seal_open_work_items(&mut self) {
        self.finish_live_assistant_message();
        self.finish_live_reasoning();
        for item in &mut self.items {
            match item.status {
                MobileTimelineItemStatus::Running | MobileTimelineItemStatus::Pending => {
                    match item.kind {
                        MobileTimelineItemKind::Status
                        | MobileTimelineItemKind::NativeCommand
                        | MobileTimelineItemKind::ToolCall => {
                            item.status = MobileTimelineItemStatus::Done;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    /// Close the previous running «Agent status» line before appending another.
    pub fn seal_agent_status_items(&mut self) {
        for item in &mut self.items {
            if item.kind == MobileTimelineItemKind::Status
                && item.status == MobileTimelineItemStatus::Running
                && item.title == "Agent status"
            {
                item.status = MobileTimelineItemStatus::Done;
            }
        }
    }

    /// Pair a finished tool result with its matching in-flight tool row.
    pub fn seal_tool_call(&mut self, tool_name: &str) {
        let title = format!("Tool: {tool_name}");
        if let Some(item) = self.items.iter_mut().rev().find(|item| {
            item.kind == MobileTimelineItemKind::ToolCall
                && item.status == MobileTimelineItemStatus::Running
                && item.title == title
        }) {
            item.status = MobileTimelineItemStatus::Done;
        }
    }

    /// Keep the chat usable when many old failures were persisted on disk.
    pub fn retain_recent(&mut self, max_items: usize) {
        if self.items.len() > max_items {
            let drop = self.items.len() - max_items;
            self.items.drain(0..drop);
        }
    }

    /// After reloading persisted events: merge legacy per-chunk Reasoning rows, drop probe noise.
    pub fn compact_for_display(&mut self) {
        self.items.retain(|item| {
            !(item.kind == MobileTimelineItemKind::AssistantMessage
                && item.body.trim() == "PROBE_OK")
        });

        let mut merged: Vec<MobileTimelineItem> = Vec::new();
        for item in self.items.drain(..) {
            if let Some(last) = merged.last_mut() {
                if last.kind == MobileTimelineItemKind::Status
                    && last.title == "Reasoning"
                    && item.kind == MobileTimelineItemKind::Status
                    && item.title == "Reasoning"
                {
                    last.body.push_str(&item.body);
                    last.status = item.status;
                    continue;
                }
            }
            merged.push(item);
        }
        self.items = merged;
        self.live_assistant_item_id = None;
        self.live_reasoning_item_id = None;
    }

    /// Drop stale boot errors so the chat column is not a wall of red after fixes.
    pub fn soften_stale_errors(&mut self) {
        let errors = self
            .items
            .iter()
            .filter(|item| item.kind == MobileTimelineItemKind::Error)
            .count();
        if errors > 2 {
            self.items
                .retain(|item| item.kind != MobileTimelineItemKind::Error);
        }
    }

    /// Ensure the model reply is visible in the conversation column (not only in status/worklog).
    pub fn publish_assistant_reply(&mut self, text: &str) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        if let Some(id) = self.live_assistant_item_id.clone() {
            if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
                if item.body.trim().is_empty() {
                    item.body = text.to_string();
                }
                item.status = MobileTimelineItemStatus::Done;
            }
            self.live_assistant_item_id = None;
            return;
        }
        if let Some(item) = self
            .items
            .iter_mut()
            .rev()
            .find(|item| item.kind == MobileTimelineItemKind::AssistantMessage)
        {
            if item.body.trim().is_empty() {
                item.body = text.to_string();
                item.status = MobileTimelineItemStatus::Done;
                return;
            }
            if item.body.contains(text) || text.contains(item.body.as_str()) {
                item.status = MobileTimelineItemStatus::Done;
                return;
            }
        }
        self.push_assistant_message(text);
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

    /// Status lines that describe a finished turn should not keep the work log in «выполняется».
    pub fn status_message_is_terminal(message: &str) -> bool {
        let lower = message.to_lowercase();
        lower.contains("готово")
            || lower.contains("finished")
            || lower.contains("completed")
            || lower.contains("turn finished")
            || lower.contains("approval continuation finished")
            || lower.contains("калибровка termux завершена")
            || lower.contains("termux result injected")
            || lower.starts_with("done ·")
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
    use super::{
        timeline_kind_label, MobileTimelineItemKind, MobileTimelineItemStatus, MobileTimelineState,
    };

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
        assert_eq!(
            timeline_kind_label(&MobileTimelineItemKind::ToolCall),
            "TOOL"
        );
        assert_eq!(
            timeline_kind_label(&MobileTimelineItemKind::Approval),
            "APPROVAL"
        );
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

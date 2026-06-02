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
    /// Workspace-relative paths for "Open in Files" from chat / tool cards.
    pub linked_file_paths: Vec<String>,
    /// Safety snapshot id for one-tap rollback from chat / work log.
    pub linked_snapshot_id: Option<String>,
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
            linked_file_paths: Vec::new(),
            linked_snapshot_id: None,
        }
    }

    pub fn snapshot_id(&self) -> Option<String> {
        if let Some(id) = self.linked_snapshot_id.clone() {
            return Some(id);
        }
        if self.title == "Safety snapshot" {
            return parse_safety_snapshot_id(&self.body);
        }
        None
    }

    pub fn with_linked_file_paths(mut self, paths: Vec<String>) -> Self {
        self.linked_file_paths = paths;
        self
    }
}

fn linked_paths_since_last_user_message(
    items: &[MobileTimelineItem],
    assistant_index: usize,
) -> Vec<String> {
    let start = items[..assistant_index]
        .iter()
        .rposition(|item| item.kind == MobileTimelineItemKind::UserMessage)
        .map(|index| index + 1)
        .unwrap_or(0);
    let mut paths = Vec::new();
    for item in &items[start..assistant_index] {
        merge_linked_paths(&mut paths, item.linked_file_paths.clone());
        if matches!(
            item.kind,
            MobileTimelineItemKind::ToolCall
                | MobileTimelineItemKind::Status
                | MobileTimelineItemKind::NativeCommand
                | MobileTimelineItemKind::Attachment
        ) {
            merge_linked_paths(
                &mut paths,
                crate::chat_file_links::extract_paths_from_text(&format!(
                    "{} {}",
                    item.title, item.body
                )),
            );
        }
    }
    paths
}

pub const TOOL_STEP_INPUT_MARKER: &str = "── Input ──\n";
pub const TOOL_STEP_OUTPUT_MARKER: &str = "\n\n── Output ──\n";

pub fn tool_name_from_step_title(title: &str) -> Option<&str> {
    title
        .strip_prefix("Tool: ")
        .or_else(|| title.strip_prefix("Tool result: "))
}

pub fn parse_safety_snapshot_id(body: &str) -> Option<String> {
    let token = body.split('·').next()?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

pub fn format_tool_step_with_output(input: &str, output: &str) -> String {
    format!("{TOOL_STEP_INPUT_MARKER}{input}{TOOL_STEP_OUTPUT_MARKER}{output}")
}

pub fn split_tool_step_body(body: &str) -> Option<(&str, &str)> {
    let input_start = body.find(TOOL_STEP_INPUT_MARKER)? + TOOL_STEP_INPUT_MARKER.len();
    let output_marker = body.find(TOOL_STEP_OUTPUT_MARKER)?;
    let input = &body[input_start..output_marker];
    let output_start = output_marker + TOOL_STEP_OUTPUT_MARKER.len();
    Some((input, &body[output_start..]))
}

fn merge_tool_result_into_call(call: &mut MobileTimelineItem, result: &MobileTimelineItem) -> bool {
    if call.kind != MobileTimelineItemKind::ToolCall
        || result.kind != MobileTimelineItemKind::ToolCall
    {
        return false;
    }
    let Some(call_name) = tool_name_from_step_title(&call.title) else {
        return false;
    };
    let Some(result_name) = tool_name_from_step_title(&result.title) else {
        return false;
    };
    if call.title.starts_with("Tool result:") || !result.title.starts_with("Tool result:") {
        return false;
    }
    if call_name != result_name {
        return false;
    }
    call.body = format_tool_step_with_output(&call.body, &result.body);
    call.status = result.status.clone();
    merge_linked_paths(
        &mut call.linked_file_paths,
        result.linked_file_paths.clone(),
    );
    true
}

fn merge_linked_paths(existing: &mut Vec<String>, mut more: Vec<String>) {
    for path in more.drain(..) {
        if path.is_empty() {
            continue;
        }
        if !existing.iter().any(|p| p == &path) {
            existing.push(path);
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MobileTimelineState {
    next_id: u64,
    pub items: Vec<MobileTimelineItem>,
    live_assistant_item_id: Option<String>,
    live_reasoning_item_id: Option<String>,
    /// UI: expand «Work log» while tools / reasoning are in flight.
    pub open_worklog_hint: bool,
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

    /// Push a timeline row with a stable id (e.g. approval request id for inline UI wiring).
    pub fn push_with_id(
        &mut self,
        id: impl Into<String>,
        kind: MobileTimelineItemKind,
        status: MobileTimelineItemStatus,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> String {
        let id = id.into();
        self.next_id += 1;
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
        self.enrich_assistant_file_links(&id);
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
            self.enrich_assistant_file_links(&id);
        }
        self.finish_live_reasoning();
    }

    /// Attach file paths from this turn's tools (and assistant text) to the reply bubble.
    pub fn enrich_assistant_file_links(&mut self, assistant_id: &str) {
        let Some(assistant_idx) = self.items.iter().position(|item| item.id == assistant_id) else {
            return;
        };
        let body = self.items[assistant_idx].body.clone();
        let mut paths = crate::chat_file_links::extract_paths_from_text(&body);
        merge_linked_paths(
            &mut paths,
            linked_paths_since_last_user_message(&self.items, assistant_idx),
        );
        if let Some(item) = self.items.get_mut(assistant_idx) {
            merge_linked_paths(&mut item.linked_file_paths, paths);
        }
    }

    pub fn attach_linked_paths(&mut self, item_id: &str, paths: Vec<String>) {
        if paths.is_empty() {
            return;
        }
        if let Some(item) = self.items.iter_mut().find(|row| row.id == item_id) {
            merge_linked_paths(&mut item.linked_file_paths, paths);
        }
    }

    pub fn attach_linked_snapshot(&mut self, item_id: &str, snapshot_id: impl Into<String>) {
        let snapshot_id = snapshot_id.into();
        if snapshot_id.is_empty() {
            return;
        }
        if let Some(item) = self.items.iter_mut().find(|row| row.id == item_id) {
            item.linked_snapshot_id = Some(snapshot_id);
        }
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

    /// Attach tool output to the in-flight `Tool: …` row (one card per tool, Cursor-style).
    pub fn complete_tool_call(
        &mut self,
        tool_name: &str,
        success: bool,
        output: &str,
        paths: Vec<String>,
    ) -> Option<String> {
        let title = format!("Tool: {tool_name}");
        let item_id = self
            .items
            .iter()
            .rev()
            .find(|item| {
                item.kind == MobileTimelineItemKind::ToolCall
                    && item.title == title
                    && item.status == MobileTimelineItemStatus::Running
            })
            .map(|item| item.id.clone())?;
        if let Some(item) = self.items.iter_mut().find(|row| row.id == item_id) {
            let input = item.body.clone();
            item.body = format_tool_step_with_output(&input, output);
            item.status = if success {
                MobileTimelineItemStatus::Done
            } else {
                MobileTimelineItemStatus::Failed
            };
        }
        self.attach_linked_paths(&item_id, paths);
        Some(item_id)
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
                if merge_tool_result_into_call(last, &item) {
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
        parse_safety_snapshot_id, timeline_kind_label, MobileTimelineItemKind,
        MobileTimelineItemStatus, MobileTimelineState,
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

    #[test]
    fn parses_safety_snapshot_id_from_body() {
        assert_eq!(
            parse_safety_snapshot_id("snap-abc · 3 file(s) · 120 bytes"),
            Some("snap-abc".to_string())
        );
    }

    #[test]
    fn complete_tool_call_merges_output_into_running_row() {
        let mut timeline = MobileTimelineState::default();
        let id = timeline.push(
            MobileTimelineItemKind::ToolCall,
            MobileTimelineItemStatus::Running,
            "Tool: write_file",
            r#"{"path":"src/a.rs"}"#.to_string(),
        );
        let completed = timeline.complete_tool_call(
            "write_file",
            true,
            "Wrote 10 bytes",
            vec!["src/a.rs".to_string()],
        );
        assert_eq!(completed.as_deref(), Some(id.as_str()));
        assert_eq!(timeline.len(), 1);
        assert!(timeline.items[0].body.contains("── Output ──"));
        assert!(timeline.items[0]
            .linked_file_paths
            .contains(&"src/a.rs".to_string()));
    }

    #[test]
    fn compact_for_display_merges_legacy_tool_result_rows() {
        let mut timeline = MobileTimelineState::default();
        timeline.push(
            MobileTimelineItemKind::ToolCall,
            MobileTimelineItemStatus::Done,
            "Tool: read_file",
            r#"{"path":"Cargo.toml"}"#.to_string(),
        );
        timeline.push(
            MobileTimelineItemKind::ToolCall,
            MobileTimelineItemStatus::Done,
            "Tool result: read_file",
            "ok".to_string(),
        );
        timeline.compact_for_display();
        assert_eq!(timeline.len(), 1);
        assert!(timeline.items[0].body.contains("── Output ──"));
        assert!(timeline.items[0].body.contains("ok"));
    }

    #[test]
    fn assistant_reply_inherits_paths_from_tools_in_same_turn() {
        let mut timeline = MobileTimelineState::default();
        timeline.push_user_message("create demo");
        let tool_id = timeline.push(
            MobileTimelineItemKind::ToolCall,
            MobileTimelineItemStatus::Done,
            "Tool result: write_file",
            "Wrote 3 bytes to pkg/demo.rs",
        );
        timeline.attach_linked_paths(&tool_id, vec!["pkg/demo.rs".to_string()]);
        timeline.append_live_assistant_delta("Готово, файл создан.");
        timeline.finish_live_assistant_message();
        let assistant = timeline
            .items
            .iter()
            .find(|item| item.kind == MobileTimelineItemKind::AssistantMessage)
            .expect("assistant bubble");
        assert!(
            assistant
                .linked_file_paths
                .contains(&"pkg/demo.rs".to_string()),
            "paths from tools should appear on the assistant reply"
        );
    }
}

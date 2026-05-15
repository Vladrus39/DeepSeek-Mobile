//! Tool-call request contract and parser.
//!
//! DeepSeek Mobile should not rely on free-form text when the model wants to
//! act on a project. This module defines a small stable contract that can be
//! used by native API tool-calls later, and by JSON blocks today.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static TOOL_CALL_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolCallSource {
    NativeApi,
    JsonBlock,
    InlineJson,
    Manual,
    System,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    pub source: ToolCallSource,
}

impl ToolCallRequest {
    pub fn new(name: impl Into<String>, arguments: Value, source: ToolCallSource) -> Self {
        Self {
            id: new_tool_call_id(),
            name: name.into(),
            arguments,
            source,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ToolCallParseResult {
    pub final_text: String,
    pub tool_calls: Vec<ToolCallRequest>,
}

impl ToolCallParseResult {
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

pub fn parse_tool_calls_from_text(text: &str) -> ToolCallParseResult {
    let trimmed = text.trim();

    if let Some(calls) = parse_json_tool_calls(trimmed, ToolCallSource::InlineJson) {
        return ToolCallParseResult {
            final_text: String::new(),
            tool_calls: calls,
        };
    }

    let mut result = ToolCallParseResult {
        final_text: text.to_string(),
        tool_calls: Vec::new(),
    };

    for block in extract_json_code_blocks(text) {
        if let Some(mut calls) = parse_json_tool_calls(&block, ToolCallSource::JsonBlock) {
            result.tool_calls.append(&mut calls);
        }
    }

    result
}

fn parse_json_tool_calls(raw: &str, source: ToolCallSource) -> Option<Vec<ToolCallRequest>> {
    let value: Value = serde_json::from_str(raw).ok()?;
    tool_calls_from_value(&value, source)
}

fn tool_calls_from_value(value: &Value, source: ToolCallSource) -> Option<Vec<ToolCallRequest>> {
    if let Some(array) = value.get("tool_calls").and_then(Value::as_array) {
        let calls = array
            .iter()
            .filter_map(|item| tool_call_from_object(item, source.clone()))
            .collect::<Vec<_>>();
        return (!calls.is_empty()).then_some(calls);
    }

    if let Some(array) = value.get("tools").and_then(Value::as_array) {
        let calls = array
            .iter()
            .filter_map(|item| tool_call_from_object(item, source.clone()))
            .collect::<Vec<_>>();
        return (!calls.is_empty()).then_some(calls);
    }

    tool_call_from_object(value, source).map(|call| vec![call])
}

fn tool_call_from_object(value: &Value, source: ToolCallSource) -> Option<ToolCallRequest> {
    let object = value.as_object()?;

    let name = object
        .get("tool")
        .or_else(|| object.get("tool_name"))
        .or_else(|| object.get("name"))
        .and_then(Value::as_str)?;

    let arguments = object
        .get("args")
        .or_else(|| object.get("arguments"))
        .or_else(|| object.get("input"))
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let id = object
        .get("id")
        .and_then(Value::as_str)
        .map(std::string::ToString::to_string)
        .unwrap_or_else(new_tool_call_id);

    Some(ToolCallRequest {
        id,
        name: name.to_string(),
        arguments,
        source,
    })
}

fn extract_json_code_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut in_json_block = false;
    let mut current = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            if in_json_block {
                blocks.push(current.join("\n"));
                current.clear();
                in_json_block = false;
            } else {
                let fence_lang = trimmed.trim_start_matches("```").trim().to_ascii_lowercase();
                in_json_block = fence_lang == "json" || fence_lang == "tool" || fence_lang == "tool_call";
            }
            continue;
        }

        if in_json_block {
            current.push(line.to_string());
        }
    }

    blocks
}

fn new_tool_call_id() -> String {
    let seq = TOOL_CALL_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("tool-{}-{}", current_unix_time(), seq)
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{parse_tool_calls_from_text, ToolCallSource};
    use serde_json::json;

    #[test]
    fn parses_single_inline_tool_call() {
        let result = parse_tool_calls_from_text(r#"{"tool":"read_file","args":{"path":"README.md"}}"#);
        assert!(result.has_tool_calls());
        assert_eq!(result.tool_calls[0].name, "read_file");
        assert_eq!(result.tool_calls[0].arguments, json!({"path":"README.md"}));
        assert_eq!(result.tool_calls[0].source, ToolCallSource::InlineJson);
    }

    #[test]
    fn parses_tool_calls_array() {
        let result = parse_tool_calls_from_text(
            r#"{"tool_calls":[{"name":"read_file","arguments":{"path":"src/lib.rs"}},{"tool":"git_status"}]}"#,
        );
        assert_eq!(result.tool_calls.len(), 2);
        assert_eq!(result.tool_calls[0].name, "read_file");
        assert_eq!(result.tool_calls[1].name, "git_status");
    }

    #[test]
    fn parses_json_code_block_without_dropping_text() {
        let text = "I need to inspect the file.\n```json\n{\"tool\":\"read_file\",\"args\":{\"path\":\"Cargo.toml\"}}\n```";
        let result = parse_tool_calls_from_text(text);
        assert_eq!(result.final_text, text);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].source, ToolCallSource::JsonBlock);
    }

    #[test]
    fn ignores_plain_text() {
        let result = parse_tool_calls_from_text("normal answer");
        assert!(!result.has_tool_calls());
        assert_eq!(result.final_text, "normal answer");
    }
}

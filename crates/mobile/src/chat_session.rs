//! Multi-chat sessions: active thread id, new chat, clear on-screen feed (history stays on disk).

use crate::agent_timeline::MobileTimelineState;
use crate::mobile_runtime_config::{default_data_dir, MobileRuntimeConfig};
use crate::saved_timeline_loader::load_saved_timeline;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

const INDEX_FILE: &str = "chat_sessions.json";
const LEGACY_THREAD: &str = "mobile-default-thread";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatThreadMeta {
    pub id: String,
    pub title: String,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatSessionIndex {
    pub active_thread_id: String,
    #[serde(default)]
    pub threads: Vec<ChatThreadMeta>,
}

impl Default for ChatSessionIndex {
    fn default() -> Self {
        let now = unix_now();
        Self {
            active_thread_id: LEGACY_THREAD.to_string(),
            threads: vec![ChatThreadMeta {
                id: LEGACY_THREAD.to_string(),
                title: "Main chat".to_string(),
                created_at_unix: now,
                updated_at_unix: now,
            }],
        }
    }
}

fn index_path() -> PathBuf {
    default_data_dir().join(INDEX_FILE)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn load_index() -> ChatSessionIndex {
    let path = index_path();
    if !path.exists() {
        return ChatSessionIndex::default();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_index(index: &ChatSessionIndex) -> Result<(), String> {
    let path = index_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(index).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

pub fn runtime_for_active_thread() -> MobileRuntimeConfig {
    let index = load_index();
    MobileRuntimeConfig::default_mobile().with_thread_id(index.active_thread_id)
}

pub fn load_timeline_for_active_thread() -> anyhow::Result<MobileTimelineState> {
    load_saved_timeline(&runtime_for_active_thread())
}

pub fn load_timeline_for_thread(thread_id: &str) -> anyhow::Result<MobileTimelineState> {
    load_saved_timeline(&MobileRuntimeConfig::default_mobile().with_thread_id(thread_id))
}

pub fn switch_chat_thread(thread_id: &str) -> Result<MobileTimelineState, String> {
    let mut index = load_index();
    if !index.threads.iter().any(|thread| thread.id == thread_id) {
        return Err(format!("chat thread not found: {thread_id}"));
    }
    index.active_thread_id = thread_id.to_string();
    save_index(&index)?;
    load_timeline_for_thread(thread_id)
        .map(|mut timeline| {
            timeline.compact_for_display();
            timeline.soften_stale_errors();
            timeline
        })
        .map_err(|error| error.to_string())
}

/// Empty on-screen feed; persisted events for this thread remain in runtime_store.
pub fn clear_active_timeline_display() -> MobileTimelineState {
    MobileTimelineState::default()
}

/// New thread id, switch active, empty timeline (old threads remain in store).
pub fn start_new_chat(title: Option<String>) -> Result<(String, MobileTimelineState), String> {
    let mut index = load_index();
    let id = format!("chat-{}", Uuid::new_v4());
    let now = unix_now();
    let label = title.unwrap_or_else(|| format!("Чат {}", index.threads.len() + 1));
    index.threads.push(ChatThreadMeta {
        id: id.clone(),
        title: label,
        created_at_unix: now,
        updated_at_unix: now,
    });
    index.active_thread_id = id.clone();
    save_index(&index)?;
    Ok((id, MobileTimelineState::default()))
}

pub fn delete_chat_thread(thread_id: &str) -> Result<String, String> {
    let mut index = load_index();
    if index.threads.len() <= 1 {
        return Err("Cannot delete the only chat thread".to_string());
    }
    if !index.threads.iter().any(|thread| thread.id == thread_id) {
        return Err(format!("chat thread not found: {thread_id}"));
    }
    index.threads.retain(|thread| thread.id != thread_id);
    if index.active_thread_id == thread_id {
        index.active_thread_id = index
            .threads
            .first()
            .map(|thread| thread.id.clone())
            .unwrap_or_else(|| LEGACY_THREAD.to_string());
    }
    save_index(&index)?;
    Ok(index.active_thread_id.clone())
}

pub fn touch_active_thread() {
    let mut index = load_index();
    let now = unix_now();
    if let Some(thread) = index
        .threads
        .iter_mut()
        .find(|t| t.id == index.active_thread_id)
    {
        thread.updated_at_unix = now;
        let _ = save_index(&index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_data_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("deepseek-chat-session-{}-{}", name, unix_now()));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn new_chat_creates_unique_thread() {
        let dir = temp_data_dir("new");
        env::set_var("DEEPSEEK_MOBILE_DATA_DIR", dir.to_string_lossy().as_ref());
        crate::mobile_data_dir::set_mobile_data_dir(&dir);

        let (id, timeline) = start_new_chat(Some("Test".to_string())).expect("new chat");
        assert!(id.starts_with("chat-"));
        assert!(timeline.is_empty());
        let index = load_index();
        assert_eq!(index.active_thread_id, id);

        let _ = fs::remove_dir_all(dir);
    }
}

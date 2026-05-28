use crate::agent_event_adapter::push_agent_event;
use crate::agent_timeline::MobileTimelineState;
use crate::mobile_runtime_config::MobileRuntimeConfig;
use deepseek_mobile_core::{AgentEvent, RuntimeThreadStore};

pub fn load_saved_events(runtime: &MobileRuntimeConfig) -> anyhow::Result<Vec<AgentEvent>> {
    let store = match RuntimeThreadStore::open(runtime.runtime_store_root.clone()) {
        Ok(store) => store,
        Err(error) if is_benign_restore_error(&error) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let records = match store.load_events(&runtime.thread_id) {
        Ok(records) => records,
        Err(error) if is_benign_restore_error(&error) => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    Ok(records.into_iter().map(|record| record.event).collect())
}

pub fn load_saved_timeline(runtime: &MobileRuntimeConfig) -> anyhow::Result<MobileTimelineState> {
    let mut timeline = MobileTimelineState::default();

    for event in load_saved_events(runtime)? {
        push_agent_event(&mut timeline, &event);
    }

    Ok(timeline)
}

pub fn load_default_saved_timeline() -> anyhow::Result<MobileTimelineState> {
    load_saved_timeline(&MobileRuntimeConfig::default())
}

pub fn load_default_saved_events() -> anyhow::Result<Vec<AgentEvent>> {
    load_saved_events(&MobileRuntimeConfig::default())
}

pub fn load_active_saved_timeline() -> anyhow::Result<MobileTimelineState> {
    crate::chat_session::load_timeline_for_active_thread()
}

pub fn load_active_saved_events() -> anyhow::Result<Vec<AgentEvent>> {
    load_saved_events(&crate::chat_session::runtime_for_active_thread())
}

/// Empty or partially corrupt runtime JSON should not surface as a chat error banner.
///
/// Walks the full error chain because the JSON parse error is usually wrapped
/// in higher-level `.context(...)` text (e.g. "failed to read events file"),
/// so the outer `error.to_string()` would not contain "EOF while parsing".
pub fn is_benign_restore_error(error: &anyhow::Error) -> bool {
    const BENIGN_NEEDLES: &[&str] = &[
        "No such file",
        "os error 2",
        "EOF while parsing",
        "unexpected end of input",
        "expected value at line 1 column 1",
        "trailing characters",
    ];
    error.chain().any(|cause| {
        let message = cause.to_string();
        BENIGN_NEEDLES
            .iter()
            .any(|needle| message.contains(needle))
    })
}

#[cfg(test)]
mod tests {
    use super::load_saved_timeline;
    use crate::mobile_runtime_config::MobileRuntimeConfig;
    use deepseek_mobile_core::{AgentEvent, RuntimeThreadStore};
    use std::path::PathBuf;

    fn unique_test_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("deepseek-mobile-{}-{}", name, nanos))
    }

    #[test]
    fn loader_replays_saved_agent_events() {
        let base_dir = unique_test_dir("timeline-loader");
        let runtime = MobileRuntimeConfig::from_base_dir(&base_dir).with_thread_id("thread-a");
        let store = RuntimeThreadStore::open(runtime.runtime_store_root.clone())
            .expect("open runtime store");
        store
            .save_event(
                "thread-a",
                "turn-a",
                &AgentEvent::Status("restored".to_string()),
            )
            .expect("save event");

        let timeline = load_saved_timeline(&runtime).expect("load saved timeline");
        assert_eq!(timeline.len(), 1);
        assert_eq!(timeline.items[0].body, "restored");

        let _ = std::fs::remove_dir_all(base_dir);
    }

    #[test]
    fn benign_restore_error_detects_eof() {
        let err = anyhow::anyhow!("EOF while parsing a value at line 1 column 0");
        assert!(super::is_benign_restore_error(&err));
    }

    #[test]
    fn benign_restore_error_detects_eof_in_wrapped_chain() {
        let inner = anyhow::anyhow!("EOF while parsing a value at line 1 column 0");
        let wrapped = inner.context("failed to read thread events file");
        assert!(super::is_benign_restore_error(&wrapped));
    }

    #[test]
    fn benign_restore_error_detects_missing_file_in_wrapped_chain() {
        let inner = anyhow::anyhow!("No such file or directory (os error 2)");
        let wrapped = inner.context("open events file");
        assert!(super::is_benign_restore_error(&wrapped));
    }

    #[test]
    fn benign_restore_error_rejects_genuine_failure() {
        let err = anyhow::anyhow!("disk quota exceeded");
        assert!(!super::is_benign_restore_error(&err));
    }

    #[test]
    fn loader_returns_empty_timeline_when_no_events_exist() {
        let base_dir = unique_test_dir("empty-timeline-loader");
        let runtime = MobileRuntimeConfig::from_base_dir(&base_dir).with_thread_id("thread-empty");

        let timeline = load_saved_timeline(&runtime).expect("load empty timeline");
        assert!(timeline.is_empty());

        let _ = std::fs::remove_dir_all(base_dir);
    }
}

use crate::agent_event_adapter::push_agent_event;
use crate::agent_timeline::MobileTimelineState;
use crate::mobile_runtime_config::MobileRuntimeConfig;
use deepseek_mobile_core::{AgentEvent, RuntimeThreadStore};

pub fn load_saved_events(runtime: &MobileRuntimeConfig) -> anyhow::Result<Vec<AgentEvent>> {
    let store = RuntimeThreadStore::open(runtime.runtime_store_root.clone())?;
    let records = store.load_events(&runtime.thread_id)?;
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
    fn loader_returns_empty_timeline_when_no_events_exist() {
        let base_dir = unique_test_dir("empty-timeline-loader");
        let runtime = MobileRuntimeConfig::from_base_dir(&base_dir).with_thread_id("thread-empty");

        let timeline = load_saved_timeline(&runtime).expect("load empty timeline");
        assert!(timeline.is_empty());

        let _ = std::fs::remove_dir_all(base_dir);
    }
}

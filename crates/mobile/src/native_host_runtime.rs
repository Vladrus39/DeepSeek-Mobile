//! Process-wide native bridge state shared by Dioxus UI, desktop host loop and Android JNI.

use crate::android_host::{apply_host_callback_json, drain_next_host_action};
use crate::native_bridge::NativeBridgeState;
use std::sync::{Mutex, OnceLock};

static RUNTIME: OnceLock<Mutex<NativeBridgeState>> = OnceLock::new();

fn runtime() -> &'static Mutex<NativeBridgeState> {
    RUNTIME.get_or_init(|| Mutex::new(NativeBridgeState::default()))
}

pub fn with_state<R>(f: impl FnOnce(&mut NativeBridgeState) -> R) -> R {
    let mut guard = runtime()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
}

pub fn snapshot() -> NativeBridgeState {
    with_state(|bridge| bridge.clone())
}

pub fn replace(state: NativeBridgeState) {
    with_state(|bridge| *bridge = state);
}

/// Poll the next Android host action as JSON for Kotlin JNI.
pub fn poll_next_host_action_json() -> Option<String> {
    with_state(|bridge| drain_next_host_action(bridge).map(|action| action.to_json()))
}

/// Deliver a host callback JSON payload from Kotlin/desktop and return whether an event was produced.
pub fn deliver_host_callback_json(payload: &str) -> bool {
    with_state(|bridge| apply_host_callback_json(bridge, payload).is_some())
}

pub fn has_pending_commands() -> bool {
    with_state(|bridge| bridge.has_pending_commands())
}

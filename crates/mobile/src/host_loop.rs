//! Unified native host loop for desktop and Android.

#[cfg(not(target_os = "android"))]
use crate::android_host::drain_next_host_action;
#[cfg(not(target_os = "android"))]
use crate::desktop_native_host;
use crate::native_bridge::NativeBridgeState;
use crate::native_host_runtime;

#[cfg(target_os = "android")]
pub fn tick_android_from_jni() {
    native_host_runtime::with_state(|bridge| {
        while let Some(action_json) = native_host_runtime::poll_next_host_action_json() {
            // Kotlin activity drains and executes; JNI only serializes queue state.
            let _ = action_json;
            let _ = bridge;
        }
    });
}

/// Run one host tick: drain commands and execute platform handlers.
pub fn run_host_tick(bridge: &mut NativeBridgeState) -> Vec<String> {
    native_host_runtime::replace(bridge.clone());
    #[cfg(target_os = "android")]
    let notes = Vec::new();
    #[cfg(not(target_os = "android"))]
    let mut notes = Vec::new();

    #[cfg(not(target_os = "android"))]
    native_host_runtime::with_state(|global| {
        while let Some(action) = drain_next_host_action(global) {
            if let Some(note) = desktop_native_host::try_execute(&action, global) {
                notes.push(note);
            }
        }
    });

    *bridge = native_host_runtime::snapshot();
    notes
}

/// Sync UI signal from the global runtime after JNI/desktop callbacks.
pub fn sync_bridge_from_runtime(bridge: &mut NativeBridgeState) -> bool {
    let latest = native_host_runtime::snapshot();
    let changed = latest.last_event_id != bridge.last_event_id
        || latest.has_pending_commands() != bridge.has_pending_commands();
    if changed {
        *bridge = latest;
    }
    changed
}

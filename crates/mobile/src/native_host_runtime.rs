//! Process-wide native bridge state shared by Dioxus UI, desktop host loop and Android JNI.

use crate::android_host::{apply_host_callback_json, drain_next_host_action};
use crate::device_calibration;
use crate::native_bridge::{NativeBridgeState, NativeMobileEvent};
use std::sync::{Mutex, OnceLock, TryLockError};

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
    match runtime().try_lock() {
        Ok(mut guard) => {
            *guard = state;
        }
        Err(TryLockError::Poisoned(poisoned)) => {
            *poisoned.into_inner() = state;
        }
        Err(TryLockError::WouldBlock) => {
            // Re-entrant publish from methods such as NativeBridgeState::accept_event()
            // while the global runtime is already locked. In that case the caller is
            // mutating the locked state directly, so publishing again is unnecessary
            // and would deadlock on Android JNI callback delivery.
        }
    }
}

/// Poll the next Android host action as JSON for Kotlin JNI.
pub fn poll_next_host_action_json() -> Option<String> {
    with_state(|bridge| drain_next_host_action(bridge).map(|action| action.to_json()))
}

/// Deliver a host callback JSON payload from Kotlin/desktop and return whether an event was produced.
pub fn deliver_host_callback_json(payload: &str) -> bool {
    if payload.contains("termux") {
        let snippet: String = payload.chars().take(240).collect();
        device_calibration::trace_calibration("jni_callback", Some(&snippet));
    }
    let _ = device_calibration::try_note_calibration_from_host_json(payload);
    with_state(|bridge| {
        let Some(event) = apply_host_callback_json(bridge, payload) else {
            return false;
        };
        match &event {
            NativeMobileEvent::TermuxCommandCompleted(result)
                if device_calibration::is_calibration_request(&result.request_id) =>
            {
                device_calibration::note_calibration_result(
                    &result.stdout,
                    &result.stderr,
                    result.error.as_deref(),
                    result.exit_code,
                );
            }
            NativeMobileEvent::TermuxCommandFailed {
                request_id,
                message,
            } if device_calibration::is_calibration_request(request_id) => {
                device_calibration::note_calibration_result("", "", Some(message), Some(1));
            }
            _ => {}
        }
        true
    })
}

pub fn has_pending_commands() -> bool {
    with_state(|bridge| bridge.has_pending_commands())
}

#[cfg(test)]
mod tests {
    use super::deliver_host_callback_json;

    #[test]
    fn termux_callback_delivery_does_not_deadlock_while_runtime_is_locked() {
        let payload = r#"{"kind":"termux_completed","result":{"request_id":"termux-deadlock-test","stdout":"/data/data/com.termux/files/home\n","stderr":"","exit_code":0,"timed_out":false,"error":null}}"#;
        assert!(deliver_host_callback_json(payload));
    }
}

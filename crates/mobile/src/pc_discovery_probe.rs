//! Trigger PC Host mDNS discovery from ADB and write results to disk.

use crate::mobile_runtime_config::default_data_dir;
use crate::native_bridge::{NativeBridgeState, NativeMobileEvent};
use crate::native_host_runtime;
use std::fs;
use std::path::PathBuf;

const REQUEST_FLAG: &str = ".pc_discovery_probe_requested";
const RUNNING_FLAG: &str = ".pc_discovery_probe_running";
const RESULT_FILE: &str = ".pc_discovery_probe_result";
const PROBE_REQUEST_ID: &str = "deepseek-pc-discovery-probe-v1";
const PROBE_TIMEOUT_SECS: u64 = 30;

fn flag_path() -> PathBuf {
    default_data_dir().join(REQUEST_FLAG)
}

fn result_path() -> PathBuf {
    default_data_dir().join(RESULT_FILE)
}

fn running_path() -> PathBuf {
    default_data_dir().join(RUNNING_FLAG)
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn is_probe_requested() -> bool {
    flag_path().exists()
}

fn clear_request_flag() {
    let _ = fs::remove_file(flag_path());
}

fn clear_running_flag() {
    let _ = fs::remove_file(running_path());
}

fn is_running() -> bool {
    running_path().exists()
}

pub fn write_result(line: &str) {
    let path = result_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, format!("{line}\n"));
}

fn read_running() -> Option<(u64, u64)> {
    fs::read_to_string(running_path()).ok().map(|raw| {
        let mut parts = raw.trim().split(',');
        let unix = parts
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let event_id = parts
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        (unix, event_id)
    })
}

fn finalize_from_bridge(bridge: &NativeBridgeState, baseline_event_id: u64) -> bool {
    if bridge.last_event_id <= baseline_event_id {
        return false;
    }

    match bridge.last_event.clone() {
        Some(NativeMobileEvent::PcGatewayDiscoveryCompleted(report)) => {
            clear_running_flag();
            let count = report.candidates.len();
            if count > 0 {
                let first = &report.candidates[0].endpoint;
                write_result(&format!("PASS endpoints={count} first={}", first.base_url));
            } else {
                write_result("FAIL discovery completed with 0 candidates (same Wi-Fi? PC host mDNS on 8787?)");
            }
            true
        }
        Some(NativeMobileEvent::PcGatewayDiscoveryFailed(message)) => {
            clear_running_flag();
            write_result(&format!("FAIL {message}"));
            true
        }
        Some(NativeMobileEvent::PcGatewayDiscoveryUpdated(report))
            if !report.candidates.is_empty() =>
        {
            clear_running_flag();
            let count = report.candidates.len();
            let first = &report.candidates[0].endpoint;
            write_result(&format!("PASS endpoints={count} first={}", first.base_url));
            true
        }
        _ => false,
    }
}

fn reset_stale_pc_discovery_wait(bridge: &mut NativeBridgeState) {
    if bridge.active_pc_discovery_request_id.is_some() {
        bridge.active_pc_discovery_request_id = None;
        bridge.active_pc_discovery_since_unix = None;
        crate::native_host_runtime::replace(bridge.clone());
    }
}

fn tick_bridge(bridge: &mut NativeBridgeState) {
    if is_probe_requested() {
        clear_request_flag();
        reset_stale_pc_discovery_wait(bridge);
        let baseline_event_id = bridge.last_event_id;
        let _ = fs::write(
            running_path(),
            format!("{},{baseline_event_id}", unix_now_secs()),
        );
        bridge.enqueue_pc_gateway_discovery(PROBE_REQUEST_ID);
    }

    if !is_running() {
        return;
    }

    let Some((started_unix, baseline_event_id)) = read_running() else {
        clear_running_flag();
        return;
    };

    let now = unix_now_secs();
    if now.saturating_sub(started_unix) > PROBE_TIMEOUT_SECS {
        clear_running_flag();
        reset_stale_pc_discovery_wait(bridge);
        write_result("FAIL timeout (start deepseek-pc-host on 0.0.0.0:8787, same Wi-Fi)");
        return;
    }

    if finalize_from_bridge(bridge, baseline_event_id) {
        return;
    }

    if bridge.last_event_id <= baseline_event_id {
        return;
    }

    // Non-terminal discovery event (e.g. Started): keep waiting until timeout or completion.
}

/// Call each Android poll tick: start discovery once, finalize on callback or timeout.
pub fn tick() {
    native_host_runtime::with_state(|bridge| {
        tick_bridge(bridge);
    });
}

/// Drop stale `.pc_discovery_probe_running` markers so a new request can start cleanly.
pub fn recover_stale_running_marker() {
    let Some((started_unix, _)) = read_running() else {
        return;
    };
    if unix_now_secs().saturating_sub(started_unix) <= PROBE_TIMEOUT_SECS {
        return;
    }
    clear_running_flag();
    native_host_runtime::with_state(reset_stale_pc_discovery_wait);
}

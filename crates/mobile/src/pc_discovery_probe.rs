//! Trigger PC Host mDNS discovery from ADB and write results to disk.
//!
//! Optional fallback: `files/deepseek-mobile/.pc_discovery_manual_url` — one base URL per line
//! (e.g. `http://192.168.1.111:8787`) when mDNS is blocked (common with Windows PC hosts).

use crate::mobile_runtime_config::default_data_dir;
use crate::native_bridge::{NativeBridgeState, NativeMobileEvent};
use crate::native_host_runtime;
use deepseek_mobile_core::{
    PcGatewayDiscoveryReport, PcGatewayDiscoveryService, PcGatewayDiscoveryStatus,
};
use std::fs;
use std::path::PathBuf;

const REQUEST_FLAG: &str = ".pc_discovery_probe_requested";
const RUNNING_FLAG: &str = ".pc_discovery_probe_running";
const RESULT_FILE: &str = ".pc_discovery_probe_result";
const MANUAL_URL_FILE: &str = ".pc_discovery_manual_url";
const PROBE_REQUEST_ID: &str = "deepseek-pc-discovery-probe-v1";
const PROBE_TIMEOUT_SECS: u64 = 45;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeKickoff {
    None,
    Mdns,
    Manual(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeTickResult {
    Idle,
    Finished,
    ManualProbe(Vec<String>),
}

fn flag_path() -> PathBuf {
    default_data_dir().join(REQUEST_FLAG)
}

fn result_path() -> PathBuf {
    default_data_dir().join(RESULT_FILE)
}

fn running_path() -> PathBuf {
    default_data_dir().join(RUNNING_FLAG)
}

fn manual_url_path() -> PathBuf {
    default_data_dir().join(MANUAL_URL_FILE)
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

pub fn is_running() -> bool {
    running_path().exists()
}

fn clear_request_flag() {
    let _ = fs::remove_file(flag_path());
}

fn clear_running_flag() {
    let _ = fs::remove_file(running_path());
}

pub fn clear_manual_urls() {
    let _ = fs::remove_file(manual_url_path());
}

pub fn write_result(line: &str) {
    let path = result_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, format!("{line}\n"));
}

pub fn read_manual_urls() -> Vec<String> {
    fs::read_to_string(manual_url_path())
        .ok()
        .map(|raw| {
            raw.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn write_result_from_report(report: &PcGatewayDiscoveryReport, mode: &str) {
    let online = report
        .candidates
        .iter()
        .find(|candidate| candidate.status == PcGatewayDiscoveryStatus::Online);
    if let Some(candidate) = online {
        write_result(&format!(
            "PASS endpoints=1 first={} mode={mode}",
            candidate.endpoint.base_url
        ));
        return;
    }
    let count = report.candidates.len();
    if count > 0 {
        let first = &report.candidates[0].endpoint;
        write_result(&format!(
            "FAIL discovery completed with 0 online endpoints (found {count}, first={}) mode={mode}",
            first.base_url
        ));
    } else {
        write_result(&format!(
            "FAIL discovery completed with 0 candidates (same Wi-Fi? PC host on 0.0.0.0:8787?) mode={mode}"
        ));
    }
}

pub async fn probe_manual_urls(urls: &[String]) -> bool {
    if urls.is_empty() {
        return false;
    }
    let service = PcGatewayDiscoveryService::new(true);
    for url in urls {
        let report = service.from_manual_base_url(url, "manual-fallback");
        let probed = service.probe_candidates(report).await;
        if probed
            .candidates
            .iter()
            .any(|candidate| candidate.status == PcGatewayDiscoveryStatus::Online)
        {
            write_result_from_report(&probed, "manual");
            clear_manual_urls();
            clear_running_flag();
            return true;
        }
    }
    false
}

fn read_running() -> Option<(u64, u64, bool)> {
    fs::read_to_string(running_path()).ok().and_then(|raw| {
        let mut parts = raw.trim().split(',');
        let unix = parts.next()?.parse::<u64>().ok()?;
        let second = parts.next()?;
        if second == "manual" {
            Some((unix, 0, true))
        } else {
            let event_id = second.parse::<u64>().ok()?;
            Some((unix, event_id, false))
        }
    })
}

fn manual_after_empty_mdns(report: &PcGatewayDiscoveryReport) -> Option<Vec<String>> {
    let online = report
        .candidates
        .iter()
        .any(|candidate| candidate.status == PcGatewayDiscoveryStatus::Online);
    if online {
        return None;
    }
    let manual_urls = read_manual_urls();
    if manual_urls.is_empty() {
        return None;
    }
    let _ = fs::write(running_path(), format!("{},manual", unix_now_secs()));
    Some(manual_urls)
}

fn finalize_from_bridge(bridge: &NativeBridgeState, baseline_event_id: u64) -> ProbeTickResult {
    if bridge.last_event_id <= baseline_event_id {
        return ProbeTickResult::Idle;
    }

    match bridge.last_event.clone() {
        Some(NativeMobileEvent::PcGatewayDiscoveryCompleted(report)) => {
            if let Some(urls) = manual_after_empty_mdns(&report) {
                return ProbeTickResult::ManualProbe(urls);
            }
            clear_running_flag();
            clear_manual_urls();
            write_result_from_report(&report, "mdns");
            ProbeTickResult::Finished
        }
        Some(NativeMobileEvent::PcGatewayDiscoveryFailed(message)) => {
            let manual_urls = read_manual_urls();
            if !manual_urls.is_empty() {
                let _ = fs::write(running_path(), format!("{},manual", unix_now_secs()));
                return ProbeTickResult::ManualProbe(manual_urls);
            }
            clear_running_flag();
            write_result(&format!("FAIL {message} mode=mdns"));
            ProbeTickResult::Finished
        }
        Some(NativeMobileEvent::PcGatewayDiscoveryUpdated(report))
            if report
                .candidates
                .iter()
                .any(|candidate| candidate.status == PcGatewayDiscoveryStatus::Online) =>
        {
            clear_running_flag();
            clear_manual_urls();
            write_result_from_report(&report, "mdns");
            ProbeTickResult::Finished
        }
        Some(NativeMobileEvent::PcGatewayDiscoveryUpdated(report)) => manual_after_empty_mdns(&report)
            .map(ProbeTickResult::ManualProbe)
            .unwrap_or(ProbeTickResult::Idle),
        _ => ProbeTickResult::Idle,
    }
}

fn reset_stale_pc_discovery_wait(bridge: &mut NativeBridgeState) {
    if bridge.active_pc_discovery_request_id.is_some() {
        bridge.active_pc_discovery_request_id = None;
        bridge.active_pc_discovery_since_unix = None;
        crate::native_host_runtime::replace(bridge.clone());
    }
}

/// Consume `.pc_discovery_probe_requested` and start mDNS or queue manual HTTP probe.
pub fn kickoff_if_requested(bridge: &mut NativeBridgeState) -> ProbeKickoff {
    if !is_probe_requested() {
        return ProbeKickoff::None;
    }
    clear_request_flag();
    reset_stale_pc_discovery_wait(bridge);

    let manual_urls = read_manual_urls();
    if !manual_urls.is_empty() {
        let _ = fs::write(running_path(), format!("{},manual", unix_now_secs()));
        return ProbeKickoff::Manual(manual_urls);
    }

    let baseline_event_id = bridge.last_event_id;
    let _ = fs::write(
        running_path(),
        format!("{},{baseline_event_id}", unix_now_secs()),
    );
    bridge.enqueue_pc_gateway_discovery(PROBE_REQUEST_ID);
    ProbeKickoff::Mdns
}

fn tick_mdns(bridge: &mut NativeBridgeState) -> Option<Vec<String>> {
    let Some((started_unix, baseline_event_id, is_manual)) = read_running() else {
        return None;
    };
    if is_manual {
        return None;
    }

    let now = unix_now_secs();
    if now.saturating_sub(started_unix) > PROBE_TIMEOUT_SECS {
        clear_running_flag();
        reset_stale_pc_discovery_wait(bridge);
        let manual_urls = read_manual_urls();
        if manual_urls.is_empty() {
            write_result(
                "FAIL timeout (start deepseek-pc-host on 0.0.0.0:8787, same Wi-Fi, or set .pc_discovery_manual_url)",
            );
            return None;
        }
        let _ = fs::write(running_path(), format!("{},manual", unix_now_secs()));
        return Some(manual_urls);
    }

    match finalize_from_bridge(bridge, baseline_event_id) {
        ProbeTickResult::ManualProbe(urls) => return Some(urls),
        ProbeTickResult::Finished => return None,
        ProbeTickResult::Idle => {}
    }

    None
}

/// Call each Android poll tick after syncing the bridge from JNI.
pub fn tick(bridge: &mut NativeBridgeState) -> Option<Vec<String>> {
    if let Some((_, _, true)) = read_running() {
        // Manual HTTP probe already queued; async task owns completion.
        return None;
    }
    match kickoff_if_requested(bridge) {
        ProbeKickoff::Manual(urls) => return Some(urls),
        ProbeKickoff::Mdns | ProbeKickoff::None => {}
    }
    tick_mdns(bridge)
}

/// Drop stale `.pc_discovery_probe_running` markers so a new request can start cleanly.
pub fn recover_stale_running_marker() {
    let Some((started_unix, _, _)) = read_running() else {
        return;
    };
    if unix_now_secs().saturating_sub(started_unix) <= PROBE_TIMEOUT_SECS {
        return;
    }
    clear_running_flag();
    native_host_runtime::with_state(reset_stale_pc_discovery_wait);
}

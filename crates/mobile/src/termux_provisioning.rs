//! First-run Termux setup: install intent, open app, permission probe, workspace seed.

use crate::device_calibration;
use crate::mobile_runtime_config::default_data_dir;
use crate::native_bridge::{NativeBridgeState, NativeMobileCommand};
use crate::termux_state::TermuxWorkspaceState;
use std::fs;
use std::path::PathBuf;

const ONBOARDING_PROVISION_FLAG: &str = ".termux_onboarding_provision_v1";
pub const TERMUX_PACKAGE: &str = "com.termux";
pub const TERMUX_FDROID_URL: &str = "https://f-droid.org/packages/com.termux/";

pub fn request_onboarding_provision() {
    let path = onboarding_flag_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, "requested\n");
}

pub fn clear_onboarding_provision() {
    let _ = fs::remove_file(onboarding_flag_path());
}

pub fn is_onboarding_provision_pending() -> bool {
    onboarding_flag_path().exists() && !device_calibration::is_calibrated()
}

fn onboarding_flag_path() -> PathBuf {
    default_data_dir().join(ONBOARDING_PROVISION_FLAG)
}

/// Opens F-Droid Termux page (user taps Install once). LaunchApp alone fails if APK missing.
pub fn enqueue_install_termux(bridge: &mut NativeBridgeState) {
    bridge.enqueue(NativeMobileCommand::OpenUrl {
        url: TERMUX_FDROID_URL.to_string(),
    });
}

pub fn enqueue_open_termux(bridge: &mut NativeBridgeState) {
    bridge.enqueue(NativeMobileCommand::LaunchApp {
        package: TERMUX_PACKAGE.to_string(),
    });
}

pub fn enqueue_fdroid_termux(bridge: &mut NativeBridgeState) {
    bridge.enqueue(NativeMobileCommand::OpenUrl {
        url: TERMUX_FDROID_URL.to_string(),
    });
}

/// Queue a minimal RUN_COMMAND so Android/Termux can show the permission dialog.
pub fn enqueue_run_command_permission_probe(
    bridge: &mut NativeBridgeState,
    termux: &TermuxWorkspaceState,
) -> bool {
    if !termux.is_valid() || !termux.saved {
        return false;
    }
    if bridge.has_pending_commands() || bridge.is_waiting_for_termux_callback() {
        return false;
    }
    let workdir = "/data/data/com.termux/files/home".to_string();
    bridge.enqueue_termux_command(deepseek_mobile_core::TermuxExecRequest {
        request_id: device_calibration::HEALTH_TERMUX_PROBE_ID.to_string(),
        command: "pwd && echo DEEPSEEK_TERMUX_PROBE_OK".to_string(),
        working_dir: PathBuf::from(workdir),
        timeout_secs: Some(60),
    });
    true
}

/// After setup saves a Termux path, request workspace seeding on the next Android poll tick.
pub fn on_setup_saved_termux_path() {
    request_onboarding_provision();
}

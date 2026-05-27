//! One-shot Android calibration: seed Termux project + verify shell without user typing.

use crate::mobile_runtime_config::default_data_dir;
use crate::native_bridge::NativeBridgeState;
use crate::settings_state::SettingsFormState;
use crate::termux_state::TermuxWorkspaceState;
use std::fs;
use std::path::PathBuf;

#[cfg(target_os = "android")]
use deepseek_mobile_core::TermuxExecRequest;

pub const CALIBRATION_REQUEST_ID: &str = "deepseek-device-calibration-v1";
pub const HEALTH_TERMUX_PROBE_ID: &str = "deepseek-health-termux-probe-v1";
const CALIBRATION_REQUEST_FLAG: &str = ".agent_calibration_requested_v1";

fn calibration_flag_path() -> PathBuf {
    default_data_dir().join(".agent_calibrated_v1")
}

fn calibration_request_flag_path() -> PathBuf {
    default_data_dir().join(CALIBRATION_REQUEST_FLAG)
}

pub fn is_calibrated() -> bool {
    calibration_flag_path().exists()
}

pub fn is_calibration_requested() -> bool {
    calibration_request_flag_path().exists()
}

pub fn clear_calibration_request() {
    let _ = fs::remove_file(calibration_request_flag_path());
}

/// Seconds since the last `queued` line in `.calibration_trace`, if any.
pub fn seconds_since_last_queue() -> Option<u64> {
    let path = default_data_dir().join(".calibration_trace");
    let text = fs::read_to_string(path).ok()?;
    let last_queued = text
        .lines()
        .rev()
        .find(|line| line.contains("stage=queued"))?;
    let unix = last_queued.split_whitespace().next()?.parse::<u64>().ok()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(now.saturating_sub(unix))
}

pub fn should_retry_calibration() -> bool {
    if is_calibrated() {
        return false;
    }
    matches!(seconds_since_last_queue(), Some(age) if age >= 90)
}

pub fn is_calibration_request(request_id: &str) -> bool {
    request_id == CALIBRATION_REQUEST_ID
}

pub fn is_health_probe_request(request_id: &str) -> bool {
    request_id == HEALTH_TERMUX_PROBE_ID
}

/// Background Termux keep-alive / health probe (no UI).
#[cfg(target_os = "android")]
pub fn schedule_termux_health_probe(
    bridge: &mut NativeBridgeState,
    termux: &TermuxWorkspaceState,
) -> bool {
    if !termux.is_valid() || !termux.saved || bridge.has_pending_commands() {
        return false;
    }
    if bridge.is_waiting_for_termux_callback() {
        return false;
    }
    let workdir = "/data/data/com.termux/files/home".to_string();
    bridge.enqueue_termux_command(TermuxExecRequest {
        request_id: HEALTH_TERMUX_PROBE_ID.to_string(),
        command: "pwd && echo DEEPSEEK_TERMUX_PROBE_OK".to_string(),
        working_dir: PathBuf::from(&workdir),
        timeout_secs: Some(60),
    });
    true
}

#[cfg(not(target_os = "android"))]
pub fn schedule_termux_health_probe(
    _bridge: &mut NativeBridgeState,
    _termux: &TermuxWorkspaceState,
) -> bool {
    false
}

pub fn mark_calibrated() {
    let path = calibration_flag_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &path,
        format!(
            "ok\nworkspace_seeded_at_unix={}\n",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        ),
    );
}

/// Returns true if a Termux calibration command was queued.
#[cfg(target_os = "android")]
pub fn schedule_android_calibration(
    bridge: &mut NativeBridgeState,
    termux: &TermuxWorkspaceState,
    _settings: &SettingsFormState,
) -> bool {
    if is_calibrated() {
        return false;
    }
    // Hardware-test scripts set `.agent_calibration_requested_v1`.
    // First-login onboarding sets `.termux_onboarding_provision_v1`.
    if !is_calibration_requested() && !crate::termux_provisioning::is_onboarding_provision_pending()
    {
        return false;
    }
    if !termux.is_valid() || !termux.saved {
        return false;
    }
    if bridge.has_pending_commands() {
        return false;
    }
    if bridge.is_waiting_for_termux_callback() {
        let only_stale_calibration = bridge.active_termux_request_ids.len() == 1
            && bridge
                .active_termux_request_ids
                .first()
                .map(|id| id == CALIBRATION_REQUEST_ID)
                .unwrap_or(false);
        if only_stale_calibration {
            bridge.active_termux_request_ids.clear();
        } else {
            return false;
        }
    }

    // Termux RUN_COMMAND requires an existing workdir; project dir is created by the script.
    let workdir = "/data/data/com.termux/files/home".to_string();
    let command = r#"set -e
mkdir -p "$HOME/deepseek-project"
cd "$HOME/deepseek-project"
if [ ! -f README.md ]; then
  cat > README.md <<'EOF'
# deepseek-project

Рабочая область агента DeepSeek Mobile (Termux).
Сюда можно клонировать репозиторий или распаковать ZIP из приложения.
EOF
fi
mkdir -p .deepseek
pwd
ls -la
echo DEEPSEEK_CALIBRATION_OK
"#
    .to_string();

    bridge.enqueue_termux_command(TermuxExecRequest {
        request_id: CALIBRATION_REQUEST_ID.to_string(),
        command,
        working_dir: PathBuf::from(&workdir),
        timeout_secs: Some(90),
    });
    trace_calibration("queued", None);
    true
}

pub fn trace_calibration(stage: &str, detail: Option<&str>) {
    let path = default_data_dir().join(".calibration_trace");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = format!(
        "{} stage={} detail={}\n",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        stage,
        detail.unwrap_or("")
    );
    use std::io::Write;
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(line.as_bytes()));
}

#[cfg(not(target_os = "android"))]
pub fn schedule_android_calibration(
    _bridge: &mut NativeBridgeState,
    _termux: &TermuxWorkspaceState,
    _settings: &SettingsFormState,
) -> bool {
    false
}

pub fn needs_allow_external_apps() -> bool {
    let path = default_data_dir().join(".calibration_trace");
    let Ok(text) = fs::read_to_string(path) else {
        return false;
    };
    text.contains("allow-external-apps")
}

/// Parse a Kotlin `termux_completed` JSON payload and update calibration state.
/// Runs before bridge correlation so a duplicate/stale callback cannot drop a good result.
pub fn try_note_calibration_from_host_json(payload: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return false;
    };
    if value.get("kind").and_then(|v| v.as_str()) != Some("termux_completed") {
        return false;
    }
    let Some(result) = value.get("result") else {
        return false;
    };
    if result.get("request_id").and_then(|v| v.as_str()) != Some(CALIBRATION_REQUEST_ID) {
        return false;
    }
    let stdout = result
        .get("stdout")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let stderr = result
        .get("stderr")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let error = result.get("error").and_then(|v| v.as_str());
    let exit_code = result
        .get("exit_code")
        .and_then(|v| v.as_i64())
        .map(|code| code as i32);
    note_calibration_result(stdout, stderr, error, exit_code)
}

pub fn note_calibration_result(
    stdout: &str,
    stderr: &str,
    error: Option<&str>,
    exit_code: Option<i32>,
) -> bool {
    let combined = format!("{stdout}\n{stderr}\n{}", error.unwrap_or_default());
    let project_seeded = stdout.contains("deepseek-project")
        && (stdout.contains("DEEPSEEK_CALIBRATION_OK")
            || stdout.contains("total ")
            || stdout.contains("README"));
    trace_calibration(
        "result",
        Some(&format!(
            "exit={:?} ok_marker={} project_seeded={}",
            exit_code,
            combined.contains("DEEPSEEK_CALIBRATION_OK"),
            project_seeded
        )),
    );
    if exit_code == Some(0)
        || combined.contains("DEEPSEEK_CALIBRATION_OK")
        || (project_seeded && !combined.contains("allow-external-apps"))
    {
        mark_calibrated();
        clear_calibration_request();
        crate::termux_provisioning::clear_onboarding_provision();
        trace_calibration("marked_ok", None);
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_calibration_request_id() {
        assert!(is_calibration_request(CALIBRATION_REQUEST_ID));
        assert!(!is_calibration_request("other"));
    }
}

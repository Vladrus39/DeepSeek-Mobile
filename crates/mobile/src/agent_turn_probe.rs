//! Headless one-shot agent turn for ADB E2E (same stack as chat Send).

use crate::mobile_engine_runner::run_mobile_turn_with_runtime_and_observer;
use crate::mobile_runtime_config::default_data_dir;
use crate::native_bridge::NativeMobileEvent;
use crate::settings_state::load_config_for_agent_turn;
use deepseek_mobile_core::config::{ExecutionMode, ModelMode, ThinkingLevel};
use deepseek_mobile_core::format_http_transport_error;
use deepseek_mobile_core::{AgentEvent, TermuxExecResult, UserChatInput};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const REQUEST_FLAG: &str = ".agent_turn_probe_requested";
const MESSAGE_FILE: &str = ".agent_turn_probe_message";
const YOLO_FLAG: &str = ".agent_turn_probe_yolo";
const TERMUX_PWD_FLAG: &str = ".agent_turn_probe_termux_pwd";
const RESULT_FILE: &str = ".agent_turn_probe_result";
const TRACE_FILE: &str = ".agent_turn_probe_trace";

const TERMUX_PWD_PROBE_MESSAGE: &str =
    "In the Termux project workspace, use the real shell. Your first response must be exactly \
    this JSON object and no prose: {\"tool\":\"exec_shell\",\"args\":{\"command\":\"pwd\",\"timeout_secs\":30}}. \
    After the tool result is returned, reply with one line: the printed working directory path only.";

fn flag_path() -> PathBuf {
    default_data_dir().join(REQUEST_FLAG)
}

fn message_path() -> PathBuf {
    default_data_dir().join(MESSAGE_FILE)
}

fn result_path() -> PathBuf {
    default_data_dir().join(RESULT_FILE)
}

fn trace_path() -> PathBuf {
    default_data_dir().join(TRACE_FILE)
}

pub fn is_probe_requested() -> bool {
    flag_path().exists()
}

pub fn clear_probe_request() {
    let _ = fs::remove_file(flag_path());
    let _ = fs::remove_file(message_path());
    let _ = fs::remove_file(default_data_dir().join(YOLO_FLAG));
    let _ = fs::remove_file(default_data_dir().join(TERMUX_PWD_FLAG));
}

fn write_result(line: &str) {
    let path = result_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, format!("{line}\n"));
}

fn append_trace(line: &str) {
    let path = trace_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{line}");
    }
}

async fn wait_for_termux_result(
    request_id: &str,
    timeout: Duration,
) -> Result<TermuxExecResult, String> {
    let start = Instant::now();
    loop {
        let bridge = crate::native_host_runtime::snapshot();
        match bridge.last_event {
            Some(NativeMobileEvent::TermuxCommandCompleted(result))
                if result.request_id == request_id =>
            {
                return Ok(result);
            }
            Some(NativeMobileEvent::TermuxCommandFailed {
                request_id: failed_id,
                message,
            }) if failed_id == request_id => {
                return Err(message);
            }
            _ => {}
        }

        if start.elapsed() >= timeout {
            return Err(format!("timeout waiting for Termux request {request_id}"));
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

fn probe_message(termux_pwd: bool) -> String {
    if termux_pwd {
        return TERMUX_PWD_PROBE_MESSAGE.to_string();
    }
    fs::read_to_string(message_path())
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Reply with exactly: PROBE_OK".to_string())
}

fn unique_probe_thread_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("__deepseek_adb_probe__{millis}")
}

/// Runs one agent turn when [REQUEST_FLAG] exists.
pub async fn run_if_requested() {
    if !is_probe_requested() {
        return;
    }
    let termux_pwd = default_data_dir().join(TERMUX_PWD_FLAG).exists();
    let yolo = default_data_dir().join(YOLO_FLAG).exists();
    let input_message = probe_message(termux_pwd);
    let _ = fs::remove_file(trace_path());
    append_trace(&format!(
        "START termux_pwd={termux_pwd} yolo={yolo} message={:?}",
        input_message
    ));
    clear_probe_request();

    let mut config = load_config_for_agent_turn();
    if yolo {
        config.execution_mode = ExecutionMode::Yolo;
    }
    if termux_pwd {
        // Keep the device E2E deterministic: the probe validates the tool loop
        // and Android/Termux bridge, not Auto/Pro routing latency.
        config.auto_mode = false;
        config.model_mode = ModelMode::Flash;
        config.model = "deepseek-v4-flash".to_string();
        config.thinking_level = ThinkingLevel::Off;
    }
    append_trace(&format!(
        "CONFIG model={} mode={:?} exec={:?} thinking={:?}",
        config.model, config.model_mode, config.execution_mode, config.thinking_level
    ));
    if !config.api_key.trim().starts_with("sk-") {
        append_trace("FAIL api_key missing");
        write_result("FAIL api_key missing");
        return;
    }

    let write_file_probe = input_message.contains("write_file");
    let termux_file_tool_probe = write_file_probe || input_message.contains("delete_file");
    let requires_termux_tool = termux_pwd
        || input_message.contains("exec_shell")
        || termux_file_tool_probe
        || input_message.contains("DSM_PROJECT_PROBE");
    let input = UserChatInput::new(input_message);
    // Never write probe turns into the user's active chat thread, and never reuse
    // an old probe thread: otherwise a previous queued tool result can contaminate
    // the next E2E run and make the model answer from history instead of calling
    // the real tool again.
    let probe_thread_id = unique_probe_thread_id();
    append_trace(&format!("THREAD {probe_thread_id}"));
    let runtime = crate::mobile_runtime_config::MobileRuntimeConfig::default_mobile()
        .with_thread_id(probe_thread_id);
    let runtime_for_continue = runtime.clone();
    let config_for_continue = config.clone();
    let saw_text = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let saw_termux_tool = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let pending_termux_request_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let saw_flag = saw_text.clone();
    let termux_tool_flag = saw_termux_tool.clone();
    let pending_id_flag = pending_termux_request_id.clone();

    match run_mobile_turn_with_runtime_and_observer(config, input, runtime, move |event| {
        append_trace(&format!("EVENT {event:?}"));
        if let AgentEvent::TextDelta(text) = &event {
            if text.contains("PROBE_OK")
                || text.contains("HELLO_E2E")
                || text.len() > 2
            {
                saw_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        if let AgentEvent::ToolCallFinished(result) = &event {
            if let Some(request) = crate::native_bridge::termux_request_from_agent_event(&event) {
                termux_tool_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                append_trace(&format!(
                    "ENQUEUE_TERMUX tool={} request_id={} command={:?} cwd={}",
                    result.name,
                    request.request_id,
                    request.command,
                    request.working_dir.display()
                ));
                let mut bridge = crate::native_host_runtime::snapshot();
                if bridge.enqueue_termux_command_from_agent_event(&event) {
                    if let Ok(mut guard) = pending_id_flag.lock() {
                        *guard = Some(request.request_id);
                    }
                }
            }
        }
        if let AgentEvent::TermuxExecutionPending { request, .. } = &event {
            if let Ok(mut guard) = pending_id_flag.lock() {
                *guard = Some(request.request_id.clone());
            }
        }
    })
    .await
    {
        Ok(result) => {
            append_trace(&format!(
                "RESULT approval_cards={} final={:?}",
                result.approval_card_count, result.final_text
            ));
            let final_ok = result
                .final_text
                .as_deref()
                .map(|t| {
                    t.contains("PROBE_OK")
                        || t.contains("HELLO_E2E_OK")
                        || t.contains("HELLO_E2E")
                        || t.len() > 2
                })
                .unwrap_or(false);
            let termux_tool_ok = saw_termux_tool.load(std::sync::atomic::Ordering::Relaxed);
            let saw = saw_text.load(std::sync::atomic::Ordering::Relaxed);
            if result.approval_card_count > 0 {
                write_result(&format!(
                    "PARTIAL approvals={} final={:?}",
                    result.approval_card_count, result.final_text
                ));
            } else if requires_termux_tool {
                let request_id = pending_termux_request_id
                    .lock()
                    .ok()
                    .and_then(|guard| guard.clone());
                if !termux_tool_ok {
                    write_result(&format!(
                        "FAIL termux_tool_not_observed termux_tool=false final={:?}",
                        result.final_text,
                    ));
                } else if let Some(request_id) = request_id {
                    append_trace(&format!("WAIT_TERMUX request_id={request_id}"));
                    match wait_for_termux_result(&request_id, Duration::from_secs(90)).await {
                        Ok(termux_result) => {
                            append_trace(&format!(
                                "TERMUX_RESULT request_id={} exit={:?} stdout={:?} stderr={:?} error={:?}",
                                termux_result.request_id,
                                termux_result.exit_code,
                                termux_result.stdout,
                                termux_result.stderr,
                                termux_result.error
                            ));
                            if termux_file_tool_probe {
                                if termux_result.exit_code == Some(0) {
                                    write_result(&format!(
                                        "PASS termux_write_file exit=0 final={:?}",
                                        result.final_text
                                    ));
                                } else {
                                    write_result(&format!(
                                        "FAIL termux_write_file exit={:?} stderr={:?} final={:?}",
                                        termux_result.exit_code,
                                        termux_result.stderr.trim(),
                                        result.final_text
                                    ));
                                }
                            } else {
                                let real_stdout_ok = termux_result.exit_code == Some(0)
                                    && (termux_result.stdout.contains("/data/")
                                        || termux_result.stdout.contains("/home/")
                                        || termux_result.stdout.contains("HELLO_E2E"));
                                match crate::mobile_engine_runner::continue_mobile_termux_result_with_runtime(
                                    config_for_continue,
                                    termux_result.clone(),
                                    runtime_for_continue,
                                )
                                .await
                                {
                                    Ok(continued) if real_stdout_ok => {
                                        write_result(&format!(
                                            "PASS termux_real exit={:?} stdout={:?} final={:?}",
                                            termux_result.exit_code,
                                            termux_result.stdout.trim(),
                                            continued.final_text
                                        ));
                                    }
                                    Ok(continued) => {
                                        write_result(&format!(
                                            "FAIL termux_output_unexpected exit={:?} stdout={:?} final={:?}",
                                            termux_result.exit_code,
                                            termux_result.stdout.trim(),
                                            continued.final_text
                                        ));
                                    }
                                    Err(error) => {
                                        write_result(&format!(
                                            "FAIL termux_continuation_failed stdout={:?} error={}",
                                            termux_result.stdout.trim(),
                                            format_http_transport_error(&error)
                                        ));
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            write_result(&format!(
                                "FAIL termux_native_failed request_id={} error={}",
                                request_id, error
                            ));
                        }
                    }
                } else {
                    write_result(&format!(
                        "FAIL termux_request_not_queued termux_tool=true final={:?}",
                        result.final_text,
                    ));
                }
            } else if final_ok || saw {
                write_result(&format!("PASS final={:?}", result.final_text));
            } else {
                write_result(&format!(
                    "PARTIAL empty_reply final={:?}",
                    result.final_text
                ));
            }
        }
        Err(error) => {
            append_trace(&format!("ERROR {error:?}"));
            write_result(&format!("FAIL {}", format_http_transport_error(&error)));
        }
    }
}

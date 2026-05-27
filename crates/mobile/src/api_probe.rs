//! Headless DeepSeek API connectivity probe (ADB / E2E scripts).

use crate::mobile_runtime_config::default_data_dir;
use crate::settings_state::load_saved_config;
use deepseek_mobile_core::format_http_transport_error;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

const REQUEST_FLAG: &str = ".api_probe_requested";
const RESULT_FILE: &str = ".api_probe_result";

fn flag_path() -> PathBuf {
    default_data_dir().join(REQUEST_FLAG)
}

fn result_path() -> PathBuf {
    default_data_dir().join(RESULT_FILE)
}

pub fn is_probe_requested() -> bool {
    flag_path().exists()
}

pub fn clear_probe_request() {
    let _ = fs::remove_file(flag_path());
}

fn write_result(line: &str) {
    let path = result_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&path, format!("{line}\n"));
}

/// Runs once when [REQUEST_FLAG] exists. Writes [RESULT_FILE] with PASS/FAIL details.
pub async fn run_if_requested() {
    if !is_probe_requested() {
        return;
    }
    clear_probe_request();

    let Some(config) = load_saved_config() else {
        write_result("FAIL no config loaded");
        return;
    };
    let key = config.api_key.trim();
    if !key.starts_with("sk-") {
        write_result("FAIL api_key missing or invalid prefix");
        return;
    }

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(30))
        .user_agent("DeepSeek-Mobile/0.1-probe")
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            write_result(&format!("FAIL client_build {error}"));
            return;
        }
    };

    match client
        .get("https://api.deepseek.com/v1/models")
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let snippet: String = body.chars().take(120).collect();
            if status.is_success() {
                write_result(&format!("PASS http={status} body={snippet}"));
            } else if status.as_u16() == 401 {
                write_result("FAIL http=401 unauthorized (bad API key)");
            } else {
                write_result(&format!("FAIL http={status} body={snippet}"));
            }
        }
        Err(error) => {
            let wrapped = anyhow::Error::from(error);
            write_result(&format!("FAIL {}", format_http_transport_error(&wrapped)));
        }
    }
}

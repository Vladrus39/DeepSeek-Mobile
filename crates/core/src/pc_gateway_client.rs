//! Async client for the PC companion gateway.
//!
//! This is the mobile-side transport layer. The gateway protocol lives in
//! `pc_gateway.rs`; this client sends typed envelopes to a trusted PC runtime
//! host and returns typed responses back to the mobile engine/UI.

use crate::executor::CommandRequest;
use crate::pc_gateway::{
    PcGatewayConfig, PcGatewayEndpointCandidate, PcGatewayError, PcGatewayHealth,
    PcGatewayRequest, PcGatewayRequestEnvelope, PcGatewayResponse, PcGatewayResponseEnvelope,
    CommandStreamEvent,
    PcGatewaySecurityPolicy,
};
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use futures_util::StreamExt;
use tokio::sync::mpsc;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayEndpointHealth {
    pub label: String,
    pub base_url: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_latency_ms: Option<u128>,
    pub last_error: Option<String>,
}

impl PcGatewayEndpointHealth {
    pub fn score(&self) -> i64 {
        let latency_penalty = self
            .last_latency_ms
            .map(|latency| (latency / 100).min(100) as i64)
            .unwrap_or(0);
        (self.success_count as i64 * 20) - (self.failure_count as i64 * 30) - latency_penalty
    }

    pub fn is_healthy(&self) -> bool {
        self.success_count > 0 && self.failure_count <= self.success_count + 2
    }
}

#[derive(Clone)]
pub struct PcGatewayClient {
    http: Client,
    config: PcGatewayConfig,
    policy: PcGatewaySecurityPolicy,
    endpoint_health: Arc<Mutex<BTreeMap<String, PcGatewayEndpointHealth>>>,
}

impl PcGatewayClient {
    pub fn new(config: PcGatewayConfig) -> Self {
        Self {
            http: Client::new(),
            config,
            policy: PcGatewaySecurityPolicy::default(),
            endpoint_health: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn with_policy(mut self, policy: PcGatewaySecurityPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn config(&self) -> &PcGatewayConfig {
        &self.config
    }

    pub fn policy(&self) -> &PcGatewaySecurityPolicy {
        &self.policy
    }

    pub fn endpoint_plan(&self) -> Vec<PcGatewayEndpointCandidate> {
        let mut plan = self.config.endpoint_plan();
        let health = self.endpoint_health_snapshot_by_url();
        plan.sort_by(|left, right| {
            let left_health = health.get(&left.base_url).map(PcGatewayEndpointHealth::score).unwrap_or(0);
            let right_health = health.get(&right.base_url).map(PcGatewayEndpointHealth::score).unwrap_or(0);
            let left_score = left.priority as i64 + left_health;
            let right_score = right.priority as i64 + right_health;
            right_score
                .cmp(&left_score)
                .then(right.priority.cmp(&left.priority))
                .then(left.label.cmp(&right.label))
                .then(left.base_url.cmp(&right.base_url))
        });
        plan
    }

    pub fn endpoint_health_snapshot(&self) -> Vec<PcGatewayEndpointHealth> {
        let mut values: Vec<_> = self
            .endpoint_health_snapshot_by_url()
            .into_values()
            .collect();
        values.sort_by(|left, right| {
            right
                .score()
                .cmp(&left.score())
                .then(left.label.cmp(&right.label))
                .then(left.base_url.cmp(&right.base_url))
        });
        values
    }

    pub fn active_endpoint(&self) -> Option<PcGatewayEndpointHealth> {
        self.endpoint_health_snapshot()
            .into_iter()
            .find(PcGatewayEndpointHealth::is_healthy)
    }

    pub async fn health(&self) -> Result<PcGatewayHealth> {
        match self.send(PcGatewayRequest::Health).await? {
            PcGatewayResponse::Health(health) => Ok(health),
            other => Err(anyhow!("unexpected gateway health response: {:?}", other)),
        }
    }

    pub async fn list_workspaces(&self) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::ListWorkspaces).await
    }

    pub async fn read_file(&self, workspace_id: impl Into<String>, path: impl Into<String>) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::ReadFile {
            workspace_id: workspace_id.into(),
            path: path.into(),
        })
        .await
    }

    pub async fn write_file(
        &self,
        workspace_id: impl Into<String>,
        path: impl Into<String>,
        content: impl Into<String>,
    ) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::WriteFile {
            workspace_id: workspace_id.into(),
            path: path.into(),
            content: content.into(),
        })
        .await
    }

    pub async fn delete_file(&self, workspace_id: impl Into<String>, path: impl Into<String>) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::DeleteFile {
            workspace_id: workspace_id.into(),
            path: path.into(),
        })
        .await
    }

    pub async fn list_dir(&self, workspace_id: impl Into<String>, path: impl Into<String>) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::ListDir {
            workspace_id: workspace_id.into(),
            path: path.into(),
        })
        .await
    }

    pub async fn execute_command(
        &self,
        workspace_id: impl Into<String>,
        command: CommandRequest,
        environment_id: Option<String>,
    ) -> Result<PcGatewayResponse> {
        self.policy
            .validate_command(&command)
            .map_err(|message| anyhow!(message))?;

        self.send(PcGatewayRequest::ExecuteCommand {
            workspace_id: workspace_id.into(),
            command,
            environment_id,
        })
        .await
    }


    /// Execute a command with streaming output via SSE.
    /// Returns an mpsc receiver that produces CommandStreamEvent items.
    pub async fn stream_command(
        &self,
        workspace_id: impl Into<String>,
        command: CommandRequest,
    ) -> Result<mpsc::Receiver<CommandStreamEvent>> {
        self.policy.validate_command(&command).map_err(|m| anyhow!(m))?;
        let workspace_id = workspace_id.into();
        let (tx, rx) = mpsc::channel::<CommandStreamEvent>(64);
        let http = self.http.clone();
        let endpoints = self.endpoint_plan();
        let auth_token = self.config.auth_token.clone();
        let config = self.config.clone();
        let tx_err = tx.clone();
        tokio::spawn(async move {
            let mut last_error = String::new();
            for endpoint in &endpoints {
                if let Err(e) = endpoint.validate(config.allow_http_on_local_network) {
                    last_error = e;
                    continue;
                }
                let url = format!("{}/v1/gateway/exec/stream", endpoint.base_url.trim_end_matches('/'));
                let mut builder = http.post(&url).json(&serde_json::json!({"workspace_id": workspace_id, "command": command}));
                if let Some(ref token) = auth_token {
                    builder = builder.bearer_auth(token);
                }
                match builder.send().await {
                    Ok(response) => {
                        if !response.status().is_success() {
                            let status = response.status();
                            let text = response.text().await.unwrap_or_default();
                            let _ = tx_err.send(CommandStreamEvent::Error(format!("HTTP {}: {}", status, text))).await;
                            continue;
                        }
                        let mut bytes_stream = response.bytes_stream();
                        let mut buf = String::new();
                        while let Some(Ok(chunk)) = bytes_stream.next().await {
                            if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                                buf.push_str(&text);
                                while let Some(pos) = buf.find("\n\n") {
                                    let raw = buf[..pos].to_string();
                                    buf = buf[pos + 2..].to_string();
                                    if let Some(ev) = parse_sse_event(&raw) {
                                        if tx.send(ev).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                            }
                        }
                        return;
                    }
                    Err(e) => {
                        last_error = e.to_string();
                        continue;
                    }
                }
            }
            let _ = tx_err.send(CommandStreamEvent::Error(format!("all endpoints failed: {}", last_error))).await;
        });
        Ok(rx)
    }
    pub async fn open_terminal(
        &self,
        workspace_id: impl Into<String>,
        cwd: Option<String>,
        environment_id: Option<String>,
    ) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::OpenTerminal {
            workspace_id: workspace_id.into(),
            cwd,
            environment_id,
        })
        .await
    }

    pub async fn terminal_input(&self, session_id: impl Into<String>, input: impl Into<String>) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::TerminalInput {
            session_id: session_id.into(),
            input: input.into(),
        })
        .await
    }

    pub async fn detect_tasks(&self, workspace_id: impl Into<String>) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::DetectTasks {
            workspace_id: workspace_id.into(),
        })
        .await
    }

    pub async fn run_task(&self, task_id: impl Into<String>) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::RunTask {
            task_id: task_id.into(),
        })
        .await
    }

    pub async fn start_dev_server(
        &self,
        workspace_id: impl Into<String>,
        command: CommandRequest,
        environment_id: Option<String>,
    ) -> Result<PcGatewayResponse> {
        self.policy
            .validate_command(&command)
            .map_err(|message| anyhow!(message))?;

        self.send(PcGatewayRequest::StartDevServer {
            workspace_id: workspace_id.into(),
            command,
            environment_id,
        })
        .await
    }

    pub async fn get_diagnostics(
        &self,
        workspace_id: impl Into<String>,
        path: Option<String>,
    ) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::GetDiagnostics {
            workspace_id: workspace_id.into(),
            path,
        })
        .await
    }

    pub async fn git_status(&self, workspace_id: impl Into<String>) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::GitStatus {
            workspace_id: workspace_id.into(),
        })
        .await
    }

    pub async fn git_diff(&self, workspace_id: impl Into<String>) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::GitDiff {
            workspace_id: workspace_id.into(),
        })
        .await
    }

    pub async fn git_commit(
        &self,
        workspace_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::GitCommit {
            workspace_id: workspace_id.into(),
            message: message.into(),
        })
        .await
    }

    pub async fn git_push(
        &self,
        workspace_id: impl Into<String>,
        remote: Option<String>,
        branch: Option<String>,
    ) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::GitPush {
            workspace_id: workspace_id.into(),
            remote,
            branch,
        })
        .await
    }

    pub async fn git_pull(
        &self,
        workspace_id: impl Into<String>,
        remote: Option<String>,
        branch: Option<String>,
    ) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::GitPull {
            workspace_id: workspace_id.into(),
            remote,
            branch,
        })
        .await
    }

    pub async fn git_branch(&self, workspace_id: impl Into<String>) -> Result<PcGatewayResponse> {
        self.send(PcGatewayRequest::GitBranch {
            workspace_id: workspace_id.into(),
        })
        .await
    }

    pub async fn send(&self, request: PcGatewayRequest) -> Result<PcGatewayResponse> {
        let mut errors = Vec::new();
        for endpoint in self.endpoint_plan() {
            if let Err(error) = endpoint.validate(self.config.allow_http_on_local_network) {
                self.record_endpoint_failure(&endpoint, error.clone());
                errors.push(format!("{} {} rejected: {}", endpoint.label, endpoint.base_url, error));
                continue;
            }
            let started_at = Instant::now();
            match self.send_to_endpoint(request.clone(), &endpoint).await {
                Ok(response) => {
                    self.record_endpoint_success(&endpoint, started_at.elapsed().as_millis());
                    return Ok(response);
                }
                Err(error) => {
                    self.record_endpoint_failure(&endpoint, error.to_string());
                    errors.push(format!(
                        "{} {} failed: {}",
                        endpoint.label, endpoint.base_url, error
                    ));
                }
            }
        }

        Err(anyhow!(
            "all PC gateway endpoints failed for '{}': {}",
            self.config.label,
            errors.join("; ")
        ))
    }

    async fn send_to_endpoint(
        &self,
        request: PcGatewayRequest,
        endpoint: &PcGatewayEndpointCandidate,
    ) -> Result<PcGatewayResponse> {
        let envelope = PcGatewayRequestEnvelope::new(self.config.device_id.clone(), request);
        let url = format!("{}/v1/gateway/request", endpoint.base_url.trim_end_matches('/'));
        let mut builder = self.http.post(url).json(&envelope);

        if let Some(token) = self.config.auth_token.as_deref() {
            builder = builder.bearer_auth(token);
        }

        let http_response = builder.send().await?;
        if !http_response.status().is_success() {
            let status = http_response.status();
            let text = http_response.text().await.unwrap_or_default();
            return Err(anyhow!("PC gateway HTTP error {}: {}", status, text));
        }

        let response_envelope: PcGatewayResponseEnvelope = http_response.json().await?;
        match response_envelope.response {
            PcGatewayResponse::Error(PcGatewayError { code, message }) => {
                Err(anyhow!("PC gateway error {}: {}", code, message))
            }
            response => Ok(response),
        }
    }

    fn record_endpoint_success(&self, endpoint: &PcGatewayEndpointCandidate, latency_ms: u128) {
        if let Ok(mut health) = self.endpoint_health.lock() {
            let item = health
                .entry(endpoint.base_url.clone())
                .or_insert_with(|| health_record(endpoint));
            item.label = endpoint.label.clone();
            item.success_count = item.success_count.saturating_add(1);
            item.last_latency_ms = Some(latency_ms);
            item.last_error = None;
        }
    }

    fn record_endpoint_failure(&self, endpoint: &PcGatewayEndpointCandidate, error: String) {
        if let Ok(mut health) = self.endpoint_health.lock() {
            let item = health
                .entry(endpoint.base_url.clone())
                .or_insert_with(|| health_record(endpoint));
            item.label = endpoint.label.clone();
            item.failure_count = item.failure_count.saturating_add(1);
            item.last_error = Some(error);
        }
    }

    fn endpoint_health_snapshot_by_url(&self) -> BTreeMap<String, PcGatewayEndpointHealth> {
        self.endpoint_health
            .lock()
            .map(|health| health.clone())
            .unwrap_or_default()
    }
}

fn health_record(endpoint: &PcGatewayEndpointCandidate) -> PcGatewayEndpointHealth {
    PcGatewayEndpointHealth {
        label: endpoint.label.clone(),
        base_url: endpoint.base_url.clone(),
        success_count: 0,
        failure_count: 0,
        last_latency_ms: None,
        last_error: None,
    }
}


/// Parse a single SSE event string (without the trailing `\n\n`).
fn parse_sse_event(raw: &str) -> Option<CommandStreamEvent> {
    let data = raw.strip_prefix("data: ")?;
    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    match value.get("kind")?.as_str()? {
        "stdout" => Some(CommandStreamEvent::Stdout(value.get("data")?.as_str()?.to_string())),
        "stderr" => Some(CommandStreamEvent::Stderr(value.get("data")?.as_str()?.to_string())),
        "exit" => Some(CommandStreamEvent::Exit(value.get("code")?.as_i64().map(|c| c as i32))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{PcGatewayClient, PcGatewayEndpointHealth};
    use crate::executor::CommandRequest;
    use crate::pc_gateway::{PcGatewayConfig, PcGatewayEndpointCandidate, PcGatewayTransportMode};
    use std::path::PathBuf;

    #[test]
    fn keeps_gateway_config() {
        let mut config = PcGatewayConfig::new(
            "pc-1",
            "Laptop",
            "http://192.168.1.10:8787",
            "phone-1",
        );
        config.transport_mode = PcGatewayTransportMode::LocalNetworkHttp;
        config.allow_http_on_local_network = true;
        let client = PcGatewayClient::new(config.clone());
        assert_eq!(client.config().id, config.id);
        assert!(client.config().validate_base_url().is_ok());
    }

    #[test]
    fn client_endpoint_plan_prefers_offline_local_routes() {
        let config = PcGatewayConfig::tunnel_https(
            "pc-1",
            "Laptop",
            "https://pc.example.test",
            "phone-1",
        )
        .with_endpoint_candidate(PcGatewayEndpointCandidate::new(
            "same-lan",
            "http://192.168.1.10:8787",
            PcGatewayTransportMode::LocalNetworkHttp,
        ))
        .with_endpoint_candidate(PcGatewayEndpointCandidate::new(
            "direct-link",
            "http://169.254.12.10:8787",
            PcGatewayTransportMode::DirectWifiHttp,
        ));
        let client = PcGatewayClient::new(config);
        let plan = client.endpoint_plan();
        assert_eq!(plan[0].label, "direct-link");
        assert_eq!(plan[1].label, "same-lan");
        assert_eq!(plan[2].label, "primary");
    }

    #[test]
    fn endpoint_health_score_rewards_success_and_penalizes_failure() {
        let good = PcGatewayEndpointHealth {
            label: "same-lan".to_string(),
            base_url: "http://192.168.1.10:8787".to_string(),
            success_count: 3,
            failure_count: 0,
            last_latency_ms: Some(20),
            last_error: None,
        };
        let bad = PcGatewayEndpointHealth {
            label: "same-lan".to_string(),
            base_url: "http://192.168.1.10:8787".to_string(),
            success_count: 0,
            failure_count: 3,
            last_latency_ms: Some(20),
            last_error: Some("timeout".to_string()),
        };
        assert!(good.score() > bad.score());
        assert!(good.is_healthy());
        assert!(!bad.is_healthy());
    }

    #[test]
    fn policy_rejects_blocked_command_before_transport() {
        let mut config = PcGatewayConfig::new(
            "pc-1",
            "Laptop",
            "http://192.168.1.10:8787",
            "phone-1",
        );
        config.transport_mode = PcGatewayTransportMode::LocalNetworkHttp;
        config.allow_http_on_local_network = true;
        let client = PcGatewayClient::new(config);
        let command = CommandRequest {
            program: "blocked-admin-tool".to_string(),
            args: vec!["--unsafe".to_string()],
            working_dir: Some(PathBuf::from("/work/project")),
        };
        assert!(client.policy().validate_command(&command).is_err());
    }
}
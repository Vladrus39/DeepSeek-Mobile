//! Async client for the PC companion gateway.
//!
//! This is the mobile-side transport layer. The gateway protocol lives in
//! `pc_gateway.rs`; this client sends typed envelopes to a trusted PC runtime
//! host and returns typed responses back to the mobile engine/UI.

use crate::executor::CommandRequest;
use crate::pc_gateway::{
    PcGatewayConfig, PcGatewayEndpointCandidate, PcGatewayError, PcGatewayHealth,
    PcGatewayRequest, PcGatewayRequestEnvelope, PcGatewayResponse, PcGatewayResponseEnvelope,
    PcGatewaySecurityPolicy,
};
use anyhow::{anyhow, Result};
use reqwest::Client;

#[derive(Clone)]
pub struct PcGatewayClient {
    http: Client,
    config: PcGatewayConfig,
    policy: PcGatewaySecurityPolicy,
}

impl PcGatewayClient {
    pub fn new(config: PcGatewayConfig) -> Self {
        Self {
            http: Client::new(),
            config,
            policy: PcGatewaySecurityPolicy::default(),
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
        self.config.endpoint_plan()
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

    pub async fn send(&self, request: PcGatewayRequest) -> Result<PcGatewayResponse> {
        let mut errors = Vec::new();
        for endpoint in self.endpoint_plan() {
            if let Err(error) = endpoint.validate(self.config.allow_http_on_local_network) {
                errors.push(format!("{} {} rejected: {}", endpoint.label, endpoint.base_url, error));
                continue;
            }
            match self.send_to_endpoint(request.clone(), &endpoint).await {
                Ok(response) => return Ok(response),
                Err(error) => errors.push(format!(
                    "{} {} failed: {}",
                    endpoint.label, endpoint.base_url, error
                )),
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
}

#[cfg(test)]
mod tests {
    use super::PcGatewayClient;
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
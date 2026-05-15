use anyhow::{anyhow, Context, Result};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use deepseek_mobile_core::{
    CommandOutput, PcEnvironmentDescriptor, PcGatewayCapability, PcGatewayConnectionStatus,
    PcGatewayDirEntry, PcGatewayError, PcGatewayHealth, PcGatewayRequest, PcGatewayRequestEnvelope,
    PcGatewayResponse, PcGatewayResponseEnvelope, PcGatewaySecurityPolicy, PcWorkspaceGrant,
};
use std::env;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::process::Command;

#[derive(Clone)]
struct PcHostState {
    config: Arc<PcHostConfig>,
}

#[derive(Clone, Debug)]
struct PcHostConfig {
    gateway_id: String,
    gateway_label: String,
    bind_addr: SocketAddr,
    auth_token: Option<String>,
    workspace: PcWorkspaceGrant,
    workspace_root: PathBuf,
    security_policy: PcGatewaySecurityPolicy,
}

impl PcHostConfig {
    fn from_env() -> Result<Self> {
        let bind_addr = env::var("DEEPSEEK_PC_HOST_BIND")
            .unwrap_or_else(|_| "127.0.0.1:8787".to_string())
            .parse::<SocketAddr>()
            .context("parse DEEPSEEK_PC_HOST_BIND")?;
        let gateway_id = env::var("DEEPSEEK_PC_HOST_ID").unwrap_or_else(|_| "pc-local".to_string());
        let gateway_label = env::var("DEEPSEEK_PC_HOST_LABEL").unwrap_or_else(|_| "Developer PC".to_string());
        let workspace_id = env::var("DEEPSEEK_PC_HOST_WORKSPACE_ID").unwrap_or_else(|_| "local".to_string());
        let workspace_root = env::var("DEEPSEEK_PC_HOST_WORKSPACE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let workspace_root = workspace_root
            .canonicalize()
            .with_context(|| format!("canonicalize workspace root {}", workspace_root.display()))?;
        let auth_token = env::var("DEEPSEEK_PC_HOST_TOKEN").ok().filter(|token| !token.is_empty());

        let workspace = PcWorkspaceGrant::new(
            workspace_id,
            workspace_root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("Workspace"),
            workspace_root.to_string_lossy(),
            host_capabilities(),
        );

        Ok(Self {
            gateway_id,
            gateway_label,
            bind_addr,
            auth_token,
            workspace,
            workspace_root,
            security_policy: PcGatewaySecurityPolicy::default(),
        })
    }

    fn response(&self, request_id: String, response: PcGatewayResponse) -> PcGatewayResponseEnvelope {
        PcGatewayResponseEnvelope {
            request_id,
            timestamp_unix: current_unix_time(),
            response,
        }
    }

    fn health(&self) -> PcGatewayHealth {
        PcGatewayHealth {
            gateway_id: self.gateway_id.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            status: PcGatewayConnectionStatus::Online,
            capabilities: host_capabilities(),
        }
    }

    fn resolve_workspace_path(&self, workspace_id: &str, path: &str) -> Result<PathBuf> {
        if workspace_id != self.workspace.id {
            return Err(anyhow!("unknown workspace id: {}", workspace_id));
        }

        if !self.security_policy.allows_path(path) {
            return Err(anyhow!("path is blocked by gateway policy: {}", path));
        }

        let relative = Path::new(path);
        if relative.is_absolute() {
            return Err(anyhow!("absolute paths are not accepted through gateway requests"));
        }

        Ok(self.workspace_root.join(relative))
    }

    async fn ensure_path_inside_workspace(&self, path: &Path) -> Result<PathBuf> {
        let canonical = if path.exists() {
            path.canonicalize()
                .with_context(|| format!("canonicalize {}", path.display()))?
        } else {
            let parent = path
                .parent()
                .ok_or_else(|| anyhow!("path has no parent: {}", path.display()))?;
            let canonical_parent = parent
                .canonicalize()
                .with_context(|| format!("canonicalize parent {}", parent.display()))?;
            canonical_parent.join(path.file_name().ok_or_else(|| anyhow!("path has no file name"))?)
        };

        if !canonical.starts_with(&self.workspace_root) {
            return Err(anyhow!("path escapes granted workspace: {}", canonical.display()));
        }
        Ok(canonical)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = PcHostConfig::from_env()?;
    let bind_addr = config.bind_addr;
    let state = PcHostState {
        config: Arc::new(config),
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/gateway/request", post(gateway_request_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("bind PC host on {}", bind_addr))?;
    println!("deepseek-pc-host listening on http://{}", bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_handler(State(state): State<PcHostState>) -> Json<PcGatewayHealth> {
    Json(state.config.health())
}

async fn gateway_request_handler(
    State(state): State<PcHostState>,
    headers: HeaderMap,
    Json(envelope): Json<PcGatewayRequestEnvelope>,
) -> Result<Json<PcGatewayResponseEnvelope>, (StatusCode, Json<PcGatewayResponseEnvelope>)> {
    if let Err(error) = authorize(&state.config, &headers) {
        let response = state.config.response(
            envelope.id,
            PcGatewayResponse::Error(PcGatewayError::new("unauthorized", error.to_string())),
        );
        return Err((StatusCode::UNAUTHORIZED, Json(response)));
    }

    let request_id = envelope.id.clone();
    let response = match handle_gateway_request(&state.config, envelope.request).await {
        Ok(response) => response,
        Err(error) => PcGatewayResponse::Error(PcGatewayError::new("host_error", error.to_string())),
    };

    Ok(Json(state.config.response(request_id, response)))
}

fn authorize(config: &PcHostConfig, headers: &HeaderMap) -> Result<()> {
    let Some(expected) = config.auth_token.as_deref() else {
        return Ok(());
    };
    let header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| anyhow!("missing Authorization header"))?;
    let token = header.strip_prefix("Bearer ").unwrap_or(header).trim();
    if token != expected {
        return Err(anyhow!("invalid bearer token"));
    }
    Ok(())
}

async fn handle_gateway_request(config: &PcHostConfig, request: PcGatewayRequest) -> Result<PcGatewayResponse> {
    match request {
        PcGatewayRequest::Health => Ok(PcGatewayResponse::Health(config.health())),
        PcGatewayRequest::ListWorkspaces => Ok(PcGatewayResponse::Workspaces(vec![config.workspace.clone()])),
        PcGatewayRequest::ListEnvironments { .. } => Ok(PcGatewayResponse::Environments(Vec::<PcEnvironmentDescriptor>::new())),
        PcGatewayRequest::ListDir { workspace_id, path } => list_dir(config, &workspace_id, &path).await,
        PcGatewayRequest::ReadFile { workspace_id, path } => read_file(config, &workspace_id, &path).await,
        PcGatewayRequest::WriteFile { workspace_id, path, content } => {
            write_file(config, &workspace_id, &path, &content).await
        }
        PcGatewayRequest::ExecuteCommand { workspace_id, command, environment_id: _ } => {
            execute_command(config, &workspace_id, command).await
        }
        PcGatewayRequest::GitStatus { workspace_id } => git_text(config, &workspace_id, "status", &["status", "--short"]).await,
        PcGatewayRequest::GitDiff { workspace_id } => git_text(config, &workspace_id, "diff", &["diff", "--"]).await,
        unsupported => Ok(PcGatewayResponse::Error(PcGatewayError::new(
            "unsupported_request",
            format!("request is not implemented by this PC host build: {:?}", unsupported),
        ))),
    }
}

async fn list_dir(config: &PcHostConfig, workspace_id: &str, path: &str) -> Result<PcGatewayResponse> {
    let path = config.resolve_workspace_path(workspace_id, path)?;
    let path = config.ensure_path_inside_workspace(&path).await?;
    let mut entries = fs::read_dir(&path)
        .await
        .with_context(|| format!("read dir {}", path.display()))?;
    let mut out = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        let absolute = entry.path();
        let relative = absolute
            .strip_prefix(&config.workspace_root)
            .unwrap_or(&absolute)
            .to_string_lossy()
            .replace('\\', "/");
        out.push(PcGatewayDirEntry {
            path: relative,
            is_dir: metadata.is_dir(),
            size_bytes: metadata.len(),
        });
    }
    out.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(PcGatewayResponse::DirEntries(out))
}

async fn read_file(config: &PcHostConfig, workspace_id: &str, path: &str) -> Result<PcGatewayResponse> {
    let requested = config.resolve_workspace_path(workspace_id, path)?;
    let path = config.ensure_path_inside_workspace(&requested).await?;
    let content = fs::read_to_string(&path)
        .await
        .with_context(|| format!("read file {}", path.display()))?;
    let relative = path
        .strip_prefix(&config.workspace_root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
    Ok(PcGatewayResponse::FileContent { path: relative, content })
}

async fn write_file(
    config: &PcHostConfig,
    workspace_id: &str,
    path: &str,
    content: &str,
) -> Result<PcGatewayResponse> {
    let requested = config.resolve_workspace_path(workspace_id, path)?;
    if let Some(parent) = requested.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create parent dir {}", parent.display()))?;
    }
    let path = config.ensure_path_inside_workspace(&requested).await?;
    fs::write(&path, content)
        .await
        .with_context(|| format!("write file {}", path.display()))?;
    let relative = path
        .strip_prefix(&config.workspace_root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
    Ok(PcGatewayResponse::FileWritten {
        path: relative,
        bytes: content.len(),
    })
}

async fn execute_command(
    config: &PcHostConfig,
    workspace_id: &str,
    command: deepseek_mobile_core::CommandRequest,
) -> Result<PcGatewayResponse> {
    if workspace_id != config.workspace.id {
        return Err(anyhow!("unknown workspace id: {}", workspace_id));
    }
    config
        .security_policy
        .validate_command(&command)
        .map_err(|message| anyhow!(message))?;

    let working_dir = match command.working_dir.as_ref() {
        Some(dir) => {
            if dir.is_absolute() {
                return Err(anyhow!("absolute working directories are not accepted"));
            }
            config.ensure_path_inside_workspace(&config.workspace_root.join(dir)).await?
        }
        None => config.workspace_root.clone(),
    };

    let output = Command::new(&command.program)
        .args(&command.args)
        .current_dir(&working_dir)
        .output()
        .await
        .with_context(|| format!("execute command {}", command.program))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok(PcGatewayResponse::CommandOutput(CommandOutput {
        status_code: output.status.code(),
        stdout: truncate_output(stdout, config.security_policy.max_output_bytes),
        stderr: truncate_output(stderr, config.security_policy.max_output_bytes),
    }))
}

async fn git_text(
    config: &PcHostConfig,
    workspace_id: &str,
    operation: &str,
    args: &[&str],
) -> Result<PcGatewayResponse> {
    if workspace_id != config.workspace.id {
        return Err(anyhow!("unknown workspace id: {}", workspace_id));
    }
    let output = Command::new("git")
        .args(args)
        .current_dir(&config.workspace_root)
        .output()
        .await
        .with_context(|| format!("git {}", operation))?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(PcGatewayResponse::GitText {
        operation: operation.to_string(),
        output: truncate_output(text, config.security_policy.max_output_bytes),
    })
}

fn truncate_output(mut text: String, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text;
    }
    text.truncate(max_bytes);
    text.push_str("\n... <truncated by pc-host policy>");
    text
}

fn host_capabilities() -> Vec<PcGatewayCapability> {
    vec![
        PcGatewayCapability::ListWorkspaces,
        PcGatewayCapability::ReadFiles,
        PcGatewayCapability::WriteFiles,
        PcGatewayCapability::ExecuteCommands,
        PcGatewayCapability::GitStatus,
        PcGatewayCapability::GitDiff,
        PcGatewayCapability::Diagnostics,
    ]
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

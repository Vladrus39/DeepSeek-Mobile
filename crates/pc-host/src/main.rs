use anyhow::{anyhow, Context, Result};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};
use deepseek_mobile_core::{
    CommandOutput, CommandRequest, PcDiagnostic, PcDiagnosticSeverity, PcEnvironmentDescriptor,
    PcGatewayCapability, PcGatewayConnectionStatus, PcGatewayDirEntry, PcGatewayError,
    PcGatewayHealth, PcGatewayRequest, PcGatewayRequestEnvelope, PcGatewayResponse,
    PcGatewayResponseEnvelope, PcGatewaySecurityPolicy, PcTaskDescriptor, PcTaskKind,
    PcWorkspaceGrant,
};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::process::Command;
use tokio::time::timeout;

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

#[derive(Clone, Debug, Deserialize)]
struct CargoCheckMessage {
    reason: Option<String>,
    message: Option<CargoDiagnosticMessage>,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoDiagnosticMessage {
    message: String,
    level: String,
    spans: Vec<CargoDiagnosticSpan>,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoDiagnosticSpan {
    file_name: String,
    line_start: u32,
    column_start: u32,
    is_primary: bool,
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

        for component in relative.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                Component::ParentDir => return Err(anyhow!("parent path segments are not accepted: {}", path)),
                Component::RootDir | Component::Prefix(_) => {
                    return Err(anyhow!("root or prefix path segments are not accepted: {}", path));
                }
            }
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

    async fn ensure_parent_inside_workspace(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("path has no parent: {}", path.display()))?;
        let existing_parent = nearest_existing_parent(parent)?;
        let canonical_parent = existing_parent
            .canonicalize()
            .with_context(|| format!("canonicalize existing parent {}", existing_parent.display()))?;
        if !canonical_parent.starts_with(&self.workspace_root) {
            return Err(anyhow!("parent path escapes granted workspace: {}", canonical_parent.display()));
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = PcHostConfig::from_env()?;
    let bind_addr = config.bind_addr;
    let gateway_label = config.gateway_label.clone();
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
    println!("deepseek-pc-host '{}' listening on http://{}", gateway_label, bind_addr);
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
        PcGatewayRequest::DetectTasks { workspace_id } => detect_tasks(config, &workspace_id).await,
        PcGatewayRequest::GetDiagnostics { workspace_id, path } => diagnostics(config, &workspace_id, path.as_deref()).await,
        PcGatewayRequest::ListDir { workspace_id, path } => list_dir(config, &workspace_id, &path).await,
        PcGatewayRequest::ReadFile { workspace_id, path } => read_file(config, &workspace_id, &path).await,
        PcGatewayRequest::WriteFile { workspace_id, path, content } => {
            write_file(config, &workspace_id, &path, &content).await
        }
        PcGatewayRequest::DeleteFile { workspace_id, path } => delete_file(config, &workspace_id, &path).await,
        PcGatewayRequest::ExecuteCommand { workspace_id, command, environment_id: _ } => {
            execute_command(config, &workspace_id, command).await
        }
        PcGatewayRequest::GitStatus { workspace_id } => git_text(config, &workspace_id, "status", &["status", "--short"]).await,
        PcGatewayRequest::GitDiff { workspace_id } => git_text(config, &workspace_id, "diff", &["diff", "--"]).await,
        PcGatewayRequest::GitCommit { workspace_id, message } => {
            git_commit(config, &workspace_id, &message).await
        }
        PcGatewayRequest::GitPush { workspace_id, remote, branch } => {
            git_push(config, &workspace_id, remote.as_deref(), branch.as_deref()).await
        }
        PcGatewayRequest::GitPull { workspace_id, remote, branch } => {
            git_pull(config, &workspace_id, remote.as_deref(), branch.as_deref()).await
        }
        PcGatewayRequest::GitBranch { workspace_id } => {
            git_text(config, &workspace_id, "branch", &["branch", "--list"]).await
        }
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
    config.ensure_parent_inside_workspace(&requested).await?;
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

async fn delete_file(config: &PcHostConfig, workspace_id: &str, path: &str) -> Result<PcGatewayResponse> {
    let requested = config.resolve_workspace_path(workspace_id, path)?;
    let path = config.ensure_path_inside_workspace(&requested).await?;
    let metadata = fs::metadata(&path)
        .await
        .with_context(|| format!("metadata for delete {}", path.display()))?;
    if metadata.is_dir() {
        return Err(anyhow!("delete_file refuses to delete directories: {}", path.display()));
    }
    fs::remove_file(&path)
        .await
        .with_context(|| format!("delete file {}", path.display()))?;
    let relative = path
        .strip_prefix(&config.workspace_root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/");
    Ok(PcGatewayResponse::FileDeleted { path: relative })
}

async fn execute_command(
    config: &PcHostConfig,
    workspace_id: &str,
    command: CommandRequest,
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
            let dir_text = dir.to_string_lossy();
            let requested = config.resolve_workspace_path(workspace_id, &dir_text)?;
            config.ensure_path_inside_workspace(&requested).await?
        }
        None => config.workspace_root.clone(),
    };

    let run = Command::new(&command.program)
        .args(&command.args)
        .current_dir(&working_dir)
        .output();
    let output = timeout(Duration::from_secs(config.security_policy.max_command_seconds), run)
        .await
        .map_err(|_| anyhow!("command timed out after {} seconds", config.security_policy.max_command_seconds))?
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
    let output = timeout(
        Duration::from_secs(config.security_policy.max_command_seconds),
        Command::new("git").args(args).current_dir(&config.workspace_root).output(),
    )
    .await
    .map_err(|_| anyhow!("git {} timed out after {} seconds", operation, config.security_policy.max_command_seconds))?
    .with_context(|| format!("git {}", operation))?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(PcGatewayResponse::GitText {
        operation: operation.to_string(),
        output: truncate_output(text, config.security_policy.max_output_bytes),
    })
}
async fn git_commit(config: &PcHostConfig, workspace_id: &str, message: &str) -> Result<PcGatewayResponse> {
    if workspace_id != config.workspace.id {
        return Err(anyhow!("unknown workspace id: {}", workspace_id));
    }
    let output = timeout(
        Duration::from_secs(config.security_policy.max_command_seconds),
        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(&config.workspace_root)
            .output(),
    )
    .await
    .map_err(|_| anyhow!("git commit timed out after {} seconds", config.security_policy.max_command_seconds))?
    .with_context(|| "git commit")?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(PcGatewayResponse::GitText {
        operation: "commit".to_string(),
        output: truncate_output(text, config.security_policy.max_output_bytes),
    })
}

async fn git_push(
    config: &PcHostConfig,
    workspace_id: &str,
    remote: Option<&str>,
    branch: Option<&str>,
) -> Result<PcGatewayResponse> {
    if workspace_id != config.workspace.id {
        return Err(anyhow!("unknown workspace id: {}", workspace_id));
    }
    let mut args = vec!["push"];
    if let Some(remote) = remote {
        args.push(remote);
        if let Some(branch) = branch {
            args.push(branch);
        }
    }
    let output = timeout(
        Duration::from_secs(config.security_policy.max_command_seconds),
        Command::new("git").args(&args).current_dir(&config.workspace_root).output(),
    )
    .await
    .map_err(|_| anyhow!("git push timed out after {} seconds", config.security_policy.max_command_seconds))?
    .with_context(|| "git push")?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(PcGatewayResponse::GitText {
        operation: "push".to_string(),
        output: truncate_output(text, config.security_policy.max_output_bytes),
    })
}

async fn git_pull(
    config: &PcHostConfig,
    workspace_id: &str,
    remote: Option<&str>,
    branch: Option<&str>,
) -> Result<PcGatewayResponse> {
    if workspace_id != config.workspace.id {
        return Err(anyhow!("unknown workspace id: {}", workspace_id));
    }
    let mut args = vec!["pull"];
    if let Some(remote) = remote {
        args.push(remote);
        if let Some(branch) = branch {
            args.push(branch);
        }
    }
    let output = timeout(
        Duration::from_secs(config.security_policy.max_command_seconds),
        Command::new("git").args(&args).current_dir(&config.workspace_root).output(),
    )
    .await
    .map_err(|_| anyhow!("git pull timed out after {} seconds", config.security_policy.max_command_seconds))?
    .with_context(|| "git pull")?;
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    Ok(PcGatewayResponse::GitText {
        operation: "pull".to_string(),
        output: truncate_output(text, config.security_policy.max_output_bytes),
    })
}

async fn detect_tasks(config: &PcHostConfig, workspace_id: &str) -> Result<PcGatewayResponse> {
    if workspace_id != config.workspace.id {
        return Err(anyhow!("unknown workspace id: {}", workspace_id));
    }
    let mut tasks = Vec::new();

    if config.workspace_root.join("Cargo.toml").exists() {
        tasks.push(task(workspace_id, "cargo-check", "Cargo check", PcTaskKind::Build, "cargo", &["check", "--workspace"]));
        tasks.push(task(workspace_id, "cargo-test", "Cargo test", PcTaskKind::Test, "cargo", &["test", "--workspace"]));
    }
    if config.workspace_root.join("package.json").exists() {
        tasks.push(task(workspace_id, "npm-install", "npm install", PcTaskKind::Install, "npm", &["install"]));
        tasks.push(task(workspace_id, "npm-test", "npm test", PcTaskKind::Test, "npm", &["test"]));
    }
    if config.workspace_root.join("pyproject.toml").exists() || config.workspace_root.join("pytest.ini").exists() {
        tasks.push(task(workspace_id, "pytest", "pytest", PcTaskKind::Test, "pytest", &[]));
    }

    Ok(PcGatewayResponse::Tasks(tasks))
}

async fn diagnostics(config: &PcHostConfig, workspace_id: &str, path: Option<&str>) -> Result<PcGatewayResponse> {
    if workspace_id != config.workspace.id {
        return Err(anyhow!("unknown workspace id: {}", workspace_id));
    }
    let requested_path = if let Some(path) = path {
        let requested = config.resolve_workspace_path(workspace_id, path)?;
        let checked = config.ensure_path_inside_workspace(&requested).await?;
        Some(checked.strip_prefix(&config.workspace_root).unwrap_or(&checked).to_path_buf())
    } else {
        None
    };

    let mut diagnostics = Vec::new();
    if config.workspace_root.join("Cargo.toml").exists() {
        diagnostics.extend(rust_cargo_diagnostics(config, requested_path.as_deref()).await?);
    }
    diagnostics.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.line.cmp(&right.line))
            .then(left.column.cmp(&right.column))
            .then(left.message.cmp(&right.message))
    });
    Ok(PcGatewayResponse::Diagnostics(diagnostics))
}

async fn rust_cargo_diagnostics(config: &PcHostConfig, requested_path: Option<&Path>) -> Result<Vec<PcDiagnostic>> {
    let output = timeout(
        Duration::from_secs(config.security_policy.max_command_seconds),
        Command::new("cargo")
            .args(["check", "--workspace", "--message-format=json"])
            .current_dir(&config.workspace_root)
            .output(),
    )
    .await
    .map_err(|_| anyhow!("cargo check diagnostics timed out after {} seconds", config.security_policy.max_command_seconds))?
    .context("execute cargo check diagnostics")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut diagnostics = Vec::new();
    for line in stdout.lines() {
        let Ok(message) = serde_json::from_str::<CargoCheckMessage>(line) else {
            continue;
        };
        if message.reason.as_deref() != Some("compiler-message") {
            continue;
        }
        let Some(message) = message.message else {
            continue;
        };
        diagnostics.extend(cargo_message_to_diagnostics(config, requested_path, message));
    }
    Ok(diagnostics)
}

fn cargo_message_to_diagnostics(
    config: &PcHostConfig,
    requested_path: Option<&Path>,
    message: CargoDiagnosticMessage,
) -> Vec<PcDiagnostic> {
    let severity = cargo_level_to_severity(&message.level);
    message
        .spans
        .into_iter()
        .filter(|span| span.is_primary)
        .filter_map(|span| {
            let path = Path::new(&span.file_name);
            let absolute = if path.is_absolute() {
                path.to_path_buf()
            } else {
                config.workspace_root.join(path)
            };
            let Ok(relative) = absolute.strip_prefix(&config.workspace_root) else {
                return None;
            };
            if let Some(requested) = requested_path {
                if normalize_path(relative) != normalize_path(requested) {
                    return None;
                }
            }
            Some(PcDiagnostic {
                path: normalize_path(relative),
                line: span.line_start,
                column: span.column_start,
                severity: severity.clone(),
                message: message.message.clone(),
                source: Some("cargo check".to_string()),
            })
        })
        .collect()
}

fn cargo_level_to_severity(level: &str) -> PcDiagnosticSeverity {
    match level {
        "error" => PcDiagnosticSeverity::Error,
        "warning" => PcDiagnosticSeverity::Warning,
        "note" | "help" => PcDiagnosticSeverity::Hint,
        _ => PcDiagnosticSeverity::Info,
    }
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn task(
    workspace_id: &str,
    id: &str,
    label: &str,
    kind: PcTaskKind,
    program: &str,
    args: &[&str],
) -> PcTaskDescriptor {
    PcTaskDescriptor {
        id: id.to_string(),
        workspace_id: workspace_id.to_string(),
        label: label.to_string(),
        kind,
        command: CommandRequest::new(program, args.iter().map(|arg| arg.to_string()).collect()),
        environment_id: None,
    }
}

fn nearest_existing_parent(path: &Path) -> Result<PathBuf> {
    let mut current = path;
    loop {
        if current.exists() {
            return Ok(current.to_path_buf());
        }
        current = current
            .parent()
            .ok_or_else(|| anyhow!("no existing parent found for {}", path.display()))?;
    }
}

fn truncate_output(text: String, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text;
    }

    let mut cut = 0;
    for (idx, _) in text.char_indices() {
        if idx > max_bytes {
            break;
        }
        cut = idx;
    }
    if cut == 0 {
        cut = text
            .char_indices()
            .nth(1)
            .map(|(idx, _)| idx)
            .unwrap_or_else(|| text.len());
    }

    let mut truncated = text[..cut].to_string();
    truncated.push_str("\n... <truncated by pc-host policy>");
    truncated
}

fn host_capabilities() -> Vec<PcGatewayCapability> {
    vec![
        PcGatewayCapability::ListWorkspaces,
        PcGatewayCapability::ReadFiles,
        PcGatewayCapability::WriteFiles,
        PcGatewayCapability::DeleteFiles,
        PcGatewayCapability::ExecuteCommands,
        PcGatewayCapability::GitStatus,
        PcGatewayCapability::GitDiff,
        PcGatewayCapability::Diagnostics,
        PcGatewayCapability::RunTests,
        PcGatewayCapability::RunBuilds,
        PcGatewayCapability::GitCommit,
        PcGatewayCapability::GitPushPull,
        PcGatewayCapability::TerminalSessions,
    ]
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{cargo_level_to_severity, normalize_path};
    use deepseek_mobile_core::PcDiagnosticSeverity;
    use std::path::Path;

    #[test]
    fn maps_cargo_levels_to_gateway_severity() {
        assert_eq!(cargo_level_to_severity("error"), PcDiagnosticSeverity::Error);
        assert_eq!(cargo_level_to_severity("warning"), PcDiagnosticSeverity::Warning);
        assert_eq!(cargo_level_to_severity("help"), PcDiagnosticSeverity::Hint);
        assert_eq!(cargo_level_to_severity("unknown"), PcDiagnosticSeverity::Info);
    }

    #[test]
    fn normalizes_windows_paths_for_protocol() {
        assert_eq!(normalize_path(Path::new("src\\main.rs")), "src/main.rs");
    }
}

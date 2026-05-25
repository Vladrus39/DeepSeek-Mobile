use anyhow::{anyhow, Context, Result};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use std::convert::Infallible;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio_stream::wrappers::ReceiverStream;
use deepseek_mobile_core::{
    CommandOutput, CommandRequest, LogRing, PcDiagnostic, PcDiagnosticSeverity,
    PcEnvironmentDescriptor, PcGatewayCapability, PcGatewayConnectionStatus, PcGatewayDirEntry,
    PcGatewayError, PcGatewayHealth, PcGatewayLogEntry, PcGatewayLogs, PcGatewayRequest,
    PcGatewayRequestEnvelope, PcGatewayResponse, PcGatewayResponseEnvelope,
    PcGatewaySecurityPolicy, PolicyPreset, PcTaskDescriptor, PcTaskKind, PcTerminalSession,
    PcWorkspaceGrant,
};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;
use std::collections::HashMap;

#[derive(Clone)]
struct PcHostState {
    config: Arc<PcHostConfig>,
    terminals: Arc<Mutex<HashMap<String, TerminalHandle>>>,
    log_ring: Arc<Mutex<LogRing>>,
    start_time: std::time::Instant,
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

/// Active terminal session handle with process and output buffer.
struct TerminalHandle {
    child: Option<Child>,
    stdin_writer: Option<tokio::process::ChildStdin>,
    output_buffer: Vec<String>,
    exit_code: Option<i32>,
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
        let preset = env::var("DEEPSEEK_PC_HOST_POLICY")
            .ok()
            .and_then(|v| match v.to_lowercase().as_str() {
                "readonly" | "read-only" => Some(PolicyPreset::ReadOnly),
                "developer" | "dev" => Some(PolicyPreset::Developer),
                "admin" => Some(PolicyPreset::Admin),
                _ => None,
            })
            .unwrap_or(PolicyPreset::Developer);

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
            security_policy: PcGatewaySecurityPolicy::from_preset(preset),
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
            uptime_secs: 0,
            request_count: 0,
            error_count: 0,
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
        terminals: Arc::new(Mutex::new(HashMap::new())),
        log_ring: Arc::new(Mutex::new(LogRing::new(200))),
        start_time: std::time::Instant::now(),
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/gateway/request", post(gateway_request_handler))
        .route("/v1/gateway/exec/stream", post(exec_stream_handler))
        .route("/v1/gateway/logs", get(logs_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("bind PC host on {}", bind_addr))?;
    println!("deepseek-pc-host '{}' listening on http://{}", gateway_label, bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_handler(State(state): State<PcHostState>) -> Json<PcGatewayHealth> {
    let mut health = state.config.health();
    health.uptime_secs = state.start_time.elapsed().as_secs();
    let log_ring = state.log_ring.lock().await;
    let snapshot = log_ring.snapshot();
    health.request_count = snapshot.entries.len() as u64;
    health.error_count = snapshot.entries.iter().filter(|e| !e.success).count() as u64;
    Json(health)
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
    let operation = request_operation_name(&envelope.request);
    let start = std::time::Instant::now();

    let response = match handle_gateway_request(&state, envelope.request).await {
        Ok(response) => response,
        Err(error) => PcGatewayResponse::Error(PcGatewayError::new("host_error", error.to_string())),
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let success = !matches!(&response, PcGatewayResponse::Error(_));
    {
        let mut log_ring = state.log_ring.lock().await;
        log_ring.push(PcGatewayLogEntry {
            timestamp_unix: current_unix_time(),
            request_id: request_id.clone(),
            operation,
            success,
            error_message: if success { None } else { Some(format!("{:?}", response)) },
            duration_ms,
        });
    }

    Ok(Json(state.config.response(request_id, response)))
}

/// SSE streaming endpoint for long-running commands.
/// Accepts workspace_id + CommandRequest, spawns the process,
/// and streams stdout/stderr lines as SSE events.
async fn exec_stream_handler(
    State(state): State<PcHostState>,
    headers: HeaderMap,
    Json(body): Json<ExecStreamBody>,
) -> Result<Sse<ReceiverStream<Result<Event, Infallible>>>, (StatusCode, String)> {
    if let Err(e) = authorize(&state.config, &headers) {
        return Err((StatusCode::UNAUTHORIZED, e.to_string()));
    }
    if body.workspace_id != state.config.workspace.id {
        return Err((StatusCode::BAD_REQUEST, "unknown workspace".to_string()));
    }
    if let Err(e) = state.config.security_policy.validate_command(&body.command) {
        return Err((StatusCode::FORBIDDEN, e.to_string()));
    }

    let working_dir = match body.command.working_dir.as_ref() {
        Some(dir) => {
            if dir.is_absolute() {
                return Err((StatusCode::BAD_REQUEST, "absolute paths not accepted".to_string()));
            }
            match state.config.resolve_workspace_path(&body.workspace_id, &dir.to_string_lossy()) {
                Ok(resolved) => match state.config.ensure_path_inside_workspace(&resolved).await {
                    Ok(dir) => dir,
                    Err(e) => return Err((StatusCode::FORBIDDEN, e.to_string())),
                },
                Err(e) => return Err((StatusCode::BAD_REQUEST, e.to_string())),
            }
        }
        None => state.config.workspace_root.clone(),
    };

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(64);
    let max_seconds = state.config.security_policy.max_command_seconds;

    tokio::spawn(async move {
        let result = stream_process_output(body.command, working_dir, max_seconds, tx.clone()).await;
        if let Err(e) = result {
            let _ = tx
                .send(Ok(Event::default().data(
                    serde_json::json!({"kind": "stderr", "data": format!("stream error: {}", e)}).to_string(),
                )))
                .await;
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx)))
}

#[derive(Deserialize)]
struct ExecStreamBody {
    workspace_id: String,
    command: CommandRequest,
}

async fn stream_process_output(
    command: CommandRequest,
    working_dir: PathBuf,
    max_seconds: u64,
    tx: tokio::sync::mpsc::Sender<Result<Event, Infallible>>,
) -> Result<()> {
    let mut child = TokioCommand::new(&command.program)
        .args(&command.args)
        .current_dir(&working_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn {}", command.program))?;

    let stdout = child.stdout.take().context("capture stdout")?;
    let stderr = child.stderr.take().context("capture stderr")?;

    let tx_stdout = tx.clone();
    let tx_stderr = tx.clone();
    let tx_exit = tx.clone();

    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let event = Ok(Event::default().data(
                serde_json::json!({"kind": "stdout", "data": line}).to_string(),
            ));
            if tx_stdout.send(event).await.is_err() {
                break;
            }
        }
    });

    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let event = Ok(Event::default().data(
                serde_json::json!({"kind": "stderr", "data": line}).to_string(),
            ));
            if tx_stderr.send(event).await.is_err() {
                break;
            }
        }
    });

    let status = timeout(Duration::from_secs(max_seconds), child.wait())
        .await
        .map_err(|_| anyhow!("command timed out after {} seconds", max_seconds))?
        .context("wait for command")?;

    let _ = tokio::join!(stdout_task, stderr_task);

    let _ = tx_exit
        .send(Ok(Event::default().data(
            serde_json::json!({"kind": "exit", "code": status.code()}).to_string(),
        )))
        .await;

    Ok(())
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

async fn handle_gateway_request(state: &PcHostState, request: PcGatewayRequest) -> Result<PcGatewayResponse> {
    match request {
        PcGatewayRequest::Health => Ok(PcGatewayResponse::Health(state.config.health())),
        PcGatewayRequest::ListWorkspaces => Ok(PcGatewayResponse::Workspaces(vec![state.config.workspace.clone()])),
        PcGatewayRequest::ListEnvironments { .. } => Ok(PcGatewayResponse::Environments(Vec::<PcEnvironmentDescriptor>::new())),
        PcGatewayRequest::DetectTasks { workspace_id } => detect_tasks(&state.config, &workspace_id).await,
        PcGatewayRequest::GetDiagnostics { workspace_id, path } => diagnostics(&state.config, &workspace_id, path.as_deref()).await,
        PcGatewayRequest::ListDir { workspace_id, path } => list_dir(&state.config, &workspace_id, &path).await,
        PcGatewayRequest::ReadFile { workspace_id, path } => read_file(&state.config, &workspace_id, &path).await,
        PcGatewayRequest::WriteFile { workspace_id, path, content } => {
            write_file(&state.config, &workspace_id, &path, &content).await
        }
        PcGatewayRequest::DeleteFile { workspace_id, path } => delete_file(&state.config, &workspace_id, &path).await,
        PcGatewayRequest::ExecuteCommand { workspace_id, command, environment_id: _ } => {
            execute_command(&state.config, &workspace_id, command).await
        }
        PcGatewayRequest::GitStatus { workspace_id } => git_text(&state.config, &workspace_id, "status", &["status", "--short"]).await,
        PcGatewayRequest::GitDiff { workspace_id } => git_text(&state.config, &workspace_id, "diff", &["diff", "--"]).await,
        PcGatewayRequest::GitCommit { workspace_id, message } => {
            git_commit(&state.config, &workspace_id, &message).await
        }
        PcGatewayRequest::GitPush { workspace_id, remote, branch } => {
            git_push(&state.config, &workspace_id, remote.as_deref(), branch.as_deref()).await
        }
        PcGatewayRequest::GitPull { workspace_id, remote, branch } => {
            git_pull(&state.config, &workspace_id, remote.as_deref(), branch.as_deref()).await
        }
        PcGatewayRequest::GitBranch { workspace_id } => {
            git_text(&state.config, &workspace_id, "branch", &["branch", "--list"]).await
        }
        PcGatewayRequest::SnapshotCreate { workspace_id, reason } => {
            snapshot_create(&state.config, &workspace_id, &reason).await
        }
        PcGatewayRequest::SnapshotRestore { workspace_id, snapshot_id } => {
            snapshot_restore(&state.config, &workspace_id, &snapshot_id).await
        }
        PcGatewayRequest::SnapshotList { workspace_id } => snapshot_list(&state.config, &workspace_id).await,
        PcGatewayRequest::OpenTerminal { workspace_id, cwd, environment_id: _ } => {
            open_terminal(state, &workspace_id, cwd.as_deref()).await
        }
        PcGatewayRequest::TerminalInput { session_id, input } => {
            terminal_input(state, &session_id, &input).await
        }
        PcGatewayRequest::CloseTerminal { session_id } => {
            close_terminal(state, &session_id).await
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
async fn open_terminal(state: &PcHostState, workspace_id: &str, cwd: Option<&str>) -> Result<PcGatewayResponse> {
    let config = &state.config;
    if workspace_id != config.workspace.id {
        return Err(anyhow!("unknown workspace id: {}", workspace_id));
    }

    let working_dir = match cwd {
        Some(dir) => {
            let resolved = config.resolve_workspace_path(workspace_id, dir)?;
            config.ensure_path_inside_workspace(&resolved).await?
        }
        None => config.workspace_root.clone(),
    };

    let session_id = uuid_v4();
    let now = current_unix_time();

    // Use platform-appropriate shell
    #[cfg(target_os = "windows")]
    let (program, args): (&str, Vec<&str>) = ("cmd.exe", vec![]);
    #[cfg(not(target_os = "windows"))]
    let (program, args): (&str, Vec<&str>) = ("/bin/sh", vec!["-i"]);

    let mut child = Command::new(program)
        .args(&args)
        .current_dir(&working_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn terminal shell {}", program))?;

    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let sid = session_id.clone();

    let handle = TerminalHandle {
        child: Some(child),
        stdin_writer: stdin,
        output_buffer: Vec::new(),
        exit_code: None,
    };

    // Spawn background reader for stdout/stderr
    if let (Some(stdout), Some(stderr)) = (stdout, stderr) {
        let terminals = state.terminals.clone();
        let sid_bg = sid.clone();
        tokio::spawn(async move {
            let stdout_reader = BufReader::new(stdout);
            let stderr_reader = BufReader::new(stderr);
            let mut stdout_lines = stdout_reader.lines();
            let mut stderr_lines = stderr_reader.lines();

            loop {
                tokio::select! {
                    line = stdout_lines.next_line() => {
                        match line {
                            Ok(Some(text)) => {
                                let mut guard = terminals.lock().await;
                                if let Some(h) = guard.get_mut(&sid_bg) {
                                    h.output_buffer.push(text);
                                }
                            }
                            Ok(None) | Err(_) => break,
                        }
                    }
                    line = stderr_lines.next_line() => {
                        match line {
                            Ok(Some(text)) => {
                                let mut guard = terminals.lock().await;
                                if let Some(h) = guard.get_mut(&sid_bg) {
                                    h.output_buffer.push(format!("[stderr] {}", text));
                                }
                            }
                            Ok(None) | Err(_) => break,
                        }
                    }
                }
            }
        });
    }

    // Store handle
    state.terminals.lock().await.insert(sid.clone(), handle);

    let cwd_display = working_dir.display().to_string();
    Ok(PcGatewayResponse::TerminalOpened(PcTerminalSession {
        id: sid,
        workspace_id: workspace_id.to_string(),
        title: format!("{} terminal", std::env::consts::OS),
        cwd: cwd_display,
        environment_id: None,
        created_at_unix: now,
    }))
}

async fn terminal_input(state: &PcHostState, session_id: &str, input: &str) -> Result<PcGatewayResponse> {
    let mut guard = state.terminals.lock().await;
    let Some(handle) = guard.get_mut(session_id) else {
        return Err(anyhow!("terminal session not found: {}", session_id));
    };

    // Write to stdin if available
    if let Some(stdin) = handle.stdin_writer.as_mut() {
        stdin.write_all(input.as_bytes()).await
            .with_context(|| format!("write stdin to terminal {}", session_id))?;
        stdin.write_all(b"\n").await?;
    }

    // Drain and return accumulated output
    let chunks: Vec<String> = handle.output_buffer.drain(..).collect();
    let output = chunks.join("\n");

    Ok(PcGatewayResponse::TerminalOutput {
        session_id: session_id.to_string(),
        chunk: output,
    })
}

async fn close_terminal(state: &PcHostState, session_id: &str) -> Result<PcGatewayResponse> {
    let mut guard = state.terminals.lock().await;
    let Some(mut handle) = guard.remove(session_id) else {
        return Err(anyhow!("terminal session not found: {}", session_id));
    };

    // Kill process if still running
    if let Some(mut child) = handle.child.take() {
        let _ = child.kill().await;
    }

    // Drain remaining output
    let chunks: Vec<String> = handle.output_buffer.drain(..).collect();
    let _output = chunks.join("\n");

    Ok(PcGatewayResponse::TerminalClosed {
        session_id: session_id.to_string(),
        exit_code: handle.exit_code,
    })
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("term-{:x}", ts.as_nanos())
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
    if config.workspace_root.join("tsconfig.json").exists() {
        diagnostics.extend(typescript_diagnostics(config, requested_path.as_deref()).await?);
    }
    if config.workspace_root.join("pyproject.toml").exists()
        || config.workspace_root.join("setup.py").exists()
        || config.workspace_root.join("requirements.txt").exists()
    {
        diagnostics.extend(python_diagnostics(config, requested_path.as_deref()).await?);
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

async fn typescript_diagnostics(config: &PcHostConfig, _requested_path: Option<&Path>) -> Result<Vec<PcDiagnostic>> {
    let output = timeout(
        Duration::from_secs(config.security_policy.max_command_seconds),
        Command::new("npx")
            .args(["tsc", "--noEmit", "--pretty", "false"])
            .current_dir(&config.workspace_root)
            .output(),
    )
    .await
    .map_err(|_| anyhow!("tsc diagnostics timed out"))?
    .context("execute tsc diagnostics")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut diagnostics = Vec::new();
    // tsc outputs errors in format: path(line,col): error TS1234: message
    for line in stderr.lines() {
        if let Some(diag) = parse_tsc_line(line) {
            diagnostics.push(diag);
        }
    }
    Ok(diagnostics)
}

fn parse_tsc_line(line: &str) -> Option<PcDiagnostic> {
    // Format: path(line,col): error TS1234: message
    // or: path(line,col): warning TS1234: message
    let trimmed = line.trim();
    let paren_open = trimmed.find('(')?;
    let paren_close = trimmed.find(')')?;
    let colon_after = trimmed[paren_close + 1..].find(':')?;
    let global_colon = paren_close + 1 + colon_after;

    let path = trimmed[..paren_open].to_string();
    let line_col = &trimmed[paren_open + 1..paren_close];
    let (line_str, col_str) = line_col.split_once(',')
        .map(|(l, c)| (l.trim(), c.trim()))
        .unwrap_or((line_col, "1"));

    let severity = if trimmed[global_colon + 1..].trim_start().starts_with("error") {
        PcDiagnosticSeverity::Error
    } else {
        PcDiagnosticSeverity::Warning
    };

    // Split after severity code
    let message_start = trimmed[global_colon + 1..].find(':')
        .map(|pos| global_colon + 1 + pos + 1)
        .unwrap_or(global_colon + 1);
    let message = trimmed[message_start..].trim().to_string();

    Some(PcDiagnostic {
        path: path.replace('\\', "/"),
        line: line_str.parse().unwrap_or(0),
        column: col_str.parse().unwrap_or(0),
        severity,
        message,
        source: Some("tsc".to_string()),
    })
}

async fn python_diagnostics(config: &PcHostConfig, _requested_path: Option<&Path>) -> Result<Vec<PcDiagnostic>> {
    // Try ruff first, fallback to pyright
    let ruff_result = timeout(
        Duration::from_secs(config.security_policy.max_command_seconds),
        Command::new("ruff")
            .args(["check", "--output-format=json", "."])
            .current_dir(&config.workspace_root)
            .output(),
    )
    .await;

    if let Ok(Ok(output)) = ruff_result {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut diagnostics = Vec::new();
        if let Ok(ruff_diags) = serde_json::from_str::<Vec<RuffDiagnostic>>(&stdout) {
            for d in ruff_diags {
                diagnostics.push(d.to_pc_diagnostic(&config.workspace_root));
            }
        }
        if !diagnostics.is_empty() || output.status.success() {
            return Ok(diagnostics);
        }
    }

    // Fallback: try pyright
    let pyright_result = timeout(
        Duration::from_secs(config.security_policy.max_command_seconds),
        Command::new("pyright")
            .args(["--outputjson"])
            .current_dir(&config.workspace_root)
            .output(),
    )
    .await;

    if let Ok(Ok(output)) = pyright_result {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut diagnostics = Vec::new();
        if let Ok(pyright) = serde_json::from_str::<PyrightOutput>(&stdout) {
            for d in pyright.general_diagnostics {
                diagnostics.push(PcDiagnostic {
                    path: d.file.replace('\\', "/"),
                    line: d.range.start.line,
                    column: d.range.start.column,
                    severity: pyright_severity(d.severity.as_deref().unwrap_or("error")),
                    message: d.message,
                    source: Some("pyright".to_string()),
                });
            }
        }
        return Ok(diagnostics);
    }

    Ok(Vec::new())
}

#[derive(Clone, Debug, serde::Deserialize)]
struct RuffDiagnostic {
    location: RuffLocation,
    code: Option<String>,
    message: String,
    filename: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct RuffLocation {
    row: u32,
    column: u32,
}

impl RuffDiagnostic {
    fn to_pc_diagnostic(self, workspace_root: &Path) -> PcDiagnostic {
        let path = PathBuf::from(&self.filename);
        let relative = if let Ok(rel) = path.strip_prefix(workspace_root) {
            rel.to_path_buf()
        } else {
            path
        };
        PcDiagnostic {
            path: relative.to_string_lossy().replace('\\', "/"),
            line: self.location.row,
            column: self.location.column,
            severity: PcDiagnosticSeverity::Error,
            message: if let Some(code) = self.code {
                format!("{}: {}", code, self.message)
            } else {
                self.message
            },
            source: Some("ruff".to_string()),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
struct PyrightOutput {
    general_diagnostics: Vec<PyrightDiagnostic>,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct PyrightDiagnostic {
    file: String,
    severity: Option<String>,
    message: String,
    range: PyrightRange,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct PyrightRange {
    start: PyrightPosition,
}

#[derive(Clone, Debug, serde::Deserialize)]
struct PyrightPosition {
    line: u32,
    column: u32,
}

fn pyright_severity(s: &str) -> PcDiagnosticSeverity {
    match s {
        "error" => PcDiagnosticSeverity::Error,
        "warning" => PcDiagnosticSeverity::Warning,
        "information" => PcDiagnosticSeverity::Info,
        _ => PcDiagnosticSeverity::Hint,
    }
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

async fn logs_handler(
    State(state): State<PcHostState>,
    headers: HeaderMap,
) -> Result<Json<PcGatewayLogs>, (StatusCode, String)> {
    if let Err(error) = authorize(&state.config, &headers) {
        return Err((StatusCode::UNAUTHORIZED, error.to_string()));
    }
    let log_ring = state.log_ring.lock().await;
    Ok(Json(log_ring.snapshot()))
}

// ── Snapshot handlers ──

async fn snapshot_create(config: &PcHostConfig, workspace_id: &str, reason: &str) -> Result<PcGatewayResponse> {
    let workspace = deepseek_mobile_core::Workspace::new(
        workspace_id, workspace_id, config.workspace_root.clone(),
        deepseek_mobile_core::ExecutorKind::PcGateway,
    );
    let store_root = config.workspace_root.join(".deepseek-mobile").join("snapshots");
    let service = deepseek_mobile_core::WorkspaceSnapshotService::new(workspace, store_root);
    let reason = reason.to_string();
    let record = tokio::task::spawn_blocking(move || service.create_snapshot(reason))
        .await
        .map_err(|e| anyhow::anyhow!("snapshot create blocked: {}", e))??;
    Ok(PcGatewayResponse::SnapshotRecord(record))
}

async fn snapshot_restore(config: &PcHostConfig, workspace_id: &str, snapshot_id: &str) -> Result<PcGatewayResponse> {
    let workspace = deepseek_mobile_core::Workspace::new(
        workspace_id, workspace_id, config.workspace_root.clone(),
        deepseek_mobile_core::ExecutorKind::PcGateway,
    );
    let store_root = config.workspace_root.join(".deepseek-mobile").join("snapshots");
    let service = deepseek_mobile_core::WorkspaceSnapshotService::new(workspace, store_root);
    let sid = snapshot_id.to_string();
    let report = tokio::task::spawn_blocking(move || service.restore_snapshot(&sid))
        .await
        .map_err(|e| anyhow::anyhow!("snapshot restore blocked: {}", e))??;
    Ok(PcGatewayResponse::SnapshotRestoreReport(report))
}

async fn snapshot_list(config: &PcHostConfig, workspace_id: &str) -> Result<PcGatewayResponse> {
    let workspace = deepseek_mobile_core::Workspace::new(
        workspace_id, workspace_id, config.workspace_root.clone(),
        deepseek_mobile_core::ExecutorKind::PcGateway,
    );
    let store_root = config.workspace_root.join(".deepseek-mobile").join("snapshots");
    let service = deepseek_mobile_core::WorkspaceSnapshotService::new(workspace, store_root);
    let records = tokio::task::spawn_blocking(move || service.list_snapshots())
        .await
        .map_err(|e| anyhow::anyhow!("snapshot list blocked: {}", e))??;
    Ok(PcGatewayResponse::SnapshotList(records))
}

fn request_operation_name(request: &PcGatewayRequest) -> String {
    match request {
        PcGatewayRequest::Health => "health".to_string(),
        PcGatewayRequest::ListWorkspaces => "list_workspaces".to_string(),
        PcGatewayRequest::ListEnvironments { .. } => "list_environments".to_string(),
        PcGatewayRequest::DetectTasks { .. } => "detect_tasks".to_string(),
        PcGatewayRequest::IndexWorkspace { .. } => "index_workspace".to_string(),
        PcGatewayRequest::ReadFile { .. } => "read_file".to_string(),
        PcGatewayRequest::WriteFile { .. } => "write_file".to_string(),
        PcGatewayRequest::DeleteFile { .. } => "delete_file".to_string(),
        PcGatewayRequest::ListDir { .. } => "list_dir".to_string(),
        PcGatewayRequest::OpenTerminal { .. } => "open_terminal".to_string(),
        PcGatewayRequest::TerminalInput { .. } => "terminal_input".to_string(),
        PcGatewayRequest::CloseTerminal { .. } => "close_terminal".to_string(),
        PcGatewayRequest::ExecuteCommand { .. } => "execute_command".to_string(),
        PcGatewayRequest::RunTask { .. } => "run_task".to_string(),
        PcGatewayRequest::StopTask { .. } => "stop_task".to_string(),
        PcGatewayRequest::StartDevServer { .. } => "start_dev_server".to_string(),
        PcGatewayRequest::StopDevServer { .. } => "stop_dev_server".to_string(),
        PcGatewayRequest::GetDiagnostics { .. } => "get_diagnostics".to_string(),
        PcGatewayRequest::GitStatus { .. } => "git_status".to_string(),
        PcGatewayRequest::GitDiff { .. } => "git_diff".to_string(),
        PcGatewayRequest::GitCommit { .. } => "git_commit".to_string(),
        PcGatewayRequest::GitPush { .. } => "git_push".to_string(),
        PcGatewayRequest::GitPull { .. } => "git_pull".to_string(),
        PcGatewayRequest::GitBranch { .. } => "git_branch".to_string(),
        PcGatewayRequest::SnapshotCreate { .. } => "snapshot_create".to_string(),
        PcGatewayRequest::SnapshotRestore { .. } => "snapshot_restore".to_string(),
        PcGatewayRequest::SnapshotList { .. } => "snapshot_list".to_string(),
    }
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
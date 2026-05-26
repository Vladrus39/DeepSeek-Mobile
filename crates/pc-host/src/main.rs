use anyhow::{anyhow, Context, Result};
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use std::convert::Infallible;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio_stream::wrappers::ReceiverStream;
use tokio::sync::broadcast;
use deepseek_mobile_core::{
    CommandOutput, CommandRequest, LogRing, PcDiagnostic, PcDiagnosticSeverity,
    PcEnvironmentDescriptor, PcGatewayCapability, PcGatewayConnectionStatus, PcGatewayDirEntry,
    PcGatewayError, PcGatewayHealth, PcGatewayLogEntry, PcGatewayLogs, PcGatewayRequest,
    PcGatewayRequestEnvelope, PcGatewayResponse, PcGatewayResponseEnvelope,
    PcGatewaySecurityPolicy, PolicyPreset, PcRunningTaskEvent, PcRunningTaskInfo, PcTaskDescriptor, PcTaskKind,
    PcTerminalSession, PcWorkspaceGrant,
};
use serde::Deserialize;
use std::env;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;
use std::collections::HashMap;

#[derive(Clone)]
struct PcHostState {
    config: Arc<PcHostConfig>,
    terminals: Arc<Mutex<HashMap<String, TerminalHandle>>>,
    tasks: Arc<Mutex<HashMap<String, TaskHandle>>>,
    task_events: Arc<Mutex<Option<broadcast::Sender<PcRunningTaskEvent>>>>,
    log_ring: Arc<Mutex<LogRing>>,
    start_time: std::time::Instant,
}

/// Handle for a tracked background task process.
struct TaskHandle {
    id: Arc<String>,
    label: Arc<String>,
    kind: Arc<String>,
    child: Arc<Mutex<Option<Child>>>,
    log_path: PathBuf,
    started_at_unix: u64,
}



#[derive(Clone, Debug)]
struct PcHostConfig {
    gateway_id: String,
    gateway_label: String,
    bind_addr: SocketAddr,
    auth_token: Option<String>,
    workspace: PcWorkspaceGrant,
    workspace_root: PathBuf,
    trusted_paths: Vec<PathBuf>,
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

        let trusted_paths = env::var("DEEPSEEK_PC_HOST_TRUSTED_PATHS")
            .ok()
            .map(|raw| parse_trusted_paths(&raw))
            .unwrap_or_default();

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
            trusted_paths,
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

        let candidate = PathBuf::from(path);
        if candidate.is_absolute() {
            return Ok(candidate);
        }

        for component in candidate.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                Component::ParentDir => return Err(anyhow!("parent path segments are not accepted: {}", path)),
                Component::RootDir | Component::Prefix(_) => {
                    return Err(anyhow!("root or prefix path segments are not accepted: {}", path));
                }
            }
        }

        Ok(self.workspace_root.join(candidate))
    }

    async fn ensure_path_allowed(&self, path: &Path) -> Result<PathBuf> {
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

        if canonical.starts_with(&self.workspace_root) {
            return Ok(canonical);
        }
        for trusted in &self.trusted_paths {
            if canonical.starts_with(trusted) {
                return Ok(canonical);
            }
        }
        Err(anyhow!(
            "path is outside workspace and trusted grants: {}",
            canonical.display()
        ))
    }

    async fn ensure_parent_allowed(&self, path: &Path) -> Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow!("path has no parent: {}", path.display()))?;
        let existing_parent = nearest_existing_parent(parent)?;
        self.ensure_path_allowed(&existing_parent).await?;
        Ok(())
    }

    fn gateway_relative_path(&self, absolute: &Path) -> String {
        if let Ok(rel) = absolute.strip_prefix(&self.workspace_root) {
            return rel.to_string_lossy().replace('\\', "/");
        }
        for trusted in &self.trusted_paths {
            if let Ok(rel) = absolute.strip_prefix(trusted) {
                return format!(
                    "@trusted/{}",
                    rel.to_string_lossy().replace('\\', "/")
                );
            }
        }
        absolute.to_string_lossy().replace('\\', "/")
    }
}

fn parse_trusted_paths(raw: &str) -> Vec<PathBuf> {
    raw.split('|')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .filter_map(|segment| PathBuf::from(segment).canonicalize().ok())
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = PcHostConfig::from_env()?;
    let bind_addr = config.bind_addr;
    let gateway_label = config.gateway_label.clone();
    let (task_events_tx, _) = broadcast::channel::<PcRunningTaskEvent>(32);
    let state = PcHostState {
        config: Arc::new(config),
        terminals: Arc::new(Mutex::new(HashMap::new())),
        tasks: Arc::new(Mutex::new(HashMap::new())),
        task_events: Arc::new(Mutex::new(Some(task_events_tx))),
        log_ring: Arc::new(Mutex::new(LogRing::new(200))),
        start_time: std::time::Instant::now(),
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/gateway/request", post(gateway_request_handler))
        .route("/v1/gateway/exec/stream", post(exec_stream_handler))
        .route("/v1/gateway/logs", get(logs_handler))
        .route("/v1/runtime/tasks", get(runtime_tasks_handler))
        .route("/v1/runtime/tasks/events", get(runtime_task_events_handler))
        .route("/v1/runtime/tasks/{task_id}/log", get(runtime_task_log_handler))
        .with_state(state);
    // Note: /v1/runtime/tasks and log routes return task status data
    // that survives the mobile app restart because the pc-host keeps tasks in memory.

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
                Ok(resolved) => match state.config.ensure_path_allowed(&resolved).await {
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
        PcGatewayRequest::RunTask { task_id } => run_task_handler(state, &task_id).await,
        PcGatewayRequest::StopTask { task_id } => stop_task_handler(state, &task_id).await,
        PcGatewayRequest::ListTasks => list_tasks_handler(state).await,
        PcGatewayRequest::OpenPath { workspace_id, path } => {
            open_path_in_os(&state.config, &workspace_id, &path).await
        }
        unsupported => Ok(PcGatewayResponse::Error(PcGatewayError::new(
            "unsupported_request",
            format!("request is not implemented by this PC host build: {:?}", unsupported),
        ))),
    }
}

async fn open_path_in_os(config: &PcHostConfig, workspace_id: &str, path: &str) -> Result<PcGatewayResponse> {
    let requested = config.resolve_workspace_path(workspace_id, path)?;
    let allowed = config.ensure_path_allowed(&requested).await?;
    launch_path_in_os_shell(&allowed)?;
    let display = config.gateway_relative_path(&allowed);
    Ok(PcGatewayResponse::PathOpened { path: display })
}

fn launch_path_in_os_shell(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .with_context(|| format!("open {} in Explorer", path.display()))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .with_context(|| format!("open {} with macOS open", path.display()))?;
        return Ok(());
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .with_context(|| format!("open {} with xdg-open", path.display()))?;
        return Ok(());
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        let _ = path;
        Err(anyhow!("open_path is not supported on this OS"))
    }
}

async fn list_dir(config: &PcHostConfig, workspace_id: &str, path: &str) -> Result<PcGatewayResponse> {
    let path = config.resolve_workspace_path(workspace_id, path)?;
    let path = config.ensure_path_allowed(&path).await?;
    let mut entries = fs::read_dir(&path)
        .await
        .with_context(|| format!("read dir {}", path.display()))?;
    let mut out = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let metadata = entry.metadata().await?;
        let absolute = entry.path();
        let relative = config.gateway_relative_path(&absolute);
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
    let path = config.ensure_path_allowed(&requested).await?;
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
    config.ensure_parent_allowed(&requested).await?;
    if let Some(parent) = requested.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("create parent dir {}", parent.display()))?;
    }
    let path = config.ensure_path_allowed(&requested).await?;
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
    let path = config.ensure_path_allowed(&requested).await?;
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
            config.ensure_path_allowed(&requested).await?
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
            config.ensure_path_allowed(&resolved).await?
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
        let checked = config.ensure_path_allowed(&requested).await?;
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

// ── Background task handlers ──

async fn run_task_handler(state: &PcHostState, task_id: &str) -> Result<PcGatewayResponse> {
    // Detect available tasks for the workspace
    let workspace_id = &state.config.workspace.id;
    let detected = detect_tasks(&state.config, workspace_id).await?;
    let PcGatewayResponse::Tasks(tasks) = detected else {
        return Ok(PcGatewayResponse::Error(PcGatewayError::new(
            "no_tasks",
            "unable to detect tasks for this workspace",
        )));
    };

    let Some(task) = tasks.into_iter().find(|t| t.id == task_id) else {
        return Ok(PcGatewayResponse::Error(PcGatewayError::new(
            "task_not_found",
            format!("no task found with id '{}'", task_id),
        )));
    };

    // If already running, return the existing handle
    let mut handles = state.tasks.lock().await;
    if handles.contains_key(task_id) {
        return Ok(PcGatewayResponse::Error(PcGatewayError::new(
            "task_already_running",
            format!("task '{}' is already running", task_id),
        )));
    }

    // Prepare log path before spawning
    let log_dir = state.config.workspace_root.join(".deepseek-mobile").join("tasks").join("logs");
    tokio::fs::create_dir_all(&log_dir).await?;
    let log_path = log_dir.join(format!("{}.log", task_id));

    let command = &task.command;
    let mut child = tokio::process::Command::new(&command.program)
        .args(&command.args)
        .current_dir(
            command
                .working_dir
                .clone()
                .unwrap_or_else(|| state.config.workspace_root.clone()),
        )
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn task '{}'", task_id))?;

    let pid = child.id().unwrap_or(0);

    // Spawn background output collector
    let task_id_arc = Arc::new(task_id.to_string());
    let log_path_clone = log_path.clone();

    if let Some(stdout) = child.stdout.take() {
        let tid = task_id_arc.clone();
        let lp = log_path_clone.clone();
        tokio::spawn(async move {
            pipe_to_log(tid, lp, stdout).await;
        });
    }
    if let Some(stderr) = child.stderr.take() {
        let tid = task_id_arc.clone();
        let lp = log_path_clone;
        tokio::spawn(async move {
            pipe_to_log(tid, lp, stderr).await;
        });
    }

    // Wrap child so the handle and watcher can share ownership.
    let child_shared = Arc::new(Mutex::new(Some(child)));

    let label = Arc::new(task.label.clone());
    let kind = Arc::new(format!("{:?}", task.kind));
    let started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Write initial log line before moving log_path into the handle
    if let Err(e) = log_line_to_file(&log_path, &format!("Task started (pid: {})", pid)).await {
        tracing::warn!("failed to write task log: {}", e);
    }

    let handle = TaskHandle {
        id: task_id_arc.clone(),
        label,
        kind,
        child: child_shared.clone(),
        log_path: log_path.clone(),
        started_at_unix: started_at,
    };

    handles.insert(task_id.to_string(), handle);

    // Emit TaskStarted event through broadcast.
    {
        let events = state.task_events.lock().await;
        if let Some(ref tx) = *events {
            let _ = tx.send(PcRunningTaskEvent::TaskStarted(PcRunningTaskInfo {
                id: task_id.to_string(),
                label: task.label.clone(),
                kind: format!("{:?}", task.kind),
                started_at_unix: started_at,
            }));
        }
    }

    // Spawn background watcher to await child completion.
    spawn_task_watcher(state, task_id.to_string(), child_shared, log_path.clone());

    Ok(PcGatewayResponse::TaskStarted {
        task_id: task_id.to_string(),
        process_id: pid.to_string(),
    })
}

async fn stop_task_handler(state: &PcHostState, task_id: &str) -> Result<PcGatewayResponse> {
    let mut handles = state.tasks.lock().await;
    let Some(handle) = handles.remove(task_id) else {
        return Ok(PcGatewayResponse::Error(PcGatewayError::new(
            "task_not_running",
            format!("task '{}' is not running", task_id),
        )));
    };

    // Kill the child process.
    {
        let mut child_lock = handle.child.lock().await;
        if let Some(ref mut child) = *child_lock {
            let _ = child.start_kill();
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await;
        }
        *child_lock = None;
    }

    if let Err(e) = log_line_to_file(&handle.log_path, "Task stopped").await {
        tracing::warn!("failed to write task stop log: {}", e);
    }

    // Emit TaskStopped event.
    emit_task_event(state, PcRunningTaskEvent::TaskStopped { task_id: task_id.to_string() }).await;

    Ok(PcGatewayResponse::TaskStopped {
        task_id: task_id.to_string(),
    })
}

async fn list_tasks_handler(state: &PcHostState) -> Result<PcGatewayResponse> {
    let handles = state.tasks.lock().await;
    let running: Vec<PcRunningTaskInfo> = handles
        .values()
        .map(|h| PcRunningTaskInfo {
            id: h.id.to_string(),
            label: h.label.to_string(),
            kind: h.kind.to_string(),
            started_at_unix: h.started_at_unix,
        })
        .collect();
    Ok(PcGatewayResponse::TaskList(running))
}

/// Send a task event through the broadcast channel (best-effort).
async fn emit_task_event(state: &PcHostState, event: PcRunningTaskEvent) {
    let events = state.task_events.lock().await;
    if let Some(ref tx) = *events {
        let _ = tx.send(event);
    }
}

/// Background watcher: waits for the child process to exit, then emits
/// a completion / failure event and cleans up the handle.
fn spawn_task_watcher(
    state: &PcHostState,
    task_id: String,
    child_shared: Arc<Mutex<Option<Child>>>,
    log_path: PathBuf,
) {
    let state_clone = state.clone();
    let tid = task_id.clone();
    tokio::spawn(async move {
        // Take ownership of the child from the shared slot.
        let child_opt = {
            let mut lock = child_shared.lock().await;
            lock.take()
        };

        let exit_code = if let Some(mut child) = child_opt {
            match child.wait().await {
                Ok(status) => status.code(),
                Err(e) => {
                    let _ = log_line_to_file(&log_path, &format!("Task error: {}", e)).await;
                    emit_task_event(
                        &state_clone,
                        PcRunningTaskEvent::TaskFailed {
                            task_id: tid.clone(),
                            error: e.to_string(),
                        },
                    )
                    .await;
                    // Remove handle from the tasks map.
                    state_clone.tasks.lock().await.remove(&tid);
                    return;
                }
            }
        } else {
            // Child was already taken (e.g. by stop_task_handler).
            let _ = log_line_to_file(&log_path, "Task ended (stopped externally)").await;
            state_clone.tasks.lock().await.remove(&tid);
            return;
        };

        let _ = log_line_to_file(&log_path, &format!("Task completed with exit code {:?}", exit_code)).await;

        emit_task_event(
            &state_clone,
            PcRunningTaskEvent::TaskCompleted {
                task_id: tid.clone(),
                exit_code,
            },
        )
        .await;

        // Remove handle from the tasks map.
        state_clone.tasks.lock().await.remove(&tid);
    });
}

/// GET /v1/runtime/tasks/events — SSE stream of task state changes.
async fn runtime_task_events_handler(
    State(state): State<PcHostState>,
) -> Sse<ReceiverStream<Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(16);

    let task_events = state.task_events.clone();
    tokio::spawn(async move {
        let rx_opt = {
            let lock = task_events.lock().await;
            lock.as_ref().map(|sender| sender.subscribe())
        };

        let Some(mut broadcast_rx) = rx_opt else {
            let _ = tx.send(Ok(Event::default().data(r#"{"kind":"error","data":"no broadcast channel"}"#))).await;
            return;
        };

        loop {
            match broadcast_rx.recv().await {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap_or_default();
                    if tx.send(Ok(Event::default().data(data))).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    let _ = tx.send(Ok(Event::default().data(format!(r#"{{"kind":"lagged","skipped":{}}}"#, n)))).await;
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });

    Sse::new(ReceiverStream::new(rx))
}

// ── Task output logging ──

/// Pipe lines from a tokio AsyncRead stream into a task log file.
#[allow(unused_variables)]
async fn pipe_to_log(task_id: Arc<String>, log_path: PathBuf, reader: impl tokio::io::AsyncRead + Unpin) {
    let mut lines = tokio::io::BufReader::new(reader).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.is_empty() {
            continue;
        }
        let _ = log_line_to_file(&log_path, &line).await;
    }
    let _ = log_line_to_file(&log_path, "-- stdio stream ended --").await;
}

/// Write a single line to a log file (appending).
async fn log_line_to_file(path: &Path, line: &str) -> std::io::Result<()> {
    let unix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let formatted = format!("[{}] {}\n", unix, line);
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    file.write_all(formatted.as_bytes()).await?;
    Ok(())
}

// ── Runtime API handlers ──

#[derive(Clone, Debug, serde::Serialize)]
struct RuntimeTaskInfo {
    id: String,
    label: String,
    kind: String,
    started_at_unix: u64,
}

/// GET /v1/runtime/tasks — list currently running tasks
async fn runtime_tasks_handler(
    State(state): State<PcHostState>,
) -> (StatusCode, Json<Vec<RuntimeTaskInfo>>) {
    let handles = state.tasks.lock().await;
    let tasks: Vec<RuntimeTaskInfo> = handles
        .values()
        .map(|h| RuntimeTaskInfo {
            id: h.id.to_string(),
            label: h.label.to_string(),
            kind: h.kind.to_string(),
            started_at_unix: h.started_at_unix,
        })
        .collect();
    (StatusCode::OK, Json(tasks))
}

/// GET /v1/runtime/tasks/{task_id}/log — stream task log file
async fn runtime_task_log_handler(
    State(state): State<PcHostState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> Result<String, (StatusCode, String)> {
    let log_path = {
        let handles = state.tasks.lock().await;
        let Some(handle) = handles.get(&task_id) else {
            return Err((StatusCode::NOT_FOUND, format!("task '{}' not found", task_id)));
        };
        handle.log_path.clone()
    };

    if !log_path.exists() {
        return Err((StatusCode::NOT_FOUND, "log file not found".to_string()));
    }

    let content = tokio::fs::read_to_string(&log_path)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("read log: {}", e)))?;
    Ok(content)
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
        PcGatewayRequest::ListTasks => "list_tasks".to_string(),
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
        PcGatewayRequest::OpenPath { .. } => "open_path".to_string(),
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

    #[test]
    fn policy_preset_readonly_blocks_shell_execution() {
        use deepseek_mobile_core::{CommandRequest, PolicyPreset};
        let policy =
            deepseek_mobile_core::PcGatewaySecurityPolicy::from_preset(PolicyPreset::ReadOnly);
        let blocked = CommandRequest::new("cargo", vec!["check".to_string()]);
        assert!(policy.validate_command(&blocked).is_err());
        assert!(!policy.allows_program("git"));
    }
}

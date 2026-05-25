//! Tool execution coordinator.
//!
//! Android remains the orchestrator: it parses the model response, applies
//! approval policy, records timeline events and chooses the active backend. The
//! actual tool execution depends on the workspace executor. For a PC gateway
//! workspace, file/shell/git operations are sent to the PC; they are not executed
//! on Android.

use crate::executor::{CommandRequest, TermuxExecRequest};
use crate::pc_gateway::{CommandStreamEvent, PcDiagnostic, PcGatewayResponse};
use crate::pc_gateway_client::PcGatewayClient;
use crate::tool_call::ToolCallRequest;
use crate::tools::{ToolContext, ToolRegistry, ToolResult};
use crate::workspace::ExecutorKind;
use crate::workspace_diagnostics::WorkspaceDiagnosticsService;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolExecutionTarget {
    LocalAndroid,
    Termux,
    PcGateway,
    RemoteYlit,
}

impl From<&ExecutorKind> for ToolExecutionTarget {
    fn from(kind: &ExecutorKind) -> Self {
        match kind {
            ExecutorKind::LocalAndroid => ToolExecutionTarget::LocalAndroid,
            ExecutorKind::Termux => ToolExecutionTarget::Termux,
            ExecutorKind::PcGateway => ToolExecutionTarget::PcGateway,
            ExecutorKind::RemoteYlit => ToolExecutionTarget::RemoteYlit,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolExecutionRoute {
    pub target: ToolExecutionTarget,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum RemotePatchOperation {
    Replace {
        path: String,
        search: String,
        replace: String,
        expected_occurrences: Option<usize>,
    },
    Create {
        path: String,
        content: String,
        overwrite: bool,
    },
    Append {
        path: String,
        content: String,
    },
    Delete {
        path: String,
    },
}

impl RemotePatchOperation {
    fn path(&self) -> &str {
        match self {
            Self::Replace { path, .. }
            | Self::Create { path, .. }
            | Self::Append { path, .. }
            | Self::Delete { path } => path,
        }
    }
}

pub struct ToolExecutionCoordinator<'a> {
    registry: &'a ToolRegistry,
    pc_gateway: Option<&'a PcGatewayClient>,
}

impl<'a> ToolExecutionCoordinator<'a> {
    pub fn new(registry: &'a ToolRegistry) -> Self {
        Self {
            registry,
            pc_gateway: None,
        }
    }

    pub fn with_pc_gateway(mut self, client: &'a PcGatewayClient) -> Self {
        self.pc_gateway = Some(client);
        self
    }

    pub fn route(&self, call: &ToolCallRequest, context: &ToolContext) -> ToolExecutionRoute {
        match ToolExecutionTarget::from(&context.workspace.executor) {
            ToolExecutionTarget::LocalAndroid => ToolExecutionRoute {
                target: ToolExecutionTarget::LocalAndroid,
                reason: format!("tool '{}' will run through local Android workspace", call.name),
            },
            ToolExecutionTarget::Termux => ToolExecutionRoute {
                target: ToolExecutionTarget::Termux,
                reason: format!("tool '{}' will run through Termux workspace", call.name),
            },
            ToolExecutionTarget::PcGateway => ToolExecutionRoute {
                target: ToolExecutionTarget::PcGateway,
                reason: format!("tool '{}' will be sent to the paired PC gateway", call.name),
            },
            ToolExecutionTarget::RemoteYlit => ToolExecutionRoute {
                target: ToolExecutionTarget::RemoteYlit,
                reason: format!("tool '{}' will be sent to remote Y-lit runtime", call.name),
            },
        }
    }

    pub async fn execute(&self, call: &ToolCallRequest, context: &ToolContext) -> Result<ToolResult> {
        // Phone-side network tools always run locally, even when the active
        // workspace is a paired PC project.
        if runs_on_phone_regardless_of_workspace(&call.name) {
            let result = self.registry.execute(&call.name, call.arguments.clone(), context)?;
            return Ok(result);
        }

        match self.route(call, context).target {
            ToolExecutionTarget::Termux if call.name == "exec_shell" => {
                self.execute_on_termux(call, context).await
            }
            ToolExecutionTarget::LocalAndroid | ToolExecutionTarget::Termux => {
                let mut result = self.registry.execute(&call.name, call.arguments.clone(), context)?;
                if result.success && should_run_local_post_edit_diagnostics(call) {
                    attach_local_post_edit_diagnostics(
                        context,
                        extract_primary_path_for_diagnostics(call),
                        &mut result,
                    )
                    .await?;
                }
                Ok(result)
            }
            ToolExecutionTarget::PcGateway => self.execute_on_pc_gateway(call, context).await,
            ToolExecutionTarget::RemoteYlit => Err(anyhow!(
                "remote Y-lit tool execution is not wired yet for tool '{}'",
                call.name
            )),
        }
    }

    async fn execute_on_termux(&self, call: &ToolCallRequest, context: &ToolContext) -> Result<ToolResult> {
        let command = required_str(&call.arguments, "command")?.trim();
        if command.is_empty() {
            return Err(anyhow!("empty command"));
        }

        let timeout_secs = optional_u64(&call.arguments, "timeout_secs");
        let request = TermuxExecRequest::new(
            termux_request_id(call, command, &context.workspace.root),
            command,
            context.workspace.root.clone(),
        )
        .with_timeout_secs(timeout_secs);
        let request_id = request.request_id.clone();
        let working_dir = request.working_dir.display().to_string();
        Ok(ToolResult::success(format!(
            "Termux shell execution queued for Android native bridge. request_id={} working_dir={} command={}",
            request_id, working_dir, command
        ))
        .with_metadata(json!({
            "executor": "termux",
            "native_command": "RunTermuxCommand",
            "termux_execution_pending": true,
            "termux_execution_status": "pending_native_bridge",
            "termux_request_id": request_id,
            "termux_exec_request": request,
        })))
    }

    async fn execute_on_pc_gateway(&self, call: &ToolCallRequest, context: &ToolContext) -> Result<ToolResult> {
        let client = self
            .pc_gateway
            .ok_or_else(|| anyhow!("PC gateway workspace selected, but no PcGatewayClient is attached"))?;
        let workspace_id = context.workspace.id.clone();

        match call.name.as_str() {
            "read_file" => {
                let path = required_str(&call.arguments, "path")?;
                gateway_response_to_tool_result(client.read_file(workspace_id, path).await?)
            }
            "write_file" => {
                let path = required_str(&call.arguments, "path")?;
                let content = required_str(&call.arguments, "content")?;
                let response = client.write_file(workspace_id.clone(), path, content).await?;
                let mut result = gateway_response_to_tool_result(response)?;
                attach_post_edit_diagnostics(client, &workspace_id, Some(path.to_string()), &mut result).await?;
                Ok(result)
            }
            "delete_file" => {
                let path = required_str(&call.arguments, "path")?;
                gateway_response_to_tool_result(client.delete_file(workspace_id, path).await?)
            }
            "list_dir" => {
                let path = optional_str(&call.arguments, "path").unwrap_or(".");
                gateway_response_to_tool_result(client.list_dir(workspace_id, path).await?)
            }
            "edit_file" => self.execute_remote_edit_file(client, workspace_id, &call.arguments).await,
            "apply_patch" => self.execute_remote_apply_patch(client, workspace_id, &call.arguments).await,
            "exec_shell" => {
                let command = required_str(&call.arguments, "command")?;
                let request = command_request_from_shell(command, context.workspace.root.clone())?;
                let mut rx = client.stream_command(workspace_id, request).await?;
                let mut stdout = String::new();
                let mut stderr = String::new();
                let mut exit_code: Option<i32> = None;
                let mut error_msg: Option<String> = None;
                while let Some(event) = rx.recv().await {
                    match event {
                        CommandStreamEvent::Stdout(line) => {
                            stdout.push_str(&line);
                            stdout.push('\n');
                        }
                        CommandStreamEvent::Stderr(line) => {
                            stderr.push_str(&line);
                            stderr.push('\n');
                        }
                        CommandStreamEvent::Exit(code) => exit_code = code,
                        CommandStreamEvent::Error(msg) => error_msg = Some(msg),
                    }
                }
                if let Some(err) = error_msg {
                    Ok(ToolResult::error(err))
                } else {
                    let combined = format!(
                        "{}EXIT_CODE: {}\n\n{}",
                        stdout,
                        exit_code.map_or_else(|| "unknown".to_string(), |c| c.to_string()),
                        stderr
                    );
                    Ok(ToolResult::success(combined))
                }

            }
            "git" => {
                let operation = required_str(&call.arguments, "operation")?;
                match operation {
                    "status" => gateway_response_to_tool_result(client.git_status(workspace_id).await?),
                    "diff" => gateway_response_to_tool_result(client.git_diff(workspace_id).await?),
                    "commit" => {
                        let message = required_str(&call.arguments, "message")?;
                        gateway_response_to_tool_result(client.git_commit(workspace_id, message).await?)
                    }
                    "push" => {
                        let remote = optional_str(&call.arguments, "remote").map(String::from);
                        let branch = optional_str(&call.arguments, "branch").map(String::from);
                        gateway_response_to_tool_result(client.git_push(workspace_id, remote, branch).await?)
                    }
                    "pull" => {
                        let remote = optional_str(&call.arguments, "remote").map(String::from);
                        let branch = optional_str(&call.arguments, "branch").map(String::from);
                        gateway_response_to_tool_result(client.git_pull(workspace_id, remote, branch).await?)
                    }
                    "branch" => gateway_response_to_tool_result(client.git_branch(workspace_id).await?),
                    other => {
                        let args = optional_str(&call.arguments, "args").unwrap_or("");
                        let mut command_args = vec![other.to_string()];
                        command_args.extend(shell_words(args));
                        let request = CommandRequest {
                            program: "git".to_string(),
                            args: command_args,
                            working_dir: Some(context.workspace.root.clone()),
                        };
                        gateway_response_to_tool_result(client.execute_command(workspace_id, request, None).await?)
                    }
                }
            }
            "snapshot_create" => {
                let reason = optional_str(&call.arguments, "reason").unwrap_or("manual snapshot");
                gateway_response_to_tool_result(client.create_snapshot(workspace_id, reason).await?)
            }
            "snapshot_list" => {
                gateway_response_to_tool_result(client.list_snapshots(workspace_id).await?)
            }
            "snapshot_restore" => {
                let snapshot_id = required_str(&call.arguments, "snapshot_id")?;
                gateway_response_to_tool_result(client.restore_snapshot(workspace_id, snapshot_id).await?)
            }
            "detect_tasks" => {
                gateway_response_to_tool_result(client.detect_tasks(workspace_id).await?)
            }
            "task_run" => {
                let task_id = required_str(&call.arguments, "task_id")?;
                gateway_response_to_tool_result(client.run_task(task_id).await?)
            }
            "task_stop" => {
                let task_id = required_str(&call.arguments, "task_id")?;
                gateway_response_to_tool_result(client.stop_task(task_id).await?)
            }
            "task_list" => {
                gateway_response_to_tool_result(client.list_tasks().await?)
            }
            other => Err(anyhow!(
                "tool '{}' is not yet mapped to PC gateway execution",
                other
            )),
        }
    }

    async fn execute_remote_edit_file(
        &self,
        client: &PcGatewayClient,
        workspace_id: String,
        arguments: &Value,
    ) -> Result<ToolResult> {
        let path = required_str(arguments, "path")?;
        let search = required_str(arguments, "search")?;
        let replace = required_str(arguments, "replace")?;
        if search.is_empty() {
            return Err(anyhow!("search text must not be empty"));
        }

        let read = client.read_file(workspace_id.clone(), path).await?;
        let PcGatewayResponse::FileContent { content: original, .. }  = read else {
            return Err(anyhow!("read file before edit returned unexpected response"));
        };
        let backup = json!({"original": original});

        let replaced = original.replacen(&search, &replace, 1);
        if replaced == original {
            return Err(anyhow!(
                "edit_file: search string not found in '{}' from workspace '{}'",
                path,
                workspace_id
            ));
        }
        let response = client.write_file(workspace_id.clone(), path, &replaced).await?;
        let mut result = gateway_response_to_tool_result(response)?;
        result.metadata = Some(json!({"backup": backup}));
        attach_post_edit_diagnostics(client, &workspace_id, Some(path.to_string()), &mut result).await?;
        Ok(result)
    }

    async fn execute_remote_apply_patch(
        &self,
        client: &PcGatewayClient,
        workspace_id: String,
        arguments: &Value,
    ) -> Result<ToolResult> {
        // Parse patch operations from the apply_patch arguments
        let operations_value = crate::tools::patch::normalized_operations_value(arguments)?;
        let operations: Vec<RemotePatchOperation> = serde_json::from_value(operations_value)
            .map_err(|e| anyhow!("failed to parse apply_patch operations: {}", e))?;

        if operations.is_empty() {
            return Err(anyhow!("apply_patch requires at least one operation"));
        }

        // Backup all affected files
        let mut backups = BTreeMap::new();
        for op in &operations {
            let path = op.path();
            if !backups.contains_key(path) {
                let read = client.read_file(workspace_id.clone(), path).await;
                match read {
                    Ok(PcGatewayResponse::FileContent { content, .. }) => {
                        backups.insert(path.to_string(), content);
                    }
                    Ok(_) => return Err(anyhow!("read file before apply_patch returned unexpected response")),
                    Err(_) if matches!(op, RemotePatchOperation::Create { .. }) => {
                        // File doesn't exist yet for Create operation — that's fine
                    }
                    Err(e) => return Err(anyhow!("failed to read '{}' as pre-patch backup: {}", path, e)),
                }
            }
        }

        let mut applied = Vec::new();
        for op in &operations {
            let path = op.path();
            match op {
                RemotePatchOperation::Replace { search, replace, .. } => {
                    let content = if let Some(backup) = backups.get(path) {
                        backup.clone()
                    } else {
                        let read = client.read_file(workspace_id.clone(), path).await?;
                        match read {
                            PcGatewayResponse::FileContent { content, .. } => content,
                            _ => return Err(anyhow!("read file '{}' returned unexpected response", path)),
                        }
                    };
                    let new_content = content.replacen(search, replace, 1);
                    if new_content == content {
                        return Err(anyhow!("apply_patch replace: search string not found in '{}'", path));
                    }
                    client.write_file(workspace_id.clone(), path, &new_content).await?;
                    backups.insert(path.to_string(), new_content);
                    applied.push(format!("replaced in {}", path));
                }
                RemotePatchOperation::Create { content, overwrite, .. } => {
                    if let Some(_backup) = backups.get(path) {
                        if !overwrite {
                            return Err(anyhow!("file '{}' already exists and overwrite is not set", path));
                        }
                    }
                    client.write_file(workspace_id.clone(), path, content).await?;
                    backups.insert(path.to_string(), content.clone());
                    applied.push(format!("created {}", path));
                }
                RemotePatchOperation::Append { content, .. } => {
                    let existing = if let Some(backup) = backups.get(path) {
                        backup.clone()
                    } else {
                        match client.read_file(workspace_id.clone(), path).await? {
                            PcGatewayResponse::FileContent { content, .. } => content,
                            _ => return Err(anyhow!("read file '{}' returned unexpected response", path)),
                        }
                    };
                    let new_content = format!("{}\n{}", existing.trim_end(), content);
                    client.write_file(workspace_id.clone(), path, &new_content).await?;
                    backups.insert(path.to_string(), new_content);
                    applied.push(format!("appended to {}", path));
                }
                RemotePatchOperation::Delete { .. } => {
                    client.delete_file(workspace_id.clone(), path).await?;
                    backups.remove(path);
                    applied.push(format!("deleted {}", path));
                }
            }
        }

        let mut result = ToolResult::success(serde_json::to_string_pretty(&applied)?);
        result.metadata = Some(json!({
            "backups_exist": !backups.is_empty(),
            "operations": applied
        }));

        // Run post-edit diagnostics after batch operations
        if let Some(first_op) = operations.first() {
            let path = first_op.path();
            attach_post_edit_diagnostics(client, &workspace_id, Some(path.to_string()), &mut result).await?;
        }

        Ok(result)
    }

    pub fn registry(&self) -> &ToolRegistry {
        self.registry
    }
}

fn should_run_local_post_edit_diagnostics(call: &ToolCallRequest) -> bool {
    matches!(
        call.name.as_str(),
        "write_file" | "edit_file" | "apply_patch"
    )
}

fn extract_primary_path_for_diagnostics(call: &ToolCallRequest) -> Option<String> {
    call.arguments.get("path").and_then(Value::as_str).map(String::from)
}

async fn attach_local_post_edit_diagnostics(
    context: &ToolContext,
    path: Option<String>,
    result: &mut ToolResult,
) -> Result<()> {
    let service = WorkspaceDiagnosticsService::new(context.workspace.clone());
    let report = service.run_post_edit_diagnostics(path.clone()).await;
    let summary = report.summary();
    let diagnostics = report.diagnostics.clone();
    let provider = report.provider.clone();
    let status = format!("{:?}", report.status);
    let message = report.message.clone();
    let mut metadata = result.metadata.take().unwrap_or(json!({}));
    if let serde_json::Value::Object(ref mut map) = metadata {
        map.insert("diagnostics".to_string(), json!(report));
        map.insert("post_edit_diagnostics".to_string(), json!(diagnostics));
        map.insert("post_edit_diagnostics_summary".to_string(), json!(summary));
        map.insert("post_edit_diagnostics_provider".to_string(), json!(provider));
        map.insert("post_edit_diagnostics_status".to_string(), json!(status));
        if let Some(path) = path.as_ref() {
            map.insert("post_edit_diagnostics_path".to_string(), json!(path));
        }
        if let Some(message) = message {
            map.insert("post_edit_diagnostics_message".to_string(), json!(message));
        }
    }
    if !report.diagnostics.is_empty() {
        result.content.push_str("\n\n--- Diagnostics ---\n");
        result.content.push_str(&summary);
    }
    result.metadata = Some(metadata);
    Ok(())
}

async fn attach_post_edit_diagnostics(
    client: &PcGatewayClient,
    workspace_id: &str,
    path: Option<String>,
    result: &mut ToolResult,
) -> Result<()> {
    match client.get_diagnostics(workspace_id.to_string(), path.clone()).await {
        Ok(PcGatewayResponse::Diagnostics(diags)) => {
            let summary = summarize_diagnostics(&diags);
            let mut metadata = result.metadata.take().unwrap_or(json!({}));
            if let serde_json::Value::Object(ref mut map) = metadata {
                map.insert("diagnostics".to_string(), json!(diags.clone()));
                map.insert("post_edit_diagnostics".to_string(), json!(diags.clone()));
                map.insert("post_edit_diagnostics_summary".to_string(), json!(summary));
                map.insert("post_edit_diagnostics_provider".to_string(), json!("pc-gateway"));
                map.insert("post_edit_diagnostics_status".to_string(), json!("Completed"));
                if let Some(path) = path.as_ref() {
                    map.insert("post_edit_diagnostics_path".to_string(), json!(path));
                }
            }
            if !diags.is_empty() {
                result.content.push_str("\n\n--- Diagnostics ---\n");
                result.content.push_str(&summary);
            }
            result.metadata = Some(metadata);
        }
        Ok(_) => {
            // Unexpected response type from diagnostics endpoint
        }
        Err(error) => {
            let mut metadata = result.metadata.take().unwrap_or(json!({}));
            if let serde_json::Value::Object(ref mut map) = metadata {
                map.insert("diagnostics_error".to_string(), json!(error.to_string()));
                map.insert("post_edit_diagnostics_error".to_string(), json!(error.to_string()));
                if let Some(path) = path.as_ref() {
                    map.insert("post_edit_diagnostics_path".to_string(), json!(path));
                }
            }
            result.metadata = Some(metadata);
        }
    }
    Ok(())
}

fn summarize_diagnostics(diagnostics: &[PcDiagnostic]) -> String {
    if diagnostics.is_empty() {
        return "No diagnostics reported.".to_string();
    }
    let error_count = diagnostics
        .iter()
        .filter(|item| item.severity == crate::pc_gateway::PcDiagnosticSeverity::Error)
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|item| item.severity == crate::pc_gateway::PcDiagnosticSeverity::Warning)
        .count();
    let mut lines = vec![format!(
        "{} diagnostic(s): {} error(s), {} warning(s)",
        diagnostics.len(), error_count, warning_count
    )];
    for item in diagnostics.iter().take(8) {
        lines.push(format!(
            "- {:?} {}:{}:{} — {}",
            item.severity, item.path, item.line, item.column, item.message
        ));
    }
    if diagnostics.len() > 8 {
        lines.push(format!("- ... {} more diagnostic(s)", diagnostics.len() - 8));
    }
    lines.join("\n")
}

fn command_request_from_shell(command: &str, root: PathBuf) -> Result<CommandRequest> {
    let trimmed = command.trim();
    let parts = shell_words(trimmed);
    if parts.is_empty() {
        return Err(anyhow!("empty command"));
    }
    Ok(CommandRequest {
        program: parts[0].clone(),
        args: parts[1..].to_vec(),
        working_dir: Some(root),
    })
}

fn shell_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_single => current.push(ch),
            '\\' => escaped = true,
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            ' ' | '\t' if !in_single && !in_double => {
                if !current.is_empty() {
                    words.push(current.clone());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn gateway_response_to_tool_result(response: PcGatewayResponse) -> Result<ToolResult> {
    match response {
        PcGatewayResponse::FileContent { path, content } => Ok(
            ToolResult::success(content).with_metadata(json!({"path": path}))
        ),
        PcGatewayResponse::FileWritten { path, bytes } => Ok(
            ToolResult::success(format!("written {} ({} bytes)", path, bytes))
                .with_metadata(json!({"path": path, "bytes": bytes}))
        ),
        PcGatewayResponse::FileDeleted { path } => Ok(
            ToolResult::success(format!("deleted {}", path))
                .with_metadata(json!({"path": path}))
        ),
        PcGatewayResponse::DirEntries(entries) => {
            let text = entries.iter()
                .map(|e| format!("{}/{} ({})", if e.is_dir { "d" } else { "-" }, e.path, e.size_bytes))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(ToolResult::success(text).with_metadata(json!({"entries": entries})))
        }
        PcGatewayResponse::CommandOutput(output) => {
            let text = format!("{}EXIT_CODE: {}\n\n{}",
                output.stdout,
                output.status_code.map_or("unknown".to_string(), |c| c.to_string()),
                output.stderr,
            );
            Ok(ToolResult::success(text).with_metadata(json!({
                "status_code": output.status_code,
                "stdout_size": output.stdout.len(),
                "stderr_size": output.stderr.len(),
            })))
        }
        PcGatewayResponse::Diagnostics(diags) => {
            let text = if diags.is_empty() {
                "No diagnostics reported.".to_string()
            } else {
                diags.iter()
                    .map(|d| format!("{:?} {}:{}:{} — {}", d.severity, d.path, d.line, d.column, d.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(ToolResult::success(text).with_metadata(json!({"diagnostics": diags})))
        }
        PcGatewayResponse::Tasks(tasks) => {
            let text = serde_json::to_string_pretty(&tasks)?;
            Ok(ToolResult::success(text).with_metadata(json!({"tasks": tasks})))
        }
        PcGatewayResponse::TerminalOpened(session) => {
            Ok(ToolResult::success(serde_json::to_string_pretty(&session)?)
                .with_metadata(json!({"terminal_session": session})))
        }
        PcGatewayResponse::TerminalOutput { session_id, chunk } => {
            Ok(ToolResult::success(chunk).with_metadata(json!({"terminal_session_id": session_id})))
        }
        PcGatewayResponse::TerminalClosed { session_id, exit_code } => {
            Ok(ToolResult::success(format!("terminal {} closed (exit: {:?})", session_id, exit_code))
                .with_metadata(json!({"terminal_session_id": session_id, "exit_code": exit_code})))
        }
        PcGatewayResponse::SnapshotRecord(snapshot) => {
            Ok(ToolResult::success(format!(
                "Created snapshot {} with {} file(s), {} bytes",
                snapshot.id, snapshot.file_count, snapshot.total_bytes
            ))
            .with_metadata(serde_json::to_value(snapshot)?))
        }
        PcGatewayResponse::SnapshotList(snapshots) => {
            Ok(ToolResult::success(serde_json::to_string_pretty(&snapshots)?)
                .with_metadata(serde_json::to_value(snapshots)?))
        }
        PcGatewayResponse::SnapshotRestoreReport(report) => {
            Ok(ToolResult::success(serde_json::to_string_pretty(&report)?)
                .with_metadata(serde_json::to_value(report)?))
        }
        PcGatewayResponse::TaskStarted { task_id, process_id } => {
            Ok(ToolResult::success(format!("task {} started (pid {})", task_id, process_id))
                .with_metadata(json!({"task_id": task_id, "process_id": process_id})))
        }
        PcGatewayResponse::TaskStopped { task_id } => {
            Ok(ToolResult::success(format!("task {} stopped", task_id))
                .with_metadata(json!({"task_id": task_id})))
        }
        PcGatewayResponse::TaskList(tasks) => {
            Ok(ToolResult::success(serde_json::to_string_pretty(&tasks)?)
                .with_metadata(serde_json::to_value(tasks)?))
        }
        PcGatewayResponse::Error(error) => Ok(ToolResult::error(format!("{}: {}", error.code, error.message))),
        _ => Ok(ToolResult::success("unhandled PC gateway response variant".to_string())),
    }
}

fn required_str<'a>(input: &'a Value, key: &str) -> Result<&'a str> {
    input
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing required string field '{}'", key))
}

fn optional_str<'a>(input: &'a Value, key: &str) -> Option<&'a str> {
    input.get(key).and_then(Value::as_str)
}

fn optional_u64(input: &Value, key: &str) -> Option<u64> {
    input.get(key).and_then(Value::as_u64)
}

fn runs_on_phone_regardless_of_workspace(tool_name: &str) -> bool {
    matches!(tool_name, "web_fetch" | "web_search") || tool_name.starts_with("github_")
}

fn termux_request_id(call: &ToolCallRequest, command: &str, root: &PathBuf) -> String {
    let raw = if call.id.trim().is_empty() {
        let mut hasher = DefaultHasher::new();
        call.name.hash(&mut hasher);
        command.hash(&mut hasher);
        root.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    } else {
        call.id.clone()
    };
    let mut sanitized = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    sanitized.truncate(96);
    let sanitized = sanitized.trim_matches('-');
    if sanitized.is_empty() {
        "termux-command".to_string()
    } else if sanitized.starts_with("termux-") {
        sanitized.to_string()
    } else {
        format!("termux-{}", sanitized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_call::ToolCallSource;

    #[test]
    fn route_returns_local_for_local_workspace() {
        let tool_call = ToolCallRequest {
            id: "call-1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": "test.txt"}),
            source: ToolCallSource::JsonBlock,
        };
        let workspace = crate::workspace::Workspace::new(
            "w1", "Test", PathBuf::from("."), ExecutorKind::LocalAndroid,
        );
        let context = ToolContext::new(workspace);
        let registry = crate::tools::ToolRegistry::new();

        let coordinator = ToolExecutionCoordinator::new(&registry);
        let route = coordinator.route(&tool_call, &context);
        assert_eq!(route.target, ToolExecutionTarget::LocalAndroid);
    }

    #[test]
    fn route_returns_pc_for_pc_workspace() {
        let tool_call = ToolCallRequest {
            id: "call-1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": "test.txt"}),
            source: ToolCallSource::JsonBlock,
        };
        let workspace = crate::workspace::Workspace::new(
            "w1", "Test", PathBuf::from("."), ExecutorKind::PcGateway,
        );
        let context = ToolContext::new(workspace);
        let registry = crate::tools::ToolRegistry::new();

        let coordinator = ToolExecutionCoordinator::new(&registry);
        let route = coordinator.route(&tool_call, &context);
        assert_eq!(route.target, ToolExecutionTarget::PcGateway);
    }

    #[test]
    fn shell_words_basic() {
        let words = shell_words("npm run build");
        assert_eq!(words, vec!["npm", "run", "build"]);
    }

    #[test]
    fn shell_words_single_quoted() {
        let words = shell_words("echo 'hello world'");
        assert_eq!(words, vec!["echo", "hello world"]);
    }

    #[test]
    fn shell_words_double_quoted() {
        let words = shell_words("echo \"hello world\"");
        assert_eq!(words, vec!["echo", "hello world"]);
    }

    #[test]
    fn shell_words_empty_returns_empty() {
        let words = shell_words("");
        assert!(words.is_empty());
    }

    #[test]
    fn gateway_response_file_content() {
        let response = PcGatewayResponse::FileContent {
            path: "test.txt".to_string(),
            content: "hello".to_string(),
        };
        let result = gateway_response_to_tool_result(response).unwrap();
        assert!(result.success);
        assert_eq!(result.content, "hello");
        let meta = result.metadata.unwrap();
        assert_eq!(meta.get("path").and_then(|v| v.as_str()), Some("test.txt"));
    }

    #[test]
    fn gateway_response_error() {
        let response = PcGatewayResponse::Error(crate::pc_gateway::PcGatewayError::new("e1", "msg"));
        let result = gateway_response_to_tool_result(response).unwrap();
        assert!(!result.success);
        assert!(result.content.contains("msg"));
    }

    #[test]
    fn remote_patch_operation_deserializes_normalized_unified_diff() {
        let operations_value = crate::tools::patch::normalized_operations_value(&json!({
            "unified_diff": "\
--- a/a.txt
+++ b/a.txt
@@ -1 +1 @@
-one
+two
"
        }))
        .unwrap();

        let operations: Vec<RemotePatchOperation> =
            serde_json::from_value(operations_value).unwrap();
        assert_eq!(
            operations,
            vec![RemotePatchOperation::Replace {
                path: "a.txt".to_string(),
                search: "one\n".to_string(),
                replace: "two\n".to_string(),
                expected_occurrences: Some(1),
            }]
        );
    }

    #[tokio::test]
    async fn termux_exec_shell_returns_pending_native_request_metadata() {
        let tool_call = ToolCallRequest {
            id: "call-1".to_string(),
            name: "exec_shell".to_string(),
            arguments: json!({"command": "pwd", "timeout_secs": 7}),
            source: ToolCallSource::Manual,
        };
        let workspace = crate::workspace::Workspace::new(
            "w-termux",
            "Termux",
            PathBuf::from("/data/data/com.termux/files/home/project"),
            ExecutorKind::Termux,
        );
        let context = ToolContext::new(workspace);
        let registry = crate::tools::ToolRegistry::new();

        let result = ToolExecutionCoordinator::new(&registry)
            .execute(&tool_call, &context)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.content.contains("queued"));
        let metadata = result.metadata.expect("termux metadata");
        assert_eq!(
            metadata
                .get("termux_execution_pending")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata.get("termux_request_id").and_then(Value::as_str),
            Some("termux-call-1")
        );
        let request: TermuxExecRequest =
            serde_json::from_value(metadata.get("termux_exec_request").unwrap().clone()).unwrap();
        assert_eq!(request.request_id, "termux-call-1");
        assert_eq!(request.command, "pwd");
        assert_eq!(request.timeout_secs, Some(7));
        assert_eq!(
            request.working_dir,
            PathBuf::from("/data/data/com.termux/files/home/project")
        );
    }
}

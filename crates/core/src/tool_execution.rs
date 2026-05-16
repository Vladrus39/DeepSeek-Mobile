//! Tool execution coordinator.
//!
//! Android remains the orchestrator: it parses the model response, applies
//! approval policy, records timeline events and chooses the active backend. The
//! actual tool execution depends on the workspace executor. For a PC gateway
//! workspace, file/shell/git operations are sent to the PC; they are not executed
//! on Android.

use crate::executor::CommandRequest;
use crate::pc_gateway::{PcDiagnostic, PcDiagnosticSeverity, PcGatewayResponse};
use crate::pc_gateway_client::PcGatewayClient;
use crate::tool_call::ToolCallRequest;
use crate::tools::{ToolContext, ToolRegistry, ToolResult};
use crate::workspace::ExecutorKind;
use crate::workspace_diagnostics::WorkspaceDiagnosticsService;
use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
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

#[derive(Clone, Debug, PartialEq, Eq)]
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
        match self.route(call, context).target {
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
                gateway_response_to_tool_result(client.execute_command(workspace_id, request, None).await?)
            }
            "git" => {
                let operation = required_str(&call.arguments, "operation")?;
                match operation {
                    "status" => gateway_response_to_tool_result(client.git_status(workspace_id).await?),
                    "diff" => gateway_response_to_tool_result(client.git_diff(workspace_id).await?),
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
        let content = match read {
            PcGatewayResponse::FileContent { content, .. } => content,
            other => return gateway_response_to_tool_result(other),
        };
        let count = content.matches(search).count();
        if count == 0 {
            return Err(anyhow!("search text not found in {}", path));
        }
        let updated = content.replace(search, replace);
        let write = client.write_file(workspace_id.clone(), path, updated).await?;
        let mut result = gateway_response_to_tool_result(write)?;
        result.content = format!("Replaced {} occurrence(s) in {}", count, path);
        attach_post_edit_diagnostics(client, &workspace_id, Some(path.to_string()), &mut result).await?;
        Ok(result)
    }

    async fn execute_remote_apply_patch(
        &self,
        client: &PcGatewayClient,
        workspace_id: String,
        arguments: &Value,
    ) -> Result<ToolResult> {
        let operations = parse_remote_patch_operations(arguments)?;
        let mut originals: BTreeMap<String, Option<String>> = BTreeMap::new();
        let mut changed_files = BTreeSet::new();

        let result = async {
            for operation in operations.iter() {
                let path = operation.path();
                if !originals.contains_key(path) {
                    originals.insert(path.to_string(), remote_read_optional(client, &workspace_id, path).await?);
                }
                apply_remote_patch_operation(client, &workspace_id, operation).await?;
                changed_files.insert(path.to_string());
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;

        if let Err(error) = result {
            rollback_remote_patch(client, &workspace_id, originals).await?;
            return Err(error);
        }

        let mut result = ToolResult::success(format!(
            "Applied {} PC patch operation(s) across {} file(s): {}",
            operations.len(),
            changed_files.len(),
            changed_files.iter().cloned().collect::<Vec<_>>().join(", ")
        ))
        .with_metadata(json!({
            "operations_applied": operations.len(),
            "changed_files": changed_files.iter().cloned().collect::<Vec<_>>(),
            "source": "pc_gateway"
        }));

        attach_post_edit_diagnostics(client, &workspace_id, None, &mut result).await?;
        Ok(result)
    }
}

fn should_run_local_post_edit_diagnostics(call: &ToolCallRequest) -> bool {
    match call.name.as_str() {
        "write_file" | "edit_file" | "apply_patch" => true,
        "file_ops" => file_ops_may_modify(&call.arguments),
        _ => false,
    }
}

fn file_ops_may_modify(arguments: &Value) -> bool {
    arguments
        .get("operation")
        .and_then(Value::as_str)
        .map(|operation| matches!(operation, "write" | "write_file" | "edit" | "edit_file"))
        .unwrap_or(false)
}

fn extract_primary_path_for_diagnostics(call: &ToolCallRequest) -> Option<String> {
    match call.name.as_str() {
        "write_file" | "edit_file" => optional_str(&call.arguments, "path").map(str::to_string),
        "file_ops" => optional_str(&call.arguments, "path").map(str::to_string),
        _ => None,
    }
}

async fn attach_local_post_edit_diagnostics(
    context: &ToolContext,
    path: Option<String>,
    result: &mut ToolResult,
) -> Result<()> {
    let report = WorkspaceDiagnosticsService::new(context.workspace.clone())
        .run_post_edit_diagnostics(path.clone())
        .await;
    let summary = report.summary();
    result.content.push_str("\n\nPost-edit diagnostics:\n");
    result.content.push_str(&summary);
    merge_metadata(
        result,
        json!({
            "post_edit_diagnostics_status": report.status,
            "post_edit_diagnostics_provider": report.provider,
            "post_edit_diagnostics": report.diagnostics,
            "post_edit_diagnostics_path": path,
            "post_edit_diagnostics_summary": summary,
            "post_edit_diagnostics_message": report.message,
            "post_edit_diagnostics_source": "workspace_diagnostics"
        }),
    );
    Ok(())
}

async fn remote_read_optional(
    client: &PcGatewayClient,
    workspace_id: &str,
    path: &str,
) -> Result<Option<String>> {
    match client.read_file(workspace_id.to_string(), path.to_string()).await {
        Ok(PcGatewayResponse::FileContent { content, .. }) => Ok(Some(content)),
        Ok(PcGatewayResponse::Error(_)) => Ok(None),
        Ok(other) => Err(anyhow!("unexpected read_file response during patch backup: {:?}", other)),
        Err(_) => Ok(None),
    }
}

async fn apply_remote_patch_operation(
    client: &PcGatewayClient,
    workspace_id: &str,
    operation: &RemotePatchOperation,
) -> Result<()> {
    match operation {
        RemotePatchOperation::Replace {
            path,
            search,
            replace,
            expected_occurrences,
        } => {
            if search.is_empty() {
                return Err(anyhow!("replace operation has empty search text in {}", path));
            }
            let content = match client.read_file(workspace_id.to_string(), path.clone()).await? {
                PcGatewayResponse::FileContent { content, .. } => content,
                other => return Err(anyhow!("unexpected read_file response during replace: {:?}", other)),
            };
            let count = content.matches(search).count();
            if count == 0 {
                return Err(anyhow!("search text not found in {}", path));
            }
            if let Some(expected) = expected_occurrences {
                if count != *expected {
                    return Err(anyhow!(
                        "replace occurrence mismatch in {}: expected {}, found {}",
                        path,
                        expected,
                        count
                    ));
                }
            }
            let updated = content.replace(search, replace);
            ensure_file_written(client.write_file(workspace_id.to_string(), path.clone(), updated).await?)
        }
        RemotePatchOperation::Create {
            path,
            content,
            overwrite,
        } => {
            if !overwrite && remote_read_optional(client, workspace_id, path).await?.is_some() {
                return Err(anyhow!("create operation refuses to overwrite existing file: {}", path));
            }
            ensure_file_written(client.write_file(workspace_id.to_string(), path.clone(), content.clone()).await?)
        }
        RemotePatchOperation::Append { path, content } => {
            let existing = match client.read_file(workspace_id.to_string(), path.clone()).await? {
                PcGatewayResponse::FileContent { content, .. } => content,
                other => return Err(anyhow!("unexpected read_file response during append: {:?}", other)),
            };
            ensure_file_written(
                client
                    .write_file(workspace_id.to_string(), path.clone(), format!("{}{}", existing, content))
                    .await?,
            )
        }
        RemotePatchOperation::Delete { path } => ensure_file_deleted(
            client
                .delete_file(workspace_id.to_string(), path.clone())
                .await?,
        ),
    }
}

async fn rollback_remote_patch(
    client: &PcGatewayClient,
    workspace_id: &str,
    originals: BTreeMap<String, Option<String>>,
) -> Result<()> {
    for (path, original) in originals.into_iter().rev() {
        match original {
            Some(content) => {
                let response = client.write_file(workspace_id.to_string(), path, content).await?;
                ensure_file_written(response)?;
            }
            None => {
                let _ = client.delete_file(workspace_id.to_string(), path).await;
            }
        }
    }
    Ok(())
}

fn parse_remote_patch_operations(input: &Value) -> Result<Vec<RemotePatchOperation>> {
    let operations = input
        .get("operations")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("apply_patch requires an operations array"))?;
    if operations.is_empty() {
        return Err(anyhow!("apply_patch operations array must not be empty"));
    }

    operations
        .iter()
        .enumerate()
        .map(parse_remote_patch_operation)
        .collect()
}

fn parse_remote_patch_operation((index, operation): (usize, &Value)) -> Result<RemotePatchOperation> {
    let op_type = operation
        .get("type")
        .or_else(|| operation.get("operation"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("patch operation {} is missing type", index))?;
    let path = required_str(operation, "path")?.to_string();

    match op_type {
        "replace" => {
            let search = required_str(operation, "search")?.to_string();
            let replace = required_str(operation, "replace")?.to_string();
            let expected_occurrences = operation
                .get("expected_occurrences")
                .and_then(Value::as_u64)
                .map(|value| value as usize);
            Ok(RemotePatchOperation::Replace {
                path,
                search,
                replace,
                expected_occurrences,
            })
        }
        "create" => {
            let content = required_str(operation, "content")?.to_string();
            let overwrite = operation
                .get("overwrite")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            Ok(RemotePatchOperation::Create { path, content, overwrite })
        }
        "append" => {
            let content = required_str(operation, "content")?.to_string();
            Ok(RemotePatchOperation::Append { path, content })
        }
        "delete" | "remove" => Ok(RemotePatchOperation::Delete { path }),
        other => Err(anyhow!("unsupported patch operation {}: {}", index, other)),
    }
}

fn ensure_file_written(response: PcGatewayResponse) -> Result<()> {
    match response {
        PcGatewayResponse::FileWritten { .. } => Ok(()),
        PcGatewayResponse::Error(error) => Err(anyhow!("PC gateway error {}: {}", error.code, error.message)),
        other => Err(anyhow!("unexpected write_file response: {:?}", other)),
    }
}

fn ensure_file_deleted(response: PcGatewayResponse) -> Result<()> {
    match response {
        PcGatewayResponse::FileDeleted { .. } => Ok(()),
        PcGatewayResponse::Error(error) => Err(anyhow!("PC gateway error {}: {}", error.code, error.message)),
        other => Err(anyhow!("unexpected delete_file response: {:?}", other)),
    }
}

async fn attach_post_edit_diagnostics(
    client: &PcGatewayClient,
    workspace_id: &str,
    path: Option<String>,
    result: &mut ToolResult,
) -> Result<()> {
    match client.get_diagnostics(workspace_id.to_string(), path.clone()).await {
        Ok(PcGatewayResponse::Diagnostics(diagnostics)) => {
            let summary = diagnostic_summary(&diagnostics);
            if !summary.is_empty() {
                result.content.push_str("\n\nPost-edit diagnostics:\n");
                result.content.push_str(&summary);
            }
            merge_metadata(
                result,
                json!({
                    "post_edit_diagnostics": diagnostics,
                    "post_edit_diagnostics_path": path,
                    "post_edit_diagnostics_summary": summary,
                }),
            );
        }
        Ok(other) => {
            merge_metadata(
                result,
                json!({
                    "post_edit_diagnostics_error": format!("unexpected diagnostics response: {:?}", other),
                    "post_edit_diagnostics_path": path,
                }),
            );
        }
        Err(error) => {
            merge_metadata(
                result,
                json!({
                    "post_edit_diagnostics_error": error.to_string(),
                    "post_edit_diagnostics_path": path,
                }),
            );
        }
    }
    Ok(())
}

fn diagnostic_summary(diagnostics: &[PcDiagnostic]) -> String {
    if diagnostics.is_empty() {
        return "No diagnostics reported.".to_string();
    }
    let error_count = diagnostics
        .iter()
        .filter(|item| item.severity == PcDiagnosticSeverity::Error)
        .count();
    let warning_count = diagnostics
        .iter()
        .filter(|item| item.severity == PcDiagnosticSeverity::Warning)
        .count();
    let mut lines = vec![format!(
        "{} diagnostic(s): {} error(s), {} warning(s)",
        diagnostics.len(),
        error_count,
        warning_count
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

fn merge_metadata(result: &mut ToolResult, new_metadata: Value) {
    let mut base = match result.metadata.take() {
        Some(Value::Object(object)) => object,
        Some(other) => {
            let mut object = Map::new();
            object.insert("previous_metadata".to_string(), other);
            object
        }
        None => Map::new(),
    };

    if let Value::Object(object) = new_metadata {
        for (key, value) in object {
            base.insert(key, value);
        }
    }
    result.metadata = Some(Value::Object(base));
}

fn gateway_response_to_tool_result(response: PcGatewayResponse) -> Result<ToolResult> {
    match response {
        PcGatewayResponse::FileContent { path, content } => Ok(
            ToolResult::success(content).with_metadata(json!({ "path": path, "source": "pc_gateway" })),
        ),
        PcGatewayResponse::FileWritten { path, bytes } => Ok(ToolResult::success(format!(
            "PC gateway wrote {} bytes to {}",
            bytes, path
        ))
        .with_metadata(json!({ "path": path, "bytes": bytes, "source": "pc_gateway" }))),
        PcGatewayResponse::FileDeleted { path } => Ok(ToolResult::success(format!(
            "PC gateway deleted {}",
            path
        ))
        .with_metadata(json!({ "path": path, "source": "pc_gateway" }))),
        PcGatewayResponse::DirEntries(entries) => {
            let metadata = serde_json::to_value(&entries)?;
            Ok(ToolResult::success(serde_json::to_string_pretty(&metadata)?).with_metadata(metadata))
        }
        PcGatewayResponse::CommandOutput(output) => Ok(ToolResult::success(format!(
            "stdout:\n{}\n\nstderr:\n{}",
            output.stdout, output.stderr
        ))
        .with_metadata(json!({
            "status_code": output.status_code,
            "source": "pc_gateway"
        }))),
        PcGatewayResponse::GitText { operation, output } => Ok(ToolResult::success(output)
            .with_metadata(json!({ "operation": operation, "source": "pc_gateway" }))),
        PcGatewayResponse::Diagnostics(items) => {
            let metadata = serde_json::to_value(&items)?;
            Ok(ToolResult::success(serde_json::to_string_pretty(&metadata)?).with_metadata(metadata))
        }
        PcGatewayResponse::Error(error) => Ok(ToolResult::error(format!(
            "PC gateway error {}: {}",
            error.code, error.message
        ))),
        other => Ok(ToolResult::success(format!("PC gateway response: {:?}", other))),
    }
}

fn command_request_from_shell(command: &str, working_dir: PathBuf) -> Result<CommandRequest> {
    let words = shell_words(command);
    let Some((program, args)) = words.split_first() else {
        return Err(anyhow!("empty shell command"));
    };
    Ok(CommandRequest {
        program: program.clone(),
        args: args.to_vec(),
        working_dir: Some(working_dir),
    })
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

fn shell_words(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(std::string::ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        command_request_from_shell, diagnostic_summary, merge_metadata, parse_remote_patch_operations,
        should_run_local_post_edit_diagnostics, ToolExecutionCoordinator, ToolExecutionTarget,
    };
    use crate::pc_gateway::{PcDiagnostic, PcDiagnosticSeverity};
    use crate::tool_call::{ToolCallRequest, ToolCallSource};
    use crate::tools::{ToolContext, ToolRegistry, ToolResult};
    use crate::workspace::{ExecutorKind, Workspace};
    use serde_json::{json, Value};

    #[test]
    fn routes_pc_workspace_tools_to_pc_gateway() {
        let registry = ToolRegistry::new();
        let coordinator = ToolExecutionCoordinator::new(&registry);
        let context = ToolContext::new(Workspace::new("w1", "Project", "/pc/project", ExecutorKind::PcGateway));
        let call = ToolCallRequest::new("read_file", json!({"path":"README.md"}), ToolCallSource::Manual);
        let route = coordinator.route(&call, &context);
        assert_eq!(route.target, ToolExecutionTarget::PcGateway);
    }

    #[test]
    fn routes_local_workspace_tools_to_local_registry() {
        let registry = ToolRegistry::new();
        let coordinator = ToolExecutionCoordinator::new(&registry);
        let context = ToolContext::new(Workspace::new("w1", "Project", "/phone/project", ExecutorKind::LocalAndroid));
        let call = ToolCallRequest::new("read_file", json!({"path":"README.md"}), ToolCallSource::Manual);
        let route = coordinator.route(&call, &context);
        assert_eq!(route.target, ToolExecutionTarget::LocalAndroid);
    }

    #[test]
    fn builds_command_request_from_simple_shell_command() {
        let request = command_request_from_shell("cargo check --workspace", "/project".into()).unwrap();
        assert_eq!(request.program, "cargo");
        assert_eq!(request.args, vec!["check", "--workspace"]);
    }

    #[test]
    fn local_post_edit_hook_runs_only_after_file_changes() {
        assert!(should_run_local_post_edit_diagnostics(&ToolCallRequest::new(
            "write_file",
            json!({"path":"src/lib.rs","content":"x"}),
            ToolCallSource::Manual,
        )));
        assert!(should_run_local_post_edit_diagnostics(&ToolCallRequest::new(
            "edit_file",
            json!({"path":"src/lib.rs","search":"a","replace":"b"}),
            ToolCallSource::Manual,
        )));
        assert!(should_run_local_post_edit_diagnostics(&ToolCallRequest::new(
            "apply_patch",
            json!({"operations":[{"type":"append","path":"src/lib.rs","content":"x"}]}),
            ToolCallSource::Manual,
        )));
        assert!(!should_run_local_post_edit_diagnostics(&ToolCallRequest::new(
            "read_file",
            json!({"path":"src/lib.rs"}),
            ToolCallSource::Manual,
        )));
    }

    #[test]
    fn parses_remote_apply_patch_operations() {
        let operations = parse_remote_patch_operations(&json!({
            "operations": [
                {"type":"replace","path":"README.md","search":"old","replace":"new","expected_occurrences":1},
                {"type":"create","path":"src/lib.rs","content":"pub fn ok() {}","overwrite":false},
                {"type":"delete","path":"old.txt"}
            ]
        }))
        .unwrap();
        assert_eq!(operations.len(), 3);
        assert_eq!(operations[0].path(), "README.md");
        assert_eq!(operations[2].path(), "old.txt");
    }

    #[test]
    fn summarizes_post_edit_diagnostics() {
        let diagnostics = vec![PcDiagnostic {
            path: "src/main.rs".to_string(),
            line: 10,
            column: 5,
            severity: PcDiagnosticSeverity::Error,
            message: "cannot find value".to_string(),
            source: Some("cargo check".to_string()),
        }];
        let summary = diagnostic_summary(&diagnostics);
        assert!(summary.contains("1 diagnostic"));
        assert!(summary.contains("cannot find value"));
    }

    #[test]
    fn merges_post_edit_diagnostics_metadata() {
        let mut result = ToolResult::success("ok").with_metadata(json!({"path":"src/main.rs"}));
        merge_metadata(&mut result, json!({"post_edit_diagnostics_summary":"No diagnostics reported."}));
        let metadata = result.metadata.unwrap();
        assert_eq!(metadata["path"], Value::String("src/main.rs".to_string()));
        assert_eq!(metadata["post_edit_diagnostics_summary"], Value::String("No diagnostics reported.".to_string()));
    }
}
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
use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};
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
                self.registry.execute(&call.name, call.arguments.clone(), context)
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
            "list_dir" => {
                let path = optional_str(&call.arguments, "path").unwrap_or(".");
                gateway_response_to_tool_result(client.list_dir(workspace_id, path).await?)
            }
            "edit_file" => self.execute_remote_edit_file(client, workspace_id, &call.arguments).await,
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
    use super::{command_request_from_shell, diagnostic_summary, merge_metadata, ToolExecutionCoordinator, ToolExecutionTarget};
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
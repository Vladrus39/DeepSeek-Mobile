//! MCP client for HTTP/SSE and stdio servers (tools/list + tools/call).

use crate::mcp::{McpServerConfig, McpServerStatus, McpToolDescriptor, McpTransport};
use anyhow::{anyhow, Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Mutex;
use std::time::Duration;
use tokio::task;

#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct ToolsListResult {
    tools: Vec<McpRemoteTool>,
}

#[derive(Debug, Deserialize)]
struct McpRemoteTool {
    name: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "inputSchema", default)]
    input_schema: Value,
}

struct StdioSession {
    _child: Child,
    stdin: ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    next_id: u64,
}

static STDIO_SESSIONS: Mutex<Option<HashMap<String, StdioSession>>> = Mutex::new(None);

fn stdio_sessions() -> std::sync::MutexGuard<'static, Option<HashMap<String, StdioSession>>> {
    STDIO_SESSIONS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Connect to an MCP server and return discovered tools.
pub async fn connect_mcp_server(
    config: &McpServerConfig,
) -> Result<(McpServerStatus, Vec<McpToolDescriptor>)> {
    if !config.enabled {
        return Ok((McpServerStatus::Disconnected, Vec::new()));
    }

    match &config.transport {
        McpTransport::Stdio { command, args, env } => {
            let tools = task::spawn_blocking({
                let command = command.clone();
                let args = args.clone();
                let env = env.clone();
                let server_name = config.name.clone();
                move || connect_stdio_sync(&server_name, &command, &args, &env)
            })
            .await
            .context("stdio MCP connect task failed")??;
            Ok((McpServerStatus::Connected, tools))
        }
        McpTransport::HttpSse { url, headers } => {
            let tools = list_tools_http(url, headers).await?;
            Ok((McpServerStatus::Connected, tools))
        }
    }
}

/// Invoke an MCP tool on a connected server (loads registry from the default path).
pub async fn invoke_mcp_tool(server: &str, tool_name: &str, arguments: Value) -> Result<String> {
    invoke_mcp_tool_at_path(&default_mcp_path(), server, tool_name, arguments).await
}

/// Invoke an MCP tool using the MCP registry file at `registry_path`.
pub async fn invoke_mcp_tool_at_path(
    registry_path: &std::path::Path,
    server: &str,
    tool_name: &str,
    arguments: Value,
) -> Result<String> {
    let registry = crate::mcp::McpClientRegistry::load_or_default(registry_path)?;
    let config = registry
        .servers
        .iter()
        .find(|entry| entry.config.name == server)
        .map(|entry| entry.config.clone())
        .ok_or_else(|| anyhow!("MCP server '{server}' is not configured"))?;

    if !config.enabled {
        return Err(anyhow!("MCP server '{server}' is disabled"));
    }

    match &config.transport {
        McpTransport::Stdio { .. } => {
            let response = task::spawn_blocking({
                let tool_name = tool_name.to_string();
                let arguments = arguments.clone();
                let config = config.clone();
                move || invoke_stdio_tool_with_config(&config, &tool_name, arguments)
            })
            .await
            .context("stdio MCP invoke task failed")??;
            Ok(response)
        }
        McpTransport::HttpSse { url, headers } => {
            invoke_http_tool_with_config(url, headers, tool_name, arguments).await
        }
    }
}

fn connect_stdio_sync(
    server_name: &str,
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<Vec<McpToolDescriptor>> {
    let mut child = Command::new(command);
    child.args(args);
    for (key, value) in env {
        child.env(key, value);
    }
    child
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = child
        .spawn()
        .with_context(|| format!("spawn MCP stdio server {command}"))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("stdio stdin unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("stdio stdout unavailable"))?;
    let mut session = StdioSession {
        _child: child,
        stdin,
        stdout: BufReader::new(stdout),
        next_id: 1,
    };

    let init_id = session.next_id;
    session.next_id += 1;
    write_json_rpc(
        &mut session.stdin,
        init_id,
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "deepseek-mobile", "version": "0.1.0"}
        }),
    )?;
    let _ = read_json_rpc(&mut session.stdout, init_id)?;

    let list_id = session.next_id;
    session.next_id += 1;
    write_json_rpc(&mut session.stdin, list_id, "tools/list", json!({}))?;
    let list_response = read_json_rpc(&mut session.stdout, list_id)?;
    if list_response.get("error").is_some() {
        return Err(anyhow!("MCP tools/list error: {}", list_response));
    }
    let result = list_response
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow!("MCP tools/list missing result"))?;
    let listed: ToolsListResult = serde_json::from_value(result)
        .map_err(|error| anyhow!("parse MCP tools/list result: {}", error))?;

    let mut sessions = stdio_sessions();
    let map = sessions.get_or_insert_with(HashMap::new);
    map.insert(server_name.to_string(), session);

    Ok(listed
        .tools
        .into_iter()
        .map(|tool| McpToolDescriptor {
            name: tool.name,
            server: server_name.to_string(),
            description: tool.description,
            input_schema: tool.input_schema,
        })
        .collect())
}

fn invoke_stdio_tool(server: &str, tool_name: &str, arguments: Value) -> Result<Option<String>> {
    let mut sessions = stdio_sessions();
    let map = sessions
        .as_mut()
        .ok_or_else(|| anyhow!("no stdio MCP sessions"))?;
    let session = map
        .get_mut(server)
        .ok_or_else(|| anyhow!("stdio MCP server '{}' is not connected", server))?;
    let request_id = session.next_id;
    session.next_id += 1;
    write_json_rpc(
        &mut session.stdin,
        request_id,
        "tools/call",
        json!({
            "name": tool_name,
            "arguments": arguments
        }),
    )?;
    let response = read_json_rpc(&mut session.stdout, request_id)?;
    if let Some(error) = response.get("error") {
        return Err(anyhow!("MCP tools/call error: {}", error));
    }
    Ok(Some(serde_json::to_string_pretty(
        &response.get("result").cloned().unwrap_or(response),
    )?))
}

async fn invoke_http_tool_with_config(
    url: &str,
    headers: &HashMap<String, String>,
    tool_name: &str,
    arguments: Value,
) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .context("build MCP HTTP client")?;
    let endpoint = mcp_endpoint(url);
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": arguments
        }
    });
    let mut request = client.post(&endpoint).json(&body);
    for (key, value) in headers {
        request = request.header(key, value);
    }
    let response: JsonRpcResponse = request.send().await?.json().await?;
    if let Some(error) = response.error {
        return Err(anyhow!("MCP tools/call error: {}", error.message));
    }
    Ok(serde_json::to_string_pretty(
        &response.result.unwrap_or(Value::Null),
    )?)
}

fn invoke_stdio_tool_with_config(
    config: &McpServerConfig,
    tool_name: &str,
    arguments: Value,
) -> Result<String> {
    if let Some(response) = invoke_stdio_tool(&config.name, tool_name, arguments.clone())? {
        return Ok(response);
    }
    let McpTransport::Stdio { command, args, env } = config.transport.clone() else {
        return Err(anyhow!("server '{}' is not stdio MCP", config.name));
    };
    invoke_stdio_tool_ephemeral(&command, &args, &env, tool_name, arguments)
}

fn invoke_stdio_tool_ephemeral(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
    tool_name: &str,
    arguments: Value,
) -> Result<String> {
    let mut child = Command::new(command);
    child.args(args);
    for (key, value) in env {
        child.env(key, value);
    }
    child
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = child
        .spawn()
        .with_context(|| format!("spawn MCP stdio server {command}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("stdio stdin unavailable"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("stdio stdout unavailable"))?;
    let mut reader = BufReader::new(stdout);

    let mut next_id = 1u64;
    write_json_rpc(
        &mut stdin,
        next_id,
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "deepseek-mobile", "version": "0.1.0"}
        }),
    )?;
    let _ = read_json_rpc(&mut reader, next_id)?;
    next_id += 1;
    write_json_rpc(
        &mut stdin,
        next_id,
        "tools/call",
        json!({
            "name": tool_name,
            "arguments": arguments
        }),
    )?;
    let response = read_json_rpc(&mut reader, next_id)?;
    if response.get("error").is_some() {
        return Err(anyhow!("MCP tools/call error: {}", response));
    }
    Ok(serde_json::to_string_pretty(
        &response.get("result").cloned().unwrap_or(response),
    )?)
}

async fn list_tools_http(
    base_url: &str,
    headers: &HashMap<String, String>,
) -> Result<Vec<McpToolDescriptor>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .context("build MCP HTTP client")?;

    let endpoint = mcp_endpoint(base_url);
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let mut request = client.post(&endpoint).json(&body);
    for (key, value) in headers {
        request = request.header(key, value);
    }

    let response = request
        .send()
        .await
        .with_context(|| format!("MCP tools/list request to {}", endpoint))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "MCP tools/list failed with HTTP {}",
            response.status()
        ));
    }

    let payload: JsonRpcResponse = response.json().await.context("parse MCP tools/list JSON")?;

    if let Some(error) = payload.error {
        return Err(anyhow!("MCP tools/list error: {}", error.message));
    }

    let result = payload
        .result
        .ok_or_else(|| anyhow!("MCP tools/list returned no result"))?;
    let listed: ToolsListResult = serde_json::from_value(result)
        .map_err(|error| anyhow!("parse MCP tools/list result: {}", error))?;

    Ok(listed
        .tools
        .into_iter()
        .map(|tool| McpToolDescriptor {
            name: tool.name,
            server: String::new(),
            description: tool.description,
            input_schema: tool.input_schema,
        })
        .collect())
}

fn write_json_rpc(stdin: &mut ChildStdin, id: u64, method: &str, params: Value) -> Result<()> {
    let line = serde_json::to_string(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    }))?;
    stdin.write_all(line.as_bytes())?;
    stdin.write_all(b"\n")?;
    stdin.flush()?;
    Ok(())
}

fn read_json_rpc(reader: &mut BufReader<std::process::ChildStdout>, id: u64) -> Result<Value> {
    let mut line = String::new();
    for _ in 0..32 {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let payload: Value = serde_json::from_str(trimmed)
            .with_context(|| format!("parse MCP line: {}", trimmed))?;
        if payload.get("id").and_then(Value::as_u64) == Some(id) {
            return Ok(payload);
        }
    }
    Err(anyhow!("timed out waiting for MCP response id={}", id))
}

fn mcp_endpoint(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/mcp") || trimmed.ends_with("/sse") {
        trimmed.to_string()
    } else {
        format!("{}/mcp", trimmed)
    }
}

pub fn default_mcp_path() -> std::path::PathBuf {
    std::env::var("DEEPSEEK_MOBILE_DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from(".deepseek-mobile"))
        .join("mcp.json")
}

/// Merge declared tools from config with remote discovery.
pub fn tools_for_server(
    server_name: &str,
    declared: &[McpToolDescriptor],
    remote: Vec<McpToolDescriptor>,
) -> Vec<McpToolDescriptor> {
    if !remote.is_empty() {
        return remote
            .into_iter()
            .map(|mut tool| {
                tool.server = server_name.to_string();
                tool
            })
            .collect();
    }
    declared
        .iter()
        .cloned()
        .map(|mut tool| {
            tool.server = server_name.to_string();
            tool
        })
        .collect()
}

/// Load all tools from connected MCP servers in the registry file.
pub async fn load_connected_mcp_tools(path: &std::path::Path) -> Vec<McpToolDescriptor> {
    let Ok(registry) = crate::mcp::McpClientRegistry::load_or_default(path) else {
        return Vec::new();
    };
    registry
        .servers
        .iter()
        .filter(|server| server.status == McpServerStatus::Connected)
        .flat_map(|server| server.tools.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::mcp_endpoint;

    #[test]
    fn mcp_endpoint_appends_suffix_when_missing() {
        assert_eq!(
            mcp_endpoint("http://localhost:3000"),
            "http://localhost:3000/mcp"
        );
        assert_eq!(
            mcp_endpoint("http://localhost:3000/mcp"),
            "http://localhost:3000/mcp"
        );
    }
}

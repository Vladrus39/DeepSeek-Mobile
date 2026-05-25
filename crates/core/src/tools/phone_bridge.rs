//! Phone-native control tools (URLs, share sheet, app launch).
//!
//! Execution returns metadata consumed by the Android native bridge; no shell
//! access is required on LocalAndroid workspaces.

use super::{required_str, ToolCapability, ToolContext, ToolResult, ToolSpec};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PhoneNativeRequest {
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl PhoneNativeRequest {
    pub fn open_url(url: impl Into<String>) -> Self {
        Self {
            action: "open_url".to_string(),
            url: Some(url.into()),
            path: None,
            package: None,
            mime_type: None,
        }
    }

    pub fn share_file(path: impl Into<String>, mime_type: Option<String>) -> Self {
        Self {
            action: "share_file".to_string(),
            url: None,
            path: Some(path.into()),
            package: None,
            mime_type,
        }
    }

    pub fn launch_app(package: impl Into<String>) -> Self {
        Self {
            action: "launch_app".to_string(),
            url: None,
            path: None,
            package: Some(package.into()),
            mime_type: None,
        }
    }
}

pub struct PhoneControlTool;

impl ToolSpec for PhoneControlTool {
    fn name(&self) -> &str {
        "phone_control"
    }

    fn description(&self) -> &str {
        "Control the Android device UI: open a URL in the browser, share a sandbox file, or launch an installed app by package name. Requires the app to be foregrounded so the native bridge can drain commands."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["open_url", "share_file", "launch_app"],
                    "description": "Native action to perform on the phone."
                },
                "url": { "type": "string", "description": "HTTP/HTTPS or intent URL (open_url)." },
                "path": { "type": "string", "description": "Absolute path inside the phone workspace or trusted grant (share_file)." },
                "package": { "type": "string", "description": "Android package name, e.g. com.termux (launch_app)." },
                "mime_type": { "type": "string", "description": "Optional MIME for share_file." }
            },
            "required": ["action"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability::RequiresApproval]
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let action = required_str(&input, "action")?;
        let request = match action {
            "open_url" => {
                let url = required_str(&input, "url")?.trim();
                if url.is_empty() {
                    return Err(anyhow!("url is required for open_url"));
                }
                PhoneNativeRequest::open_url(url)
            }
            "share_file" => {
                let path = required_str(&input, "path")?;
                let resolved = context.resolve_path(path)?;
                if !resolved.is_file() {
                    return Err(anyhow!("share_file path is not a file: {}", resolved.display()));
                }
                PhoneNativeRequest::share_file(
                    resolved.display().to_string(),
                    input
                        .get("mime_type")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                )
            }
            "launch_app" => {
                let package = required_str(&input, "package")?.trim();
                if package.is_empty() {
                    return Err(anyhow!("package is required for launch_app"));
                }
                PhoneNativeRequest::launch_app(package)
            }
            other => return Err(anyhow!("unsupported phone_control action: {}", other)),
        };

        Ok(ToolResult::success(format!(
            "Phone native action '{}' queued for Android bridge.",
            request.action
        ))
        .with_metadata(json!({
            "executor": "phone_native",
            "phone_native_pending": true,
            "phone_native_status": "pending_native_bridge",
            "phone_native_request": request,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::{PhoneControlTool, ToolSpec};
    use crate::tools::ToolContext;
    use crate::workspace::{ExecutorKind, Workspace};
    use serde_json::{json, Value};
    use std::path::PathBuf;

    #[test]
    fn open_url_emits_native_metadata() {
        let tool = PhoneControlTool;
        let workspace = Workspace::new("w1", "Phone", PathBuf::from("."), ExecutorKind::LocalAndroid);
        let result = tool
            .execute(
                json!({"action": "open_url", "url": "https://example.com"}),
                &ToolContext::new(workspace),
            )
            .unwrap();
        assert!(result.success);
        let metadata = result.metadata.unwrap();
        assert_eq!(
            metadata
                .get("phone_native_pending")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            metadata
                .get("phone_native_request")
                .and_then(|v| v.get("action"))
                .and_then(Value::as_str),
            Some("open_url")
        );
    }
}

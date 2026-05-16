//! Mobile-safe patch application tool.
//!
//! This is intentionally operation-based instead of shelling out to `git apply`.
//! The model can submit a batch of exact operations, every target path is checked
//! through the workspace boundary, and the whole patch is applied atomically: if
//! one operation fails, previously modified files are restored from memory.

use super::{required_str, ApprovalRequirement, ToolCapability, ToolContext, ToolResult, ToolSpec};
use crate::workspace_files::WorkspaceFileService;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub struct ApplyPatchTool;

#[derive(Clone, Debug, PartialEq, Eq)]
enum PatchOperation {
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct AppliedPatchSummary {
    changed_files: BTreeSet<String>,
    operations_applied: usize,
}

impl ToolSpec for ApplyPatchTool {
    fn name(&self) -> &str {
        "apply_patch"
    }

    fn description(&self) -> &str {
        "Apply a batch of exact workspace file changes atomically. Supports replace, create, append and delete operations."
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "operations": {
                    "type": "array",
                    "description": "Patch operations applied in order. The batch is atomic.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string", "enum": ["replace", "create", "append", "delete"] },
                            "path": { "type": "string" },
                            "search": { "type": "string", "description": "Exact text to replace for replace operations" },
                            "replace": { "type": "string", "description": "Replacement text for replace operations" },
                            "content": { "type": "string", "description": "Content for create/append operations" },
                            "overwrite": { "type": "boolean", "description": "Allow create operation to overwrite an existing file" },
                            "expected_occurrences": { "type": "integer", "description": "Optional exact occurrence count for replace operations" }
                        },
                        "required": ["type", "path"]
                    }
                }
            },
            "required": ["operations"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability::WritesFiles,
            ToolCapability::RequiresApproval,
            ToolCapability::Sandboxable,
        ]
    }

    fn approval_requirement(&self) -> ApprovalRequirement {
        ApprovalRequirement::Required
    }

    fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult> {
        let operations = parse_operations(&input)?;
        let summary = apply_operations_atomically(&operations, context)?;
        Ok(ToolResult::success(format!(
            "Applied {} patch operation(s) across {} file(s): {}",
            summary.operations_applied,
            summary.changed_files.len(),
            summary.changed_files.iter().cloned().collect::<Vec<_>>().join(", ")
        ))
        .with_metadata(json!({
            "operations_applied": summary.operations_applied,
            "changed_files": summary.changed_files.into_iter().collect::<Vec<_>>()
        })))
    }
}

fn parse_operations(input: &Value) -> Result<Vec<PatchOperation>> {
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
        .map(|(index, operation)| parse_operation(index, operation))
        .collect()
}

fn parse_operation(index: usize, operation: &Value) -> Result<PatchOperation> {
    let op_type = operation
        .get("type")
        .or_else(|| operation.get("operation"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("patch operation {} is missing type", index))?;
    let path = required_str(operation, "path")?.to_string();

    match op_type {
        "replace" => {
            let search = required_str(operation, "search")?.to_string();
            if search.is_empty() {
                return Err(anyhow!("replace operation {} has empty search text", index));
            }
            let replace = required_str(operation, "replace")?.to_string();
            let expected_occurrences = operation
                .get("expected_occurrences")
                .and_then(Value::as_u64)
                .map(|value| value as usize);
            Ok(PatchOperation::Replace {
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
            Ok(PatchOperation::Create { path, content, overwrite })
        }
        "append" => {
            let content = required_str(operation, "content")?.to_string();
            Ok(PatchOperation::Append { path, content })
        }
        "delete" | "remove" => Ok(PatchOperation::Delete { path }),
        other => Err(anyhow!("unsupported patch operation {}: {}", index, other)),
    }
}

fn apply_operations_atomically(operations: &[PatchOperation], context: &ToolContext) -> Result<AppliedPatchSummary> {
    let service = WorkspaceFileService::new(context.workspace.clone());
    let mut originals: BTreeMap<String, Option<String>> = BTreeMap::new();
    let mut changed_files = BTreeSet::new();

    let result = (|| -> Result<()> {
        for operation in operations {
            let path = operation.path();
            if !originals.contains_key(path) {
                originals.insert(path.to_string(), service.read_text_file(path).ok());
            }
            apply_operation(operation, &service)?;
            changed_files.insert(path.to_string());
        }
        Ok(())
    })();

    if let Err(error) = result {
        rollback(&service, originals)?;
        return Err(error);
    }

    Ok(AppliedPatchSummary {
        changed_files,
        operations_applied: operations.len(),
    })
}

fn apply_operation(operation: &PatchOperation, service: &WorkspaceFileService) -> Result<()> {
    match operation {
        PatchOperation::Replace {
            path,
            search,
            replace,
            expected_occurrences,
        } => {
            let content = service.read_text_file(path)?;
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
            service.write_text_file(path, &updated)
        }
        PatchOperation::Create {
            path,
            content,
            overwrite,
        } => {
            if !*overwrite && service.read_text_file(path).is_ok() {
                return Err(anyhow!("create operation refuses to overwrite existing file: {}", path));
            }
            service.write_text_file(path, content)
        }
        PatchOperation::Append { path, content } => {
            let mut existing = service.read_text_file(path)?;
            existing.push_str(content);
            service.write_text_file(path, &existing)
        }
        PatchOperation::Delete { path } => service.delete_file(path),
    }
}

fn rollback(service: &WorkspaceFileService, originals: BTreeMap<String, Option<String>>) -> Result<()> {
    for (path, original) in originals.into_iter().rev() {
        match original {
            Some(content) => service.write_text_file(&path, &content)?,
            None => {
                let _ = service.delete_file(&path);
            }
        }
    }
    Ok(())
}

impl PatchOperation {
    fn path(&self) -> &str {
        match self {
            PatchOperation::Replace { path, .. }
            | PatchOperation::Create { path, .. }
            | PatchOperation::Append { path, .. }
            | PatchOperation::Delete { path } => path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ApplyPatchTool;
    use crate::tools::{ToolContext, ToolSpec};
    use crate::workspace::{ExecutorKind, Workspace};
    use serde_json::json;
    use std::fs;

    fn context(name: &str) -> (ToolContext, std::path::PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "deepseek_mobile_apply_patch_test_{}_{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let workspace = Workspace::new("w1", "Workspace", root.clone(), ExecutorKind::LocalAndroid);
        (ToolContext::new(workspace), root)
    }

    #[test]
    fn applies_replace_create_and_append_operations() {
        let (context, root) = context("batch");
        fs::write(root.join("README.md"), "hello world\n").unwrap();

        ApplyPatchTool
            .execute(
                json!({
                    "operations": [
                        {"type":"replace","path":"README.md","search":"world","replace":"mobile","expected_occurrences":1},
                        {"type":"create","path":"src/lib.rs","content":"pub fn ok() -> bool { true }\n"},
                        {"type":"append","path":"README.md","content":"done\n"}
                    ]
                }),
                &context,
            )
            .unwrap();

        assert_eq!(fs::read_to_string(root.join("README.md")).unwrap(), "hello mobile\ndone\n");
        assert!(fs::read_to_string(root.join("src/lib.rs")).unwrap().contains("pub fn ok"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rolls_back_previous_operations_when_later_operation_fails() {
        let (context, root) = context("rollback");
        fs::write(root.join("README.md"), "v1\n").unwrap();

        let err = ApplyPatchTool
            .execute(
                json!({
                    "operations": [
                        {"type":"replace","path":"README.md","search":"v1","replace":"v2","expected_occurrences":1},
                        {"type":"replace","path":"README.md","search":"missing","replace":"x","expected_occurrences":1}
                    ]
                }),
                &context,
            )
            .unwrap_err();

        assert!(err.to_string().contains("search text not found"));
        assert_eq!(fs::read_to_string(root.join("README.md")).unwrap(), "v1\n");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_path_traversal_through_workspace_service() {
        let (context, root) = context("path_escape");
        let err = ApplyPatchTool
            .execute(
                json!({
                    "operations": [
                        {"type":"create","path":"../escape.txt","content":"no"}
                    ]
                }),
                &context,
            )
            .unwrap_err();
        assert!(err.to_string().contains("outside workspace"));
        assert!(!root.parent().unwrap().join("escape.txt").exists());
        let _ = fs::remove_dir_all(root);
    }
}

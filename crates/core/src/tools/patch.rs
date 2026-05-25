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
        "Apply workspace file changes atomically. Supports exact operation batches and standard unified diff patches."
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
                },
                "unified_diff": {
                    "type": "string",
                    "description": "Optional standard unified diff. Use this instead of operations when adapting git/apply_patch style patches."
                },
                "patch": {
                    "type": "string",
                    "description": "Alias for unified_diff."
                }
            }
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
    if input.get("operations").is_some() {
        let operations = input
            .get("operations")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("apply_patch operations must be an array"))?;
        if operations.is_empty() {
            return Err(anyhow!("apply_patch operations array must not be empty"));
        }

        return operations
            .iter()
            .enumerate()
            .map(|(index, operation)| parse_operation(index, operation))
            .collect();
    }

    let unified_diff = input
        .get("unified_diff")
        .or_else(|| input.get("patch"))
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("apply_patch requires either an operations array or unified_diff string"))?;
    parse_unified_diff_operations(unified_diff)
}

pub(crate) fn normalized_operations_value(input: &Value) -> Result<Value> {
    let operations = parse_operations(input)?;
    Ok(Value::Array(
        operations
            .iter()
            .map(patch_operation_to_value)
            .collect::<Vec<_>>(),
    ))
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedUnifiedHunk {
    old_text: String,
    new_text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LastHunkTarget {
    None,
    Old,
    New,
    Both,
}

fn parse_unified_diff_operations(diff: &str) -> Result<Vec<PatchOperation>> {
    if diff.trim().is_empty() {
        return Err(anyhow!("unified_diff must not be empty"));
    }

    let lines = diff.split_inclusive('\n').collect::<Vec<_>>();
    let mut index = 0;
    let mut saw_file_header = false;
    let mut operations = Vec::new();

    while index < lines.len() {
        if !lines[index].starts_with("--- ") {
            index += 1;
            continue;
        }

        saw_file_header = true;
        let old_path = parse_diff_header_path(lines[index], "--- ")?;
        index += 1;

        if index >= lines.len() || !lines[index].starts_with("+++ ") {
            return Err(anyhow!("unified diff file header is missing matching +++ line"));
        }
        let new_path = parse_diff_header_path(lines[index], "+++ ")?;
        index += 1;

        let mut hunks = Vec::new();
        while index < lines.len() {
            if lines[index].starts_with("--- ") {
                break;
            }
            if lines[index].starts_with("@@ ") || lines[index].starts_with("@@") {
                hunks.push(parse_unified_hunk(&lines, &mut index)?);
            } else {
                index += 1;
            }
        }

        append_file_patch_operations(old_path, new_path, hunks, &mut operations)?;
    }

    if !saw_file_header {
        return Err(anyhow!("unified diff must contain --- and +++ file headers"));
    }
    if operations.is_empty() {
        return Err(anyhow!("unified diff did not contain supported file changes"));
    }

    Ok(operations)
}

fn parse_diff_header_path(line: &str, prefix: &str) -> Result<Option<String>> {
    let raw = line
        .strip_prefix(prefix)
        .ok_or_else(|| anyhow!("invalid unified diff header: {}", line.trim_end()))?;
    let without_line_ending = raw.trim_end_matches(['\r', '\n']);
    let without_timestamp = without_line_ending
        .split_once('\t')
        .map(|(path, _)| path)
        .unwrap_or(without_line_ending)
        .trim_end();

    if without_timestamp == "/dev/null" {
        return Ok(None);
    }

    let unquoted = without_timestamp
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(without_timestamp);
    let normalized = unquoted
        .strip_prefix("a/")
        .or_else(|| unquoted.strip_prefix("b/"))
        .unwrap_or(unquoted)
        .replace('\\', "/");

    if normalized.trim().is_empty() {
        return Err(anyhow!("unified diff contains an empty file path"));
    }

    Ok(Some(normalized))
}

fn parse_unified_hunk(lines: &[&str], index: &mut usize) -> Result<ParsedUnifiedHunk> {
    let (old_len, new_len) = parse_hunk_lengths(lines[*index])?;
    *index += 1;

    let mut old_seen = 0usize;
    let mut new_seen = 0usize;
    let mut old_text = String::new();
    let mut new_text = String::new();
    let mut last_target = LastHunkTarget::None;

    while *index < lines.len() && (old_seen < old_len || new_seen < new_len) {
        let line = lines[*index];
        if line.starts_with("\\ ") {
            trim_last_hunk_newline(&mut old_text, &mut new_text, last_target);
            *index += 1;
            continue;
        }

        let prefix = line
            .as_bytes()
            .first()
            .copied()
            .ok_or_else(|| anyhow!("invalid empty unified diff hunk line"))? as char;
        let content = &line[1..];

        match prefix {
            ' ' => {
                old_text.push_str(content);
                new_text.push_str(content);
                old_seen += 1;
                new_seen += 1;
                last_target = LastHunkTarget::Both;
            }
            '-' => {
                old_text.push_str(content);
                old_seen += 1;
                last_target = LastHunkTarget::Old;
            }
            '+' => {
                new_text.push_str(content);
                new_seen += 1;
                last_target = LastHunkTarget::New;
            }
            other => {
                return Err(anyhow!(
                    "unsupported unified diff hunk line prefix '{}'",
                    other
                ));
            }
        }

        *index += 1;
    }

    if old_seen != old_len || new_seen != new_len {
        return Err(anyhow!(
            "unified diff hunk length mismatch: expected -{} +{}, got -{} +{}",
            old_len,
            new_len,
            old_seen,
            new_seen
        ));
    }

    Ok(ParsedUnifiedHunk { old_text, new_text })
}

fn parse_hunk_lengths(line: &str) -> Result<(usize, usize)> {
    let mut parts = line.split_whitespace();
    let marker = parts.next().unwrap_or_default();
    if marker != "@@" {
        return Err(anyhow!("invalid unified diff hunk header: {}", line.trim_end()));
    }
    let old_range = parts
        .next()
        .ok_or_else(|| anyhow!("unified diff hunk header missing old range"))?;
    let new_range = parts
        .next()
        .ok_or_else(|| anyhow!("unified diff hunk header missing new range"))?;
    Ok((
        parse_hunk_range_len(old_range, '-')?,
        parse_hunk_range_len(new_range, '+')?,
    ))
}

fn parse_hunk_range_len(token: &str, prefix: char) -> Result<usize> {
    let range = token
        .strip_prefix(prefix)
        .ok_or_else(|| anyhow!("invalid unified diff range '{}'", token))?;
    if let Some((_, len)) = range.split_once(',') {
        len.parse::<usize>()
            .map_err(|error| anyhow!("invalid unified diff range length '{}': {}", len, error))
    } else {
        Ok(1)
    }
}

fn trim_last_hunk_newline(
    old_text: &mut String,
    new_text: &mut String,
    last_target: LastHunkTarget,
) {
    match last_target {
        LastHunkTarget::Old => trim_trailing_line_ending(old_text),
        LastHunkTarget::New => trim_trailing_line_ending(new_text),
        LastHunkTarget::Both => {
            trim_trailing_line_ending(old_text);
            trim_trailing_line_ending(new_text);
        }
        LastHunkTarget::None => {}
    }
}

fn trim_trailing_line_ending(text: &mut String) {
    if text.ends_with('\n') {
        text.pop();
        if text.ends_with('\r') {
            text.pop();
        }
    }
}

fn append_file_patch_operations(
    old_path: Option<String>,
    new_path: Option<String>,
    hunks: Vec<ParsedUnifiedHunk>,
    operations: &mut Vec<PatchOperation>,
) -> Result<()> {
    match (old_path, new_path) {
        (None, Some(path)) => {
            let content = hunks
                .into_iter()
                .map(|hunk| hunk.new_text)
                .collect::<String>();
            operations.push(PatchOperation::Create {
                path,
                content,
                overwrite: false,
            });
        }
        (Some(path), None) => {
            operations.push(PatchOperation::Delete { path });
        }
        (Some(old_path), Some(new_path)) => {
            if old_path != new_path {
                return Err(anyhow!(
                    "unified diff renames are not supported by apply_patch: {} -> {}",
                    old_path,
                    new_path
                ));
            }
            for hunk in hunks {
                if hunk.old_text == hunk.new_text {
                    continue;
                }
                if hunk.old_text.is_empty() {
                    return Err(anyhow!(
                        "zero-context insertion hunks are not supported for existing file '{}'; use operations instead",
                        new_path
                    ));
                }
                operations.push(PatchOperation::Replace {
                    path: new_path.clone(),
                    search: hunk.old_text,
                    replace: hunk.new_text,
                    expected_occurrences: Some(1),
                });
            }
        }
        (None, None) => {
            return Err(anyhow!(
                "unified diff cannot use /dev/null for both old and new paths"
            ));
        }
    }

    Ok(())
}

fn patch_operation_to_value(operation: &PatchOperation) -> Value {
    match operation {
        PatchOperation::Replace {
            path,
            search,
            replace,
            expected_occurrences,
        } => json!({
            "type": "replace",
            "path": path,
            "search": search,
            "replace": replace,
            "expected_occurrences": expected_occurrences,
        }),
        PatchOperation::Create {
            path,
            content,
            overwrite,
        } => json!({
            "type": "create",
            "path": path,
            "content": content,
            "overwrite": overwrite,
        }),
        PatchOperation::Append { path, content } => json!({
            "type": "append",
            "path": path,
            "content": content,
        }),
        PatchOperation::Delete { path } => json!({
            "type": "delete",
            "path": path,
        }),
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
    fn applies_unified_diff_to_existing_file() {
        let (context, root) = context("unified_modify");
        fs::write(root.join("README.md"), "hello world\nold line\n").unwrap();

        ApplyPatchTool
            .execute(
                json!({
                    "unified_diff": "\
--- a/README.md
+++ b/README.md
@@ -1,2 +1,2 @@
-hello world
+hello mobile
 old line
"
                }),
                &context,
            )
            .unwrap();

        assert_eq!(
            fs::read_to_string(root.join("README.md")).unwrap(),
            "hello mobile\nold line\n"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn applies_unified_diff_create_file() {
        let (context, root) = context("unified_create");

        ApplyPatchTool
            .execute(
                json!({
                    "patch": "\
--- /dev/null
+++ b/src/lib.rs
@@ -0,0 +1,3 @@
+pub fn ok() -> bool {
+    true
+}
"
                }),
                &context,
            )
            .unwrap();

        assert!(fs::read_to_string(root.join("src/lib.rs"))
            .unwrap()
            .contains("pub fn ok"));
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
    fn unified_diff_rollback_preserves_prior_files_on_later_failure() {
        let (context, root) = context("unified_rollback");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        fs::write(root.join("b.txt"), "two\n").unwrap();

        let err = ApplyPatchTool
            .execute(
                json!({
                    "unified_diff": "\
--- a/a.txt
+++ b/a.txt
@@ -1 +1 @@
-one
+ONE
--- a/b.txt
+++ b/b.txt
@@ -1 +1 @@
-missing
+MISSING
"
                }),
                &context,
            )
            .unwrap_err();

        assert!(err.to_string().contains("search text not found"));
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "one\n");
        assert_eq!(fs::read_to_string(root.join("b.txt")).unwrap(), "two\n");
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

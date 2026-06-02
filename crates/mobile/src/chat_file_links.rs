//! Extract workspace file paths from tool results and chat text for "open in Files" links.

use serde_json::Value;
use std::collections::HashSet;

const FILE_EXTENSIONS: &[&str] = &[
    ".rs", ".py", ".md", ".toml", ".json", ".txt", ".kt", ".kts", ".java", ".tsx", ".ts", ".js",
    ".html", ".css", ".gradle", ".xml", ".yml", ".yaml", ".sh", ".ps1", ".bat", ".cmd", ".zip",
];

pub fn extract_paths_from_tool_result(
    tool_name: &str,
    output: &str,
    metadata: Option<&Value>,
) -> Vec<String> {
    let mut paths = HashSet::new();
    if let Some(meta) = metadata {
        collect_path_fields(meta, &mut paths);
    }
    if let Some(path) = parse_write_file_output(output) {
        paths.insert(path);
    }
    let _ = tool_name;
    for path in extract_paths_from_text(output) {
        paths.insert(path);
    }
    sort_paths(paths)
}

pub fn extract_paths_from_tool_args(tool_name: &str, args_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(args_json) else {
        return Vec::new();
    };
    let mut paths = HashSet::new();
    collect_path_fields(&value, &mut paths);
    if matches!(
        tool_name,
        "write_file" | "read_file" | "edit_file" | "delete_file" | "list_dir" | "open_path"
    ) {
        if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
            paths.insert(normalize_path(path));
        }
    }
    sort_paths(paths)
}

pub fn extract_paths_from_text(text: &str) -> Vec<String> {
    let mut paths = HashSet::new();
    for token in text.split_whitespace() {
        let trimmed = trim_path_token(token);
        if looks_like_file_path(trimmed) {
            paths.insert(normalize_path(trimmed));
        }
    }
    for segment in text.split(['\n', ',', ';', '(', ')', '[', ']', '`', '"', '\'']) {
        let trimmed = trim_path_token(segment);
        if looks_like_file_path(trimmed) {
            paths.insert(normalize_path(trimmed));
        }
    }
    sort_paths(paths)
}

fn collect_path_fields(value: &Value, out: &mut HashSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "path" || key.ends_with("_path") || key == "file" {
                    if let Some(text) = child.as_str() {
                        if looks_like_file_path(text) {
                            out.insert(normalize_path(text));
                        }
                    }
                }
                collect_path_fields(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_path_fields(item, out);
            }
        }
        _ => {}
    }
}

fn parse_write_file_output(output: &str) -> Option<String> {
    let marker = " to ";
    let idx = output.rfind(marker)?;
    let path = output[idx + marker.len()..].trim();
    if path.is_empty() || path.contains(' ') {
        return None;
    }
    Some(normalize_path(path))
}

fn trim_path_token(token: &str) -> &str {
    token.trim_matches(|c: char| {
        !c.is_alphanumeric() && c != '/' && c != '.' && c != '_' && c != '-' && c != '\\'
    })
}

fn looks_like_file_path(path: &str) -> bool {
    if path.len() < 3 || path.contains("://") {
        return false;
    }
    FILE_EXTENSIONS
        .iter()
        .any(|ext| path.ends_with(ext) || path.contains(&format!("{ext}/")))
}

fn normalize_path(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    while normalized.starts_with("./") {
        normalized = normalized[2..].to_string();
    }
    if normalized.starts_with('/') && normalized.len() > 1 {
        if let Some(idx) = normalized.rfind("/deepseek-mobile/Project workspace/") {
            normalized =
                normalized[idx + "/deepseek-mobile/Project workspace/".len()..].to_string();
        } else if let Some(idx) = normalized.rfind("/deepseek-mobile/workspace/") {
            normalized = normalized[idx + "/deepseek-mobile/workspace/".len()..].to_string();
        } else if let Some(idx) = normalized.rfind("/files/deepseek-mobile/Project workspace/") {
            normalized =
                normalized[idx + "/files/deepseek-mobile/Project workspace/".len()..].to_string();
        } else if let Some(idx) = normalized.rfind("/files/deepseek-mobile/workspace/") {
            normalized = normalized[idx + "/files/deepseek-mobile/workspace/".len()..].to_string();
        }
    }
    normalized
}

fn sort_paths(paths: HashSet<String>) -> Vec<String> {
    let mut sorted: Vec<String> = paths.into_iter().collect();
    sorted.sort();
    sorted
}

#[cfg(test)]
mod tests {
    use super::{
        extract_paths_from_text, extract_paths_from_tool_args, extract_paths_from_tool_result,
        normalize_path,
    };
    #[test]
    fn parses_write_file_output() {
        let paths =
            extract_paths_from_tool_result("write_file", "Wrote 12 bytes to src/demo.rs", None);
        assert!(paths.contains(&"src/demo.rs".to_string()));
    }

    #[test]
    fn parses_tool_args_path() {
        let paths = extract_paths_from_tool_args(
            "write_file",
            r#"{"path":"pkg/calc.py","content":"print(1)"}"#,
        );
        assert_eq!(paths, vec!["pkg/calc.py".to_string()]);
    }

    #[test]
    fn extracts_paths_from_assistant_text() {
        let paths = extract_paths_from_text("Created hello.py and pkg/calc.py for you.");
        assert!(paths.contains(&"hello.py".to_string()));
        assert!(paths.contains(&"pkg/calc.py".to_string()));
    }

    #[test]
    fn strips_workspace_prefix() {
        assert_eq!(
            normalize_path(
                "/data/user/0/com.deepseek.mobile/files/deepseek-mobile/workspace/src/a.rs"
            ),
            "src/a.rs"
        );
    }
}

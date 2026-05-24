use anyhow::{anyhow, Result};
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const DEFAULT_MAX_TREE_ENTRIES: usize = 250;
pub const DEFAULT_MAX_FILE_BYTES: u64 = 256 * 1024;
pub const DEFAULT_MAX_FILE_CHARS: usize = 80_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectEntryKind {
    Directory,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectTreeEntry {
    pub path: String,
    pub name: String,
    pub kind: ProjectEntryKind,
    pub depth: usize,
    pub size_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectTreeSnapshot {
    pub root: String,
    pub entries: Vec<ProjectTreeEntry>,
    pub truncated: bool,
}

impl ProjectTreeSnapshot {
    pub fn file_count(&self) -> usize {
        self.entries.iter().filter(|entry| matches!(entry.kind, ProjectEntryKind::File)).count()
    }

    pub fn directory_count(&self) -> usize {
        self.entries.iter().filter(|entry| matches!(entry.kind, ProjectEntryKind::Directory)).count()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectFilePreview {
    pub path: String,
    pub display_name: String,
    pub content: String,
    pub size_bytes: u64,
    pub line_count: usize,
    pub truncated: bool,
}

pub fn scan_project_tree(root: impl AsRef<Path>, max_entries: usize) -> Result<ProjectTreeSnapshot> {
    let root = root.as_ref().to_path_buf();
    let mut entries = Vec::new();
    let mut truncated = false;

    if root.exists() {
        scan_dir(&root, &root, 0, max_entries, &mut entries, &mut truncated)?;
    }

    Ok(ProjectTreeSnapshot {
        root: root.to_string_lossy().to_string(),
        entries,
        truncated,
    })
}

pub fn read_project_file(
    root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
    max_bytes: u64,
) -> Result<ProjectFilePreview> {
    let root = root.as_ref();
    let relative_path = relative_path.as_ref();
    let file_path = safe_join(root, relative_path)?;
    let metadata = fs::metadata(&file_path)?;
    if !metadata.is_file() {
        return Err(anyhow!("not a file: {}", relative_path.display()));
    }
    if metadata.len() > max_bytes {
        return Err(anyhow!("file too large for mobile preview: {} > {} bytes", metadata.len(), max_bytes));
    }

    let text = fs::read_to_string(&file_path)
        .map_err(|error| anyhow!("failed to read UTF-8 project file {}: {}", relative_path.display(), error))?;
    let line_count = text.lines().count();
    let truncated = text.chars().count() > DEFAULT_MAX_FILE_CHARS;
    let content = if truncated {
        let mut out = text.chars().take(DEFAULT_MAX_FILE_CHARS).collect::<String>();
        out.push_str("\n...[project file preview truncated]...");
        out
    } else {
        text
    };

    Ok(ProjectFilePreview {
        path: normalize_relative_path(relative_path)?,
        display_name: relative_path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string(),
        content,
        size_bytes: metadata.len(),
        line_count,
        truncated,
    })
}

/// Scan a remote PC gateway workspace directory tree.
pub async fn scan_pc_gateway_tree(
    client: &deepseek_mobile_core::PcGatewayClient,
    workspace_id: &str,
    max_entries: usize,
) -> Result<ProjectTreeSnapshot> {
    let response = client.list_dir(workspace_id, ".").await?;
    let entries = match response {
        deepseek_mobile_core::PcGatewayResponse::DirEntries(entries) => entries,
        other => return Err(anyhow!("unexpected gateway response: {:?}", other)),
    };

    let mut tree_entries = Vec::new();
    let mut truncated = false;

    for entry in entries.iter().take(max_entries) {
        if tree_entries.len() >= max_entries {
            truncated = true;
            break;
        }
        let name = std::path::Path::new(&entry.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&entry.path)
            .to_string();

        if is_ignored_name(&name) {
            continue;
        }

        if entry.is_dir {
            // Recursively scan subdirectories
            let sub_response = client.list_dir(workspace_id, &entry.path).await?;
            if let deepseek_mobile_core::PcGatewayResponse::DirEntries(sub_entries) = sub_response {
                for sub in sub_entries {
                    if tree_entries.len() >= max_entries {
                        truncated = true;
                        break;
                    }
                    let sub_name = std::path::Path::new(&sub.path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&sub.path)
                        .to_string();
                    if is_ignored_name(&sub_name) {
                        continue;
                    }
                    tree_entries.push(ProjectTreeEntry {
                        path: sub.path.clone(),
                        name: sub_name,
                        kind: if sub.is_dir { ProjectEntryKind::Directory } else { ProjectEntryKind::File },
                        depth: 1,
                        size_bytes: None,
                    });
                }
            }
            tree_entries.push(ProjectTreeEntry {
                path: entry.path.clone(),
                name,
                kind: ProjectEntryKind::Directory,
                depth: 0,
                size_bytes: None,
            });
        } else {
            tree_entries.push(ProjectTreeEntry {
                path: entry.path.clone(),
                name,
                kind: ProjectEntryKind::File,
                depth: 0,
                size_bytes: None,
            });
        }
    }

    Ok(ProjectTreeSnapshot {
        root: workspace_id.to_string(),
        entries: tree_entries,
        truncated,
    })
}

/// Read a file from a remote PC gateway workspace.
pub async fn read_pc_gateway_file(
    client: &deepseek_mobile_core::PcGatewayClient,
    workspace_id: &str,
    relative_path: &str,
    max_bytes: u64,
) -> Result<ProjectFilePreview> {
    let response = client.read_file(workspace_id, relative_path).await?;
    let content = match response {
        deepseek_mobile_core::PcGatewayResponse::FileContent { content, .. } => content,
        other => return Err(anyhow!("unexpected gateway response: {:?}", other)),
    };

    if content.len() as u64 > max_bytes {
        return Err(anyhow!("file too large for mobile preview: {} > {} bytes", content.len(), max_bytes));
    }

    let line_count = content.lines().count();
    let truncated = content.chars().count() > DEFAULT_MAX_FILE_CHARS;
    let display_content = if truncated {
        let mut out = content.chars().take(DEFAULT_MAX_FILE_CHARS).collect::<String>();
        out.push_str("\n...[project file preview truncated]...");
        out
    } else {
        content.clone()
    };

    Ok(ProjectFilePreview {
        path: normalize_relative_path(std::path::Path::new(relative_path))?,
        display_name: std::path::Path::new(relative_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string(),
        content: display_content,
        size_bytes: content.len() as u64,
        line_count,
        truncated,
    })
}

pub fn choose_default_preview_file(snapshot: &ProjectTreeSnapshot) -> Option<String> {
    snapshot
        .entries
        .iter()
        .find(|entry| matches!(entry.kind, ProjectEntryKind::File) && is_previewable_text_path(&entry.path))
        .map(|entry| entry.path.clone())
}

pub fn is_previewable_text_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".rs")
        || lower.ends_with(".py")
        || lower.ends_with(".js")
        || lower.ends_with(".ts")
        || lower.ends_with(".tsx")
        || lower.ends_with(".jsx")
        || lower.ends_with(".json")
        || lower.ends_with(".md")
        || lower.ends_with(".toml")
        || lower.ends_with(".yaml")
        || lower.ends_with(".yml")
        || lower.ends_with(".txt")
        || lower.ends_with(".html")
        || lower.ends_with(".css")
        || lower.ends_with(".xml")
        || lower.ends_with(".sh")
}

fn scan_dir(
    root: &Path,
    dir: &Path,
    depth: usize,
    max_entries: usize,
    entries: &mut Vec<ProjectTreeEntry>,
    truncated: &mut bool,
) -> Result<()> {
    if entries.len() >= max_entries {
        *truncated = true;
        return Ok(());
    }

    let mut children = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .filter(|entry| !is_ignored_name(&entry.file_name().to_string_lossy()))
        .collect::<Vec<_>>();
    children.sort_by_key(|entry| entry.path());

    for child in children {
        if entries.len() >= max_entries {
            *truncated = true;
            break;
        }
        let path = child.path();
        let metadata = match child.metadata() {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let relative = match path.strip_prefix(root) {
            Ok(relative) => relative,
            Err(_) => continue,
        };
        let name = child.file_name().to_string_lossy().to_string();

        if metadata.is_dir() {
            entries.push(ProjectTreeEntry {
                path: normalize_relative_path(relative)?,
                name,
                kind: ProjectEntryKind::Directory,
                depth,
                size_bytes: None,
            });
            scan_dir(root, &path, depth + 1, max_entries, entries, truncated)?;
        } else if metadata.is_file() {
            entries.push(ProjectTreeEntry {
                path: normalize_relative_path(relative)?,
                name,
                kind: ProjectEntryKind::File,
                depth,
                size_bytes: Some(metadata.len()),
            });
        }
    }
    Ok(())
}

fn is_ignored_name(name: &str) -> bool {
    matches!(name, ".git" | "target" | "node_modules" | ".gradle" | "build" | "dist" | ".idea")
}

fn safe_join(root: &Path, relative_path: &Path) -> Result<PathBuf> {
    let normalized = normalize_relative_path(relative_path)?;
    Ok(root.join(normalized))
}

fn normalize_relative_path(path: &Path) -> Result<String> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(anyhow!("unsafe project path: {}", path.display()));
            }
        }
    }
    Ok(out.to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::{choose_default_preview_file, read_project_file, scan_project_tree};
    use std::fs;

    fn unique_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("deepseek-mobile-project-{}-{}", name, nanos))
    }

    #[test]
    fn scans_project_tree_and_reads_file_preview() {
        let root = unique_dir("tree");
        fs::create_dir_all(root.join("src")).expect("create test dir");
        fs::write(root.join("src/main.rs"), "fn main() { println!(\"hi\"); }\n").expect("write file");
        let snapshot = scan_project_tree(&root, 50).expect("scan tree");
        assert_eq!(snapshot.file_count(), 1);
        assert_eq!(choose_default_preview_file(&snapshot).as_deref(), Some("src/main.rs"));
        let preview = read_project_file(&root, "src/main.rs", 4096).expect("read preview");
        assert_eq!(preview.path, "src/main.rs");
        assert!(preview.content.contains("fn main"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_path_traversal_preview() {
        let root = unique_dir("unsafe");
        fs::create_dir_all(&root).expect("create test dir");
        let result = read_project_file(&root, "../secret.txt", 4096);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(root);
    }
}
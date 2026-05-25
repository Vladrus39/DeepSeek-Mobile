//! Workspace import/export helpers.
//!
//! Provides ZIP-based project import (unpack archive into workspace)
//! and export (bundle workspace files into a shareable archive).

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::{fs, io};

/// Extract a ZIP archive into a workspace directory.
///
/// All paths inside the archive are resolved relative to `workspace_root`.
/// Parent-directory entries (`..`) are rejected.
pub fn import_project(zip_path: &Path, workspace_root: &Path) -> Result<()> {
    if !zip_path.is_file() {
        return Err(anyhow!("zip file not found: {}", zip_path.display()));
    }

    fs::create_dir_all(workspace_root)
        .with_context(|| format!("create workspace root {}", workspace_root.display()))?;

    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("open zip archive {}", zip_path.display()))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();
        let Some(entry_path) = safe_zip_path(&entry_name) else {
            return Err(anyhow!(
                "rejected unsafe path in zip archive: {:?}",
                entry_name
            ));
        };

        let target = workspace_root.join(&entry_path);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = fs::File::create(&target)?;
            io::copy(&mut entry, &mut out)?;
        }
    }

    Ok(())
}

/// Create a ZIP archive containing the workspace directory contents.
///
/// The archive is saved at `output_path`. The `.deepseek-mobile` metadata
/// directory is excluded from the archive.
pub fn export_project(workspace_root: &Path, output_path: &Path) -> Result<()> {
    if !workspace_root.is_dir() {
        return Err(anyhow!(
            "workspace root is not a directory: {}",
            workspace_root.display()
        ));
    }

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let file = fs::File::create(output_path)?;
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    add_dir_to_zip(&mut writer, workspace_root, workspace_root, options)?;
    writer.finish()?;
    Ok(())
}

fn add_dir_to_zip(
    writer: &mut zip::ZipWriter<fs::File>,
    base: &Path,
    dir: &Path,
    options: zip::write::FileOptions,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip the metadata directory
        if path.is_dir() && path.file_name().map_or(false, |n| n == ".deepseek-mobile") {
            continue;
        }

        let relative = path.strip_prefix(base)?;

        if path.is_dir() {
            let dir_path = format!("{}/", zip_entry_name(relative));
            writer.add_directory(&dir_path, options)?;
            add_dir_to_zip(writer, base, &path, options)?;
        } else {
            writer.start_file(zip_entry_name(relative), options)?;
            let mut f = fs::File::open(&path)?;
            io::copy(&mut f, writer)?;
        }
    }
    Ok(())
}

fn zip_entry_name(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// Security check: reject parent-directory traversals and absolute paths in ZIP
/// entry names. ZIP names are normalized with `/`, but backslashes are also
/// treated as separators so Windows-style traversal is rejected consistently on
/// Linux/macOS CI and Android.
fn safe_zip_path(raw: &str) -> Option<PathBuf> {
    let mut safe = PathBuf::new();

    let normalized = raw.replace('\\', "/");
    if normalized.starts_with('/') {
        return None;
    }
    let path = Path::new(&normalized);
    if path.is_absolute() {
        return None;
    }

    for segment in normalized.split('/') {
        match segment {
            "" | "." => {}
            ".." => return None,
            value if value.contains(':') => return None,
            value => safe.push(value),
        }
    }

    if safe.as_os_str().is_empty() {
        return None;
    }
    Some(safe)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn import_extracts_file_structure() {
        let dir = temp_dir("import-extracts");
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("project.zip");
        let workspace = dir.join("ws");

        // Create a minimal ZIP
        {
            let f = fs::File::create(&zip_path).unwrap();
            let mut writer = zip::ZipWriter::new(f);
            let opts = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("README.md", opts).unwrap();
            writer.write_all(b"# Project\n").unwrap();
            writer.start_file("src/main.rs", opts).unwrap();
            writer.write_all(b"fn main() {}").unwrap();
            writer.finish().unwrap();
        }

        import_project(&zip_path, &workspace).unwrap();

        assert!(workspace.join("README.md").exists());
        assert_eq!(
            fs::read_to_string(workspace.join("README.md")).unwrap(),
            "# Project\n"
        );
        assert!(workspace.join("src/main.rs").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn import_rejects_parent_dir_traversal() {
        let dir = temp_dir("import-traverse");
        fs::create_dir_all(&dir).unwrap();
        let zip_path = dir.join("evil.zip");
        let workspace = dir.join("ws");

        // Manually craft a ZIP with .. path via bytes
        let bytes = {
            use std::io::Cursor;
            let mut buf = Cursor::new(Vec::new());
            {
                let mut writer = zip::ZipWriter::new(&mut buf);
                let opts = zip::write::FileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored);
                writer.start_file("../secrets.txt", opts).unwrap();
                writer.finish().unwrap();
            }
            buf.into_inner()
        };
        fs::write(&zip_path, bytes).unwrap();

        let err = import_project(&zip_path, &workspace)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unsafe") || err.contains("safe"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_and_reimport_preserves_files() {
        let dir = temp_dir("export-reimport");
        let workspace = dir.join("ws");
        fs::create_dir_all(workspace.join("src")).unwrap();
        fs::write(workspace.join("README.md"), "# Export Test\n").unwrap();
        fs::write(workspace.join("src/lib.rs"), "pub fn hello() {}").unwrap();

        // Export
        let export_path = dir.join("export.zip");
        export_project(&workspace, &export_path).unwrap();
        assert!(export_path.exists());

        // Reimport into a fresh workspace
        let fresh = dir.join("fresh");
        import_project(&export_path, &fresh).unwrap();

        assert_eq!(
            fs::read_to_string(fresh.join("README.md")).unwrap(),
            "# Export Test\n"
        );
        assert!(fresh.join("src/lib.rs").exists());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_excludes_meta_dir() {
        let dir = temp_dir("export-meta");
        let workspace = dir.join("ws");
        fs::create_dir_all(workspace.join(".deepseek-mobile")).unwrap();
        fs::write(workspace.join("file.txt"), "hello").unwrap();

        let export_path = dir.join("export.zip");
        export_project(&workspace, &export_path).unwrap();

        // Check that .deepseek-mobile is not in the ZIP
        let f = fs::File::open(&export_path).unwrap();
        let mut archive = zip::ZipArchive::new(f).unwrap();
        for i in 0..archive.len() {
            let name = archive.by_index(i).unwrap().name().to_string();
            assert!(
                !name.starts_with(".deepseek-mobile"),
                "found meta dir: {}",
                name
            );
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn safe_zip_path_rejects_parent_traversal() {
        assert!(safe_zip_path("../secrets.txt").is_none());
        assert!(safe_zip_path("src/../../etc/passwd").is_none());
        assert!(safe_zip_path("src\\..\\secrets.txt").is_none());
    }

    #[test]
    fn safe_zip_path_rejects_absolute_and_windows_prefix_paths() {
        assert!(safe_zip_path("/etc/passwd").is_none());
        assert!(safe_zip_path("C:/Users/secret.txt").is_none());
        assert!(safe_zip_path("C:\\Users\\secret.txt").is_none());
        assert!(safe_zip_path("C:Users/secret.txt").is_none());
        assert!(safe_zip_path("src/C:secret.txt").is_none());
    }

    #[test]
    fn safe_zip_path_accepts_normal_paths() {
        assert_eq!(
            safe_zip_path("readme.md").unwrap(),
            PathBuf::from("readme.md")
        );
        assert_eq!(
            safe_zip_path("src/main.rs").unwrap(),
            PathBuf::from("src/main.rs")
        );
    }

    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "deepseek-ws-io-{}-{}-{}",
            label,
            std::process::id(),
            id,
        ))
    }
}

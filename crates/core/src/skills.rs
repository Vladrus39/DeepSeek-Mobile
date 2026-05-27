//! Skills — local instruction bundles with manifest-based discovery.
//!
//! Skills are lightweight folders containing a `SKILL.md` file with frontmatter.
//! Discovery paths mirror the DeepSeek TUI convention:
//!   ~/.deepseek/skills, <workspace>/.agents/skills, etc.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ── Manifest ──

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub description: String,
    /// Path to the SKILL.md file
    pub path: PathBuf,
    /// Whether the skill is currently active
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Companion file entries relative to the skill folder
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl SkillManifest {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let (frontmatter, _body) = parse_frontmatter(&content)?;
        let name = frontmatter
            .get("name")
            .cloned()
            .ok_or_else(|| anyhow!("missing 'name' in frontmatter of {}", path.display()))?;
        let description = frontmatter
            .get("description")
            .cloned()
            .unwrap_or_else(|| "No description".to_string());

        let dir = path.parent().unwrap_or(Path::new("."));
        let files = list_companion_files(dir);

        Ok(Self {
            name,
            description,
            path: path.to_path_buf(),
            enabled: true,
            files,
        })
    }
}

// ── Registry ──

#[derive(Clone, Debug, Default)]
pub struct SkillRegistry {
    pub skills: Vec<SkillManifest>,
}

impl SkillRegistry {
    /// Scan the given directories for SKILL.md files and load manifests.
    pub fn discover(paths: &[PathBuf]) -> Result<Self> {
        let mut skills = Vec::new();
        for root in paths {
            if !root.exists() {
                continue;
            }
            for entry in walk_skill_dirs(root)? {
                let manifest = SkillManifest::load_from_file(&entry)?;
                // Avoid duplicates by name (first found wins)
                if skills
                    .iter()
                    .any(|s: &SkillManifest| s.name == manifest.name)
                {
                    continue;
                }
                skills.push(manifest);
            }
        }
        Ok(Self { skills })
    }

    /// Default discovery paths for mobile.
    pub fn discover_default() -> Result<Self> {
        let data_dir = PathBuf::from(
            std::env::var("DEEPSEEK_MOBILE_DATA_DIR")
                .unwrap_or_else(|_| ".deepseek-mobile".to_string()),
        );
        let home = dirs_fallback();
        let paths = vec![
            data_dir.join("skills"),
            home.join(".deepseek").join("skills"),
            home.join(".agents").join("skills"),
            home.join(".claude").join("skills"),
        ];
        Self::discover(&paths)
    }

    /// List enabled skills for injection into the model context.
    pub fn enabled(&self) -> Vec<&SkillManifest> {
        self.skills.iter().filter(|s| s.enabled).collect()
    }

    /// Enable or disable a skill by name.
    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(skill) = self.skills.iter_mut().find(|s| s.name == name) {
            skill.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Load the full SKILL.md body for a skill.
    pub fn load_body(&self, name: &str) -> Result<String> {
        let skill = self
            .skills
            .iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow!("skill not found: {}", name))?;
        let content = fs::read_to_string(&skill.path)?;
        let (_frontmatter, body) = parse_frontmatter(&content)?;
        Ok(body)
    }

    /// Build a compact context injection string listing enabled skills.
    pub fn context_injection(&self) -> Option<String> {
        let enabled = self.enabled();
        if enabled.is_empty() {
            return None;
        }
        let lines: Vec<String> = enabled
            .iter()
            .map(|s| format!("- {}: {}", s.name, s.description))
            .collect();
        Some(format!(
            "## Active Skills\n\nThe following skills are available and active:\n\n{}\n\n",
            lines.join("\n")
        ))
    }
}

// ── Helpers ──

fn parse_frontmatter(content: &str) -> Result<(HashMap<String, String>, String)> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Ok((HashMap::new(), content.to_string()));
    }
    let end_of_first = trimmed[3..]
        .find("---")
        .ok_or_else(|| anyhow!("unclosed frontmatter"))?;
    let fm = trimmed[4..3 + end_of_first].trim();
    let body = trimmed[3 + end_of_first + 3..].trim().to_string();

    let mut map = HashMap::new();
    for line in fm.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    Ok((map, body))
}

fn walk_skill_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();
    if !root.is_dir() {
        return Ok(results);
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let skill_md = path.join("SKILL.md");
            if skill_md.is_file() {
                results.push(skill_md);
            }
        }
    }
    results.sort();
    Ok(results)
}

fn list_companion_files(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "SKILL.md" {
                continue;
            }
            if entry.path().is_file() {
                files.push(name);
            }
        }
    }
    files.sort();
    files
}

fn dirs_fallback() -> PathBuf {
    dirs_next().unwrap_or_else(|| PathBuf::from("."))
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_frontmatter() {
        let input = "---\nname: test-skill\ndescription: A test\n---\n\n# Body\nInstructions.";
        let (fm, body) = parse_frontmatter(input).unwrap();
        assert_eq!(fm.get("name").unwrap(), "test-skill");
        assert_eq!(fm.get("description").unwrap(), "A test");
        assert!(body.contains("# Body"));
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let (fm, body) = parse_frontmatter("# Just a header").unwrap();
        assert!(fm.is_empty());
        assert!(body.contains("# Just a header"));
    }

    #[test]
    fn test_skill_registry_discover() {
        let dir = temp_dir();
        let skill_dir = dir.join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let mut f = fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(
            f,
            "---\nname: my-skill\ndescription: Test\n---\n\nDo things."
        )
        .unwrap();

        let registry = SkillRegistry::discover(&[dir.clone()]).unwrap();
        assert_eq!(registry.skills.len(), 1);
        assert_eq!(registry.skills[0].name, "my-skill");
        assert!(registry.skills[0].enabled);
        clean(&dir);
    }

    #[test]
    fn test_skill_set_enabled() {
        let dir = temp_dir();
        let skill_dir = dir.join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let mut f = fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(f, "---\nname: my-skill\ndescription: T\n---\n\nDo.").unwrap();

        let mut registry = SkillRegistry::discover(&[dir.clone()]).unwrap();
        assert!(registry.set_enabled("my-skill", false));
        assert!(registry.enabled().is_empty());
        assert!(registry.set_enabled("my-skill", true));
        assert_eq!(registry.enabled().len(), 1);
        clean(&dir);
    }

    #[test]
    fn test_skill_load_body() {
        let dir = temp_dir();
        let skill_dir = dir.join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let mut f = fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(f, "---\nname: my-skill\ndescription: T\n---\n\nStep 1.").unwrap();

        let registry = SkillRegistry::discover(&[dir.clone()]).unwrap();
        let body = registry.load_body("my-skill").unwrap();
        assert!(body.contains("Step 1"));
        clean(&dir);
    }

    #[test]
    fn test_context_injection() {
        let dir = temp_dir();
        let skill_dir = dir.join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let mut f = fs::File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(f, "---\nname: my-skill\ndescription: Does X\n---\n\nDo X.").unwrap();

        let registry = SkillRegistry::discover(&[dir.clone()]).unwrap();
        let ctx = registry.context_injection().unwrap();
        assert!(ctx.contains("my-skill"));
        assert!(ctx.contains("Does X"));
        clean(&dir);
    }

    #[test]
    fn test_duplicate_name_first_wins() {
        let dir = temp_dir();
        let d1 = dir.join("skill-a");
        let d2 = dir.join("skill-b");
        fs::create_dir_all(&d1).unwrap();
        fs::create_dir_all(&d2).unwrap();
        let mut f1 = fs::File::create(d1.join("SKILL.md")).unwrap();
        writeln!(f1, "---\nname: dup\ndescription: first\n---\n\nfirst").unwrap();
        let mut f2 = fs::File::create(d2.join("SKILL.md")).unwrap();
        writeln!(f2, "---\nname: dup\ndescription: second\n---\n\nsecond").unwrap();

        let registry = SkillRegistry::discover(&[dir.clone()]).unwrap();
        assert_eq!(registry.skills.len(), 1);
        assert_eq!(registry.skills[0].description, "first");
        clean(&dir);
    }

    fn temp_dir() -> PathBuf {
        let pid = std::process::id();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("deepseek-skills-test-{}-{}", pid, ts))
    }

    fn clean(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }
}

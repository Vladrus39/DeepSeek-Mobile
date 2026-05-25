use deepseek_mobile_core::SkillRegistry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::mobile_runtime_config::default_data_dir;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct SkillsStateFile {
    #[serde(default)]
    enabled: HashMap<String, bool>,
}

#[derive(Clone, Debug)]
pub struct SkillsUiState {
    pub registry: SkillRegistry,
    pub last_error: Option<String>,
    skills_state_path: PathBuf,
}

impl Default for SkillsUiState {
    fn default() -> Self {
        Self {
            registry: SkillRegistry::default(),
            last_error: None,
            skills_state_path: default_data_dir().join("skills-state.json"),
        }
    }
}

impl SkillsUiState {
    /// Discover skills from default paths and reload the UI state.
    pub fn refresh(&mut self) {
        match SkillRegistry::discover_default() {
            Ok(mut reg) => {
                if let Ok(saved) = self.load_enabled_map() {
                    for skill in reg.skills.iter_mut() {
                        if let Some(enabled) = saved.get(&skill.name) {
                            skill.enabled = *enabled;
                        }
                    }
                }
                self.registry = reg;
                self.last_error = None;
            }
            Err(e) => {
                self.last_error = Some(format!("Failed to discover skills: {}", e));
            }
        }
    }

    /// Toggle skill enabled/disabled.
    pub fn toggle_skill(&mut self, name: &str, enabled: bool) {
        if self.registry.set_enabled(name, enabled) {
            let _ = self.save_enabled_map();
        }
    }

    /// Number of enabled skills.
    pub fn enabled_count(&self) -> usize {
        self.registry.enabled().len()
    }

    fn load_enabled_map(&self) -> Result<HashMap<String, bool>, String> {
        if !self.skills_state_path.exists() {
            return Ok(HashMap::new());
        }
        let bytes = fs::read_to_string(&self.skills_state_path).map_err(|e| e.to_string())?;
        let file: SkillsStateFile = serde_json::from_str(&bytes).map_err(|e| e.to_string())?;
        Ok(file.enabled)
    }

    fn save_enabled_map(&self) -> Result<(), String> {
        if let Some(parent) = self.skills_state_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut enabled = HashMap::new();
        for skill in &self.registry.skills {
            enabled.insert(skill.name.clone(), skill.enabled);
        }
        let file = SkillsStateFile { enabled };
        fs::write(
            &self.skills_state_path,
            serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())
    }
}

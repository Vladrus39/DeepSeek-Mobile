use deepseek_mobile_core::SkillRegistry;


#[derive(Clone, Debug)]
pub struct SkillsUiState {
    pub registry: SkillRegistry,
    pub last_error: Option<String>,
}

impl Default for SkillsUiState {
    fn default() -> Self {
        Self {
            registry: SkillRegistry::default(),
            last_error: None,
        }
    }
}

impl SkillsUiState {
    /// Discover skills from default paths and reload the UI state.
    pub fn refresh(&mut self) {
        match SkillRegistry::discover_default() {
            Ok(reg) => {
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
        self.registry.set_enabled(name, enabled);
    }

    /// Number of enabled skills.
    pub fn enabled_count(&self) -> usize {
        self.registry.enabled().len()
    }
}

//! Auto mode router - chooses v4-pro or flash + thinking level

use crate::config::Config;

pub struct ModelRouter {
    config: Config,
}

impl ModelRouter {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Auto routing logic (simplified from original TUI)
    pub fn route(&self, task_complexity: &str) -> (String, String) {
        if self.config.auto_mode {
            match task_complexity {
                "simple" => ("deepseek-v4-flash".to_string(), "off".to_string()),
                "complex" => ("deepseek-v4-pro".to_string(), "high".to_string()),
                _ => ("deepseek-v4-pro".to_string(), "max".to_string()),
            }
        } else {
            (self.config.model.clone(), self.config.thinking_level.clone())
        }
    }
}
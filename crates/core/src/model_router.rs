//! Auto model router.
//!
//! The mobile agent should behave like the TUI auto mode: small and cheap tasks
//! go to Flash, while complex coding, large-context and repair tasks go to Pro.

use crate::config::{Config, ModelMode, ThinkingLevel};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RouteDecision {
    pub model: String,
    pub thinking_level: ThinkingLevel,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskProfile {
    pub prompt: String,
    pub estimated_context_tokens: usize,
    pub has_code_context: bool,
    pub requests_file_changes: bool,
    pub requests_shell_or_git: bool,
    pub error_repair: bool,
}

impl TaskProfile {
    pub fn from_prompt(prompt: impl Into<String>, estimated_context_tokens: usize) -> Self {
        let prompt = prompt.into();
        let lower = prompt.to_lowercase();

        let has_code_context = contains_any(
            &lower,
            &[
                "code",
                "rust",
                "python",
                "javascript",
                "typescript",
                "cargo",
                "npm",
                "ошибка",
                "код",
                "проект",
                "файл",
                "сборка",
                "тест",
            ],
        );
        let requests_file_changes = contains_any(
            &lower,
            &[
                "edit",
                "write",
                "patch",
                "change",
                "fix",
                "update",
                "создай",
                "измени",
                "исправь",
                "добавь",
                "обнови",
                "перепиши",
            ],
        );
        let requests_shell_or_git = contains_any(
            &lower,
            &[
                "shell",
                "terminal",
                "git",
                "commit",
                "push",
                "pull",
                "cargo check",
                "pytest",
                "npm test",
                "терминал",
                "гит",
                "коммит",
            ],
        );
        let error_repair = contains_any(
            &lower,
            &[
                "error",
                "failed",
                "panic",
                "compile",
                "bug",
                "ошибка",
                "не собирается",
                "упало",
                "сломалось",
            ],
        );

        Self {
            prompt,
            estimated_context_tokens,
            has_code_context,
            requests_file_changes,
            requests_shell_or_git,
            error_repair,
        }
    }
}

pub struct ModelRouter {
    config: Config,
}

impl ModelRouter {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn route_prompt(
        &self,
        prompt: impl Into<String>,
        estimated_context_tokens: usize,
    ) -> RouteDecision {
        let profile = TaskProfile::from_prompt(prompt, estimated_context_tokens);
        self.route(&profile)
    }

    pub fn route(&self, profile: &TaskProfile) -> RouteDecision {
        match self.config.model_mode {
            ModelMode::Flash => RouteDecision {
                model: "deepseek-v4-flash".to_string(),
                thinking_level: ThinkingLevel::Off,
                reason: "manual Flash mode".to_string(),
            },
            ModelMode::Pro => RouteDecision {
                model: "deepseek-v4-pro".to_string(),
                thinking_level: self.config.thinking_level.clone(),
                reason: "manual Pro mode".to_string(),
            },
            ModelMode::Auto => self.route_auto(profile),
        }
    }

    fn route_auto(&self, profile: &TaskProfile) -> RouteDecision {
        let requires_pro = profile.estimated_context_tokens > 64_000
            || profile.requests_file_changes
            || profile.requests_shell_or_git
            || profile.error_repair
            || (profile.has_code_context && profile.estimated_context_tokens > 16_000);

        if requires_pro {
            RouteDecision {
                model: "deepseek-v4-pro".to_string(),
                thinking_level: self.auto_pro_thinking_level(),
                reason: "auto selected Pro for complex coding or large-context task".to_string(),
            }
        } else {
            RouteDecision {
                model: "deepseek-v4-flash".to_string(),
                thinking_level: ThinkingLevel::Off,
                reason: "auto selected Flash for lightweight task".to_string(),
            }
        }
    }

    fn auto_pro_thinking_level(&self) -> ThinkingLevel {
        match self.config.thinking_level {
            ThinkingLevel::Off => ThinkingLevel::High,
            _ => self.config.thinking_level.clone(),
        }
    }
}

fn contains_any(input: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| input.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::{ModelRouter, TaskProfile};
    use crate::config::{Config, ModelMode, ThinkingLevel};

    #[test]
    fn auto_routes_simple_prompt_to_flash() {
        let router = ModelRouter::new(Config::default());
        let decision = router.route_prompt("explain what this button does", 512);

        assert_eq!(decision.model, "deepseek-v4-flash");
        assert_eq!(decision.thinking_level, ThinkingLevel::Off);
    }

    #[test]
    fn auto_routes_file_changes_to_pro() {
        let router = ModelRouter::new(Config::default());
        let profile = TaskProfile::from_prompt("исправь ошибку в rust проекте", 8_000);
        let decision = router.route(&profile);

        assert_eq!(decision.model, "deepseek-v4-pro");
        assert_eq!(decision.thinking_level, ThinkingLevel::High);
    }

    #[test]
    fn auto_routes_complex_prompt_to_pro_with_selected_thinking() {
        let config = Config {
            thinking_level: ThinkingLevel::Low,
            ..Config::default()
        };
        let router = ModelRouter::new(config);
        let decision = router.route_prompt("исправь ошибку в rust проекте", 8_000);

        assert_eq!(decision.model, "deepseek-v4-pro");
        assert_eq!(decision.thinking_level, ThinkingLevel::Low);
    }

    #[test]
    fn manual_flash_overrides_auto() {
        let config = Config::default().with_model_mode(ModelMode::Flash);
        let router = ModelRouter::new(config);
        let decision = router.route_prompt("fix huge broken project", 200_000);

        assert_eq!(decision.model, "deepseek-v4-flash");
    }
}

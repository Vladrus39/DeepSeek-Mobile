//! Configuration

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    pub auto_mode: bool,
    pub model_mode: ModelMode,
    pub thinking_level: ThinkingLevel,
    pub external_access: ExternalAccessMode,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelMode {
    Auto,
    Flash,
    Pro,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThinkingLevel {
    Off,
    Low,
    Medium,
    High,
    Max,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExternalAccessMode {
    WorkspaceOnly,
    AskEveryTime,
    AllowedByUserGrant,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            model: "deepseek-v4-flash".to_string(),
            auto_mode: true,
            model_mode: ModelMode::Auto,
            thinking_level: ThinkingLevel::High,
            external_access: ExternalAccessMode::WorkspaceOnly,
        }
    }
}

impl Config {
    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_key = key;
        self
    }

    pub fn with_model_mode(mut self, mode: ModelMode) -> Self {
        self.auto_mode = mode == ModelMode::Auto;
        self.model_mode = mode;
        self
    }

    pub fn with_external_access(mut self, mode: ExternalAccessMode) -> Self {
        self.external_access = mode;
        self
    }
}

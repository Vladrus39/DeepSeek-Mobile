use deepseek_mobile_core::config::{
    Config, ExecutionMode, ExternalAccessMode, ModelMode, ThinkingLevel,
};
use deepseek_mobile_core::ConfigStore;
use std::path::PathBuf;

use crate::mobile_runtime_config::default_data_dir;

/// In-memory settings form state that mirrors `Config`.
#[derive(Clone, Debug)]
pub struct SettingsFormState {
    pub api_key: String,
    pub model_mode: ModelMode,
    pub execution_mode: ExecutionMode,
    pub thinking_level: ThinkingLevel,
    pub external_access: ExternalAccessMode,
    pub trusted_external_paths: String,
    pub github_token: String,
    pub github_repo: String,
    pub github_branch: String,
    pub auto_commit_push: bool,
    pub saved: bool,
    pub save_error: Option<String>,
}

impl Default for SettingsFormState {
    fn default() -> Self {
        Self::from_config(&load_saved_config().unwrap_or_default())
    }
}

impl SettingsFormState {
    pub fn from_config(config: &Config) -> Self {
        Self {
            api_key: config.api_key.clone(),
            model_mode: config.model_mode.clone(),
            execution_mode: config.execution_mode.clone(),
            thinking_level: config.thinking_level.clone(),
            external_access: config.external_access.clone(),
            trusted_external_paths: config.trusted_external_paths.join("\n"),
            github_token: config.github_token.clone().unwrap_or_default(),
            github_repo: config.github_repo.clone().unwrap_or_default(),
            github_branch: config.github_branch.clone().unwrap_or_default(),
            auto_commit_push: config.auto_commit_push,
            saved: false,
            save_error: None,
        }
    }

    pub fn to_config(&self) -> Config {
        Config {
            api_key: self.api_key.clone(),
            model: match self.model_mode {
                ModelMode::Pro => "deepseek-v4-pro".to_string(),
                ModelMode::Auto | ModelMode::Flash => "deepseek-v4-flash".to_string(),
            },
            auto_mode: self.model_mode == ModelMode::Auto,
            model_mode: self.model_mode.clone(),
            execution_mode: self.execution_mode.clone(),
            thinking_level: self.thinking_level.clone(),
            external_access: self.external_access.clone(),
            trusted_external_paths: self
                .trusted_external_paths
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect(),
            github_token: if self.github_token.is_empty() {
                None
            } else {
                Some(self.github_token.clone())
            },
            github_repo: if self.github_repo.is_empty() {
                None
            } else {
                Some(self.github_repo.clone())
            },
            github_branch: if self.github_branch.is_empty() {
                None
            } else {
                Some(self.github_branch.clone())
            },
            auto_commit_push: self.auto_commit_push,
        }
    }
}

pub fn config_store() -> ConfigStore {
    ConfigStore::new(default_data_dir())
}

pub fn config_file_path() -> PathBuf {
    config_store().config_path()
}

pub fn load_saved_config() -> Option<Config> {
    let store = config_store();
    if store.config_path().exists() || store.secrets_path().exists() {
        store.load().ok()
    } else {
        None
    }
}

pub fn save_config(config: &Config) -> Result<(), String> {
    config_store()
        .save(config)
        .map_err(|error| error.to_string())
}

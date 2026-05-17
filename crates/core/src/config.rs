//! Configuration

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionMode {
    Plan,
    Agent,
    Yolo,
}

impl Default for ExecutionMode {
    fn default() -> Self {
        Self::Agent
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    pub auto_mode: bool,
    pub model_mode: ModelMode,
    pub execution_mode: ExecutionMode,
    pub thinking_level: ThinkingLevel,
    pub external_access: ExternalAccessMode,
    pub github_token: Option<String>,
    pub github_repo: Option<String>,
    pub github_branch: Option<String>,
    pub auto_commit_push: bool,
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
            execution_mode: ExecutionMode::default(),
            thinking_level: ThinkingLevel::High,
            external_access: ExternalAccessMode::WorkspaceOnly,
            github_token: None,
            github_repo: None,
            github_branch: None,
            auto_commit_push: false,
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

    pub fn with_github_token(mut self, token: String) -> Self {
        self.github_token = Some(token);
        self
    }

    pub fn with_github_repo(mut self, repo: String) -> Self {
        self.github_repo = Some(repo);
        self
    }

    pub fn with_github_branch(mut self, branch: String) -> Self {
        self.github_branch = Some(branch);
        self
    }

    pub fn with_execution_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    pub fn with_auto_commit_push(mut self, enabled: bool) -> Self {
        self.auto_commit_push = enabled;
        self
    }
}

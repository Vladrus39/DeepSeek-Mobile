//! Configuration

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    pub auto_mode: bool,
    pub thinking_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            model: "deepseek-v4-flash".to_string(),
            auto_mode: true,
            thinking_level: "high".to_string(),
        }
    }
}

impl Config {
    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_key = key;
        self
    }
}
//! Agent module - core logic from DeepSeek-TUI

use crate::config::Config;
use anyhow::Result;

pub struct DeepSeekAgent {
    config: Config,
}

impl DeepSeekAgent {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn run(&self, input: String) -> Result<String> {
        // TODO: Full implementation with tool calling, streaming, auto mode
        // This will be expanded with real logic from original TUI
        println!("[Agent] Processing: {}", input);
        Ok(format!("Agent response to: {}", input))
    }
}
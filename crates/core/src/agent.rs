//! DeepSeek Agent with tool calling (expanded from original TUI)

use crate::config::Config;
use crate::tools::ToolRegistry;
use anyhow::Result;
use std::sync::Arc;

pub struct DeepSeekAgent {
    config: Config,
    tools: Arc<ToolRegistry>,
}

impl DeepSeekAgent {
    pub fn new(config: Config) -> Self {
        let mut tools = ToolRegistry::new();
        tools.register(Box::new(crate::tools::file_ops::FileOpsTool));
        // TODO: register more tools (shell, git, web, etc.)
        
        Self {
            config,
            tools: Arc::new(tools),
        }
    }

    pub async fn run(&self, input: String) -> Result<String> {
        println!("[Agent] Received: {}", input);
        
        // TODO: Real tool calling loop + reasoning
        // For now: simple processing
        if input.contains("file") || input.contains("файл") {
            if let Some(tool) = self.tools.get("file_ops") {
                return tool.execute(&input);
            }
        }
        
        Ok(format!("[Agent] Processed: {}. Tools available: {}", 
            input, 
            self.tools.tools.len()))
    }
}
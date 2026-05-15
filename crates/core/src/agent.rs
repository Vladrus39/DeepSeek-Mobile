//! DeepSeek Agent with tools

use crate::config::Config;
use crate::api_client::{DeepSeekClient, Message};
use crate::tools::ToolRegistry;
use anyhow::Result;
use std::sync::Arc;

pub struct DeepSeekAgent {
    config: Config,
    tools: Arc<ToolRegistry>,
    client: DeepSeekClient,
}

impl DeepSeekAgent {
    pub fn new(config: Config) -> Self {
        let mut tools = ToolRegistry::new();
        tools.register(Box::new(crate::tools::file_ops::FileOpsTool));
        tools.register(Box::new(crate::tools::shell::ShellTool));
        tools.register(Box::new(crate::tools::git::GitTool));
        
        let client = DeepSeekClient::new(config.api_key.clone());
        
        Self {
            config: config.clone(),
            tools: Arc::new(tools),
            client,
        }
    }

    pub async fn run(&self, input: String) -> Result<String> {
        let messages = vec![
            Message { role: "system".to_string(), content: "You are a helpful coding assistant. Use tools when needed.".to_string() },
            Message { role: "user".to_string(), content: input },
        ];
        
        match self.client.chat(&self.config.model, messages).await {
            Ok(response) => Ok(response),
            Err(e) => Ok(format!("[Agent] Error: {}. Fallback response for: {}", e, input)),
        }
    }
}
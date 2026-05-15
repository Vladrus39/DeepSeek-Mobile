//! DeepSeek Agent - now with real API integration

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
        
        let client = DeepSeekClient::new(config.api_key.clone());
        
        Self {
            config: config.clone(),
            tools: Arc::new(tools),
            client,
        }
    }

    pub async fn run(&self, input: String) -> Result<String> {
        println!("[Agent] Processing input...");
        
        // Build messages for API
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are a helpful coding assistant running on mobile. Be concise and practical.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: input,
            },
        ];
        
        // Call real DeepSeek API
        match self.client.chat(&self.config.model, messages).await {
            Ok(response) => {
                println!("[Agent] Got response from API");
                Ok(response)
            }
            Err(e) => {
                println!("[Agent] API error: {}", e);
                // Fallback to simple response if API fails
                Ok(format!("[Fallback] Could not reach DeepSeek API. Your input: {}", input))
            }
        }
    }
}
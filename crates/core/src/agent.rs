//! DeepSeek Agent with streaming support

use crate::config::Config;
use crate::api_client::{DeepSeekClient, Message};
use anyhow::Result;
use tokio::sync::mpsc;

pub struct DeepSeekAgent {
    config: Config,
    client: DeepSeekClient,
}

impl DeepSeekAgent {
    pub fn new(config: Config) -> Self {
        let client = DeepSeekClient::new(config.api_key.clone());

        Self {
            config: config.clone(),
            client,
        }
    }

    /// Non-streaming
    pub async fn run(&self, input: String) -> Result<String> {
        let messages = vec![
            Message { role: "system".to_string(), content: "You are a helpful coding assistant.".to_string() },
            Message { role: "user".to_string(), content: input },
        ];
        self.client.chat(&self.config.model, messages).await
    }

    /// Streaming version - returns receiver with text deltas
    pub async fn run_stream(&self, input: String) -> Result<mpsc::Receiver<String>> {
        let messages = vec![
            Message { role: "system".to_string(), content: "You are a helpful coding assistant.".to_string() },
            Message { role: "user".to_string(), content: input },
        ];
        self.client.chat_stream(&self.config.model, messages).await
    }

    pub async fn run_with_messages(&self, messages: Vec<Message>) -> Result<String> {
        self.client.chat(&self.config.model, messages).await
    }
}

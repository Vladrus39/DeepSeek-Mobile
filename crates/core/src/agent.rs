//! DeepSeek Agent with streaming support and full message history.
//!
//! The agent wraps the DeepSeek API client and provides both non-streaming
//! and streaming execution. Streaming returns structured `StreamDelta` items
//! that distinguish between V4 reasoning tokens and final text.

use crate::api_client::{DeepSeekClient, Message, StreamDelta};
use crate::config::Config;
use anyhow::Result;
use tokio::sync::mpsc;

pub struct DeepSeekAgent {
    config: Config,
    client: DeepSeekClient,
}

impl DeepSeekAgent {
    pub fn new(config: Config) -> Self {
        let client = DeepSeekClient::new(config.api_key.clone());
        Self { config, client }
    }

    /// Non-streaming chat with system prompt + single user message.
    pub async fn run(&self, input: String) -> Result<String> {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are a helpful coding assistant.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: input,
            },
        ];
        self.client.chat(&self.config.model, messages).await
    }

    /// Non-streaming chat with full message history (system + conversation).
    pub async fn run_with_messages(&self, messages: Vec<Message>) -> Result<String> {
        self.client.chat(&self.config.model, messages).await
    }

    /// Streaming chat with system prompt + single user message.
    /// Returns a receiver of structured `StreamDelta` items.
    pub async fn run_stream(&self, input: String) -> Result<mpsc::Receiver<StreamDelta>> {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are a helpful coding assistant.".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: input,
            },
        ];
        self.client.chat_stream(&self.config.model, messages).await
    }

    /// Streaming chat with full message history.
    /// Use this when the engine has accumulated conversation context.
    pub async fn run_stream_with_messages(
        &self,
        messages: Vec<Message>,
    ) -> Result<mpsc::Receiver<StreamDelta>> {
        self.client.chat_stream(&self.config.model, messages).await
    }

    /// Streaming chat with an explicit model override.
    /// Used when the ModelRouter selects a model different from the default config.
    pub async fn run_stream_with_messages_and_model(
        &self,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<mpsc::Receiver<StreamDelta>> {
        self.client.chat_stream(model, messages).await
    }

    /// Non-streaming chat with an explicit model override.
    pub async fn run_with_messages_and_model(
        &self,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<String> {
        self.client.chat(model, messages).await
    }
}

//! DeepSeek API Client with streaming support (OpenAI-compatible)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use futures_util::StreamExt;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    delta: Option<Delta>,
    message: Option<Message>,
}

#[derive(Deserialize, Debug)]
struct Delta {
    content: Option<String>,
}

pub struct DeepSeekClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl DeepSeekClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.deepseek.com/v1".to_string(),
        }
    }

    /// Non-streaming chat
    pub async fn chat(&self, model: &str, messages: Vec<Message>) -> Result<String> {
        let req = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
            temperature: Some(0.7),
        };

        let resp = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await?;

        let chat_resp: ChatResponse = resp.json().await?;
        
        if let Some(choice) = chat_resp.choices.first() {
            if let Some(msg) = &choice.message {
                return Ok(msg.content.clone());
            }
        }
        
        Ok("No response".to_string())
    }

    /// Streaming chat (for real-time reasoning blocks)
    pub async fn chat_stream(&self, model: &str, messages: Vec<Message>) -> Result<()> {
        println!("[API] Starting stream for model: {}", model);
        // TODO: Implement real SSE streaming
        // For now this is prepared for future streaming
        Ok(())
    }
}
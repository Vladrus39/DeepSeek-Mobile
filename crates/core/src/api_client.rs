//! DeepSeek API Client (OpenAI-compatible)

use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
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

    pub async fn chat(&self, model: &str, messages: Vec<Message>) -> Result<String> {
        let req = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
        };
        
        // Placeholder - real implementation will use streaming
        println!("[API] Calling DeepSeek model: {}", model);
        
        // TODO: Real HTTP call + streaming support
        Ok("[API Response] Placeholder - will be replaced with real streaming".to_string())
    }
}
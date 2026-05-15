//! DeepSeek API Client - Real implementation ready

use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
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
    message: Option<Message>,
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
        if self.api_key.is_empty() {
            return Err(anyhow!("DEEPSEEK_API_KEY is not set"));
        }

        let req = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
        };

        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("DeepSeek API error {}: {}", status, text));
        }

        let chat_resp: ChatResponse = response.json().await?;
        
        chat_resp.choices
            .first()
            .and_then(|c| c.message.as_ref())
            .map(|m| m.content.clone())
            .ok_or_else(|| anyhow!("No response from model"))
    }
}
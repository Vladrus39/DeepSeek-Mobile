//! DeepSeek API Client with real streaming support (OpenAI-compatible SSE)
//!
//! Supports DeepSeek V4 reasoning tokens via `reasoning_content` in deltas.

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
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
    delta: Option<Delta>,
}

#[derive(Deserialize, Debug)]
struct Delta {
    content: Option<String>,
    /// DeepSeek V4 reasoning/thinking tokens
    #[serde(rename = "reasoning_content")]
    reasoning_content: Option<String>,
}

/// A structured streaming delta from the DeepSeek API.
/// Distinguishes between final visible text and V4 thinking tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamDelta {
    /// Final visible text content
    Text(String),
    /// V4 reasoning/thinking tokens (may arrive before or interleaved with text)
    Reasoning(String),
    /// Stream completed
    Done,
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

    /// Non-streaming chat (simple)
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

    /// Streaming chat — returns a receiver for structured deltas.
    /// Emits `Text`, `Reasoning`, and `Done` variants.
    /// V4 models send reasoning tokens before or interleaved with final text.
    pub async fn chat_stream(
        &self,
        model: &str,
        messages: Vec<Message>,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamDelta>> {
        if self.api_key.is_empty() {
            return Err(anyhow!("DEEPSEEK_API_KEY is not set"));
        }

        let req = ChatRequest {
            model: model.to_string(),
            messages,
            stream: true,
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

        let (tx, rx) = mpsc::channel::<StreamDelta>(256);

        // Background task to process SSE stream
        tokio::spawn(async move {
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk) = stream.next().await {
                if let Ok(bytes) = chunk {
                    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
                        buffer.push_str(&text);

                        while let Some(pos) = buffer.find("\n\n") {
                            let event_block = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();

                            for line in event_block.lines() {
                                let line = line.trim();
                                if let Some(data) = line.strip_prefix("data: ") {
                                    let data = data.trim();
                                    if data == "[DONE]" {
                                        let _ = tx.send(StreamDelta::Done).await;
                                        return;
                                    }

                                    // Parse delta — supports both content and reasoning_content
                                    if let Ok(parsed) = serde_json::from_str::<ChatResponse>(data) {
                                        if let Some(choice) = parsed.choices.first() {
                                            if let Some(delta) = &choice.delta {
                                                // Emit reasoning first (V4 thinking tokens)
                                                if let Some(reasoning) = &delta.reasoning_content {
                                                    if !reasoning.is_empty() {
                                                        let _ = tx.send(StreamDelta::Reasoning(reasoning.clone())).await;
                                                    }
                                                }
                                                // Emit final text content
                                                if let Some(content) = &delta.content {
                                                    if !content.is_empty() {
                                                        let _ = tx.send(StreamDelta::Text(content.clone())).await;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Stream ended without [DONE] — send Done anyway
            let _ = tx.send(StreamDelta::Done).await;
        });

        Ok(rx)
    }
}

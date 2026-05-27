//! DeepSeek API Client with real streaming support (OpenAI-compatible SSE)
//!
//! Supports DeepSeek V4 reasoning tokens via `reasoning_content` in deltas.

use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;

fn build_http_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .connect_timeout(Duration::from_secs(45))
        .user_agent("DeepSeek-Mobile/0.1")
        .build()
        .context("build HTTP client")
}

/// User-facing message for failed API transport (Android/desktop).
pub fn format_http_transport_error(error: &anyhow::Error) -> String {
    let chain = error
        .chain()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(" | ");
    let lower = chain.to_lowercase();
    if lower.contains("deepseek_api_key") || lower.contains("api_key is not set") {
        return "Не задан API-ключ DeepSeek. Откройте ☰ → Настройки и сохраните ключ sk-…"
            .to_string();
    }
    if lower.contains("dns") || lower.contains("lookup") {
        return format!("Нет DNS/интернета для api.deepseek.com. Проверьте Wi‑Fi или мобильные данные. ({chain})");
    }
    if lower.contains("certificate") || lower.contains("tls") || lower.contains("ssl") {
        return format!(
            "Ошибка TLS при подключении к DeepSeek API. Проверьте дату на телефоне, VPN/firewall/proxy и разрешение сети для приложения. ({chain})"
        );
    }
    if lower.contains("timed out") || lower.contains("timeout") {
        return format!("Таймаут запроса к DeepSeek API. Повторите или проверьте сеть. ({chain})");
    }
    format!("Сеть DeepSeek API: {chain}")
}

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
        let client = build_http_client().unwrap_or_else(|_| Client::new());
        Self {
            client,
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

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await
            .with_context(|| format!("POST {}/chat/completions", self.base_url))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow!("DeepSeek API error {}: {}", status, text));
        }

        let chat_resp: ChatResponse = response.json().await.context("parse DeepSeek API JSON")?;

        chat_resp
            .choices
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

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await
            .with_context(|| format!("POST {}/chat/completions (stream)", self.base_url))?;

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

                        while let Some((pos, sep_len)) = find_sse_event_separator(&buffer) {
                            let event_block = buffer[..pos].to_string();
                            buffer = buffer[pos + sep_len..].to_string();

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
                                                        let _ = tx
                                                            .send(StreamDelta::Reasoning(
                                                                reasoning.clone(),
                                                            ))
                                                            .await;
                                                    }
                                                }
                                                // Emit final text content
                                                if let Some(content) = &delta.content {
                                                    if !content.is_empty() {
                                                        let _ = tx
                                                            .send(StreamDelta::Text(
                                                                content.clone(),
                                                            ))
                                                            .await;
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

fn find_sse_event_separator(buffer: &str) -> Option<(usize, usize)> {
    ["\r\n\r\n", "\n\n", "\r\r"]
        .iter()
        .filter_map(|separator| buffer.find(separator).map(|pos| (pos, separator.len())))
        .min_by_key(|(pos, _)| *pos)
}

#[cfg(test)]
mod tests {
    use super::find_sse_event_separator;

    #[test]
    fn sse_separator_detects_lf_blocks() {
        assert_eq!(find_sse_event_separator("data: {}\n\nnext"), Some((8, 2)));
    }

    #[test]
    fn sse_separator_detects_crlf_blocks() {
        assert_eq!(
            find_sse_event_separator("data: {}\r\n\r\nnext"),
            Some((8, 4))
        );
    }
}

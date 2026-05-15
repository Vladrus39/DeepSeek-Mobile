//! DeepSeek Mobile Core

pub mod agent;
pub mod tools;
pub mod config;
pub mod session;
pub mod model_router;
pub mod api_client;

pub use agent::DeepSeekAgent;
pub use config::Config;
pub use api_client::{DeepSeekClient, Message};

pub struct DeepSeekCore {
    agent: DeepSeekAgent,
    client: DeepSeekClient,
}

impl DeepSeekCore {
    pub fn new(config: Config) -> Self {
        let agent = DeepSeekAgent::new(config.clone());
        let client = DeepSeekClient::new(config.api_key.clone());
        Self { agent, client }
    }

    pub async fn process(&self, input: String) -> anyhow::Result<String> {
        self.agent.run(input).await
    }

    /// New method for chat with message history
    pub async fn process_with_messages(&self, messages: Vec<Message>) -> anyhow::Result<String> {
        // For now use the last user message
        if let Some(last) = messages.last() {
            if last.role == "user" {
                return self.agent.run(last.content.clone()).await;
            }
        }
        self.agent.run("Hello".to_string()).await
    }
}
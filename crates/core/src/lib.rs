//! DeepSeek Mobile Core
//! Full port + adaptation from DeepSeek-TUI

pub mod agent;
pub mod tools;
pub mod config;
pub mod session;
pub mod model_router;
pub mod api_client;

pub use agent::DeepSeekAgent;
pub use config::Config;
pub use api_client::DeepSeekClient;

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
}
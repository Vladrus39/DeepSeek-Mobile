//! DeepSeek Mobile Core
//! Полный перенос логики из DeepSeek-TUI

pub mod agent;
pub mod tools;
pub mod config;
pub mod session;
pub mod model_router;

pub use agent::DeepSeekAgent;
pub use config::Config;

/// Главная точка входа в core
pub struct DeepSeekCore {
    agent: DeepSeekAgent,
}

impl DeepSeekCore {
    pub fn new(config: Config) -> Self {
        let agent = DeepSeekAgent::new(config);
        Self { agent }
    }

    pub async fn process(&self, input: String) -> anyhow::Result<String> {
        self.agent.run(input).await
    }
}
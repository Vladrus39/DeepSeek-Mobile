pub mod agent;
pub mod config;
pub mod tools;
pub mod session;
pub mod model_router;

pub use agent::DeepSeekAgent;
pub use config::Config;

#[derive(Clone)]
pub struct Core {
    agent: DeepSeekAgent,
}

impl Core {
    pub fn new(config: Config) -> Self {
        Self {
            agent: DeepSeekAgent::new(config),
        }
    }

    pub async fn send_message(&self, message: String) -> anyhow::Result<String> {
        self.agent.process(message).await
    }
}
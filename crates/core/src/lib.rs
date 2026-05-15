//! DeepSeek Mobile Core

pub mod agent;
pub mod api_client;
pub mod config;
pub mod events;
pub mod executor;
pub mod model_router;
pub mod session;
pub mod tools;
pub mod workspace;

pub use agent::DeepSeekAgent;
pub use api_client::{DeepSeekClient, Message};
pub use config::Config;
pub use events::{AgentEvent, ApprovalRequest, PatchProposal, RiskLevel, ToolCallEvent, ToolResultEvent};
pub use executor::{CommandOutput, CommandRequest, DisabledExecutor, Executor};
pub use session::Session;
pub use workspace::{ExecutorKind, Workspace};

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

    pub async fn process_with_messages(&self, messages: Vec<Message>) -> anyhow::Result<String> {
        self.agent.run_with_messages(messages).await
    }
}
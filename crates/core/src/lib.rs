//! DeepSeek Mobile Core

pub mod agent;
pub mod api_client;
pub mod approval;
pub mod config;
pub mod context;
pub mod engine;
pub mod events;
pub mod executor;
pub mod model_router;
pub mod runtime_store;
pub mod session;
pub mod tools;
pub mod turn;
pub mod workspace;
pub mod workspace_files;

pub use agent::DeepSeekAgent;
pub use api_client::{DeepSeekClient, Message};
pub use approval::{
    categorize_tool, classify_risk, should_request_approval, ApprovalMode, ApprovalRisk,
    MobileApprovalRequest, ReviewDecision, ToolCategory,
};
pub use config::{Config, ExternalAccessMode, ModelMode, ThinkingLevel};
pub use context::{
    estimate_messages_tokens, estimate_text_tokens, CompressionStrategy, ContextBudget,
    ContextCompressionPlan, ContextManager,
};
pub use engine::{EngineTurnResult, MobileEngine};
pub use events::{AgentEvent, ApprovalRequest, PatchProposal, RiskLevel, ToolCallEvent, ToolResultEvent};
pub use executor::{CommandOutput, CommandRequest, DisabledExecutor, Executor};
pub use model_router::{ModelRouter, RouteDecision, TaskProfile};
pub use runtime_store::{
    RuntimeEventRecord, RuntimeThreadStore, RuntimeTurnStatus, ThreadRecord, TurnItemKind,
    TurnItemLifecycleStatus, TurnItemRecord, TurnRecord,
};
pub use session::Session;
pub use tools::{ApprovalRequirement, ToolCapability, ToolContext, ToolRegistry, ToolResult, ToolSpec};
pub use turn::{TokenUsage, TurnContext, TurnStatus, TurnToolCall};
pub use workspace::{ExecutorKind, Workspace};
pub use workspace_files::{WorkspaceFileEntry, WorkspaceFileService};

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
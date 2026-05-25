//! DeepSeek Mobile Core

pub mod agent;
pub mod api_client;
pub mod approval;
pub mod approval_card;
pub mod approval_session;
pub mod auto_commit;
pub mod approval_session_runtime;
pub mod chat_input;
pub mod config;
pub mod context;
pub mod engine;
pub mod events;
pub mod executor;
pub mod github;
pub mod model_router;
pub mod pc_gateway;
pub mod pc_gateway_client;
pub mod pc_gateway_discovery;
pub mod pc_logs;
pub mod pc_pairing;
pub mod runtime_store;
pub mod session;
pub mod snapshots;
pub mod tool_call;
pub mod tool_execution;
pub mod tool_loop;
pub mod tools;
pub mod turn;
pub mod workspace;
pub mod workspace_connection;
pub mod workspace_connection_store;
pub mod workspace_diagnostics;
pub mod workspace_files;

pub use agent::DeepSeekAgent;
pub use api_client::{DeepSeekClient, Message, StreamDelta};
pub use approval::{
    categorize_tool, classify_risk, should_request_approval, ApprovalMode, ApprovalRisk,
    MobileApprovalRequest, ReviewDecision, ToolCategory,
};
pub use approval_card::{
    approval_cards_from_records, sanitize_value_for_preview, ApprovalCardAction, ApprovalCardSeverity,
    ApprovalCardStatus, ApprovalCardView,
};
pub use approval_session::{
    can_grant_for_session, ApprovalSessionGrant, ApprovalSessionPolicy, ApprovalSessionScope,
};
pub use approval_session_runtime::{ApprovalSessionRuntimeRecord, ApprovalSessionRuntimeStore};
pub use chat_input::{UserAttachmentKind, UserAttachmentRef, UserChatInput};
pub use config::{Config, ExecutionMode, ExternalAccessMode, ModelMode, ThinkingLevel};
pub use context::{
    estimate_messages_tokens, estimate_text_tokens, CompressionStrategy, ContextBudget,
    ContextCompressionPlan, ContextManager,
};
pub use engine::{
    EngineApprovalContinuationResult, EnginePendingApprovalSnapshot, EngineTurnResult, MobileEngine,
};
pub use events::{AgentEvent, ApprovalRequest, PatchProposal, RiskLevel, ToolCallEvent, ToolResultEvent};
pub use executor::{
    CommandOutput, CommandRequest, DisabledExecutor, Executor, PcGatewayExecutorPlan,
    PcGatewayPlannedExecutor, TermuxExecRequest, TermuxExecResult,
};
pub use github::{GitHubBranch, GitHubClient, GitHubCommitResult, GitHubContentEntry, GitHubFileContent,
    GitHubIssue, GitHubPullRequest, GitHubRepo, GitHubRepoInfo};
pub use model_router::{ModelRouter, RouteDecision, TaskProfile};
pub use pc_gateway::{
    is_private_or_loopback_http_url, validate_gateway_base_url,
    validate_gateway_base_url_for_transport, PcDiagnostic, PcDiagnosticSeverity,
    PcEnvironmentDescriptor, PcEnvironmentKind, PcGatewayCapability, PcGatewayConfig,
    PcGatewayConnectionStatus, PcGatewayDirEntry, PcGatewayEndpointCandidate, PcGatewayError,
    PcGatewayHealth, PcGatewayPairingRequest, PcGatewayPairingResponse, PcGatewayRequest,
    PcGatewayRequestEnvelope, PcGatewayResponse, PcGatewayResponseEnvelope,
    PcGatewaySecurityPolicy, PcGatewayTransportMode, PcGatewayTrustLevel, PolicyPreset, PcPreviewDescriptor,
    CommandStreamEvent, PcTaskDescriptor, PcTaskKind, PcTerminalSession, PcWorkspaceGrant,
    PcRunningTaskInfo,
    PcWorkspaceIndexSummary,
};
pub use pc_gateway_client::{PcGatewayClient, PcGatewayEndpointHealth};
pub use pc_gateway_discovery::{
    PcGatewayDiscoveryCandidate, PcGatewayDiscoveryReport, PcGatewayDiscoveryService,
    PcGatewayDiscoverySource, PcGatewayDiscoveryStatus, PcGatewayMdnsRecord,
    DEFAULT_PC_GATEWAY_PORT, PC_GATEWAY_MDNS_SERVICE,
};
pub use pc_logs::{LogRing, PcGatewayLogEntry, PcGatewayLogs};
pub use pc_pairing::{PcGatewayPairingBundle, PcPairingLaunchScript, PcPairingPlatform};
pub use runtime_store::{
    ApprovalDecisionRecord, PendingApprovalRecord, RuntimeEventRecord, RuntimeThreadStore,
    RuntimeTurnStatus, ThreadRecord, TurnItemKind, TurnItemLifecycleStatus, TurnItemRecord,
    TurnRecord,
};
pub use session::Session;
pub use snapshots::{
    WorkspaceRestoreReport, WorkspaceSnapshotFile, WorkspaceSnapshotRecord, WorkspaceSnapshotService,
};
pub use tool_call::{parse_tool_calls_from_text, ToolCallParseResult, ToolCallRequest, ToolCallSource};
pub use tool_execution::{ToolExecutionCoordinator, ToolExecutionRoute, ToolExecutionTarget};
pub use tool_loop::{
    continue_pending_tool_approval, continue_pending_tool_approval_with_session, execute_approved_call,
    process_model_text_with_tools, process_model_text_with_tools_and_session, PendingToolCallApproval,
    ToolLoopExecutionRecord, ToolLoopOutcome,
};
pub use tools::{ApprovalRequirement, ToolCapability, ToolContext, ToolRegistry, ToolResult, ToolSpec};
pub use turn::{TokenUsage, TurnContext, TurnStatus, TurnToolCall};
pub use workspace::{ExecutorKind, Workspace};
pub use workspace_connection::{
    WorkspaceBackendKind, WorkspaceConnection, WorkspaceConnectionManager, WorkspaceConnectionStatus,
    WorkspaceSelectionPolicy,
};
pub use workspace_connection_store::{WorkspaceConnectionStore, WorkspaceConnectionStoreFile};
pub use workspace_diagnostics::{
    WorkspaceDiagnosticsReport, WorkspaceDiagnosticsService, WorkspaceDiagnosticsStatus,
};
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

    pub async fn process_chat_input(&self, input: UserChatInput) -> anyhow::Result<String> {
        self.agent.run(input.to_prompt_text()).await
    }
}

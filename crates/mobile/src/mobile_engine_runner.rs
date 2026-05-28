use crate::mobile_runtime_config::{default_data_dir, MobileRuntimeConfig};
use deepseek_mobile_core::{
    AgentEvent, ApprovalCardView, ApprovalSessionRuntimeStore, Config, MobileEngine,
    ReviewDecision, RuntimeThreadStore, Session, SkillRegistry, TermuxExecResult, TokenUsage,
    TurnContext, TurnStatus, UserChatInput,
};
use std::collections::HashMap;
use std::fs;

#[derive(Clone, Debug, PartialEq)]
pub struct MobileTurnUiResult {
    pub events: Vec<AgentEvent>,
    pub final_text: Option<String>,
    pub approval_cards: Vec<ApprovalCardView>,
    pub approval_card_count: usize,
    pub runtime_store_root: String,
    pub workspace_root: String,
    pub thread_id: String,
}

impl MobileTurnUiResult {
    pub fn has_pending_approvals(&self) -> bool {
        self.approval_card_count > 0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MobileApprovalContinuationUiResult {
    pub events: Vec<AgentEvent>,
    pub final_text: Option<String>,
    pub executed_count: usize,
    pub session_grant_count: usize,
    pub remaining_approval_cards: Vec<ApprovalCardView>,
}

impl MobileApprovalContinuationUiResult {
    pub fn has_remaining_approvals(&self) -> bool {
        !self.remaining_approval_cards.is_empty()
    }
}

pub async fn run_mobile_turn(
    config: Config,
    input: UserChatInput,
) -> anyhow::Result<MobileTurnUiResult> {
    run_mobile_turn_with_runtime(config, input, MobileRuntimeConfig::default()).await
}

pub async fn run_mobile_turn_with_runtime(
    config: Config,
    input: UserChatInput,
    runtime: MobileRuntimeConfig,
) -> anyhow::Result<MobileTurnUiResult> {
    run_mobile_turn_with_runtime_and_observer(config, input, runtime, |_| {}).await
}

pub async fn run_mobile_turn_streaming<F>(
    config: Config,
    input: UserChatInput,
    on_event: F,
) -> anyhow::Result<MobileTurnUiResult>
where
    F: Fn(AgentEvent) + 'static,
{
    run_mobile_turn_with_runtime_and_observer(
        config,
        input,
        crate::chat_session::runtime_for_active_thread(),
        on_event,
    )
    .await
}

pub async fn run_mobile_turn_with_runtime_and_observer<F>(
    config: Config,
    input: UserChatInput,
    runtime: MobileRuntimeConfig,
    on_event: F,
) -> anyhow::Result<MobileTurnUiResult>
where
    F: Fn(AgentEvent) + 'static,
{
    let store = RuntimeThreadStore::open(runtime.runtime_store_root.clone())?;
    let approval_session_store =
        ApprovalSessionRuntimeStore::new(runtime.runtime_store_root.clone());
    let approval_session = approval_session_store.load(&runtime.thread_id)?;

    // Load or create session with conversation history
    let session_path = runtime.session_file_path();
    let session = Session::load_or_new(&runtime.thread_id, &session_path)?;

    let mut engine = build_engine(config, &runtime, store)?
        .with_approval_session(approval_session)
        .with_session(session)
        .with_event_observer(on_event);

    // Streaming: live TextDelta in the chat timeline on phone (non-streaming hid the reply).
    let result = engine
        .run_turn_with_streaming(input.to_prompt_text())
        .await?;
    approval_session_store.save(runtime.thread_id.clone(), engine.approval_session())?;

    // Persist session history after turn
    engine.session().save_to_file(&session_path)?;
    let approval_card_count = result.approval_cards.len();
    Ok(MobileTurnUiResult {
        events: result.events,
        final_text: result.final_text,
        approval_cards: result.approval_cards,
        approval_card_count,
        runtime_store_root: runtime.runtime_store_root_display(),
        workspace_root: runtime.workspace_root_display(),
        thread_id: runtime.thread_id,
    })
}

pub fn load_mobile_approval_cards(
    runtime: MobileRuntimeConfig,
) -> anyhow::Result<Vec<ApprovalCardView>> {
    let store = RuntimeThreadStore::open(runtime.runtime_store_root.clone())?;
    let engine = build_engine(Config::default(), &runtime, store)?;
    engine.pending_approval_cards_for_current_thread()
}

pub fn load_default_mobile_approval_cards() -> anyhow::Result<Vec<ApprovalCardView>> {
    load_mobile_approval_cards(MobileRuntimeConfig::default())
}

pub async fn continue_mobile_approval(
    config: Config,
    approval_id: String,
    decision: ReviewDecision,
) -> anyhow::Result<MobileApprovalContinuationUiResult> {
    continue_mobile_approval_with_runtime_and_observer(
        config,
        approval_id,
        decision,
        MobileRuntimeConfig::default(),
        |_| {},
    )
    .await
}

pub async fn continue_mobile_approval_with_runtime(
    config: Config,
    approval_id: String,
    decision: ReviewDecision,
    runtime: MobileRuntimeConfig,
) -> anyhow::Result<MobileApprovalContinuationUiResult> {
    continue_mobile_approval_with_runtime_and_observer(
        config,
        approval_id,
        decision,
        runtime,
        |_| {},
    )
    .await
}

pub async fn continue_mobile_approval_with_runtime_and_observer<F>(
    config: Config,
    approval_id: String,
    decision: ReviewDecision,
    runtime: MobileRuntimeConfig,
    on_event: F,
) -> anyhow::Result<MobileApprovalContinuationUiResult>
where
    F: Fn(AgentEvent) + 'static,
{
    let store = RuntimeThreadStore::open(runtime.runtime_store_root.clone())?;
    let approval_session_store =
        ApprovalSessionRuntimeStore::new(runtime.runtime_store_root.clone());
    let approval_session = approval_session_store.load(&runtime.thread_id)?;
    let pending = store.load_pending_approval(&approval_id)?;
    let turn_record = store.load_turn(&pending.turn_id)?;

    let mut turn = TurnContext::new(100);
    turn.id = turn_record.id.clone();
    turn.status = TurnStatus::WaitingForApproval;
    turn.created_at_unix = turn_record.created_at_unix;
    turn.updated_at_unix = turn_record.updated_at_unix;
    turn.usage = TokenUsage {
        input_tokens: turn_record.usage_input_tokens,
        output_tokens: turn_record.usage_output_tokens,
        reasoning_tokens: turn_record.usage_reasoning_tokens,
    };
    turn.error = turn_record.error.clone();

    // Load session for continuation
    let session_path = runtime.session_file_path();
    let session = Session::load_or_new(&runtime.thread_id, &session_path)?;

    let mut engine = build_engine(config, &runtime, store)?
        .with_approval_session(approval_session)
        .with_session(session)
        .with_event_observer(on_event);

    let result = engine
        .continue_stored_approval(&approval_id, decision, turn)
        .await?;
    approval_session_store.save(runtime.thread_id, engine.approval_session())?;

    // Persist session history after continuation
    engine.session().save_to_file(&session_path)?;

    Ok(MobileApprovalContinuationUiResult {
        events: result.events,
        final_text: result.final_text,
        executed_count: result.executed.len(),
        session_grant_count: result.session_grants_created.len(),
        remaining_approval_cards: result.approval_cards,
    })
}

/// Continue a turn that was paused waiting for a Termux command result.
pub async fn continue_mobile_termux_result(
    config: Config,
    termux_result: TermuxExecResult,
) -> anyhow::Result<MobileTurnUiResult> {
    continue_mobile_termux_result_with_runtime(
        config,
        termux_result,
        MobileRuntimeConfig::default(),
    )
    .await
}

/// Continue the thread that originally queued the Termux request.
///
/// Chat history can switch between multiple threads while Android is executing
/// the command in Termux. The callback only carries `request_id`, so resolve the
/// pending record from the runtime store and continue that saved thread instead
/// of assuming `mobile-default-thread`.
pub async fn continue_mobile_termux_result_for_saved_request(
    config: Config,
    termux_result: TermuxExecResult,
) -> anyhow::Result<MobileTurnUiResult> {
    let base_runtime = MobileRuntimeConfig::default_mobile();
    let store = RuntimeThreadStore::open(base_runtime.runtime_store_root.clone())?;
    let pending = store.load_pending_termux(&termux_result.request_id)?;
    let runtime = base_runtime.with_thread_id(pending.thread_id);
    continue_mobile_termux_result_with_runtime(config, termux_result, runtime).await
}

/// Continue a turn that was paused waiting for a Termux command result,
/// using the provided runtime configuration.
pub async fn continue_mobile_termux_result_with_runtime(
    config: Config,
    termux_result: TermuxExecResult,
    runtime: MobileRuntimeConfig,
) -> anyhow::Result<MobileTurnUiResult> {
    let store = RuntimeThreadStore::open(runtime.runtime_store_root.clone())?;
    let approval_session_store =
        ApprovalSessionRuntimeStore::new(runtime.runtime_store_root.clone());
    let approval_session = approval_session_store.load(&runtime.thread_id)?;
    let session_path = runtime.session_file_path();
    let session = Session::load_or_new(&runtime.thread_id, &session_path)?;

    let mut engine = build_engine(config, &runtime, store)?
        .with_approval_session(approval_session)
        .with_session(session);

    let result = engine.continue_termux_result(termux_result).await?;
    approval_session_store.save(runtime.thread_id.clone(), engine.approval_session())?;
    engine.session().save_to_file(&session_path)?;

    let approval_card_count = result.approval_cards.len();
    Ok(MobileTurnUiResult {
        events: result.events,
        final_text: result.final_text,
        approval_cards: result.approval_cards,
        approval_card_count,
        runtime_store_root: runtime.runtime_store_root_display(),
        workspace_root: runtime.workspace_root_display(),
        thread_id: runtime.thread_id,
    })
}

fn build_engine(
    config: Config,
    runtime: &MobileRuntimeConfig,
    store: RuntimeThreadStore,
) -> anyhow::Result<MobileEngine> {
    let engine = MobileEngine::new(config)
        .with_runtime_store(store)
        .with_thread_id(runtime.thread_id.clone());

    let skills_context = load_skills_context_for_engine();
    let mcp_tools = load_mcp_tools_for_engine();

    let engine = if let Some(connection) = runtime.workspace_connection.as_ref() {
        engine.with_workspace_connection(connection)?
    } else {
        engine.with_workspace(runtime.workspace_root.clone())
    };

    Ok(engine
        .with_skills_context(skills_context)
        .with_mcp_tools(&mcp_tools))
}

fn load_mcp_tools_for_engine() -> Vec<deepseek_mobile_core::McpToolDescriptor> {
    use deepseek_mobile_core::{tools_for_server, McpClientRegistry, McpServerStatus};

    let path = default_data_dir().join("mcp.json");
    let mut registry = McpClientRegistry::load_or_default(&path).unwrap_or_default();
    if !registry.all_tools().is_empty() {
        return registry.all_tools();
    }

    let configs: Vec<_> = registry
        .servers
        .iter()
        .map(|server| server.config.clone())
        .collect();
    for config in configs {
        if !config.enabled || config.declared_tools.is_empty() {
            continue;
        }
        let tools = tools_for_server(&config.name, &config.declared_tools, Vec::new());
        registry.set_status(&config.name, McpServerStatus::Connected);
        registry.set_tools(&config.name, tools);
    }
    let _ = registry.save(&path);
    registry.all_tools()
}

fn load_skills_context_for_engine() -> Option<String> {
    let mut registry = SkillRegistry::discover_default().ok()?;
    let state_path = default_data_dir().join("skills-state.json");
    if state_path.exists() {
        if let Ok(bytes) = fs::read_to_string(&state_path) {
            if let Ok(saved) = serde_json::from_str::<HashMap<String, bool>>(&bytes) {
                for skill in registry.skills.iter_mut() {
                    if let Some(enabled) = saved.get(&skill.name) {
                        skill.enabled = *enabled;
                    }
                }
            } else if let Ok(file) = serde_json::from_str::<SkillsStateFile>(&bytes) {
                for skill in registry.skills.iter_mut() {
                    if let Some(enabled) = file.enabled.get(&skill.name) {
                        skill.enabled = *enabled;
                    }
                }
            }
        }
    }
    registry.full_context_injection()
}

#[derive(serde::Deserialize)]
struct SkillsStateFile {
    #[serde(default)]
    enabled: HashMap<String, bool>,
}

#[cfg(test)]
mod tests {
    use super::{MobileApprovalContinuationUiResult, MobileTurnUiResult};

    #[test]
    fn ui_result_reports_pending_approvals() {
        let result = MobileTurnUiResult {
            events: Vec::new(),
            final_text: None,
            approval_cards: Vec::new(),
            approval_card_count: 1,
            runtime_store_root: ".deepseek-mobile/runtime_store".to_string(),
            workspace_root: ".deepseek-mobile/workspace".to_string(),
            thread_id: "mobile-default-thread".to_string(),
        };
        assert!(result.has_pending_approvals());
    }

    #[test]
    fn ui_result_without_cards_has_no_pending_approvals() {
        let result = MobileTurnUiResult {
            events: Vec::new(),
            final_text: Some("done".to_string()),
            approval_cards: Vec::new(),
            approval_card_count: 0,
            runtime_store_root: ".deepseek-mobile/runtime_store".to_string(),
            workspace_root: ".deepseek-mobile/workspace".to_string(),
            thread_id: "mobile-default-thread".to_string(),
        };
        assert!(!result.has_pending_approvals());
    }

    #[test]
    fn continuation_result_reports_remaining_approvals() {
        let result = MobileApprovalContinuationUiResult {
            events: Vec::new(),
            final_text: None,
            executed_count: 0,
            session_grant_count: 0,
            remaining_approval_cards: Vec::new(),
        };
        assert!(!result.has_remaining_approvals());
    }
}

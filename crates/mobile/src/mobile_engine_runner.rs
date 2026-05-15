use crate::mobile_runtime_config::MobileRuntimeConfig;
use deepseek_mobile_core::{
    AgentEvent, ApprovalCardView, Config, MobileEngine, ReviewDecision, RuntimeThreadStore,
    TokenUsage, TurnContext, TurnStatus, UserChatInput,
};

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
    pub executed_count: usize,
    pub session_grant_count: usize,
    pub remaining_approval_cards: Vec<ApprovalCardView>,
}

impl MobileApprovalContinuationUiResult {
    pub fn has_remaining_approvals(&self) -> bool {
        !self.remaining_approval_cards.is_empty()
    }
}

pub async fn run_mobile_turn(config: Config, input: UserChatInput) -> anyhow::Result<MobileTurnUiResult> {
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
        MobileRuntimeConfig::default(),
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
    let engine = MobileEngine::new(config)
        .with_runtime_store(store)
        .with_thread_id(runtime.thread_id.clone())
        .with_workspace(runtime.workspace_root.clone())
        .with_event_observer(on_event);

    let result = engine.run_turn(input.to_prompt_text()).await?;
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

pub fn load_mobile_approval_cards(runtime: MobileRuntimeConfig) -> anyhow::Result<Vec<ApprovalCardView>> {
    let store = RuntimeThreadStore::open(runtime.runtime_store_root.clone())?;
    let engine = MobileEngine::new(Config::default())
        .with_runtime_store(store)
        .with_thread_id(runtime.thread_id)
        .with_workspace(runtime.workspace_root);
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
    continue_mobile_approval_with_runtime(config, approval_id, decision, MobileRuntimeConfig::default()).await
}

pub async fn continue_mobile_approval_with_runtime(
    config: Config,
    approval_id: String,
    decision: ReviewDecision,
    runtime: MobileRuntimeConfig,
) -> anyhow::Result<MobileApprovalContinuationUiResult> {
    let store = RuntimeThreadStore::open(runtime.runtime_store_root.clone())?;
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

    let engine = MobileEngine::new(config)
        .with_runtime_store(store)
        .with_thread_id(runtime.thread_id)
        .with_workspace(runtime.workspace_root);

    let result = engine
        .continue_stored_approval(&approval_id, decision, turn)
        .await?;
    let remaining_approval_cards = engine.pending_approval_cards_for_current_thread()?;

    Ok(MobileApprovalContinuationUiResult {
        events: result.events,
        executed_count: result.executed.len(),
        session_grant_count: result.session_grants_created.len(),
        remaining_approval_cards,
    })
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
            executed_count: 0,
            session_grant_count: 0,
            remaining_approval_cards: Vec::new(),
        };
        assert!(!result.has_remaining_approvals());
    }
}
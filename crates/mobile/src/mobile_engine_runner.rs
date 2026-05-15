use crate::mobile_runtime_config::MobileRuntimeConfig;
use deepseek_mobile_core::{
    AgentEvent, Config, MobileEngine, RuntimeThreadStore, UserChatInput,
};

#[derive(Clone, Debug, PartialEq)]
pub struct MobileTurnUiResult {
    pub events: Vec<AgentEvent>,
    pub final_text: Option<String>,
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

pub async fn run_mobile_turn(config: Config, input: UserChatInput) -> anyhow::Result<MobileTurnUiResult> {
    run_mobile_turn_with_runtime(config, input, MobileRuntimeConfig::default()).await
}

pub async fn run_mobile_turn_with_runtime(
    config: Config,
    input: UserChatInput,
    runtime: MobileRuntimeConfig,
) -> anyhow::Result<MobileTurnUiResult> {
    let store = RuntimeThreadStore::open(runtime.runtime_store_root.clone())?;
    let engine = MobileEngine::new(config)
        .with_runtime_store(store)
        .with_thread_id(runtime.thread_id.clone())
        .with_workspace(runtime.workspace_root.clone());

    let result = engine.run_turn(input.to_prompt_text()).await?;
    Ok(MobileTurnUiResult {
        events: result.events,
        final_text: result.final_text,
        approval_card_count: result.approval_cards.len(),
        runtime_store_root: runtime.runtime_store_root_display(),
        workspace_root: runtime.workspace_root_display(),
        thread_id: runtime.thread_id,
    })
}

#[cfg(test)]
mod tests {
    use super::MobileTurnUiResult;

    #[test]
    fn ui_result_reports_pending_approvals() {
        let result = MobileTurnUiResult {
            events: Vec::new(),
            final_text: None,
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
            approval_card_count: 0,
            runtime_store_root: ".deepseek-mobile/runtime_store".to_string(),
            workspace_root: ".deepseek-mobile/workspace".to_string(),
            thread_id: "mobile-default-thread".to_string(),
        };
        assert!(!result.has_pending_approvals());
    }
}
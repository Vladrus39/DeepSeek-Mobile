use deepseek_mobile_core::{AgentEvent, Config, MobileEngine, UserChatInput};

#[derive(Clone, Debug, PartialEq)]
pub struct MobileTurnUiResult {
    pub events: Vec<AgentEvent>,
    pub final_text: Option<String>,
    pub approval_card_count: usize,
}

impl MobileTurnUiResult {
    pub fn has_pending_approvals(&self) -> bool {
        self.approval_card_count > 0
    }
}

pub async fn run_mobile_turn(config: Config, input: UserChatInput) -> anyhow::Result<MobileTurnUiResult> {
    let engine = MobileEngine::new(config);
    let result = engine.run_turn(input.to_prompt_text()).await?;
    Ok(MobileTurnUiResult {
        events: result.events,
        final_text: result.final_text,
        approval_card_count: result.approval_cards.len(),
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
        };
        assert!(result.has_pending_approvals());
    }

    #[test]
    fn ui_result_without_cards_has_no_pending_approvals() {
        let result = MobileTurnUiResult {
            events: Vec::new(),
            final_text: Some("done".to_string()),
            approval_card_count: 0,
        };
        assert!(!result.has_pending_approvals());
    }
}

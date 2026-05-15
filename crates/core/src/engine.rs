//! Mobile engine skeleton.
//!
//! This is the bridge between the Android UI and the reusable agent core. It
//! already models turn lifecycle and event emission; the next step is to add
//! streaming/tool-call orchestration on top of this skeleton.

use crate::agent::DeepSeekAgent;
use crate::api_client::Message;
use crate::config::Config;
use crate::context::ContextManager;
use crate::events::AgentEvent;
use crate::model_router::ModelRouter;
use crate::turn::{TurnContext, TurnStatus};
use anyhow::Result;

pub struct MobileEngine {
    config: Config,
    agent: DeepSeekAgent,
    router: ModelRouter,
    context_manager: ContextManager,
    max_steps: u32,
}

impl MobileEngine {
    pub fn new(config: Config) -> Self {
        Self {
            agent: DeepSeekAgent::new(config.clone()),
            router: ModelRouter::new(config.clone()),
            context_manager: ContextManager::default(),
            config,
            max_steps: 100,
        }
    }

    pub fn with_max_steps(mut self, max_steps: u32) -> Self {
        self.max_steps = max_steps;
        self
    }

    pub async fn run_turn(&self, user_input: String) -> Result<EngineTurnResult> {
        let mut events = Vec::new();
        let mut turn = TurnContext::new(self.max_steps);
        turn.start();

        events.push(AgentEvent::TurnStarted {
            turn_id: turn.id.clone(),
        });

        let route = self.router.route_prompt(&user_input, 0);
        events.push(AgentEvent::Status(format!(
            "Model route: {} ({})",
            route.model, route.reason
        )));

        let messages = vec![Message {
            role: "user".to_string(),
            content: user_input,
        }];
        let compression_plan = self.context_manager.plan_for_messages(&messages);
        if compression_plan.should_compress {
            events.push(AgentEvent::Status(format!(
                "Context compression planned: {:?}",
                compression_plan.strategy
            )));
        }

        let response = self.agent.run_with_messages(messages.clone()).await;
        match response {
            Ok(text) => {
                events.push(AgentEvent::MessageStarted {
                    index: 0,
                    role: "assistant".to_string(),
                });
                events.push(AgentEvent::TextDelta(text.clone()));
                events.push(AgentEvent::MessageFinished { index: 0 });
                turn.complete();
                events.push(AgentEvent::TurnFinished {
                    turn_id: turn.id.clone(),
                    status: TurnStatus::Completed,
                    usage: turn.usage.clone(),
                    error: None,
                });
                events.push(AgentEvent::Finished);
                Ok(EngineTurnResult {
                    turn,
                    events,
                    final_text: Some(text),
                })
            }
            Err(error) => {
                let error_text = error.to_string();
                turn.fail(error_text.clone());
                events.push(AgentEvent::Error(error_text.clone()));
                events.push(AgentEvent::TurnFinished {
                    turn_id: turn.id.clone(),
                    status: TurnStatus::Failed,
                    usage: turn.usage.clone(),
                    error: Some(error_text.clone()),
                });
                Ok(EngineTurnResult {
                    turn,
                    events,
                    final_text: None,
                })
            }
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}

#[derive(Clone, Debug)]
pub struct EngineTurnResult {
    pub turn: TurnContext,
    pub events: Vec<AgentEvent>,
    pub final_text: Option<String>,
}

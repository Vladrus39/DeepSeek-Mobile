//! Context management primitives.
//!
//! Huge projects cannot be sent to the model as one raw prompt. The mobile
//! agent needs a context layer that can keep recent messages, summaries,
//! selected files, diagnostics and tool outputs under a model-specific budget.

use crate::api_client::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextBudget {
    pub max_input_tokens: usize,
    pub reserve_output_tokens: usize,
}

impl ContextBudget {
    pub fn effective_input_limit(&self) -> usize {
        self.max_input_tokens.saturating_sub(self.reserve_output_tokens)
    }
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            max_input_tokens: 128_000,
            reserve_output_tokens: 8_000,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextCompressionPlan {
    pub should_compress: bool,
    pub estimated_tokens: usize,
    pub budget: ContextBudget,
    pub strategy: CompressionStrategy,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompressionStrategy {
    None,
    SummarizeOlderMessages,
    SummarizeProjectFiles,
    KeepRecentAndRelevant,
}

pub struct ContextManager {
    budget: ContextBudget,
}

impl ContextManager {
    pub fn new(budget: ContextBudget) -> Self {
        Self { budget }
    }

    pub fn budget(&self) -> &ContextBudget {
        &self.budget
    }

    pub fn plan_for_messages(&self, messages: &[Message]) -> ContextCompressionPlan {
        let estimated_tokens = estimate_messages_tokens(messages);
        let limit = self.budget.effective_input_limit();

        let strategy = if estimated_tokens <= limit {
            CompressionStrategy::None
        } else if messages.len() > 12 {
            CompressionStrategy::SummarizeOlderMessages
        } else {
            CompressionStrategy::KeepRecentAndRelevant
        };

        ContextCompressionPlan {
            should_compress: strategy != CompressionStrategy::None,
            estimated_tokens,
            budget: self.budget.clone(),
            strategy,
        }
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new(ContextBudget::default())
    }
}

pub fn estimate_messages_tokens(messages: &[Message]) -> usize {
    messages
        .iter()
        .map(|message| estimate_text_tokens(&message.role) + estimate_text_tokens(&message.content))
        .sum()
}

pub fn estimate_text_tokens(text: &str) -> usize {
    // Conservative approximation: one token per four UTF-8 bytes, with a
    // minimum of one token for non-empty text. This keeps the logic dependency
    // free until a provider-specific tokenizer is added.
    if text.is_empty() {
        0
    } else {
        (text.len() / 4).max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::{CompressionStrategy, ContextBudget, ContextManager};
    use crate::api_client::Message;

    #[test]
    fn does_not_compress_small_history() {
        let manager = ContextManager::new(ContextBudget {
            max_input_tokens: 1_000,
            reserve_output_tokens: 100,
        });
        let messages = vec![Message {
            role: "user".to_string(),
            content: "hello".to_string(),
        }];

        let plan = manager.plan_for_messages(&messages);

        assert!(!plan.should_compress);
        assert_eq!(plan.strategy, CompressionStrategy::None);
    }

    #[test]
    fn compresses_large_history() {
        let manager = ContextManager::new(ContextBudget {
            max_input_tokens: 32,
            reserve_output_tokens: 8,
        });
        let messages = (0..20)
            .map(|idx| Message {
                role: "user".to_string(),
                content: format!("large message number {} with repeated content", idx),
            })
            .collect::<Vec<_>>();

        let plan = manager.plan_for_messages(&messages);

        assert!(plan.should_compress);
        assert_eq!(plan.strategy, CompressionStrategy::SummarizeOlderMessages);
    }
}

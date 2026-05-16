use crate::api_client::Message;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONTEXT_TOKEN_LIMIT: usize = 128_000;
pub const DEFAULT_RESPONSE_RESERVE_TOKENS: usize = 8_000;
pub const DEFAULT_SYSTEM_RESERVE_TOKENS: usize = 2_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompressionStrategy {
    None,
    TruncateOld,
    Summarize,
    SummarizeAndTruncate,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextBudget {
    pub max_context_tokens: usize,
    pub response_reserve_tokens: usize,
    pub system_reserve_tokens: usize,
}

impl ContextBudget {
    pub fn new(max_context_tokens: usize) -> Self {
        Self {
            max_context_tokens,
            response_reserve_tokens: DEFAULT_RESPONSE_RESERVE_TOKENS,
            system_reserve_tokens: DEFAULT_SYSTEM_RESERVE_TOKENS,
        }
    }

    pub fn usable_input_tokens(&self) -> usize {
        self.max_context_tokens
            .saturating_sub(self.response_reserve_tokens)
            .saturating_sub(self.system_reserve_tokens)
    }

    pub fn with_response_reserve(mut self, reserve: usize) -> Self {
        self.response_reserve_tokens = reserve;
        self
    }

    pub fn with_system_reserve(mut self, reserve: usize) -> Self {
        self.system_reserve_tokens = reserve;
        self
    }
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self::new(DEFAULT_CONTEXT_TOKEN_LIMIT)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextCompressionPlan {
    pub estimated_tokens: usize,
    pub usable_input_tokens: usize,
    pub over_budget_tokens: usize,
    pub should_compress: bool,
    pub strategy: CompressionStrategy,
    pub keep_recent_messages: usize,
    pub dropped_messages: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompressionPlan {
    pub should_compress: bool,
    pub strategy: CompressionStrategy,
}

pub struct ContextManager {
    budget: ContextBudget,
    min_recent_messages: usize,
}

impl ContextManager {
    pub fn new(max_context_tokens: usize) -> Self {
        Self::with_budget(ContextBudget::new(max_context_tokens))
    }

    pub fn with_budget(budget: ContextBudget) -> Self {
        Self { budget, min_recent_messages: 8 }
    }

    pub fn with_min_recent_messages(mut self, count: usize) -> Self {
        self.min_recent_messages = count.max(1);
        self
    }

    pub fn budget(&self) -> &ContextBudget {
        &self.budget
    }

    pub fn plan(&self, messages: &[Message]) -> ContextCompressionPlan {
        let estimated_tokens = estimate_messages_tokens(messages);
        let usable = self.budget.usable_input_tokens();
        if estimated_tokens <= usable {
            return ContextCompressionPlan {
                estimated_tokens,
                usable_input_tokens: usable,
                over_budget_tokens: 0,
                should_compress: false,
                strategy: CompressionStrategy::None,
                keep_recent_messages: messages.len(),
                dropped_messages: 0,
            };
        }
        let keep_recent_messages = self.recent_message_count_for_budget(messages, usable);
        let dropped_messages = messages.len().saturating_sub(keep_recent_messages);
        let over_budget_tokens = estimated_tokens.saturating_sub(usable);
        let strategy = if dropped_messages == 0 {
            CompressionStrategy::Summarize
        } else if over_budget_tokens > usable / 2 {
            CompressionStrategy::SummarizeAndTruncate
        } else {
            CompressionStrategy::TruncateOld
        };
        ContextCompressionPlan {
            estimated_tokens,
            usable_input_tokens: usable,
            over_budget_tokens,
            should_compress: true,
            strategy,
            keep_recent_messages,
            dropped_messages,
        }
    }

    pub fn plan_for_messages(&self, messages: &[Message]) -> CompressionPlan {
        let plan = self.plan(messages);
        CompressionPlan { should_compress: plan.should_compress, strategy: plan.strategy }
    }

    pub fn fit_messages(&self, messages: &[Message]) -> Vec<Message> {
        let plan = self.plan(messages);
        if !plan.should_compress || plan.keep_recent_messages >= messages.len() {
            return messages.to_vec();
        }
        let mut fitted = Vec::new();
        if let Some(first) = messages.first() {
            if first.role == "system" {
                fitted.push(first.clone());
            }
        }
        let start = messages.len().saturating_sub(plan.keep_recent_messages);
        for message in messages.iter().skip(start) {
            if message.role == "system" && fitted.iter().any(|existing| existing.role == "system") {
                continue;
            }
            fitted.push(message.clone());
        }
        fitted
    }

    fn recent_message_count_for_budget(&self, messages: &[Message], usable_tokens: usize) -> usize {
        let mut total = 0usize;
        let mut count = 0usize;
        for message in messages.iter().rev() {
            let estimate = estimate_message_tokens(message);
            if count >= self.min_recent_messages && total.saturating_add(estimate) > usable_tokens {
                break;
            }
            total = total.saturating_add(estimate);
            count += 1;
        }
        count.min(messages.len())
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::with_budget(ContextBudget::default())
    }
}

pub fn estimate_messages_tokens(messages: &[Message]) -> usize {
    messages.iter().map(estimate_message_tokens).sum::<usize>() + messages.len() * 4
}

pub fn estimate_message_tokens(message: &Message) -> usize {
    estimate_text_tokens(&message.role) + estimate_text_tokens(&message.content) + 8
}

pub fn estimate_text_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let chars = text.chars().count();
    let words = text.split_whitespace().count();
    let char_estimate = chars.div_ceil(4);
    let word_estimate = words.saturating_mul(4).div_ceil(3);
    char_estimate.max(word_estimate)
}
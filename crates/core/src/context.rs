//! ContextManager - improved version

use crate::api_client::Message;

#[derive(Default, Clone)]
pub struct ContextManager {
    max_messages: usize,
}

impl ContextManager {
    pub fn new(max_messages: usize) -> Self {
        Self { max_messages }
    }

    pub fn plan_for_messages(&self, messages: &[Message]) -> CompressionPlan {
        if messages.len() > self.max_messages {
            CompressionPlan {
                should_compress: true,
                strategy: CompressionStrategy::TruncateOld,
            }
        } else {
            CompressionPlan { should_compress: false, strategy: CompressionStrategy::None }
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompressionPlan {
    pub should_compress: bool,
    pub strategy: CompressionStrategy,
}

#[derive(Debug, Clone)]
pub enum CompressionStrategy {
    None,
    TruncateOld,
    Summarize,
}

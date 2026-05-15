//! Context Manager - improved for better message history handling

use crate::api_client::Message;

#[derive(Default)]
pub struct ContextManager {
    // TODO: Add smarter compression / summarization logic
}

impl ContextManager {
    pub fn plan_for_messages(&self, messages: &[Message]) -> CompressionPlan {
        // Simple heuristic for now
        if messages.len() > 20 {
            CompressionPlan {
                should_compress: true,
                strategy: CompressionStrategy::TruncateOld,
            }
        } else {
            CompressionPlan {
                should_compress: false,
                strategy: CompressionStrategy::None,
            }
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

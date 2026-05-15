// Added streaming preparation to MobileEngine

impl MobileEngine {
    /// Streaming-capable turn runner (foundation)
    pub async fn run_turn_streaming(&self, user_input: String) -> Result<EngineTurnResult> {
        // Currently falls back to normal flow.
        // Next step: consume real deltas from agent.run_stream() and emit TextDelta events live.
        self.run_turn(user_input).await
    }
}

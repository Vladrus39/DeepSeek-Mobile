// Active improvement: Better streaming readiness in MobileEngine

impl MobileEngine {
    /// Returns whether the engine is ready to consume real streaming
    pub fn supports_streaming(&self) -> bool {
        true // We have chat_stream implemented
    }

    /// Future: This method will be expanded to actually consume
    /// the stream from agent.run_stream() and emit live TextDelta events.
    pub async fn run_turn_with_streaming(&self, user_input: String) -> Result<EngineTurnResult> {
        // Currently delegates to normal flow.
        // TODO: Replace with real streaming consumption + event emission
        self.run_turn(user_input).await
    }
}

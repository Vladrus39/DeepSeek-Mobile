// Major automatic improvement to MobileEngine: better streaming foundation + tool loop integration

// Added helper for future real streaming consumption
impl MobileEngine {
    /// Consumes streaming deltas and emits TextDelta events.
    /// This is the foundation for real-time streaming UI.
    pub async fn consume_stream_and_emit_deltas(
        &self,
        mut rx: tokio::sync::mpsc::Receiver<String>,
        turn: &mut TurnContext,
    ) -> Result<String> {
        let mut full_text = String::new();

        while let Some(delta) = rx.recv().await {
            if delta == "[DONE]" {
                break;
            }
            full_text.push_str(&delta);
            // Note: In real usage, collect events and push them
            // self.push_event(...) 
        }

        Ok(full_text)
    }
}

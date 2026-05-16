use deepseek_mobile_core::{AgentEvent, PcDiagnostic, PcDiagnosticSeverity};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticsUiState {
    pub summary: Option<String>,
    pub diagnostics: Vec<PcDiagnostic>,
    pub path: Option<String>,
    pub provider: Option<String>,
    pub status: Option<String>,
    pub error: Option<String>,
}

impl DiagnosticsUiState {
    pub fn apply_agent_event(&mut self, event: &AgentEvent) {
        let AgentEvent::ToolCallFinished(result) = event else {
            return;
        };
        let Some(metadata) = result.metadata.as_ref() else {
            return;
        };

        let has_diagnostics_payload = metadata.get("post_edit_diagnostics").is_some()
            || metadata.get("post_edit_diagnostics_summary").is_some()
            || metadata.get("post_edit_diagnostics_error").is_some();
        if !has_diagnostics_payload {
            return;
        }

        self.summary = metadata
            .get("post_edit_diagnostics_summary")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        self.path = metadata
            .get("post_edit_diagnostics_path")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        self.provider = metadata
            .get("post_edit_diagnostics_provider")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        self.status = metadata
            .get("post_edit_diagnostics_status")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        self.error = metadata
            .get("post_edit_diagnostics_error")
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .or_else(|| {
                metadata
                    .get("post_edit_diagnostics_message")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            });
        self.diagnostics = metadata
            .get("post_edit_diagnostics")
            .and_then(|value| serde_json::from_value::<Vec<PcDiagnostic>>(value.clone()).ok())
            .unwrap_or_default();
    }

    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|item| item.severity == PcDiagnosticSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|item| item.severity == PcDiagnosticSeverity::Warning)
            .count()
    }

    pub fn has_data(&self) -> bool {
        self.summary.is_some() || self.error.is_some() || !self.diagnostics.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::DiagnosticsUiState;
    use deepseek_mobile_core::{AgentEvent, ToolResultEvent};

    #[test]
    fn diagnostics_metadata_updates_latest_report() {
        let mut state = DiagnosticsUiState::default();
        state.apply_agent_event(&AgentEvent::ToolCallFinished(ToolResultEvent {
            id: "tool-1".to_string(),
            name: "write_file".to_string(),
            success: true,
            output: "ok".to_string(),
            metadata: Some(serde_json::json!({
                "post_edit_diagnostics_summary": "1 diagnostic(s): 1 error(s), 0 warning(s)",
                "post_edit_diagnostics_path": "src/main.rs",
                "post_edit_diagnostics": [{
                    "path": "src/main.rs",
                    "line": 7,
                    "column": 3,
                    "severity": "Error",
                    "message": "expected expression",
                    "source": "cargo check"
                }]
            })),
        }));

        assert!(state.has_data());
        assert_eq!(state.path.as_deref(), Some("src/main.rs"));
        assert_eq!(state.error_count(), 1);
        assert_eq!(state.warning_count(), 0);
    }
}

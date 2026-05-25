//! Terminal session state for mobile UI.
//!
//! Tracks active terminal sessions from the PC gateway, recent command output,
//! and user input for sending commands to remote terminals.

use deepseek_mobile_core::PcTerminalSession;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalSessionView {
    pub id: String,
    pub title: String,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output: Vec<String>,
    pub is_open: bool,
    pub exit_code: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerminalUiState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sessions: Vec<TerminalSessionView>,
    pub selected_session_id: Option<String>,
    pub input_text: String,
    pub loading: bool,
    pub error: Option<String>,
}

impl Default for TerminalUiState {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            selected_session_id: None,
            input_text: String::new(),
            loading: false,
            error: None,
        }
    }
}

impl TerminalUiState {
    pub fn add_session(&mut self, session: PcTerminalSession) {
        self.sessions.retain(|s| s.id != session.id);
        self.sessions.push(TerminalSessionView {
            id: session.id,
            title: session.title,
            cwd: session.cwd,
            output: Vec::new(),
            is_open: true,
            exit_code: None,
        });
        self.selected_session_id = self.sessions.last().map(|s| s.id.clone());
        self.error = None;
    }

    pub fn append_output(&mut self, session_id: &str, chunk: &str) {
        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == session_id) {
            session.output.push(chunk.to_string());
        }
    }

    pub fn close_session(&mut self, session_id: &str, exit_code: Option<i32>) {
        if let Some(session) = self.sessions.iter_mut().find(|s| s.id == session_id) {
            session.is_open = false;
            session.exit_code = exit_code;
        }
        if self.selected_session_id.as_deref() == Some(session_id) {
            self.selected_session_id = None;
        }
    }

    pub fn select_session(&mut self, session_id: &str) {
        self.selected_session_id = Some(session_id.to_string());
    }

    pub fn selected_output(&self) -> Vec<&str> {
        let id = match self.selected_session_id.as_ref() {
            Some(id) => id,
            None => return Vec::new(),
        };
        self.sessions
            .iter()
            .find(|s| s.id == *id)
            .map(|s| s.output.iter().map(|l| l.as_str()).collect())
            .unwrap_or_default()
    }

    pub fn active_session_count(&self) -> usize {
        self.sessions.iter().filter(|s| s.is_open).count()
    }

    /// Save terminal state to a JSON file, truncating output to last N lines per session.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        const MAX_OUTPUT_LINES: usize = 200;
        let mut compact = self.clone();
        for session in &mut compact.sessions {
            if session.output.len() > MAX_OUTPUT_LINES {
                let keep = session.output.split_off(session.output.len() - MAX_OUTPUT_LINES);
                // Insert a marker at the start
                session.output = keep;
                session.output.insert(0, format!("...[{} lines truncated]...", session.output.len().saturating_sub(MAX_OUTPUT_LINES + 1)));
            }
        }
        let json = serde_json::to_string_pretty(&compact)?;
        std::fs::write(path.as_ref(), json)?;
        Ok(())
    }

    /// Load terminal state from a JSON file, or return default if file missing.
    pub fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let json = std::fs::read_to_string(path)?;
        let state: Self = serde_json::from_str(&json)?;
        // Mark all sessions as closed since processes don't survive restart
        let mut state = state;
        for session in &mut state.sessions {
            session.is_open = false;
        }
        state.selected_session_id = None;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session(id: &str) -> PcTerminalSession {
        PcTerminalSession {
            id: id.to_string(),
            workspace_id: "w1".to_string(),
            title: format!("Terminal {}", id),
            cwd: "/workspace".to_string(),
            environment_id: None,
            created_at_unix: 1000,
        }
    }

    #[test]
    fn tracks_session_lifecycle() {
        let mut state = TerminalUiState::default();
        state.add_session(test_session("term-1"));
        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.active_session_count(), 1);

        state.append_output("term-1", "hello");
        state.append_output("term-1", "world");
        assert_eq!(state.selected_output().len(), 2);

        state.close_session("term-1", Some(0));
        assert_eq!(state.active_session_count(), 0);
    }

    #[test]
    fn select_between_sessions() {
        let mut state = TerminalUiState::default();
        state.add_session(test_session("term-a"));
        state.add_session(test_session("term-b"));
        assert_eq!(state.selected_session_id.as_deref(), Some("term-b"));

        state.select_session("term-a");
        assert_eq!(state.selected_session_id.as_deref(), Some("term-a"));
    }
}
//! Session state for DeepSeek Mobile.
//!
//! Android can suspend or kill the mobile app at any time. The core therefore
//! keeps an explicit session model with JSON file persistence so conversation
//! history survives process death.

use crate::api_client::Message;
use crate::pc_gateway::PcDiagnostic;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionDiagnosticsContext {
    pub summary: String,
    #[serde(default)]
    pub diagnostics: Vec<PcDiagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_diagnostics: Option<SessionDiagnosticsContext>,
    pub created_at_unix: u64,
    pub updated_at_unix: u64,
}

impl Session {
    pub fn new(id: impl Into<String>) -> Self {
        let now = current_unix_time();
        Self {
            id: id.into(),
            title: "New session".to_string(),
            messages: Vec::new(),
            latest_diagnostics: None,
            created_at_unix: now,
            updated_at_unix: now,
        }
    }

    pub fn push_message(&mut self, role: impl Into<String>, content: impl Into<String>) {
        self.messages.push(Message {
            role: role.into(),
            content: content.into(),
        });
        self.updated_at_unix = current_unix_time();
    }

    pub fn last_user_message(&self) -> Option<&Message> {
        self.messages
            .iter()
            .rev()
            .find(|message| message.role == "user")
    }

    pub fn set_latest_diagnostics(&mut self, diagnostics: SessionDiagnosticsContext) {
        self.latest_diagnostics = Some(diagnostics);
        self.updated_at_unix = current_unix_time();
    }

    /// Save session to a JSON file.
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load session from a JSON file.
    /// Returns `None` if the file does not exist.
    pub fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Option<Self>> {
        if !path.as_ref().exists() {
            return Ok(None);
        }
        let json = std::fs::read_to_string(path)?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(Some(session))
    }

    /// Load session or create a new one with the given id.
    pub fn load_or_new(id: impl Into<String>, path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let id = id.into();
        match Self::load_from_file(&path)? {
            Some(session) => Ok(session),
            None => Ok(Self::new(id)),
        }
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new("default")
    }
}

fn current_unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{Session, SessionDiagnosticsContext};
    use crate::pc_gateway::{PcDiagnostic, PcDiagnosticSeverity};
    use std::path::PathBuf;

    fn test_session_path() -> PathBuf {
        std::env::temp_dir().join("deepseek_mobile_test_session.json")
    }

    #[test]
    fn session_roundtrip_save_load() {
        let path = test_session_path();
        let _ = std::fs::remove_file(&path);

        let mut session = Session::new("test-roundtrip");
        session.push_message("user", "hello");
        session.push_message("assistant", "hi there");
        session.set_latest_diagnostics(SessionDiagnosticsContext {
            summary: "1 diagnostic(s): 1 error(s), 0 warning(s)".to_string(),
            diagnostics: vec![PcDiagnostic {
                path: "src/main.rs".to_string(),
                line: 7,
                column: 3,
                severity: PcDiagnosticSeverity::Error,
                message: "expected expression".to_string(),
                source: Some("cargo check".to_string()),
            }],
            path: Some("src/main.rs".to_string()),
            provider: Some("cargo check".to_string()),
            status: Some("Completed".to_string()),
        });

        session.save_to_file(&path).expect("save");
        let loaded = Session::load_from_file(&path).expect("load").expect("some");

        assert_eq!(loaded.id, "test-roundtrip");
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(loaded.messages[0].role, "user");
        assert_eq!(loaded.messages[0].content, "hello");
        assert_eq!(loaded.messages[1].role, "assistant");
        assert_eq!(loaded.messages[1].content, "hi there");
        assert_eq!(
            loaded.latest_diagnostics.as_ref().unwrap().path.as_deref(),
            Some("src/main.rs")
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn session_load_nonexistent_returns_none() {
        let path = PathBuf::from("/tmp/deepseek_mobile_nonexistent_session.json");
        let _ = std::fs::remove_file(&path);
        let result = Session::load_from_file(&path).expect("load");
        assert!(result.is_none());
    }

    #[test]
    fn session_load_or_new_creates_fresh() {
        let path = PathBuf::from("/tmp/deepseek_mobile_load_or_new_test.json");
        let _ = std::fs::remove_file(&path);
        let session = Session::load_or_new("fresh", &path).expect("load_or_new");
        assert_eq!(session.id, "fresh");
        assert!(session.messages.is_empty());
        let _ = std::fs::remove_file(&path);
    }
}

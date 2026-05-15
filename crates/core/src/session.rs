//! Session state for DeepSeek Mobile.
//!
//! Android can suspend or kill the mobile app at any time. The core therefore
//! keeps an explicit session model that can later be persisted to JSON, SQLite,
//! or a remote runtime store.

use crate::api_client::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub title: String,
    pub messages: Vec<Message>,
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
        self.messages.iter().rev().find(|message| message.role == "user")
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

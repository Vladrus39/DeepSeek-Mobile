//! Runtime persistence adapter for approval-session grants.
//!
//! `ApprovalSessionPolicy` is still the policy type. This adapter only bridges
//! the stateless mobile runner gap: the UI may create a fresh `MobileEngine` per
//! callback, so grants need to be rehydrated for the active thread.

use crate::approval_session::ApprovalSessionPolicy;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalSessionRuntimeRecord {
    pub thread_id: String,
    pub policy: ApprovalSessionPolicy,
    pub updated_at_unix: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovalSessionRuntimeStore {
    root: PathBuf,
}

impl ApprovalSessionRuntimeStore {
    pub fn new(runtime_root: impl Into<PathBuf>) -> Self {
        Self {
            root: runtime_root.into().join("approval_sessions"),
        }
    }

    pub fn load(&self, thread_id: &str) -> Result<ApprovalSessionPolicy> {
        let path = self.path(thread_id);
        if !path.exists() {
            return Ok(ApprovalSessionPolicy::new());
        }
        let bytes = fs::read(&path).map_err(|error| {
            anyhow!(
                "failed to read approval session {}: {}",
                path.display(),
                error
            )
        })?;
        let record: ApprovalSessionRuntimeRecord = serde_json::from_slice(&bytes)?;
        Ok(record.policy)
    }

    pub fn save(
        &self,
        thread_id: impl Into<String>,
        policy: &ApprovalSessionPolicy,
    ) -> Result<ApprovalSessionRuntimeRecord> {
        let record = ApprovalSessionRuntimeRecord {
            thread_id: thread_id.into(),
            policy: policy.clone(),
            updated_at_unix: unix_time(),
        };
        fs::create_dir_all(&self.root)?;
        fs::write(
            self.path(&record.thread_id),
            serde_json::to_string_pretty(&record)?,
        )?;
        Ok(record)
    }

    pub fn clear(&self, thread_id: &str) -> Result<()> {
        let path = self.path(thread_id);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path(&self, thread_id: &str) -> PathBuf {
        self.root.join(format!("{}.json", safe_file_id(thread_id)))
    }
}

fn safe_file_id(id: &str) -> String {
    id.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::ApprovalSessionRuntimeStore;
    use crate::approval::{ApprovalRisk, MobileApprovalRequest, ToolCategory};
    use crate::approval_session::ApprovalSessionPolicy;
    use crate::tool_call::{ToolCallRequest, ToolCallSource};
    use serde_json::json;
    use std::fs;

    fn unique_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "deepseek-approval-session-store-{}-{}",
            name, nanos
        ))
    }

    #[test]
    fn saves_loads_and_clears_policy() {
        let root = unique_dir("roundtrip");
        let store = ApprovalSessionRuntimeStore::new(&root);
        let call = ToolCallRequest::new(
            "write_file",
            json!({"path":"README.md","content":"x"}),
            ToolCallSource::Manual,
        );
        let approval = MobileApprovalRequest::new(
            "write_file",
            ToolCategory::FileWrite,
            ApprovalRisk::Benign,
            call.arguments.clone(),
        );
        let mut policy = ApprovalSessionPolicy::new();
        policy.grant_for_approved_call(&approval, &call);

        store.save("thread-1", &policy).expect("save policy");
        let loaded = store.load("thread-1").expect("load policy");
        assert_eq!(loaded.grant_count(), 1);

        store.clear("thread-1").expect("clear policy");
        assert_eq!(store.load("thread-1").unwrap().grant_count(), 0);
        let _ = fs::remove_dir_all(root);
    }
}

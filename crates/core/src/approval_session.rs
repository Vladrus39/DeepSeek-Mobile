//! In-memory approval grants for the current mobile session.
//!
//! A user can approve a safe class of repeated actions for the active app
//! session. These grants are deliberately not durable: closing/restarting the
//! Android app clears them and returns to explicit approvals.

use crate::approval::{ApprovalRisk, MobileApprovalRequest, ToolCategory};
use crate::tool_call::ToolCallRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalSessionScope {
    ExactFilePath { path: String },
    GitTool,
    NetworkHost { host: String },
    ExactTool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalSessionGrant {
    pub id: String,
    pub tool_name: String,
    pub category: ToolCategory,
    pub scope: ApprovalSessionScope,
    pub created_at_unix: u64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalSessionPolicy {
    grants: Vec<ApprovalSessionGrant>,
}

impl ApprovalSessionPolicy {
    pub fn new() -> Self {
        Self { grants: Vec::new() }
    }

    pub fn grants(&self) -> &[ApprovalSessionGrant] {
        &self.grants
    }

    pub fn clear(&mut self) {
        self.grants.clear();
    }

    pub fn grant_count(&self) -> usize {
        self.grants.len()
    }

    pub fn grant_for_approved_call(
        &mut self,
        approval: &MobileApprovalRequest,
        call: &ToolCallRequest,
    ) -> Option<ApprovalSessionGrant> {
        if !can_grant_for_session(approval) {
            return None;
        }

        let scope = scope_for_call(approval, call)?;
        if let Some(existing) = self
            .grants
            .iter()
            .find(|grant| grant.tool_name == approval.tool_name && grant.category == approval.category && grant.scope == scope)
        {
            return Some(existing.clone());
        }

        let grant = ApprovalSessionGrant {
            id: format!("session-grant-{}-{}", current_unix_time(), self.grants.len() + 1),
            tool_name: approval.tool_name.clone(),
            category: approval.category.clone(),
            scope,
            created_at_unix: current_unix_time(),
        };
        self.grants.push(grant.clone());
        Some(grant)
    }

    pub fn is_call_allowed_by_session(
        &self,
        approval: &MobileApprovalRequest,
        call: &ToolCallRequest,
    ) -> bool {
        if !can_grant_for_session(approval) {
            return false;
        }

        self.grants.iter().any(|grant| {
            grant.tool_name == approval.tool_name
                && grant.category == approval.category
                && scope_matches(&grant.scope, approval, call)
        })
    }
}

pub fn can_grant_for_session(approval: &MobileApprovalRequest) -> bool {
    match (&approval.category, &approval.risk) {
        (ToolCategory::Shell, _) | (ToolCategory::Unknown, _) => false,
        (ToolCategory::Network, ApprovalRisk::Destructive) => false,
        (ToolCategory::Safe, _) => true,
        (ToolCategory::FileWrite, _) | (ToolCategory::Git, _) => true,
        (_, ApprovalRisk::Benign) => true,
    }
}

fn scope_for_call(
    approval: &MobileApprovalRequest,
    call: &ToolCallRequest,
) -> Option<ApprovalSessionScope> {
    match &approval.category {
        ToolCategory::FileWrite => argument_string(&call.arguments, "path")
            .or_else(|| argument_string(&approval.params, "path"))
            .map(|path| ApprovalSessionScope::ExactFilePath { path }),
        ToolCategory::Git => Some(ApprovalSessionScope::GitTool),
        ToolCategory::Network => argument_string(&call.arguments, "url")
            .or_else(|| argument_string(&approval.params, "url"))
            .and_then(|url| host_from_url(&url))
            .map(|host| ApprovalSessionScope::NetworkHost { host }),
        ToolCategory::Safe => Some(ApprovalSessionScope::ExactTool),
        ToolCategory::Shell | ToolCategory::Unknown => None,
    }
}

fn scope_matches(
    scope: &ApprovalSessionScope,
    approval: &MobileApprovalRequest,
    call: &ToolCallRequest,
) -> bool {
    match scope {
        ApprovalSessionScope::ExactFilePath { path } => {
            if !matches!(&approval.category, ToolCategory::FileWrite) {
                return false;
            }
            argument_string(&call.arguments, "path")
                .or_else(|| argument_string(&approval.params, "path"))
                .map_or(false, |candidate| candidate == *path)
        }
        ApprovalSessionScope::GitTool => matches!(&approval.category, ToolCategory::Git),
        ApprovalSessionScope::NetworkHost { host } => {
            if !matches!(&approval.category, ToolCategory::Network) {
                return false;
            }
            argument_string(&call.arguments, "url")
                .or_else(|| argument_string(&approval.params, "url"))
                .and_then(|url| host_from_url(&url))
                .map_or(false, |candidate| candidate == *host)
        }
        ApprovalSessionScope::ExactTool => true,
    }
}

fn argument_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(std::string::ToString::to_string)
}

fn host_from_url(url: &str) -> Option<String> {
    let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let host_port_path = without_scheme.split('/').next().unwrap_or_default();
    let host = host_port_path.split('@').last().unwrap_or(host_port_path);
    let host = host.split(':').next().unwrap_or(host).trim();
    (!host.is_empty()).then(|| host.to_ascii_lowercase())
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{can_grant_for_session, ApprovalSessionPolicy};
    use crate::approval::{ApprovalRisk, MobileApprovalRequest, ToolCategory};
    use crate::tool_call::{ToolCallRequest, ToolCallSource};
    use serde_json::json;

    #[test]
    fn file_write_session_grant_matches_same_path_only() {
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
        assert!(policy.grant_for_approved_call(&approval, &call).is_some());
        assert!(policy.is_call_allowed_by_session(&approval, &call));

        let other_call = ToolCallRequest::new(
            "write_file",
            json!({"path":"src/lib.rs","content":"x"}),
            ToolCallSource::Manual,
        );
        let other_approval = MobileApprovalRequest::new(
            "write_file",
            ToolCategory::FileWrite,
            ApprovalRisk::Benign,
            other_call.arguments.clone(),
        );
        assert!(!policy.is_call_allowed_by_session(&other_approval, &other_call));
    }

    #[test]
    fn duplicate_session_grant_does_not_grow_policy() {
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
        let first = policy.grant_for_approved_call(&approval, &call).unwrap();
        let second = policy.grant_for_approved_call(&approval, &call).unwrap();
        assert_eq!(policy.grant_count(), 1);
        assert_eq!(first.id, second.id);
    }

    #[test]
    fn shell_tools_cannot_be_granted_for_session() {
        let call = ToolCallRequest::new(
            "exec_shell",
            json!({"command":"cargo check"}),
            ToolCallSource::Manual,
        );
        let approval = MobileApprovalRequest::new(
            "exec_shell",
            ToolCategory::Shell,
            ApprovalRisk::Destructive,
            call.arguments.clone(),
        );
        assert_eq!(approval.category, ToolCategory::Shell);
        assert!(!can_grant_for_session(&approval));
        let mut policy = ApprovalSessionPolicy::new();
        assert!(policy.grant_for_approved_call(&approval, &call).is_none());
    }
}
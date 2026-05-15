//! Agent event types shared by the core and mobile UI.
//!
//! The mobile app should not wait for a single final string forever. The agent
//! core will increasingly emit events: text deltas, tool status, approval
//! requests, patch proposals and final completion.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentEvent {
    Started,
    Status(String),
    TextDelta(String),
    ReasoningDelta(String),
    ToolCallStarted(ToolCallEvent),
    ToolCallFinished(ToolResultEvent),
    ApprovalRequired(ApprovalRequest),
    PatchProposed(PatchProposal),
    Error(String),
    Finished,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolCallEvent {
    pub id: String,
    pub name: String,
    pub args: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolResultEvent {
    pub id: String,
    pub name: String,
    pub success: bool,
    pub output: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalRequest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub risk_level: RiskLevel,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Dangerous,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchProposal {
    pub id: String,
    pub file_path: String,
    pub diff: String,
}

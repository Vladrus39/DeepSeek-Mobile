//! Execution abstraction for Android, Termux, PC gateway and remote backends.
//!
//! Large projects should not be limited by the phone. The mobile app can route
//! commands either to a safe local executor, a Termux bridge, a paired PC
//! gateway, or a remote Y-lit executor while keeping the same agent/tool
//! interface.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for a Termux command execution request.
/// Used to correlate the result event sent from the Kotlin bridge
/// back to the awaiting Rust caller.
pub type TermuxRequestId = String;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandRequest {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
}

impl CommandRequest {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
            working_dir: None,
        }
    }

    pub fn with_working_dir(mut self, working_dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandOutput {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

pub trait Executor: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, request: CommandRequest) -> Result<CommandOutput>;
}

// ── Termux execution bridge ──

/// A Termux command execution request sent from Rust to the Android/Kotlin bridge.
/// The native side runs `sh -c "<command>"` in the given working directory
/// and sends back a `TermuxExecResult` event.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TermuxExecRequest {
    pub request_id: TermuxRequestId,
    pub command: String,
    pub working_dir: PathBuf,
    pub timeout_secs: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TermuxExecResult {
    pub request_id: TermuxRequestId,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub error: Option<String>,
}

impl TermuxExecResult {
    pub fn into_command_output(self) -> CommandOutput {
        CommandOutput {
            status_code: self.exit_code,
            stdout: self.stdout,
            stderr: self.stderr,
        }
    }

    /// True if the command ran to completion with exit code 0 and no bridge error.
    pub fn is_ok(&self) -> bool {
        self.error.is_none() && !self.timed_out && self.exit_code == Some(0)
    }
}

// ── Disabled executor (fallback) ──

#[derive(Default)]
pub struct DisabledExecutor;

impl Executor for DisabledExecutor {
    fn name(&self) -> &str {
        "disabled"
    }

    fn execute(&self, request: CommandRequest) -> Result<CommandOutput> {
        Ok(CommandOutput {
            status_code: None,
            stdout: format!(
                "Command execution is not enabled yet. Requested: {} {}",
                request.program,
                request.args.join(" ")
            ),
            stderr: String::new(),
        })
    }
}

// ── PC gateway planned executor ──

#[derive(Clone, Debug)]
pub struct PcGatewayExecutorPlan {
    pub gateway_id: String,
    pub workspace_id: String,
    pub request: CommandRequest,
    pub environment_id: Option<String>,
    pub requires_remote_runtime: bool,
}

impl PcGatewayExecutorPlan {
    pub fn new(
        gateway_id: impl Into<String>,
        workspace_id: impl Into<String>,
        request: CommandRequest,
    ) -> Self {
        Self {
            gateway_id: gateway_id.into(),
            workspace_id: workspace_id.into(),
            request,
            environment_id: None,
            requires_remote_runtime: true,
        }
    }

    pub fn with_environment(mut self, environment_id: impl Into<String>) -> Self {
        self.environment_id = Some(environment_id.into());
        self
    }
}

pub struct PcGatewayPlannedExecutor {
    gateway_id: String,
    workspace_id: String,
    environment_id: Option<String>,
}

impl PcGatewayPlannedExecutor {
    pub fn new(gateway_id: impl Into<String>, workspace_id: impl Into<String>) -> Self {
        Self {
            gateway_id: gateway_id.into(),
            workspace_id: workspace_id.into(),
            environment_id: None,
        }
    }

    pub fn with_environment(mut self, environment_id: impl Into<String>) -> Self {
        self.environment_id = Some(environment_id.into());
        self
    }

    pub fn plan(&self, request: CommandRequest) -> PcGatewayExecutorPlan {
        PcGatewayExecutorPlan {
            gateway_id: self.gateway_id.clone(),
            workspace_id: self.workspace_id.clone(),
            request,
            environment_id: self.environment_id.clone(),
            requires_remote_runtime: true,
        }
    }
}

impl Executor for PcGatewayPlannedExecutor {
    fn name(&self) -> &str {
        "pc_gateway_planned"
    }

    fn execute(&self, request: CommandRequest) -> Result<CommandOutput> {
        let plan = self.plan(request);
        Err(anyhow!(
            "PC gateway execution requires async transport. Planned gateway={} workspace={} command={} {}",
            plan.gateway_id,
            plan.workspace_id,
            plan.request.program,
            plan.request.args.join(" ")
        ))
    }
}

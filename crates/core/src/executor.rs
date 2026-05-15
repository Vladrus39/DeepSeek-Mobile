//! Execution abstraction for Android, Termux and remote backends.
//!
//! Large projects should not be limited by the phone. The mobile app can route
//! commands either to a safe local executor, a Termux bridge, or a remote Y-lit
//! executor while keeping the same agent/tool interface.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandRequest {
    pub program: String,
    pub args: Vec<String>,
    pub working_dir: Option<PathBuf>,
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

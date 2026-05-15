//! Shell execution tool

use super::Tool;
use anyhow::Result;

pub struct ShellTool;

impl Tool for ShellTool {
    fn name(&self) -> &str { "shell" }
    fn description(&self) -> &str { "Execute shell commands in workspace" }
    
    fn execute(&self, args: &str) -> Result<String> {
        // Placeholder - real implementation will use tokio::process
        println!("[Tool] Shell command requested: {}", args);
        Ok(format!("[Shell] Would execute: {}. (Sandbox mode in mobile)", args))
    }
}
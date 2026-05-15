//! File operations tool

use super::Tool;
use anyhow::Result;

pub struct FileOpsTool;

impl Tool for FileOpsTool {
    fn name(&self) -> &str { "file_ops" }
    fn description(&self) -> &str { "Read, write, edit files in workspace" }
    
    fn execute(&self, args: &str) -> Result<String> {
        // Placeholder - will be implemented with real file ops
        Ok(format!("File operation: {}", args))
    }
}
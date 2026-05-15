//! DeepSeek Agent with tools

use crate::api_client::{DeepSeekClient, Message};
use crate::config::Config;
use crate::tools::ToolRegistry;
use anyhow::Result;
use std::sync::Arc;

pub struct DeepSeekAgent {
    config: Config,
    tools: Arc<ToolRegistry>,
    client: DeepSeekClient,
}

impl DeepSeekAgent {
    pub fn new(config: Config) -> Self {
        let mut tools = ToolRegistry::new();
        tools.register(Box::new(crate::tools::file_ops::ReadFileTool));
        tools.register(Box::new(crate::tools::file_ops::WriteFileTool));
        tools.register(Box::new(crate::tools::file_ops::EditFileTool));
        tools.register(Box::new(crate::tools::file_ops::ListDirTool));
        tools.register(Box::new(crate::tools::file_ops::FileOpsTool));
        tools.register(Box::new(crate::tools::shell::ShellTool));
        tools.register(Box::new(crate::tools::git::GitTool));

        let client = DeepSeekClient::new(config.api_key.clone());

        Self {
            config,
            tools: Arc::new(tools),
            client,
        }
    }

    pub async fn run(&self, input: String) -> Result<String> {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: self.system_prompt(),
            },
            Message {
                role: "user".to_string(),
                content: input,
            },
        ];

        self.run_with_messages(messages).await
    }

    pub async fn run_with_messages(&self, mut messages: Vec<Message>) -> Result<String> {
        if !messages.iter().any(|message| message.role == "system") {
            messages.insert(
                0,
                Message {
                    role: "system".to_string(),
                    content: self.system_prompt(),
                },
            );
        }

        match self.client.chat(&self.config.model, messages).await {
            Ok(response) => Ok(response),
            Err(error) => Ok(format!("[Agent] Error: {}", error)),
        }
    }

    fn system_prompt(&self) -> String {
        let tool_names = self.tools.names().join(", ");
        format!(
            "You are DeepSeek Mobile, an Android-first coding agent. Available tools: {}. For now, explain planned file, shell or git actions before execution. File writes, shell commands and git actions require user approval.",
            tool_names
        )
    }
}
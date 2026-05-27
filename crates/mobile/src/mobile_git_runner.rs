use crate::git_state::{GitPanelAction, GitUiState};
use crate::mobile_runtime_config::MobileRuntimeConfig;
use deepseek_mobile_core::tool_call::ToolCallRequest;
use deepseek_mobile_core::tools::{default_mobile_tool_registry, ToolResult};
use deepseek_mobile_core::{
    ExecutorKind, PcGatewayClient, ToolCallSource, ToolContext, ToolExecutionCoordinator, Workspace,
};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub struct MobileGitActionResult {
    pub action: GitPanelAction,
    pub output: String,
    pub status_after: Option<String>,
    pub diff_after: Option<String>,
    pub branches_after: Option<String>,
}

pub async fn run_mobile_git_action(
    action: GitPanelAction,
    runtime: MobileRuntimeConfig,
    state: GitUiState,
) -> anyhow::Result<MobileGitActionResult> {
    let args = git_args_for_action(&action, &state)?;
    let result = execute_git(&runtime, args).await?;
    if !result.success {
        return Err(anyhow::anyhow!(result.content));
    }

    let mut action_result = MobileGitActionResult {
        action: action.clone(),
        output: result.content,
        status_after: None,
        diff_after: None,
        branches_after: None,
    };

    if matches!(
        action,
        GitPanelAction::Commit | GitPanelAction::Push | GitPanelAction::Pull
    ) {
        action_result.status_after = Some(successful_git_content(
            execute_git(&runtime, json!({"operation": "status"})).await?,
        )?);
        action_result.diff_after = Some(successful_git_content(
            execute_git(&runtime, json!({"operation": "diff"})).await?,
        )?);
        action_result.branches_after = Some(successful_git_content(
            execute_git(&runtime, json!({"operation": "branch"})).await?,
        )?);
    }

    Ok(action_result)
}

pub fn apply_git_action_result(state: &mut GitUiState, result: MobileGitActionResult) {
    match result.action {
        GitPanelAction::RefreshStatus => state.apply_status(result.output),
        GitPanelAction::RefreshDiff => state.apply_diff(result.output),
        GitPanelAction::ListBranches => state.apply_branch(result.output),
        GitPanelAction::Commit => state.apply_commit(result.output),
        GitPanelAction::Push | GitPanelAction::Pull => state.apply_command_output(result.output),
    }

    if let Some(status) = result.status_after {
        state.apply_status(status);
    }
    if let Some(diff) = result.diff_after {
        state.apply_diff(diff);
    }
    if let Some(branches) = result.branches_after {
        state.apply_branch(branches);
    }
}

fn git_args_for_action(action: &GitPanelAction, state: &GitUiState) -> anyhow::Result<Value> {
    match action {
        GitPanelAction::RefreshStatus => Ok(json!({"operation": "status"})),
        GitPanelAction::RefreshDiff => Ok(json!({"operation": "diff"})),
        GitPanelAction::ListBranches => Ok(json!({"operation": "branch"})),
        GitPanelAction::Commit => {
            let message = state.commit_message.trim();
            if message.is_empty() {
                return Err(anyhow::anyhow!("commit message is required"));
            }
            Ok(json!({"operation": "commit", "message": message, "files": ["."]}))
        }
        GitPanelAction::Push => {
            let mut args = json!({
                "operation": "push",
                "remote": state.remote_or_default(),
            });
            if let Some(branch) = state.branch_for_remote_action() {
                args["branch"] = json!(branch);
            }
            Ok(args)
        }
        GitPanelAction::Pull => {
            let mut args = json!({
                "operation": "pull",
                "remote": state.remote_or_default(),
            });
            if let Some(branch) = state.branch_for_remote_action() {
                args["branch"] = json!(branch);
            }
            Ok(args)
        }
    }
}

async fn execute_git(runtime: &MobileRuntimeConfig, args: Value) -> anyhow::Result<ToolResult> {
    let registry = default_mobile_tool_registry();
    let (workspace, pc_gateway) = runtime_workspace(runtime);
    let context = ToolContext::new(workspace);
    let call = ToolCallRequest::new("git", args, ToolCallSource::Manual);
    let mut coordinator = ToolExecutionCoordinator::new(&registry);
    if let Some(client) = pc_gateway.as_ref() {
        coordinator = coordinator.with_pc_gateway(client);
    }
    coordinator.execute(&call, &context).await
}

fn runtime_workspace(runtime: &MobileRuntimeConfig) -> (Workspace, Option<PcGatewayClient>) {
    if let Some(connection) = runtime.workspace_connection.as_ref() {
        let client = connection.pc_gateway.clone().map(PcGatewayClient::new);
        return (connection.to_workspace(), client);
    }

    (
        Workspace::new(
            "mobile-workspace",
            "Mobile Workspace",
            runtime.workspace_root.clone(),
            ExecutorKind::LocalAndroid,
        ),
        None,
    )
}

fn successful_git_content(result: ToolResult) -> anyhow::Result<String> {
    if result.success {
        Ok(result.content)
    } else {
        Err(anyhow::anyhow!(result.content))
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_git_action_result, run_mobile_git_action};
    use crate::git_state::{GitPanelAction, GitUiState};
    use crate::mobile_runtime_config::MobileRuntimeConfig;
    use std::fs;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn status_action_uses_existing_git_tool_route() {
        let root = temp_git_repo("status");
        fs::write(root.join("README.md"), "# Test").unwrap();
        let runtime = MobileRuntimeConfig::new("thread", root.join(".runtime"), root.clone());

        let result = run_mobile_git_action(
            GitPanelAction::RefreshStatus,
            runtime,
            GitUiState::default(),
        )
        .await
        .unwrap();

        let mut state = GitUiState::default();
        apply_git_action_result(&mut state, result);
        assert!(state.status_text.contains("README.md"));
        assert_eq!(state.changed_files, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[tokio::test]
    async fn commit_action_stages_dirty_workspace_changes() {
        let root = temp_git_repo("commit");
        configure_git_identity(&root);
        fs::write(root.join("README.md"), "# Test").unwrap();
        let runtime = MobileRuntimeConfig::new("thread", root.join(".runtime"), root.clone());
        let mut state = GitUiState::default();
        state.apply_status("?? README.md\n");
        state.set_commit_message("initial commit");

        let result = run_mobile_git_action(GitPanelAction::Commit, runtime, state)
            .await
            .unwrap();

        assert!(result.output.contains("initial commit"));
        assert_eq!(result.status_after.as_deref(), Some(""));

        let _ = fs::remove_dir_all(root);
    }

    fn temp_git_repo(label: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "deepseek-mobile-git-runner-{}-{}-{}",
            label,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(&root)
            .output()
            .unwrap();
        root
    }

    fn configure_git_identity(root: &std::path::Path) {
        Command::new("git")
            .args(["config", "user.email", "test@deepseek.mobile"])
            .current_dir(root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "DeepSeek Mobile Test"])
            .current_dir(root)
            .output()
            .unwrap();
    }
}

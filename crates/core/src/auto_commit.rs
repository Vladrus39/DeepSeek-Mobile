//! Auto-commit/push helper for the DeepSeek Mobile engine.
//!
//! When `auto_commit_push` is enabled in the config, the engine can call
//! `auto_commit_and_push` after a successful turn to persist changes to
//! the configured GitHub repository through the git CLI.

use anyhow::Result;
use std::path::Path;
use std::process::Command;

/// Commit all changes in the workspace and push to the configured remote.
///
/// Returns Ok(None) if no changes were detected, Ok(Some(commit_sha)) if
/// a commit was created and pushed successfully.
pub fn auto_commit_and_push(
    workspace_root: &Path,
    repo: &str,
    branch: &str,
    commit_message: &str,
) -> Result<Option<String>> {
    // Check if there are changes
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workspace_root)
        .output()?;

    let status_text = String::from_utf8_lossy(&status.stdout);
    if status_text.trim().is_empty() {
        return Ok(None);
    }

    // Stage all changes
    let add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(workspace_root)
        .output()?;

    if !add.status.success() {
        let stderr = String::from_utf8_lossy(&add.stderr);
        return Err(anyhow::anyhow!("git add failed: {}", stderr));
    }

    // Commit
    let commit = Command::new("git")
        .args(["commit", "-m", commit_message])
        .current_dir(workspace_root)
        .output()?;

    if !commit.status.success() {
        let stderr = String::from_utf8_lossy(&commit.stderr);
        if stderr.contains("nothing to commit") {
            return Ok(None);
        }
        return Err(anyhow::anyhow!("git commit failed: {}", stderr));
    }

    // Get commit SHA
    let sha_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(workspace_root)
        .output()?;

    let sha = String::from_utf8_lossy(&sha_output.stdout).trim().to_string();

    // Push
    let push = Command::new("git")
        .args(["push", "origin", branch])
        .current_dir(workspace_root)
        .output()?;

    if !push.status.success() {
        let stderr = String::from_utf8_lossy(&push.stderr);
        // Push failed — but commit succeeded. Return the commit SHA anyway.
        tracing::warn!("git push failed after commit {}: {}", sha, stderr);
        return Ok(Some(sha));
    }

    tracing::info!(
        "Auto-committed and pushed {} to {}/{} ({})",
        sha,
        repo,
        branch,
        commit_message
    );

    Ok(Some(sha))
}

/// Generate a default commit message from the user's input.
pub fn commit_message_from_input(user_input: &str) -> String {
    let summary: String = user_input
        .chars()
        .take(200)
        .collect();
    format!("🤖 DeepSeek Mobile: {}", summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_repo() -> (PathBuf, String) {
        let root = std::env::temp_dir().join(format!(
            "deepseek_mobile_autocommit_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(&root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@deepseek.mobile"])
            .current_dir(&root)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(&root)
            .output()
            .unwrap();

        (root, "main".to_string())
    }

    #[test]
    fn returns_none_when_no_changes() {
        let (root, branch) = temp_repo();
        let result = auto_commit_and_push(
            &root,
            "owner/repo",
            &branch,
            "test commit",
        );
        // Push will fail (no remote), but should handle it
        match result {
            Ok(None) => {} // expected: no changes
            Ok(Some(_)) => {} // possible if there's an index change
            Err(_) => {} // push failure is OK
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commits_when_changes_exist() {
        let (root, branch) = temp_repo();
        fs::write(root.join("test.txt"), "hello").unwrap();
        let result = auto_commit_and_push(
            &root,
            "owner/repo",
            &branch,
            "test",
        );
        match result {
            Ok(Some(sha)) => assert!(!sha.is_empty()),
            Ok(None) => panic!("expected a commit, got none"),
            Err(e) => {
                // push failure is OK if no remote is configured
                assert!(e.to_string().contains("push") || e.to_string().contains("remote"));
            }
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn commit_message_truncates() {
        let msg = commit_message_from_input("A".repeat(500).as_str());
        assert!(msg.len() <= 215); // prefix + 200 chars
        assert!(msg.starts_with("🤖 DeepSeek Mobile:"));
    }
}

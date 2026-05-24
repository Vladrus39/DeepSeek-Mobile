/// Git UI state for the mobile cockpit.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitPanelAction {
    RefreshStatus,
    RefreshDiff,
    ListBranches,
    Commit,
    Push,
    Pull,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GitUiState {
    pub status_text: String,
    pub diff_text: String,
    pub branch_list: String,
    pub current_branch: String,
    pub commit_message: String,
    pub remote: String,
    pub push_branch: String,
    pub loading: bool,
    pub error: Option<String>,
    pub changed_files: usize,
}

impl GitUiState {
    pub fn apply_status(&mut self, t: impl Into<String>) {
        self.status_text = t.into();
        self.changed_files = count_changed_files(&self.status_text);
        self.error = None;
        self.loading = false;
    }

    pub fn apply_diff(&mut self, t: impl Into<String>) {
        self.diff_text = t.into();
        self.error = None;
        self.loading = false;
    }

    pub fn apply_branch(&mut self, t: impl Into<String>) {
        self.branch_list = t.into();
        if let Some(branch) = current_branch_from_list(&self.branch_list) {
            self.current_branch = branch;
        }
        self.error = None;
        self.loading = false;
    }

    pub fn apply_command_output(&mut self, t: impl Into<String>) {
        self.status_text = t.into();
        self.error = None;
        self.loading = false;
    }

    pub fn apply_commit(&mut self, output: impl Into<String>) {
        self.commit_message.clear();
        self.apply_command_output(output);
    }

    pub fn set_commit_message(&mut self, message: impl Into<String>) {
        self.commit_message = message.into();
    }

    pub fn set_error(&mut self, m: impl Into<String>) {
        self.error = Some(m.into());
        self.loading = false;
    }

    pub fn set_loading(&mut self) {
        self.loading = true;
        self.error = None;
    }

    pub fn remote_or_default(&self) -> String {
        if self.remote.trim().is_empty() {
            "origin".to_string()
        } else {
            self.remote.trim().to_string()
        }
    }

    pub fn branch_for_remote_action(&self) -> Option<String> {
        let branch = if self.push_branch.trim().is_empty() {
            self.current_branch.trim()
        } else {
            self.push_branch.trim()
        };
        (!branch.is_empty()).then(|| branch.to_string())
    }

    pub fn can_commit(&self) -> bool {
        self.is_dirty() && !self.commit_message.trim().is_empty() && !self.loading
    }

    pub fn is_dirty(&self) -> bool {
        self.changed_files > 0 || !self.status_text.trim().is_empty()
    }
}

impl Default for GitUiState {
    fn default() -> Self {
        Self {
            status_text: String::new(),
            diff_text: String::new(),
            branch_list: String::new(),
            current_branch: String::new(),
            commit_message: String::new(),
            remote: "origin".to_string(),
            push_branch: String::new(),
            loading: false,
            error: None,
            changed_files: 0,
        }
    }
}

fn count_changed_files(status: &str) -> usize {
    status
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("##")
        })
        .count()
}

fn current_branch_from_list(branches: &str) -> Option<String> {
    branches.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("* ")
            .map(str::trim)
            .filter(|branch| !branch.is_empty())
            .map(String::from)
    })
}

#[cfg(test)]
mod tests {
    use super::GitUiState;

    #[test]
    fn default_remote_is_origin() {
        let state = GitUiState::default();
        assert_eq!(state.remote_or_default(), "origin");
    }

    #[test]
    fn status_counts_changed_files() {
        let mut state = GitUiState::default();
        state.apply_status(" M README.md\n?? src/main.rs\n");
        assert_eq!(state.changed_files, 2);
        assert!(state.is_dirty());
    }

    #[test]
    fn branch_list_updates_current_branch() {
        let mut state = GitUiState::default();
        state.apply_branch("  develop\n* main\n");
        assert_eq!(state.current_branch, "main");
        assert_eq!(state.branch_for_remote_action().as_deref(), Some("main"));
    }
}

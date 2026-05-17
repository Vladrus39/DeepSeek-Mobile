/// Git UI state for the mobile cockpit.

#[derive(Clone, Debug, Default, PartialEq, Eq)]
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
    pub fn apply_status(&mut self, t: impl Into<String>) { self.status_text = t.into(); self.error = None; self.loading = false; }
    pub fn apply_diff(&mut self, t: impl Into<String>) { self.diff_text = t.into(); self.error = None; self.loading = false; }
    pub fn apply_branch(&mut self, t: impl Into<String>) { self.branch_list = t.into(); self.error = None; self.loading = false; }
    pub fn set_error(&mut self, m: impl Into<String>) { self.error = Some(m.into()); self.loading = false; }
    pub fn set_loading(&mut self) { self.loading = true; self.error = None; }
    pub fn apply_commit(&mut self) { self.commit_message.clear(); self.loading = false; self.error = None; }
    pub fn is_dirty(&self) -> bool { !self.status_text.trim().is_empty() }
}

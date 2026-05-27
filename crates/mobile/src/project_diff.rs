#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectDiffLineKind {
    Context,
    Added,
    Removed,
    Hunk,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDiffLine {
    pub kind: ProjectDiffLineKind,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectDiffPreview {
    pub path: String,
    pub added_lines: usize,
    pub removed_lines: usize,
    pub lines: Vec<ProjectDiffLine>,
}

impl ProjectDiffPreview {
    pub fn empty(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            added_lines: 0,
            removed_lines: 0,
            lines: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }
}

pub fn build_text_diff_preview(
    path: impl Into<String>,
    before: &str,
    after: &str,
) -> ProjectDiffPreview {
    let path = path.into();
    if before == after {
        return ProjectDiffPreview::empty(path);
    }

    let before_lines = before.lines().collect::<Vec<_>>();
    let after_lines = after.lines().collect::<Vec<_>>();
    let mut prefix = 0;
    while prefix < before_lines.len()
        && prefix < after_lines.len()
        && before_lines[prefix] == after_lines[prefix]
    {
        prefix += 1;
    }

    let mut suffix = 0;
    while suffix + prefix < before_lines.len()
        && suffix + prefix < after_lines.len()
        && before_lines[before_lines.len() - 1 - suffix]
            == after_lines[after_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let before_end = before_lines.len().saturating_sub(suffix);
    let after_end = after_lines.len().saturating_sub(suffix);
    let context_start = prefix.saturating_sub(3);
    let context_end = (before_end + 3).min(before_lines.len());

    let mut lines = Vec::new();
    lines.push(ProjectDiffLine {
        kind: ProjectDiffLineKind::Hunk,
        text: format!(
            "@@ -{},{} +{},{} @@",
            prefix + 1,
            before_end.saturating_sub(prefix),
            prefix + 1,
            after_end.saturating_sub(prefix)
        ),
    });

    for line in &before_lines[context_start..prefix] {
        lines.push(ProjectDiffLine {
            kind: ProjectDiffLineKind::Context,
            text: format!(" {}", line),
        });
    }
    for line in &before_lines[prefix..before_end] {
        lines.push(ProjectDiffLine {
            kind: ProjectDiffLineKind::Removed,
            text: format!("-{}", line),
        });
    }
    for line in &after_lines[prefix..after_end] {
        lines.push(ProjectDiffLine {
            kind: ProjectDiffLineKind::Added,
            text: format!("+{}", line),
        });
    }
    for line in &before_lines[before_end..context_end] {
        lines.push(ProjectDiffLine {
            kind: ProjectDiffLineKind::Context,
            text: format!(" {}", line),
        });
    }

    ProjectDiffPreview {
        path,
        added_lines: after_end.saturating_sub(prefix),
        removed_lines: before_end.saturating_sub(prefix),
        lines,
    }
}

pub fn diff_line_color(kind: &ProjectDiffLineKind) -> &'static str {
    match kind {
        ProjectDiffLineKind::Context => "#d1d5db",
        ProjectDiffLineKind::Added => "#86efac",
        ProjectDiffLineKind::Removed => "#fca5a5",
        ProjectDiffLineKind::Hunk => "#93c5fd",
    }
}

#[cfg(test)]
mod tests {
    use super::{build_text_diff_preview, ProjectDiffLineKind};

    #[test]
    fn builds_small_text_diff_preview() {
        let diff = build_text_diff_preview("src/main.rs", "a\nb\nc\n", "a\nB\nc\n");
        assert_eq!(diff.added_lines, 1);
        assert_eq!(diff.removed_lines, 1);
        assert!(diff
            .lines
            .iter()
            .any(|line| line.kind == ProjectDiffLineKind::Added));
        assert!(diff
            .lines
            .iter()
            .any(|line| line.kind == ProjectDiffLineKind::Removed));
    }

    #[test]
    fn unchanged_text_returns_empty_diff() {
        let diff = build_text_diff_preview("README.md", "same", "same");
        assert!(diff.is_empty());
    }
}

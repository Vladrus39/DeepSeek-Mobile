use deepseek_mobile_core::{TermuxExecRequest, TermuxExecResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidTermuxCommand {
    pub request_id: String,
    pub command_path: String,
    pub arguments: Vec<String>,
    pub working_dir: String,
    pub background: bool,
    pub timeout_secs: Option<u64>,
}

impl AndroidTermuxCommand {
    pub fn from_request(request: &TermuxExecRequest) -> Self {
        Self {
            request_id: request.request_id.clone(),
            command_path: "/data/data/com.termux/files/usr/bin/sh".to_string(),
            arguments: vec!["-lc".to_string(), request.command.clone()],
            working_dir: request.working_dir.display().to_string(),
            background: true,
            timeout_secs: request.timeout_secs,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidTermuxCallback {
    Completed(TermuxExecResult),
    Failed { request_id: String, message: String },
}

impl AndroidTermuxCallback {
    pub fn request_id(&self) -> &str {
        match self {
            Self::Completed(result) => &result.request_id,
            Self::Failed { request_id, .. } => request_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AndroidTermuxCallback, AndroidTermuxCommand};
    use deepseek_mobile_core::{TermuxExecRequest, TermuxExecResult};
    use std::path::PathBuf;

    #[test]
    fn termux_command_wraps_shell_execution_for_run_command_intent() {
        let command = AndroidTermuxCommand::from_request(&TermuxExecRequest {
            request_id: "termux-1".to_string(),
            command: "cargo test".to_string(),
            working_dir: PathBuf::from("/data/data/com.termux/files/home/project"),
            timeout_secs: Some(30),
        });

        assert_eq!(command.request_id, "termux-1");
        assert_eq!(
            command.command_path,
            "/data/data/com.termux/files/usr/bin/sh"
        );
        assert_eq!(command.arguments, vec!["-lc", "cargo test"]);
        assert!(command.background);
    }

    #[test]
    fn callback_exposes_request_id_for_correlation() {
        let callback = AndroidTermuxCallback::Completed(TermuxExecResult {
            request_id: "termux-2".to_string(),
            stdout: "ok".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            timed_out: false,
            error: None,
        });
        assert_eq!(callback.request_id(), "termux-2");
    }
}

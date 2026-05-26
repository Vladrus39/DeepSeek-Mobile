//! First-login setup state and single-shot completion.

use crate::dev_bootstrap::prefill_api_key_for_onboarding;
use crate::settings_state::{load_saved_config, save_config, SettingsFormState};
use crate::termux_state::TermuxWorkspaceState;
use deepseek_mobile_core::config::ExecutionMode;

pub const DEFAULT_TERMUX_PROJECT_PATH: &str = "/data/data/com.termux/files/home/deepseek-project";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupSnapshot {
    pub api_ok: bool,
    pub agent_mode_ok: bool,
    pub termux_ok: bool,
    pub full_agent_ready: bool,
}

impl SetupSnapshot {
    pub fn collect(settings: &SettingsFormState, termux: &TermuxWorkspaceState) -> Self {
        let api_ok = settings.api_key.trim().starts_with("sk-");
        let agent_mode_ok = settings.execution_mode == ExecutionMode::Agent;
        let termux_ok = termux.is_valid() && termux.saved;
        let full_agent_ready = api_ok && agent_mode_ok && termux_ok;
        Self {
            api_ok,
            agent_mode_ok,
            termux_ok,
            full_agent_ready,
        }
    }

    pub fn should_show_setup(settings: &SettingsFormState, termux: &TermuxWorkspaceState) -> bool {
        !Self::collect(settings, termux).full_agent_ready
            && !load_saved_config()
                .map(|c| c.api_key.trim().starts_with("sk-") && termux.saved)
                .unwrap_or(false)
    }
}

/// Save API key, Agent mode, and Termux path in one step (first login).
pub fn complete_first_login(
    settings: &mut SettingsFormState,
    termux: &mut TermuxWorkspaceState,
    api_key: &str,
    termux_path: &str,
    sandbox_only: bool,
) -> Result<(), String> {
    let key = api_key.trim();
    if !key.starts_with("sk-") {
        return Err("api_key_prefix".to_string());
    }
    settings.api_key = key.to_string();
    settings.execution_mode = ExecutionMode::Agent;

    if !sandbox_only {
        termux.set_path(termux_path.trim());
        termux.set_label("Termux Project");
        if !termux.is_valid() {
            return Err(termux
                .validation_error
                .clone()
                .unwrap_or_else(|| "invalid_termux_path".to_string()));
        }
        termux.save();
    }

    save_config(&settings.to_config())?;
    settings.saved = true;
    settings.save_error = None;
    Ok(())
}

pub fn initial_api_key_draft() -> String {
    if let Some(config) = load_saved_config() {
        let key = config.api_key.trim();
        if key.starts_with("sk-") {
            return key.to_string();
        }
    }
    prefill_api_key_for_onboarding()
}

pub fn initial_termux_path_draft(termux: &TermuxWorkspaceState) -> String {
    if termux.saved && !termux.workspace_path.trim().is_empty() {
        termux.workspace_path.clone()
    } else {
        DEFAULT_TERMUX_PROJECT_PATH.to_string()
    }
}

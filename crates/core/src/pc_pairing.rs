//! PC host pairing bundle contract.
//!
//! The Android app should be able to generate a small one-click setup bundle for
//! a trusted PC. The user opens the generated script on the PC, it starts
//! `deepseek-pc-host` with the correct pairing token/workspace settings, and the
//! phone can then connect to the background runtime host.

use crate::pc_gateway::PcGatewayTransportMode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcPairingPlatform {
    WindowsPowerShell,
    LinuxShell,
    MacOsShell,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcPairingLaunchScript {
    pub platform: PcPairingPlatform,
    pub file_name: String,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayPairingBundle {
    pub schema_version: u32,
    pub gateway_id: String,
    pub gateway_label: String,
    pub device_id: String,
    pub device_label: String,
    pub workspace_id: String,
    pub workspace_root: String,
    pub bind_addr: String,
    pub expected_base_url: Option<String>,
    pub auth_token: String,
    pub transport_mode: PcGatewayTransportMode,
    pub expires_at_unix: Option<u64>,
    pub auto_start: bool,
}

impl PcGatewayPairingBundle {
    pub fn local_http(
        gateway_id: impl Into<String>,
        gateway_label: impl Into<String>,
        device_id: impl Into<String>,
        device_label: impl Into<String>,
        workspace_id: impl Into<String>,
        workspace_root: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: 1,
            gateway_id: gateway_id.into(),
            gateway_label: gateway_label.into(),
            device_id: device_id.into(),
            device_label: device_label.into(),
            workspace_id: workspace_id.into(),
            workspace_root: workspace_root.into(),
            bind_addr: "0.0.0.0:8787".to_string(),
            expected_base_url: None,
            auth_token: auth_token.into(),
            transport_mode: PcGatewayTransportMode::LocalNetworkHttp,
            expires_at_unix: None,
            auto_start: true,
        }
    }

    pub fn with_bind_addr(mut self, bind_addr: impl Into<String>) -> Self {
        self.bind_addr = bind_addr.into();
        self
    }

    pub fn with_expected_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.expected_base_url = Some(base_url.into());
        self
    }

    pub fn with_expiry(mut self, expires_at_unix: u64) -> Self {
        self.expires_at_unix = Some(expires_at_unix);
        self
    }

    pub fn with_auto_start(mut self, auto_start: bool) -> Self {
        self.auto_start = auto_start;
        self
    }

    pub fn pairing_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn env_file(&self) -> String {
        format!(
            "DEEPSEEK_PC_HOST_BIND={}\nDEEPSEEK_PC_HOST_ID={}\nDEEPSEEK_PC_HOST_LABEL={}\nDEEPSEEK_PC_HOST_WORKSPACE={}\nDEEPSEEK_PC_HOST_WORKSPACE_ID={}\nDEEPSEEK_PC_HOST_TOKEN={}\nDEEPSEEK_PC_HOST_DEVICE_ID={}\nDEEPSEEK_PC_HOST_DEVICE_LABEL={}\n",
            self.bind_addr,
            self.gateway_id,
            self.gateway_label,
            self.workspace_root,
            self.workspace_id,
            self.auth_token,
            self.device_id,
            self.device_label,
        )
    }

    pub fn launch_scripts(&self) -> Vec<PcPairingLaunchScript> {
        vec![
            PcPairingLaunchScript {
                platform: PcPairingPlatform::WindowsPowerShell,
                file_name: "start-deepseek-pc-host.ps1".to_string(),
                content: self.windows_powershell_script(),
            },
            PcPairingLaunchScript {
                platform: PcPairingPlatform::LinuxShell,
                file_name: "start-deepseek-pc-host.sh".to_string(),
                content: self.posix_shell_script(),
            },
            PcPairingLaunchScript {
                platform: PcPairingPlatform::MacOsShell,
                file_name: "start-deepseek-pc-host.command".to_string(),
                content: self.posix_shell_script(),
            },
        ]
    }

    pub fn windows_powershell_script(&self) -> String {
        format!(
            r#"$ErrorActionPreference = "Stop"
$env:DEEPSEEK_PC_HOST_BIND = "{bind_addr}"
$env:DEEPSEEK_PC_HOST_ID = "{gateway_id}"
$env:DEEPSEEK_PC_HOST_LABEL = "{gateway_label}"
$env:DEEPSEEK_PC_HOST_WORKSPACE = "{workspace_root}"
$env:DEEPSEEK_PC_HOST_WORKSPACE_ID = "{workspace_id}"
$env:DEEPSEEK_PC_HOST_TOKEN = "{auth_token}"
$env:DEEPSEEK_PC_HOST_DEVICE_ID = "{device_id}"
$env:DEEPSEEK_PC_HOST_DEVICE_LABEL = "{device_label}"
Write-Host "Starting DeepSeek PC Host for workspace: $env:DEEPSEEK_PC_HOST_WORKSPACE"
Write-Host "Keep this window open while the Android app is connected."
deepseek-pc-host
"#,
            bind_addr = escape_powershell(&self.bind_addr),
            gateway_id = escape_powershell(&self.gateway_id),
            gateway_label = escape_powershell(&self.gateway_label),
            workspace_root = escape_powershell(&self.workspace_root),
            workspace_id = escape_powershell(&self.workspace_id),
            auth_token = escape_powershell(&self.auth_token),
            device_id = escape_powershell(&self.device_id),
            device_label = escape_powershell(&self.device_label),
        )
    }

    pub fn posix_shell_script(&self) -> String {
        format!(
            r#"#!/usr/bin/env sh
set -eu
export DEEPSEEK_PC_HOST_BIND={bind_addr}
export DEEPSEEK_PC_HOST_ID={gateway_id}
export DEEPSEEK_PC_HOST_LABEL={gateway_label}
export DEEPSEEK_PC_HOST_WORKSPACE={workspace_root}
export DEEPSEEK_PC_HOST_WORKSPACE_ID={workspace_id}
export DEEPSEEK_PC_HOST_TOKEN={auth_token}
export DEEPSEEK_PC_HOST_DEVICE_ID={device_id}
export DEEPSEEK_PC_HOST_DEVICE_LABEL={device_label}
echo "Starting DeepSeek PC Host for workspace: $DEEPSEEK_PC_HOST_WORKSPACE"
echo "Keep this terminal open while the Android app is connected."
exec deepseek-pc-host
"#,
            bind_addr = shell_quote(&self.bind_addr),
            gateway_id = shell_quote(&self.gateway_id),
            gateway_label = shell_quote(&self.gateway_label),
            workspace_root = shell_quote(&self.workspace_root),
            workspace_id = shell_quote(&self.workspace_id),
            auth_token = shell_quote(&self.auth_token),
            device_id = shell_quote(&self.device_id),
            device_label = shell_quote(&self.device_label),
        )
    }
}

fn escape_powershell(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::PcGatewayPairingBundle;

    #[test]
    fn bundle_generates_env_file() {
        let bundle = PcGatewayPairingBundle::local_http(
            "pc-local",
            "Developer PC",
            "phone-1",
            "Android Phone",
            "local",
            "/work/project",
            "secret-token",
        );
        let env_file = bundle.env_file();
        assert!(env_file.contains("DEEPSEEK_PC_HOST_TOKEN=secret-token"));
        assert!(env_file.contains("DEEPSEEK_PC_HOST_WORKSPACE=/work/project"));
    }

    #[test]
    fn bundle_generates_pairing_json() {
        let bundle = PcGatewayPairingBundle::local_http(
            "pc-local",
            "Developer PC",
            "phone-1",
            "Android Phone",
            "local",
            "/work/project",
            "secret-token",
        );
        let json = bundle.pairing_json().unwrap();
        assert!(json.contains("pc-local"));
        assert!(json.contains("secret-token"));
    }

    #[test]
    fn bundle_generates_all_launch_scripts() {
        let bundle = PcGatewayPairingBundle::local_http(
            "pc-local",
            "Developer PC",
            "phone-1",
            "Android Phone",
            "local",
            "/work/project",
            "secret-token",
        );
        let scripts = bundle.launch_scripts();
        assert_eq!(scripts.len(), 3);
        assert!(scripts.iter().any(|script| script.file_name.ends_with(".ps1")));
        assert!(scripts.iter().any(|script| script.file_name.ends_with(".sh")));
        assert!(scripts.iter().any(|script| script.file_name.ends_with(".command")));
    }
}

use anyhow::{Context, Result};
use deepseek_mobile_core::{
    discover_pc_host_binaries, PcGatewayPairingBundle, PcGatewayTransportMode,
};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobilePcPairingRequest {
    pub gateway_id: String,
    pub gateway_label: String,
    pub device_id: String,
    pub device_label: String,
    pub workspace_id: String,
    pub workspace_root: String,
    pub auth_token: String,
    pub bind_addr: String,
    pub expected_base_url: Option<String>,
    pub expires_at_unix: Option<u64>,
    pub auto_start: bool,
    pub trusted_paths: Vec<String>,
}

impl MobilePcPairingRequest {
    pub fn new(
        gateway_id: impl Into<String>,
        gateway_label: impl Into<String>,
        device_id: impl Into<String>,
        device_label: impl Into<String>,
        workspace_id: impl Into<String>,
        workspace_root: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            gateway_id: gateway_id.into(),
            gateway_label: gateway_label.into(),
            device_id: device_id.into(),
            device_label: device_label.into(),
            workspace_id: workspace_id.into(),
            workspace_root: workspace_root.into(),
            auth_token: auth_token.into(),
            bind_addr: "0.0.0.0:8787".to_string(),
            expected_base_url: None,
            expires_at_unix: None,
            auto_start: true,
            trusted_paths: Vec::new(),
        }
    }

    pub fn with_trusted_paths(mut self, paths: Vec<String>) -> Self {
        self.trusted_paths = paths;
        self
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MobilePcPairingExport {
    pub zip_path: PathBuf,
    pub gateway_id: String,
    pub gateway_label: String,
    pub workspace_id: String,
    pub workspace_root: String,
    pub bind_addr: String,
    pub expected_base_url: Option<String>,
    pub transport_mode: PcGatewayTransportMode,
}

pub struct PcPairingManager;

impl PcPairingManager {
    pub fn build_bundle(request: MobilePcPairingRequest) -> PcGatewayPairingBundle {
        let mut bundle = PcGatewayPairingBundle::local_http(
            request.gateway_id,
            request.gateway_label,
            request.device_id,
            request.device_label,
            request.workspace_id,
            request.workspace_root,
            request.auth_token,
        )
        .with_bind_addr(request.bind_addr)
        .with_auto_start(request.auto_start);

        if let Some(base_url) = request.expected_base_url {
            bundle = bundle.with_expected_base_url(base_url);
        }
        if let Some(expires_at_unix) = request.expires_at_unix {
            bundle = bundle.with_expiry(expires_at_unix);
        }
        if !request.trusted_paths.is_empty() {
            bundle = bundle.with_trusted_paths(request.trusted_paths);
        }
        bundle
    }

    pub fn export_zip(
        request: MobilePcPairingRequest,
        output_dir: impl AsRef<Path>,
    ) -> Result<MobilePcPairingExport> {
        let bundle = Self::build_bundle(request);
        let output_dir = output_dir.as_ref();
        let safe_gateway_id = sanitize_file_component(&bundle.gateway_id);
        let zip_name = format!("deepseek-pc-host-pairing-{}.zip", safe_gateway_id);
        let zip_path = output_dir.join(zip_name);
        let host_bins = discover_pc_host_binaries(&[output_dir.to_path_buf()]);
        bundle
            .write_zip_with_host_binaries(&zip_path, Some(&host_bins))
            .with_context(|| format!("export PC pairing zip to {}", zip_path.display()))?;

        Ok(MobilePcPairingExport {
            zip_path,
            gateway_id: bundle.gateway_id,
            gateway_label: bundle.gateway_label,
            workspace_id: bundle.workspace_id,
            workspace_root: bundle.workspace_root,
            bind_addr: bundle.bind_addr,
            expected_base_url: bundle.expected_base_url,
            transport_mode: bundle.transport_mode,
        })
    }
}

fn sanitize_file_component(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "pc".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{MobilePcPairingRequest, PcPairingManager};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builds_pairing_bundle_from_mobile_request() {
        let request = sample_request();
        let bundle = PcPairingManager::build_bundle(request);
        assert_eq!(bundle.gateway_id, "pc-local");
        assert_eq!(bundle.workspace_id, "local");
        assert_eq!(bundle.bind_addr, "0.0.0.0:8787");
    }

    #[test]
    fn exports_pairing_zip() {
        let output_dir = temp_dir();
        let export = PcPairingManager::export_zip(sample_request(), &output_dir).unwrap();
        assert!(export.zip_path.exists());
        assert!(export
            .zip_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with(".zip"));
        let _ = fs::remove_dir_all(output_dir);
    }

    fn sample_request() -> MobilePcPairingRequest {
        MobilePcPairingRequest::new(
            "pc-local",
            "Developer PC",
            "phone-1",
            "Android Phone",
            "local",
            "/work/project",
            "secret-token",
        )
    }

    fn temp_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "deepseek-mobile-pairing-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}

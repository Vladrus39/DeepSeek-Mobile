//! PC host pairing bundle contract.
//!
//! The Android app should be able to generate a small one-click setup bundle for
//! a trusted PC. The user opens the generated script on the PC, it starts
//! `deepseek-pc-host` with the correct pairing token/workspace settings, and the
//! phone can then connect to the background runtime host.

use crate::pc_gateway::PcGatewayTransportMode;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

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
pub struct PcPairingBundleFile {
    pub relative_path: String,
    pub content: String,
    pub executable: bool,
}

/// Optional `deepseek-pc-host` binaries to embed in a pairing ZIP.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PcHostBinaryBundle {
    pub windows_exe: Option<PathBuf>,
    pub unix_bin: Option<PathBuf>,
}

impl PcHostBinaryBundle {
    pub fn is_empty(&self) -> bool {
        self.windows_exe.is_none() && self.unix_bin.is_none()
    }

    pub fn has_any(&self) -> bool {
        !self.is_empty()
    }
}

/// Search common repo/release locations for host binaries.
pub fn discover_pc_host_binaries(extra_roots: &[PathBuf]) -> PcHostBinaryBundle {
    let mut roots: Vec<PathBuf> = extra_roots.to_vec();
    if let Ok(cwd) = std::env::current_dir() {
        if !roots.iter().any(|root| root == &cwd) {
            roots.push(cwd);
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        if let Some(workspace_root) = find_workspace_root(&PathBuf::from(manifest)) {
            if !roots.iter().any(|root| root == &workspace_root) {
                roots.push(workspace_root);
            }
        }
    }

    let mut windows_exe = None;
    let mut unix_bin = None;
    for root in roots {
        if windows_exe.is_none() {
            windows_exe = windows_host_candidates(&root)
                .into_iter()
                .find(|path| path.is_file());
        }
        if unix_bin.is_none() {
            unix_bin = unix_host_candidates(&root)
                .into_iter()
                .find(|path| path.is_file());
        }
        if windows_exe.is_some() && unix_bin.is_some() {
            break;
        }
    }

    PcHostBinaryBundle {
        windows_exe,
        unix_bin,
    }
}

fn windows_host_candidates(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join("tools/pc-host/bin/windows-x86_64/deepseek-pc-host.exe"),
        root.join("tools/pc-host/bin/deepseek-pc-host.exe"),
        root.join("target/release/deepseek-pc-host.exe"),
    ]
}

fn find_workspace_root(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join("Cargo.toml").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn unix_host_candidates(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join("tools/pc-host/bin/linux-x86_64/deepseek-pc-host"),
        root.join("tools/pc-host/bin/deepseek-pc-host"),
        root.join("target/release/deepseek-pc-host"),
    ]
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
    /// Extra absolute paths the PC host may access (synced from phone Settings).
    #[serde(default)]
    pub trusted_paths: Vec<String>,
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

    pub fn pairing_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn env_file(&self) -> String {
        let mut out = format!(
            "DEEPSEEK_PC_HOST_BIND={}\nDEEPSEEK_PC_HOST_ID={}\nDEEPSEEK_PC_HOST_LABEL={}\nDEEPSEEK_PC_HOST_WORKSPACE={}\nDEEPSEEK_PC_HOST_WORKSPACE_ID={}\nDEEPSEEK_PC_HOST_TOKEN={}\nDEEPSEEK_PC_HOST_DEVICE_ID={}\nDEEPSEEK_PC_HOST_DEVICE_LABEL={}\n",
            self.bind_addr,
            self.gateway_id,
            self.gateway_label,
            self.workspace_root,
            self.workspace_id,
            self.auth_token,
            self.device_id,
            self.device_label,
        );
        if !self.trusted_paths.is_empty() {
            out.push_str(&format!(
                "DEEPSEEK_PC_HOST_TRUSTED_PATHS={}\n",
                self.trusted_paths.join("|")
            ));
        }
        out
    }

    pub fn readme(&self, host_binaries_included: bool) -> String {
        let host_step = if host_binaries_included {
            "1. Unzip the bundle. deepseek-pc-host is already included when the app could find a build.\n\
2. Open the launcher for your operating system:"
        } else {
            "1. Put deepseek-pc-host.exe (Windows) or deepseek-pc-host (Linux/macOS) next to this bundle, or install on PATH.\n\
2. Open the launcher for your operating system:"
        };
        format!(
            "DeepSeek PC Host pairing bundle\n\n\
This folder was generated by the Android app for one-click PC pairing.\n\n\
{host_step}\n\
   - Windows: start-deepseek-pc-host.ps1\n\
   - Linux:   start-deepseek-pc-host.sh\n\
   - macOS:   start-deepseek-pc-host.command\n\
3. Keep the terminal window open while the Android app is connected.\n\
   Optional: run scripts/install-pc-host-from-pairing.* to register autostart.\n\n\
Gateway: {gateway_label} ({gateway_id})\n\
Workspace: {workspace_root}\n\
Bind address: {bind_addr}\n\n\
Security note: this bundle contains a pairing token. Do not share it with untrusted people.\n",
            gateway_label = self.gateway_label,
            gateway_id = self.gateway_id,
            workspace_root = self.workspace_root,
            bind_addr = self.bind_addr,
        )
    }

    pub fn bundle_files(&self, host_binaries: Option<&PcHostBinaryBundle>) -> Result<Vec<PcPairingBundleFile>> {
        let host_included = host_binaries.map(PcHostBinaryBundle::has_any).unwrap_or(false);
        let mut files = vec![
            PcPairingBundleFile {
                relative_path: "pairing.json".to_string(),
                content: self.pairing_json().context("serialize pairing.json")?,
                executable: false,
            },
            PcPairingBundleFile {
                relative_path: "deepseek-pc-host.env".to_string(),
                content: self.env_file(),
                executable: false,
            },
            PcPairingBundleFile {
                relative_path: "README.txt".to_string(),
                content: self.readme(host_included),
                executable: false,
            },
        ];

        files.extend(self.launch_scripts().into_iter().map(|script| PcPairingBundleFile {
            relative_path: script.file_name,
            content: script.content,
            executable: matches!(script.platform, PcPairingPlatform::LinuxShell | PcPairingPlatform::MacOsShell),
        }));

        Ok(files)
    }

    pub fn write_directory(&self, output_dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        self.write_directory_with_host_binaries(output_dir, None)
    }

    pub fn write_directory_with_host_binaries(
        &self,
        output_dir: impl AsRef<Path>,
        host_binaries: Option<&PcHostBinaryBundle>,
    ) -> Result<Vec<PathBuf>> {
        let output_dir = output_dir.as_ref();
        fs::create_dir_all(output_dir)
            .with_context(|| format!("create pairing bundle directory {}", output_dir.display()))?;

        let mut written = Vec::new();
        for file in self.bundle_files(host_binaries)? {
            let path = output_dir.join(&file.relative_path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("create pairing bundle parent {}", parent.display()))?;
            }
            fs::write(&path, file.content)
                .with_context(|| format!("write pairing bundle file {}", path.display()))?;
            set_executable_if_needed(&path, file.executable)?;
            written.push(path);
        }
        if let Some(hosts) = host_binaries {
            if let Some(windows) = hosts.windows_exe.as_ref() {
                let path = output_dir.join("deepseek-pc-host.exe");
                fs::copy(windows, &path).with_context(|| {
                    format!(
                        "copy Windows PC host binary {} -> {}",
                        windows.display(),
                        path.display()
                    )
                })?;
                written.push(path);
            }
            if let Some(unix) = hosts.unix_bin.as_ref() {
                let path = output_dir.join("deepseek-pc-host");
                fs::copy(unix, &path).with_context(|| {
                    format!(
                        "copy Unix PC host binary {} -> {}",
                        unix.display(),
                        path.display()
                    )
                })?;
                set_executable_if_needed(&path, true)?;
                written.push(path);
            }
        }
        Ok(written)
    }

    pub fn write_zip(&self, output_zip: impl AsRef<Path>) -> Result<PathBuf> {
        self.write_zip_with_host_binaries(output_zip, None)
    }

    pub fn write_zip_with_host_binaries(
        &self,
        output_zip: impl AsRef<Path>,
        host_binaries: Option<&PcHostBinaryBundle>,
    ) -> Result<PathBuf> {
        let output_zip = output_zip.as_ref();
        if let Some(parent) = output_zip.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create pairing zip parent {}", parent.display()))?;
        }

        let file = fs::File::create(output_zip)
            .with_context(|| format!("create pairing zip {}", output_zip.display()))?;
        let mut zip = ZipWriter::new(file);

        for bundle_file in self.bundle_files(host_binaries)? {
            let unix_permissions = if bundle_file.executable { 0o755 } else { 0o644 };
            let options = FileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .unix_permissions(unix_permissions);
            zip.start_file(bundle_file.relative_path, options)
                .context("start pairing zip entry")?;
            zip.write_all(bundle_file.content.as_bytes())
                .context("write pairing zip entry")?;
        }

        if let Some(hosts) = host_binaries {
            if let Some(windows) = hosts.windows_exe.as_ref() {
                write_zip_binary_file(
                    &mut zip,
                    "deepseek-pc-host.exe",
                    windows,
                    false,
                )?;
            }
            if let Some(unix) = hosts.unix_bin.as_ref() {
                write_zip_binary_file(&mut zip, "deepseek-pc-host", unix, true)?;
            }
        }

        zip.finish().context("finish pairing zip")?;
        Ok(output_zip.to_path_buf())
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

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$LocalCandidates = @(
    (Join-Path $ScriptDir "deepseek-pc-host.exe"),
    (Join-Path $ScriptDir "deepseek-pc-host"),
    (Join-Path $ScriptDir "bin\deepseek-pc-host.exe")
)
$LocalHost = $LocalCandidates | Where-Object {{ Test-Path $_ }} | Select-Object -First 1

if ($LocalHost) {{
    & $LocalHost
}} elseif (Get-Command "deepseek-pc-host" -ErrorAction SilentlyContinue) {{
    deepseek-pc-host
}} else {{
    Write-Error "deepseek-pc-host was not found. Place deepseek-pc-host.exe next to this script, install it on PATH, or install the DeepSeek PC Host release package."
    exit 127
}}
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

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ -x "$SCRIPT_DIR/deepseek-pc-host" ]; then
  exec "$SCRIPT_DIR/deepseek-pc-host"
elif [ -x "$SCRIPT_DIR/bin/deepseek-pc-host" ]; then
  exec "$SCRIPT_DIR/bin/deepseek-pc-host"
elif command -v deepseek-pc-host >/dev/null 2>&1; then
  exec deepseek-pc-host
else
  echo "deepseek-pc-host was not found." >&2
  echo "Place deepseek-pc-host next to this script, install it on PATH, or install the DeepSeek PC Host release package." >&2
  exit 127
fi
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

#[cfg(unix)]
fn set_executable_if_needed(path: &Path, executable: bool) -> Result<()> {
    if !executable {
        return Ok(());
    }
    let mut permissions = fs::metadata(path)
        .with_context(|| format!("read permissions for {}", path.display()))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .with_context(|| format!("set executable permissions for {}", path.display()))
}

#[cfg(not(unix))]
fn set_executable_if_needed(_path: &Path, _executable: bool) -> Result<()> {
    Ok(())
}

fn escape_powershell(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn write_zip_binary_file(
    zip: &mut ZipWriter<fs::File>,
    relative_path: &str,
    source: &Path,
    executable: bool,
) -> Result<()> {
    let bytes = fs::read(source)
        .with_context(|| format!("read PC host binary {}", source.display()))?;
    let unix_permissions = if executable { 0o755 } else { 0o644 };
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(unix_permissions);
    zip.start_file(relative_path, options)
        .with_context(|| format!("start pairing zip binary entry {}", relative_path))?;
    zip.write_all(&bytes)
        .with_context(|| format!("write pairing zip binary entry {}", relative_path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::PcGatewayPairingBundle;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn bundle_generates_env_file() {
        let bundle = sample_bundle();
        let env_file = bundle.env_file();
        assert!(env_file.contains("DEEPSEEK_PC_HOST_TOKEN=secret-token"));
        assert!(env_file.contains("DEEPSEEK_PC_HOST_WORKSPACE=/work/project"));
    }

    #[test]
    fn bundle_generates_pairing_json() {
        let bundle = sample_bundle();
        let json = bundle.pairing_json().unwrap();
        assert!(json.contains("pc-local"));
        assert!(json.contains("secret-token"));
    }

    #[test]
    fn bundle_generates_all_launch_scripts() {
        let bundle = sample_bundle();
        let scripts = bundle.launch_scripts();
        assert_eq!(scripts.len(), 3);
        assert!(scripts.iter().any(|script| script.file_name.ends_with(".ps1")));
        assert!(scripts.iter().any(|script| script.file_name.ends_with(".sh")));
        assert!(scripts.iter().any(|script| script.file_name.ends_with(".command")));
    }

    #[test]
    fn launch_scripts_prefer_bundled_host_then_path_fallback() {
        let bundle = sample_bundle();
        let windows = bundle.windows_powershell_script();
        assert!(windows.contains("deepseek-pc-host.exe"));
        assert!(windows.contains("Get-Command \"deepseek-pc-host\""));
        assert!(windows.contains("install the DeepSeek PC Host release package"));

        let posix = bundle.posix_shell_script();
        assert!(posix.contains("$SCRIPT_DIR/deepseek-pc-host"));
        assert!(posix.contains("command -v deepseek-pc-host"));
        assert!(posix.contains("install the DeepSeek PC Host release package"));
    }

    #[test]
    fn bundle_file_manifest_contains_expected_files() {
        let bundle = sample_bundle();
        let files = bundle.bundle_files(None).unwrap();
        assert!(files.iter().any(|file| file.relative_path == "pairing.json"));
        assert!(files.iter().any(|file| file.relative_path == "deepseek-pc-host.env"));
        assert!(files.iter().any(|file| file.relative_path == "README.txt"));
        assert!(files.iter().any(|file| file.relative_path == "start-deepseek-pc-host.ps1"));
    }

    #[test]
    fn writes_bundle_directory() {
        let bundle = sample_bundle();
        let dir = temp_path("dir");
        let written = bundle.write_directory(&dir).unwrap();
        assert!(written.len() >= 6);
        assert!(dir.join("pairing.json").exists());
        assert!(dir.join("deepseek-pc-host.env").exists());
        assert!(dir.join("start-deepseek-pc-host.ps1").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn writes_bundle_zip() {
        let bundle = sample_bundle();
        let zip_path = temp_path("zip").with_extension("zip");
        let written = bundle.write_zip(&zip_path).unwrap();
        assert_eq!(written, zip_path);
        assert!(zip_path.exists());
        assert!(fs::metadata(&zip_path).unwrap().len() > 0);
        let _ = fs::remove_file(zip_path);
    }

    #[test]
    fn writes_bundle_zip_with_embedded_host_binary() {
        let bundle = sample_bundle();
        let host_dir = temp_path("host-bin");
        fs::create_dir_all(&host_dir).unwrap();
        let host_path = host_dir.join("deepseek-pc-host.exe");
        fs::write(&host_path, b"fake-pc-host-binary").unwrap();

        let zip_path = temp_path("zip-with-host").with_extension("zip");
        let hosts = super::PcHostBinaryBundle {
            windows_exe: Some(host_path),
            unix_bin: None,
        };
        bundle
            .write_zip_with_host_binaries(&zip_path, Some(&hosts))
            .unwrap();
        assert!(zip_path.exists());

        let file = fs::File::open(&zip_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut found = false;
        for index in 0..archive.len() {
            let entry = archive.by_index(index).unwrap();
            if entry.name() == "deepseek-pc-host.exe" {
                found = true;
            }
        }
        assert!(found, "zip should contain embedded deepseek-pc-host.exe");
        let _ = fs::remove_file(zip_path);
        let _ = fs::remove_dir_all(host_dir);
    }

    fn sample_bundle() -> PcGatewayPairingBundle {
        PcGatewayPairingBundle::local_http(
            "pc-local",
            "Developer PC",
            "phone-1",
            "Android Phone",
            "local",
            "/work/project",
            "secret-token",
        )
    }

    fn temp_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "deepseek-pairing-test-{}-{}-{}",
            label,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}

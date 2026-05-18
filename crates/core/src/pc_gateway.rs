//! PC companion gateway protocol.
//!
//! The mobile app can be a control cockpit while a trusted PC/laptop acts as a
//! workspace runtime host: files, terminals, git, tests, dev servers, previews,
//! diagnostics and project indexing live on the PC; the phone controls them.
//! The gateway must never expose the whole computer by default: the PC grants
//! explicit workspaces, the phone sends structured requests, and risky actions
//! still go through the approval layer.
//!
//! The PC gateway does not require public internet. The preferred offline mode
//! is direct local connectivity: phone and PC on the same Wi-Fi/LAN, phone
//! hotspot, PC hotspot, USB-tethered LAN, or another private network segment.
//! Public internet/tunnel modes are optional convenience modes, not a baseline
//! requirement.

use crate::executor::{CommandOutput, CommandRequest};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static GATEWAY_REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcGatewayCapability {
    ListWorkspaces,
    ReadFiles,
    WriteFiles,
    DeleteFiles,
    ApplyPatch,
    ExecuteCommands,
    TerminalSessions,
    ManageEnvironments,
    GitStatus,
    GitDiff,
    GitCommit,
    GitPushPull,
    RunTests,
    RunBuilds,
    DevServerPreview,
    Diagnostics,
    LanguageServer,
    WorkspaceIndex,
    WatchFileChanges,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcGatewayTrustLevel {
    Unpaired,
    PairedReadOnly,
    PairedWorkspaceWrite,
    PairedCommandExecution,
    Admin,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcGatewayConnectionStatus {
    Offline,
    PairingRequired,
    Online,
    Unauthorized,
    VersionMismatch,
    Error(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcGatewayTransportMode {
    InternetHttps,
    TunnelHttps,
    LocalNetworkHttps,
    LocalNetworkHttp,
    DirectWifiHttps,
    DirectWifiHttp,
    LoopbackHttp,
}

impl PcGatewayTransportMode {
    pub fn requires_internet(&self) -> bool {
        matches!(self, PcGatewayTransportMode::InternetHttps | PcGatewayTransportMode::TunnelHttps)
    }

    pub fn allows_plain_http(&self) -> bool {
        matches!(
            self,
            PcGatewayTransportMode::LocalNetworkHttp
                | PcGatewayTransportMode::DirectWifiHttp
                | PcGatewayTransportMode::LoopbackHttp
        )
    }

    pub fn is_local_only(&self) -> bool {
        matches!(
            self,
            PcGatewayTransportMode::LocalNetworkHttps
                | PcGatewayTransportMode::LocalNetworkHttp
                | PcGatewayTransportMode::DirectWifiHttps
                | PcGatewayTransportMode::DirectWifiHttp
                | PcGatewayTransportMode::LoopbackHttp
        )
    }

    pub fn default_priority(&self) -> u16 {
        match self {
            PcGatewayTransportMode::LoopbackHttp => 130,
            PcGatewayTransportMode::DirectWifiHttps | PcGatewayTransportMode::DirectWifiHttp => 120,
            PcGatewayTransportMode::LocalNetworkHttps | PcGatewayTransportMode::LocalNetworkHttp => 110,
            PcGatewayTransportMode::TunnelHttps => 60,
            PcGatewayTransportMode::InternetHttps => 40,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayEndpointCandidate {
    pub label: String,
    pub base_url: String,
    pub transport_mode: PcGatewayTransportMode,
    pub priority: u16,
}

impl PcGatewayEndpointCandidate {
    pub fn new(
        label: impl Into<String>,
        base_url: impl Into<String>,
        transport_mode: PcGatewayTransportMode,
    ) -> Self {
        Self {
            label: label.into(),
            base_url: base_url.into(),
            priority: transport_mode.default_priority(),
            transport_mode,
        }
    }

    pub fn with_priority(mut self, priority: u16) -> Self {
        self.priority = priority;
        self
    }

    pub fn requires_internet(&self) -> bool {
        self.transport_mode.requires_internet()
    }

    pub fn is_local_only(&self) -> bool {
        self.transport_mode.is_local_only()
    }

    pub fn validate(&self, allow_http_on_local_network: bool) -> Result<(), String> {
        validate_gateway_base_url_for_transport(
            &self.base_url,
            &self.transport_mode,
            allow_http_on_local_network,
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayConfig {
    pub id: String,
    pub label: String,
    pub base_url: String,
    pub device_id: String,
    pub auth_token: Option<String>,
    pub trust_level: PcGatewayTrustLevel,
    pub transport_mode: PcGatewayTransportMode,
    pub allow_http_on_local_network: bool,
    #[serde(default)]
    pub endpoint_candidates: Vec<PcGatewayEndpointCandidate>,
}

impl PcGatewayConfig {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        base_url: impl Into<String>,
        device_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            base_url: base_url.into(),
            device_id: device_id.into(),
            auth_token: None,
            trust_level: PcGatewayTrustLevel::Unpaired,
            transport_mode: PcGatewayTransportMode::InternetHttps,
            allow_http_on_local_network: false,
            endpoint_candidates: Vec::new(),
        }
    }

    pub fn local_network_http(
        id: impl Into<String>,
        label: impl Into<String>,
        base_url: impl Into<String>,
        device_id: impl Into<String>,
    ) -> Self {
        let mut config = Self::new(id, label, base_url, device_id);
        config.transport_mode = PcGatewayTransportMode::LocalNetworkHttp;
        config.allow_http_on_local_network = true;
        config
    }

    pub fn direct_wifi_http(
        id: impl Into<String>,
        label: impl Into<String>,
        base_url: impl Into<String>,
        device_id: impl Into<String>,
    ) -> Self {
        let mut config = Self::new(id, label, base_url, device_id);
        config.transport_mode = PcGatewayTransportMode::DirectWifiHttp;
        config.allow_http_on_local_network = true;
        config
    }

    pub fn local_network_https(
        id: impl Into<String>,
        label: impl Into<String>,
        base_url: impl Into<String>,
        device_id: impl Into<String>,
    ) -> Self {
        let mut config = Self::new(id, label, base_url, device_id);
        config.transport_mode = PcGatewayTransportMode::LocalNetworkHttps;
        config
    }

    pub fn tunnel_https(
        id: impl Into<String>,
        label: impl Into<String>,
        base_url: impl Into<String>,
        device_id: impl Into<String>,
    ) -> Self {
        let mut config = Self::new(id, label, base_url, device_id);
        config.transport_mode = PcGatewayTransportMode::TunnelHttps;
        config
    }

    pub fn with_endpoint_candidate(mut self, candidate: PcGatewayEndpointCandidate) -> Self {
        self.endpoint_candidates.push(candidate);
        self
    }

    pub fn add_endpoint_candidate(&mut self, candidate: PcGatewayEndpointCandidate) {
        self.endpoint_candidates.push(candidate);
    }

    pub fn endpoint_plan(&self) -> Vec<PcGatewayEndpointCandidate> {
        let mut candidates = vec![PcGatewayEndpointCandidate::new(
            "primary",
            self.base_url.clone(),
            self.transport_mode.clone(),
        )];
        candidates.extend(self.endpoint_candidates.clone());
        candidates.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then(left.label.cmp(&right.label))
                .then(left.base_url.cmp(&right.base_url))
        });
        candidates.dedup_by(|left, right| left.base_url == right.base_url);
        candidates
    }

    pub fn requires_internet(&self) -> bool {
        self.transport_mode.requires_internet()
    }

    pub fn can_use_without_internet(&self) -> bool {
        self.endpoint_plan().iter().any(|candidate| candidate.is_local_only())
    }

    pub fn is_local_only(&self) -> bool {
        self.transport_mode.is_local_only()
    }

    pub fn validate_base_url(&self) -> Result<(), String> {
        validate_gateway_base_url_for_transport(&self.base_url, &self.transport_mode, self.allow_http_on_local_network)
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyPreset { ReadOnly, Developer, Admin }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewaySecurityPolicy {
    pub require_tls: bool,
    pub require_per_action_approval: bool,
    pub max_command_seconds: u64,
    pub max_output_bytes: usize,
    pub allowed_programs: Vec<String>,
    pub blocked_path_fragments: Vec<String>,
}

impl Default for PcGatewaySecurityPolicy {
    fn default() -> Self {
        Self {
            require_tls: true,
            require_per_action_approval: true,
            max_command_seconds: 60,
            max_output_bytes: 512 * 1024,
            allowed_programs: vec![
                "cargo".to_string(),
                "git".to_string(),
                "npm".to_string(),
                "npx".to_string(),
                "pnpm".to_string(),
                "yarn".to_string(),
                "python".to_string(),
                "python3".to_string(),
                "pytest".to_string(),
                "node".to_string(),
                "bun".to_string(),
                "deno".to_string(),
                "go".to_string(),
                "rustc".to_string(),
                "java".to_string(),
                "gradle".to_string(),
                "mvn".to_string(),
                "docker".to_string(),
                "docker-compose".to_string(),
            ],
            blocked_path_fragments: vec![
                ".ssh".to_string(),
                ".gnupg".to_string(),
                "AppData/Roaming".to_string(),
                "Library/Keychains".to_string(),
                "/etc".to_string(),
            ],
        }
    }
}

impl PcGatewaySecurityPolicy {
    pub fn read_only() -> Self {
        Self { max_command_seconds: 0, max_output_bytes: 64 * 1024, allowed_programs: vec![], ..Self::default() }
    }
    pub fn developer() -> Self { Self::default() }
    pub fn admin() -> Self {
        Self {
            require_tls: false,
            require_per_action_approval: false,
            max_command_seconds: 600,
            max_output_bytes: 4 * 1024 * 1024,
            allowed_programs: vec![],
            blocked_path_fragments: vec![],
        }
    }
    pub fn from_preset(preset: PolicyPreset) -> Self {
        match preset {
            PolicyPreset::ReadOnly => Self::read_only(),
            PolicyPreset::Developer => Self::developer(),
            PolicyPreset::Admin => Self::admin(),
        }
    }

    pub fn allows_program(&self, program: &str) -> bool {
        self.allowed_programs
            .iter()
            .any(|allowed| allowed == program || program.ends_with(&format!("/{}", allowed)))
    }

    pub fn allows_path(&self, path: &str) -> bool {
        !self
            .blocked_path_fragments
            .iter()
            .any(|fragment| path.contains(fragment))
    }

    pub fn validate_command(&self, request: &CommandRequest) -> Result<(), String> {
        if !self.allows_program(&request.program) {
            return Err(format!("program is not allowed by gateway policy: {}", request.program));
        }
        if let Some(working_dir) = request.working_dir.as_ref() {
            let path = working_dir.to_string_lossy();
            if !self.allows_path(&path) {
                return Err(format!("working directory is blocked by gateway policy: {}", path));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcWorkspaceGrant {
    pub id: String,
    pub name: String,
    pub root: String,
    pub capabilities: Vec<PcGatewayCapability>,
    pub created_at_unix: u64,
    pub expires_at_unix: Option<u64>,
}

impl PcWorkspaceGrant {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        root: impl Into<String>,
        capabilities: Vec<PcGatewayCapability>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            root: root.into(),
            capabilities,
            created_at_unix: current_unix_time(),
            expires_at_unix: None,
        }
    }

    pub fn has_capability(&self, capability: &PcGatewayCapability) -> bool {
        self.capabilities.contains(capability)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcEnvironmentKind {
    System,
    PythonVenv,
    Conda,
    Node,
    Rust,
    Go,
    Java,
    Docker,
    DevContainer,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcEnvironmentDescriptor {
    pub id: String,
    pub name: String,
    pub kind: PcEnvironmentKind,
    pub root: Option<String>,
    pub command_prefix: Vec<String>,
    pub detected_tools: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcTerminalSession {
    pub id: String,
    pub workspace_id: String,
    pub title: String,
    pub cwd: String,
    pub environment_id: Option<String>,
    pub created_at_unix: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcTaskKind {
    Build,
    Test,
    Run,
    Format,
    Lint,
    Install,
    DevServer,
    Custom,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcTaskDescriptor {
    pub id: String,
    pub workspace_id: String,
    pub label: String,
    pub kind: PcTaskKind,
    pub command: CommandRequest,
    pub environment_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcPreviewDescriptor {
    pub id: String,
    pub workspace_id: String,
    pub label: String,
    pub local_url: String,
    pub gateway_url: Option<String>,
    pub process_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcDiagnostic {
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub severity: PcDiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcDiagnosticSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcWorkspaceIndexSummary {
    pub workspace_id: String,
    pub files_indexed: u64,
    pub symbols_indexed: u64,
    pub last_indexed_at_unix: Option<u64>,
    pub status: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayPairingRequest {
    pub device_id: String,
    pub device_label: String,
    pub public_key_hint: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayPairingResponse {
    pub accepted: bool,
    pub gateway_id: String,
    pub gateway_label: String,
    pub auth_token: Option<String>,
    pub granted_workspaces: Vec<PcWorkspaceGrant>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayRequestEnvelope {
    pub id: String,
    pub device_id: String,
    pub timestamp_unix: u64,
    pub request: PcGatewayRequest,
}

impl PcGatewayRequestEnvelope {
    pub fn new(device_id: impl Into<String>, request: PcGatewayRequest) -> Self {
        let seq = GATEWAY_REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            id: format!("pcgw-{}-{}", current_unix_time(), seq),
            device_id: device_id.into(),
            timestamp_unix: current_unix_time(),
            request,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcGatewayRequest {
    Health,
    ListWorkspaces,
    ListEnvironments { workspace_id: String },
    DetectTasks { workspace_id: String },
    IndexWorkspace { workspace_id: String },
    ReadFile { workspace_id: String, path: String },
    WriteFile { workspace_id: String, path: String, content: String },
    DeleteFile { workspace_id: String, path: String },
    ListDir { workspace_id: String, path: String },
    OpenTerminal { workspace_id: String, cwd: Option<String>, environment_id: Option<String> },
    TerminalInput { session_id: String, input: String },
    CloseTerminal { session_id: String },
    ExecuteCommand { workspace_id: String, command: CommandRequest, environment_id: Option<String> },
    RunTask { task_id: String },
    StopTask { task_id: String },
    StartDevServer { workspace_id: String, command: CommandRequest, environment_id: Option<String> },
    StopDevServer { preview_id: String },
    GetDiagnostics { workspace_id: String, path: Option<String> },
    GitStatus { workspace_id: String },
    GitDiff { workspace_id: String },
    GitCommit { workspace_id: String, message: String },
    GitPush { workspace_id: String, remote: Option<String>, branch: Option<String> },
    GitPull { workspace_id: String, remote: Option<String>, branch: Option<String> },
    GitBranch { workspace_id: String },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayResponseEnvelope {
    pub request_id: String,
    pub timestamp_unix: u64,
    pub response: PcGatewayResponse,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PcGatewayResponse {
    Health(PcGatewayHealth),
    Workspaces(Vec<PcWorkspaceGrant>),
    Environments(Vec<PcEnvironmentDescriptor>),
    Tasks(Vec<PcTaskDescriptor>),
    WorkspaceIndex(PcWorkspaceIndexSummary),
    FileContent { path: String, content: String },
    FileWritten { path: String, bytes: usize },
    FileDeleted { path: String },
    DirEntries(Vec<PcGatewayDirEntry>),
    TerminalOpened(PcTerminalSession),
    TerminalOutput { session_id: String, chunk: String },
    TerminalClosed { session_id: String, exit_code: Option<i32> },
    CommandOutput(CommandOutput),
    TaskStarted { task_id: String, process_id: String },
    TaskStopped { task_id: String },
    PreviewStarted(PcPreviewDescriptor),
    PreviewStopped { preview_id: String },
    Diagnostics(Vec<PcDiagnostic>),
    GitText { operation: String, output: String },
    Error(PcGatewayError),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayHealth {
    pub gateway_id: String,
    pub version: String,
    pub status: PcGatewayConnectionStatus,
    pub capabilities: Vec<PcGatewayCapability>,
    /// Seconds since the PC-host process started.
    #[serde(default)]
    pub uptime_secs: u64,
    /// Total number of gateway requests processed.
    #[serde(default)]
    pub request_count: u64,
    /// Number of requests that resulted in an error.
    #[serde(default)]
    pub error_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayDirEntry {
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PcGatewayError {
    pub code: String,
    pub message: String,
}

impl PcGatewayError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub fn validate_gateway_base_url(base_url: &str, allow_http_on_local_network: bool) -> Result<(), String> {
    let mode = if allow_http_on_local_network {
        PcGatewayTransportMode::LocalNetworkHttp
    } else {
        PcGatewayTransportMode::InternetHttps
    };
    validate_gateway_base_url_for_transport(base_url, &mode, allow_http_on_local_network)
}

pub fn validate_gateway_base_url_for_transport(
    base_url: &str,
    transport_mode: &PcGatewayTransportMode,
    allow_http_on_local_network: bool,
) -> Result<(), String> {
    let base_url = base_url.trim();
    if base_url.starts_with("https://") {
        return Ok(());
    }

    if base_url.starts_with("http://")
        && transport_mode.allows_plain_http()
        && allow_http_on_local_network
        && is_private_or_loopback_http_url(base_url)
    {
        return Ok(());
    }

    Err("gateway URL must use HTTPS unless explicitly allowed for private local/offline network pairing".to_string())
}

pub fn is_private_or_loopback_http_url(base_url: &str) -> bool {
    let Some(host_port_path) = base_url.strip_prefix("http://") else {
        return false;
    };
    let host_port = host_port_path.split('/').next().unwrap_or_default();
    let host = host_port.split(':').next().unwrap_or_default().to_ascii_lowercase();

    host == "localhost"
        || host == "127.0.0.1"
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
        || is_172_private_address(&host)
}

fn is_172_private_address(host: &str) -> bool {
    let mut parts = host.split('.');
    let first = parts.next();
    let second = parts.next().and_then(|value| value.parse::<u8>().ok());
    first == Some("172") && second.is_some_and(|octet| (16..=31).contains(&octet))
}

fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}


/// Streaming event from a long-running command execution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum CommandStreamEvent {
    Stdout(String),
    Stderr(String),
    Exit(Option<i32>),
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::{
        is_private_or_loopback_http_url, validate_gateway_base_url, validate_gateway_base_url_for_transport,
        PcGatewayCapability, PcGatewayConfig, PcGatewayEndpointCandidate, PcGatewaySecurityPolicy,
        PcGatewayTransportMode, PcWorkspaceGrant,
    };
    use crate::executor::CommandRequest;
    use std::path::PathBuf;

    #[test]
    fn rejects_plain_http_by_default() {
        let result = validate_gateway_base_url("http://192.168.1.10:8787", false);
        assert!(result.is_err());
    }

    #[test]
    fn allows_local_http_when_explicitly_enabled() {
        let result = validate_gateway_base_url("http://192.168.1.10:8787", true);
        assert!(result.is_ok());
    }

    #[test]
    fn allows_https_gateway_urls() {
        let result = validate_gateway_base_url("https://gateway.example.test", false);
        assert!(result.is_ok());
    }

    #[test]
    fn local_network_config_does_not_require_internet() {
        let config = PcGatewayConfig::local_network_http(
            "pc-1",
            "Laptop",
            "http://192.168.1.10:8787",
            "phone-1",
        );
        assert!(config.validate_base_url().is_ok());
        assert!(config.is_local_only());
        assert!(config.can_use_without_internet());
        assert!(!config.requires_internet());
    }

    #[test]
    fn direct_wifi_config_allows_link_local_addresses() {
        let config = PcGatewayConfig::direct_wifi_http(
            "pc-1",
            "Laptop Hotspot",
            "http://169.254.12.10:8787",
            "phone-1",
        );
        assert!(config.validate_base_url().is_ok());
        assert!(config.is_local_only());
        assert!(config.can_use_without_internet());
        assert!(!config.requires_internet());
    }

    #[test]
    fn endpoint_plan_prefers_direct_and_local_routes_before_internet() {
        let config = PcGatewayConfig::tunnel_https(
            "pc-1",
            "Laptop",
            "https://pc.example.test",
            "phone-1",
        )
        .with_endpoint_candidate(PcGatewayEndpointCandidate::new(
            "same-lan",
            "http://192.168.1.10:8787",
            PcGatewayTransportMode::LocalNetworkHttp,
        ))
        .with_endpoint_candidate(PcGatewayEndpointCandidate::new(
            "direct-link",
            "http://169.254.12.10:8787",
            PcGatewayTransportMode::DirectWifiHttp,
        ));
        let plan = config.endpoint_plan();
        assert_eq!(plan[0].label, "direct-link");
        assert_eq!(plan[1].label, "same-lan");
        assert_eq!(plan[2].label, "primary");
        assert!(config.can_use_without_internet());
    }

    #[test]
    fn validates_full_172_private_range() {
        assert!(is_private_or_loopback_http_url("http://172.16.0.2:8787"));
        assert!(is_private_or_loopback_http_url("http://172.31.255.2:8787"));
        assert!(!is_private_or_loopback_http_url("http://172.32.0.2:8787"));
    }

    #[test]
    fn transport_rejects_public_http_even_when_plain_http_mode_exists() {
        let result = validate_gateway_base_url_for_transport(
            "http://example.com:8787",
            &PcGatewayTransportMode::LocalNetworkHttp,
            true,
        );
        assert!(result.is_err());
    }

    #[test]
    fn workspace_grant_checks_capabilities() {
        let grant = PcWorkspaceGrant::new(
            "w1",
            "Project",
            "/work/project",
            vec![PcGatewayCapability::ReadFiles],
        );
        assert!(grant.has_capability(&PcGatewayCapability::ReadFiles));
        assert!(!grant.has_capability(&PcGatewayCapability::WriteFiles));
    }

    #[test]
    fn command_policy_blocks_unknown_programs() {
        let policy = PcGatewaySecurityPolicy::default();
        let request = CommandRequest {
            program: "rm".to_string(),
            args: vec!["-rf".to_string(), "/".to_string()],
            working_dir: Some(PathBuf::from("/work/project")),
        };
        assert!(policy.validate_command(&request).is_err());
    }

    #[test]
    fn command_policy_allows_developer_tools() {
        let policy = PcGatewaySecurityPolicy::default();
        let request = CommandRequest {
            program: "cargo".to_string(),
            args: vec!["check".to_string()],
            working_dir: Some(PathBuf::from("/work/project")),
        };
        assert!(policy.validate_command(&request).is_ok());
    }
}
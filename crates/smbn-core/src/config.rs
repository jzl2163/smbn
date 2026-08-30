use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const CURRENT_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppConfig {
    pub version: u32,
    pub server: ServerConfig,
    pub listeners: Vec<ListenerConfig>,
    pub shares: Vec<ShareConfig>,
    pub users: Vec<UserConfig>,
    pub app: UiConfig,
    pub logging: LoggingConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_CONFIG_VERSION,
            server: ServerConfig::default(),
            listeners: vec![ListenerConfig::ipv4_default(), ListenerConfig::ipv6_default()],
            shares: Vec::new(),
            users: Vec::new(),
            app: UiConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ServerConfig {
    /// NetBIOS discovery name. SMB2/3 clients normally connect by DNS name or IP.
    pub netbios_name: String,
    pub workgroup: String,
    pub authentication: AuthenticationMode,
    pub enable_smb1: bool,
    pub enable_smb2: bool,
    pub enable_smb3: bool,
    /// Sends an unsolicited ECHO after the connection has been inactive this long.
    /// Zero disables SMBLibrary's inactivity monitor.
    pub inactivity_timeout_seconds: u64,
    pub reject_remote_subnets: Vec<String>,
    pub allow_remote_subnets: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let machine_name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "SMBN".to_owned());
        Self {
            netbios_name: machine_name.chars().take(15).collect(),
            workgroup: "WORKGROUP".to_owned(),
            authentication: AuthenticationMode::Independent,
            enable_smb1: false,
            enable_smb2: true,
            enable_smb3: true,
            inactivity_timeout_seconds: 300,
            reject_remote_subnets: Vec::new(),
            allow_remote_subnets: Vec::new(),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthenticationMode {
    Independent,
    IntegratedWindows,
}

impl Default for AuthenticationMode {
    fn default() -> Self {
        Self::Independent
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ListenerConfig {
    pub id: String,
    /// IPv4/IPv6 literal. Use 0.0.0.0 and :: for wildcard listeners.
    pub address: String,
    pub port: u16,
    pub transport: Transport,
    /// Registers and answers the NetBIOS name on UDP/137. IPv4 only.
    pub netbios_name_service: bool,
    pub enabled: bool,
}

impl ListenerConfig {
    pub fn ipv4_default() -> Self {
        Self {
            id: "direct-ipv4".to_owned(),
            address: "0.0.0.0".to_owned(),
            port: 445,
            transport: Transport::DirectTcp,
            netbios_name_service: false,
            enabled: true,
        }
    }

    pub fn ipv6_default() -> Self {
        Self {
            id: "direct-ipv6".to_owned(),
            address: "::".to_owned(),
            port: 445,
            transport: Transport::DirectTcp,
            netbios_name_service: false,
            // Disabled by default to avoid ambiguous dual-stack wildcard binding behavior.
            // Users can enable it alone or bind IPv4/IPv6 to concrete interface addresses.
            enabled: false,
        }
    }
}

impl Default for ListenerConfig {
    fn default() -> Self {
        Self::ipv4_default()
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    DirectTcp,
    NetbiosOverTcp,
}

impl Default for Transport {
    fn default() -> Self {
        Self::DirectTcp
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct ShareConfig {
    pub id: String,
    pub name: String,
    pub path: String,
    pub comment: String,
    pub enabled: bool,
    pub hidden: bool,
    pub read_only: bool,
    /// Case-insensitive account names. "Users" grants all authenticated users.
    pub read_access: Vec<String>,
    pub write_access: Vec<String>,
}

impl Default for ShareConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            path: String::new(),
            comment: String::new(),
            enabled: true,
            hidden: false,
            read_only: false,
            read_access: vec!["Users".to_owned()],
            write_access: vec!["Users".to_owned()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UserConfig {
    pub id: String,
    pub account_name: String,
    pub enabled: bool,
    /// Base64-encoded DPAPI CurrentUser blob. Never plaintext on disk.
    pub protected_password: String,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            account_name: String::new(),
            enabled: true,
            protected_password: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct UiConfig {
    pub start_with_windows: bool,
    pub start_server_on_launch: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub light_mode: bool,
    pub confirm_exit_while_running: bool,
    pub language: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            start_with_windows: false,
            start_server_on_launch: false,
            minimize_to_tray: true,
            close_to_tray: true,
            light_mode: true,
            confirm_exit_while_running: true,
            language: "zh-CN".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: LogLevel,
    pub max_file_mib: u32,
    pub retained_files: u32,
    pub gui_tail_lines: usize,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Information,
            max_file_mib: 8,
            retained_files: 4,
            gui_tail_lines: 500,
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Error,
    Warning,
    Information,
    Debug,
    Verbose,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Information
    }
}

/// Plaintext credentials only exist in memory while constructing one start request.
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct PlainUser {
    pub account_name: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub server: ServerConfig,
    pub listeners: Vec<ListenerConfig>,
    pub shares: Vec<ShareConfig>,
    pub users: Vec<PlainUser>,
    pub logging: LoggingConfig,
}

use crate::EngineConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const IPC_PROTOCOL_VERSION: u32 = 1;
pub const MAX_IPC_MESSAGE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub version: u32,
    pub id: u64,
    pub token: String,
    pub command: String,
    #[serde(default)]
    pub payload: Value,
}

impl RequestEnvelope {
    pub fn new(id: u64, token: impl Into<String>, command: impl Into<String>, payload: Value) -> Self {
        Self {
            version: IPC_PROTOCOL_VERSION,
            id,
            token: token.into(),
            command: command.into(),
            payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub version: u32,
    pub id: u64,
    pub ok: bool,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub error: Option<EngineError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartPayload {
    pub config: EngineConfig,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EngineState {
    Stopped,
    Starting,
    Running,
    Stopping,
    Faulted,
}

impl Default for EngineState {
    fn default() -> Self {
        Self::Stopped
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EngineStatus {
    pub state: EngineState,
    pub started_at_utc: Option<String>,
    pub uptime_seconds: u64,
    pub listener_count: usize,
    pub share_count: usize,
    pub user_count: usize,
    pub session_count: usize,
    pub open_file_count: usize,
    pub last_error: Option<String>,
    pub engine_version: String,
    pub smblibrary_version: String,
    pub dropped_log_entries: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionInfo {
    pub listener_id: String,
    pub client_endpoint: String,
    pub dialect: String,
    pub user_name: String,
    pub machine_name: String,
    pub open_file_count: usize,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiagnosticsResult {
    pub checks: Vec<DiagnosticCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub severity: String,
    pub name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LogTail {
    pub lines: Vec<String>,
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use crate::{AuthenticationMode, ListenerConfig, LogLevel, ServerConfig, Transport};

    #[test]
    fn wire_enums_are_snake_case() {
        assert_eq!(serde_json::to_string(&AuthenticationMode::IntegratedWindows).unwrap(), "\"integrated_windows\"");
        assert_eq!(serde_json::to_string(&Transport::DirectTcp).unwrap(), "\"direct_tcp\"");
        assert_eq!(serde_json::to_string(&LogLevel::Information).unwrap(), "\"information\"");

        let mut server = ServerConfig::default();
        server.authentication = AuthenticationMode::IntegratedWindows;
        let json = serde_json::to_value(server).unwrap();
        assert_eq!(json["authentication"], "integrated_windows");

        let listener = ListenerConfig::ipv4_default();
        let json = serde_json::to_value(listener).unwrap();
        assert_eq!(json["transport"], "direct_tcp");
    }
}

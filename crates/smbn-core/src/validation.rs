use crate::{AppConfig, AuthenticationMode, Transport};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::IpAddr;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigIssue {
    pub severity: IssueSeverity,
    pub path: String,
    pub message: String,
}

impl ConfigIssue {
    fn error(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self { severity: IssueSeverity::Error, path: path.into(), message: message.into() }
    }

    fn warning(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self { severity: IssueSeverity::Warning, path: path.into(), message: message.into() }
    }
}

pub fn validate_config(config: &AppConfig) -> Vec<ConfigIssue> {
    let mut issues = Vec::new();

    if config.version != crate::CURRENT_CONFIG_VERSION {
        issues.push(ConfigIssue::error("version", "不支持的配置版本"));
    }

    let netbios = config.server.netbios_name.trim();
    if netbios.is_empty() || netbios.chars().count() > 15 {
        issues.push(ConfigIssue::error("server.netbios_name", "NetBIOS 名称必须为 1–15 个字符"));
    }
    if !netbios.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        issues.push(ConfigIssue::error("server.netbios_name", "NetBIOS 名称只能包含 ASCII 字母、数字和连字符"));
    }
    let workgroup = config.server.workgroup.trim();
    if workgroup.is_empty() || workgroup.chars().count() > 15 {
        issues.push(ConfigIssue::error("server.workgroup", "工作组名称必须为 1–15 个字符"));
    }

    if !config.server.enable_smb1 && !config.server.enable_smb2 && !config.server.enable_smb3 {
        issues.push(ConfigIssue::error("server.protocols", "至少启用一个 SMB 协议版本"));
    }
    if config.server.enable_smb3 && !config.server.enable_smb2 {
        issues.push(ConfigIssue::error("server.enable_smb3", "SMBLibrary 要求启用 SMB2 后才能启用 SMB3"));
    }
    if config.server.enable_smb1 {
        issues.push(ConfigIssue::warning("server.enable_smb1", "SMB1 已过时且安全性较弱；仅为兼容旧设备时启用"));
    }

    let mut listener_ids = HashSet::new();
    let mut endpoints = HashSet::new();
    let mut enabled_listener_count = 0usize;
    for (index, listener) in config.listeners.iter().enumerate() {
        let base = format!("listeners[{index}]");
        if listener.id.trim().is_empty() || !listener_ids.insert(listener.id.to_ascii_lowercase()) {
            issues.push(ConfigIssue::error(format!("{base}.id"), "监听器 ID 不能为空且必须唯一"));
        }
        if !listener.enabled {
            continue;
        }
        enabled_listener_count += 1;
        let address = match listener.address.trim().parse::<IpAddr>() {
            Ok(value) => value,
            Err(_) => {
                issues.push(ConfigIssue::error(format!("{base}.address"), "监听地址必须是 IPv4 或 IPv6 字面量"));
                continue;
            }
        };
        if listener.port == 0 {
            issues.push(ConfigIssue::error(format!("{base}.port"), "端口必须在 1–65535 范围内"));
        }
        let endpoint_key = format!("{}:{}:{:?}", address, listener.port, listener.transport);
        if !endpoints.insert(endpoint_key) {
            issues.push(ConfigIssue::error(base.clone(), "存在重复的监听地址、端口和传输组合"));
        }
        if address.is_ipv6() && listener.transport == Transport::NetbiosOverTcp {
            issues.push(ConfigIssue::error(format!("{base}.transport"), "NetBIOS over TCP 仅支持 IPv4；IPv6 请使用 Direct TCP"));
        }
        if address.is_ipv6() && listener.netbios_name_service {
            issues.push(ConfigIssue::error(format!("{base}.netbios_name_service"), "NetBIOS 名称服务只支持 IPv4"));
        }
        if listener.netbios_name_service && address.is_unspecified() {
            issues.push(ConfigIssue::error(format!("{base}.address"), "名称服务必须绑定到具体 IPv4 地址，不能使用 0.0.0.0"));
        }
        if matches!(listener.port, 139 | 445) {
            issues.push(ConfigIssue::warning(format!("{base}.port"), "Windows Server/LanmanServer 服务可能已占用 139/445 端口"));
        }
    }
    if enabled_listener_count == 0 {
        issues.push(ConfigIssue::error("listeners", "至少启用一个监听器"));
    }

    let mut share_ids = HashSet::new();
    let mut share_names = HashSet::new();
    let mut enabled_share_count = 0usize;
    for (index, share) in config.shares.iter().enumerate() {
        let base = format!("shares[{index}]");
        if share.id.trim().is_empty() || !share_ids.insert(share.id.to_ascii_lowercase()) {
            issues.push(ConfigIssue::error(format!("{base}.id"), "共享 ID 不能为空且必须唯一"));
        }
        if !share.enabled {
            continue;
        }
        enabled_share_count += 1;
        let name = share.name.trim();
        if name.is_empty() || name.len() > 80 {
            issues.push(ConfigIssue::error(format!("{base}.name"), "共享名必须为 1–80 个字符"));
        }
        if name.chars().any(|c| matches!(c, '\\' | '/' | '[' | ']' | ':' | ';' | '|' | '=' | ',' | '+' | '*' | '?' | '<' | '>' | '"')) {
            issues.push(ConfigIssue::error(format!("{base}.name"), "共享名包含 Windows 不允许的字符"));
        }
        if !share_names.insert(name.to_ascii_lowercase()) {
            issues.push(ConfigIssue::error(format!("{base}.name"), "共享名必须唯一（不区分大小写）"));
        }
        if share.path.trim().is_empty() {
            issues.push(ConfigIssue::error(format!("{base}.path"), "共享路径不能为空"));
        } else if !is_windows_absolute_path(&share.path) {
            issues.push(ConfigIssue::error(format!("{base}.path"), "共享路径必须是绝对路径"));
        }
        if share.read_access.is_empty() {
            issues.push(ConfigIssue::warning(format!("{base}.read_access"), "未配置读取主体，该共享将无法读取"));
        }
        if !share.read_only && share.write_access.is_empty() {
            issues.push(ConfigIssue::warning(format!("{base}.write_access"), "未配置写入主体，该共享将无法写入"));
        }
    }
    if enabled_share_count == 0 {
        issues.push(ConfigIssue::error("shares", "至少启用一个共享"));
    }

    let mut user_ids = HashSet::new();
    let mut account_names = HashSet::new();
    let mut enabled_user_count = 0usize;
    for (index, user) in config.users.iter().enumerate() {
        let base = format!("users[{index}]");
        if user.id.trim().is_empty() || !user_ids.insert(user.id.to_ascii_lowercase()) {
            issues.push(ConfigIssue::error(format!("{base}.id"), "用户 ID 不能为空且必须唯一"));
        }
        if !user.enabled {
            continue;
        }
        enabled_user_count += 1;
        if user.account_name.trim().is_empty() {
            issues.push(ConfigIssue::error(format!("{base}.account_name"), "账户名不能为空"));
        }
        if !account_names.insert(user.account_name.to_ascii_lowercase()) {
            issues.push(ConfigIssue::error(format!("{base}.account_name"), "账户名必须唯一（不区分大小写）"));
        }
        if user.protected_password.is_empty() {
            issues.push(ConfigIssue::error(format!("{base}.protected_password"), "该用户尚未设置密码"));
        }
    }
    if config.server.authentication == AuthenticationMode::Independent && enabled_user_count == 0 {
        issues.push(ConfigIssue::error("users", "独立认证模式至少需要一个已启用用户"));
    }

    if !(1..=1024).contains(&config.logging.max_file_mib) {
        issues.push(ConfigIssue::error("logging.max_file_mib", "单个日志文件大小必须为 1–1024 MiB"));
    }
    if !(1..=100).contains(&config.logging.retained_files) {
        issues.push(ConfigIssue::error("logging.retained_files", "日志保留数量必须为 1–100"));
    }
    if !(50..=10_000).contains(&config.logging.gui_tail_lines) {
        issues.push(ConfigIssue::error("logging.gui_tail_lines", "界面日志行数必须为 50–10000"));
    }

    issues
}

fn is_windows_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    let drive_absolute = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    let unc_absolute = value.starts_with(r"\\") || value.starts_with("//");
    drive_absolute || unc_absolute
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ShareConfig, UserConfig};

    #[test]
    fn default_requires_share_and_user() {
        let issues = validate_config(&AppConfig::default());
        assert!(issues.iter().any(|i| i.path == "shares"));
        assert!(issues.iter().any(|i| i.path == "users"));
    }

    #[test]
    fn valid_minimum_configuration_has_no_errors() {
        let mut config = AppConfig::default();
        config.shares.push(ShareConfig {
            id: "share-1".into(),
            name: "data".into(),
            path: r"C:\Data".into(),
            ..ShareConfig::default()
        });
        config.users.push(UserConfig {
            id: "user-1".into(),
            account_name: "alice".into(),
            protected_password: "dpapi".into(),
            ..UserConfig::default()
        });
        let errors = validate_config(&config)
            .into_iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn ipv6_netbios_is_rejected() {
        let mut config = AppConfig::default();
        config.listeners[1].enabled = true;
        config.listeners[1].transport = Transport::NetbiosOverTcp;
        let issues = validate_config(&config);
        assert!(issues.iter().any(|i| i.path == "listeners[1].transport"));
    }
}

use crate::paths::{AppPaths, EngineLaunch};
use anyhow::{anyhow, bail, Context, Result};
use rand::distr::{Alphanumeric, SampleString};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use smbn_core::{
    DiagnosticsResult, EngineConfig, EngineStatus, LogTail, ResponseEnvelope, SessionInfo,
    StartPayload, IPC_PROTOCOL_VERSION, MAX_IPC_MESSAGE_BYTES,
};
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use zeroize::Zeroize;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_RETRY_DELAY: Duration = Duration::from_millis(40);
const STARTUP_LOG_TAIL_BYTES: usize = 16 * 1024;

pub struct EngineClient {
    pipe_path: String,
    token: String,
    request_id: AtomicU64,
    request_gate: Mutex<()>,
    child: Mutex<Child>,
    shutdown_sent: AtomicBool,
    bootstrap_log_path: PathBuf,
}

impl EngineClient {
    pub fn launch(paths: &AppPaths) -> Result<Self> {
        let launch = paths.engine_executable().ok_or_else(|| {
            anyhow!(
                "找不到 SMB 引擎。请将 Smbn.Engine.exe 或 Smbn.Engine.dll 放到 {} 或其 engine 子目录。",
                paths.exe_dir.display()
            )
        })?;
        let suffix = Alphanumeric
            .sample_string(&mut rand::rng(), 24)
            .to_ascii_lowercase();
        let pipe_name = format!("smbn-{}-{suffix}", std::process::id());
        let pipe_path = format!(r"\\.\pipe\{pipe_name}");
        let token = Alphanumeric.sample_string(&mut rand::rng(), 64);
        let bootstrap_log_path = paths.log_dir.join("engine-bootstrap.log");

        let mut command = match launch {
            EngineLaunch::Executable(path) => Command::new(path),
            EngineLaunch::DotnetDll(path) => {
                let mut command = Command::new("dotnet");
                command.arg(path);
                command
            }
        };
        command
            .arg("--pipe")
            .arg(&pipe_name)
            .arg("--parent")
            .arg(std::process::id().to_string())
            .arg("--log-dir")
            .arg(&paths.log_dir)
            .env("SMBN_IPC_TOKEN", &token)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(bootstrap_log(&bootstrap_log_path)?))
            .creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
        let child = command.spawn().context("无法启动 Smbn.Engine")?;

        let client = Self {
            pipe_path,
            token,
            request_id: AtomicU64::new(1),
            request_gate: Mutex::new(()),
            child: Mutex::new(child),
            shutdown_sent: AtomicBool::new(false),
            bootstrap_log_path,
        };
        client.wait_until_ready()?;
        Ok(client)
    }

    pub fn start(&self, config: EngineConfig) -> Result<()> {
        let _: Value = self.request("start", &StartPayload { config })?;
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let _: Value = self.request("stop", &json!({}))?;
        Ok(())
    }

    pub fn status(&self) -> Result<EngineStatus> {
        self.request("status", &json!({}))
    }

    pub fn sessions(&self) -> Result<Vec<SessionInfo>> {
        self.request("sessions", &json!({}))
    }

    pub fn terminate_session(&self, listener_id: &str, client_endpoint: &str) -> Result<()> {
        let _: Value = self.request(
            "terminate_session",
            &json!({ "listener_id": listener_id, "client_endpoint": client_endpoint }),
        )?;
        Ok(())
    }

    pub fn tail_logs(&self, maximum: usize) -> Result<LogTail> {
        self.request("tail_logs", &json!({ "maximum": maximum }))
    }

    pub fn diagnostics(&self, config: &EngineConfig) -> Result<DiagnosticsResult> {
        self.request("diagnostics", &json!({ "config": config }))
    }

    pub fn trim_memory(&self) -> Result<()> {
        let _: Value = self.request("trim_memory", &json!({}))?;
        Ok(())
    }

    pub fn shutdown(&self) -> Result<()> {
        if self.shutdown_sent.load(Ordering::Acquire) {
            return Ok(());
        }
        let _: Value = self.request("shutdown", &json!({}))?;
        self.shutdown_sent.store(true, Ordering::Release);
        Ok(())
    }

    pub fn process_has_exited(&self) -> bool {
        self.process_exit_status().is_some()
    }

    fn process_exit_status(&self) -> Option<std::process::ExitStatus> {
        self.child
            .lock()
            .ok()
            .and_then(|mut child| child.try_wait().ok())
            .flatten()
    }

    fn wait_until_ready(&self) -> Result<()> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        let mut last_error = None;
        while Instant::now() < deadline {
            if let Some(status) = self.process_exit_status() {
                let code = status
                    .code()
                    .map(|value| format!("{value} (0x{:08X})", value as u32))
                    .unwrap_or_else(|| status.to_string());
                let log_tail = read_bootstrap_log(&self.bootstrap_log_path);
                let runtime_hint = if log_tail.contains("You must install or update .NET")
                    || log_tail.contains("The framework 'Microsoft.NETCore.App'")
                    || log_tail.contains("Failed to load hostfxr")
                {
                    "\n提示：此引擎包依赖系统 .NET 8 x64 Runtime。请改用自包含发布包，或安装匹配的 .NET 8 Runtime。"
                } else {
                    ""
                };
                if log_tail.is_empty() {
                    bail!(
                        "SMB 引擎在初始化期间意外退出（退出状态：{code}）。\n启动日志：{}{}",
                        self.bootstrap_log_path.display(),
                        runtime_hint
                    );
                }
                bail!(
                    "SMB 引擎在初始化期间意外退出（退出状态：{code}）。\n启动日志：{}\n{}{}",
                    self.bootstrap_log_path.display(),
                    log_tail,
                    runtime_hint
                );
            }
            match self.request::<_, Value>("ping", &json!({})) {
                Ok(_) => return Ok(()),
                Err(error) => last_error = Some(error),
            }
            thread::sleep(REQUEST_RETRY_DELAY);
        }
        Err(last_error.unwrap_or_else(|| anyhow!("等待 SMB 引擎命名管道超时")))
    }

    fn request<P: Serialize + ?Sized, T: DeserializeOwned>(
        &self,
        command: &str,
        payload: &P,
    ) -> Result<T> {
        let _guard = self
            .request_gate
            .lock()
            .map_err(|_| anyhow!("IPC 请求锁已损坏"))?;
        let id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let envelope = BorrowedRequestEnvelope {
            version: IPC_PROTOCOL_VERSION,
            id,
            token: &self.token,
            command,
            payload,
        };
        let mut bytes = serde_json::to_vec(&envelope).context("无法序列化 IPC 请求")?;
        if bytes.len() > MAX_IPC_MESSAGE_BYTES {
            bytes.zeroize();
            bail!("IPC 请求超过 {} 字节限制", MAX_IPC_MESSAGE_BYTES);
        }

        let mut pipe = self.open_pipe()?;
        let write_result = (|| -> std::io::Result<()> {
            pipe.write_all(&(bytes.len() as u32).to_le_bytes())?;
            pipe.write_all(&bytes)?;
            pipe.flush()
        })();
        bytes.zeroize();
        write_result.context("写入 IPC 请求失败")?;

        let mut length = [0u8; 4];
        pipe.read_exact(&mut length).context("读取 IPC 响应长度失败")?;
        let length = u32::from_le_bytes(length) as usize;
        if length == 0 || length > MAX_IPC_MESSAGE_BYTES {
            bail!("IPC 响应长度 {length} 无效");
        }
        let mut response_bytes = vec![0u8; length];
        pipe.read_exact(&mut response_bytes)
            .context("读取 IPC 响应失败")?;
        let response: ResponseEnvelope =
            serde_json::from_slice(&response_bytes).context("IPC 响应 JSON 无效")?;
        self.validate_response(id, &response)?;
        serde_json::from_value(response.payload).context("IPC 响应数据类型不匹配")
    }

    fn open_pipe(&self) -> Result<File> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match OpenOptions::new()
                .read(true)
                .write(true)
                .open(&self.pipe_path)
            {
                Ok(file) => return Ok(file),
                Err(error)
                    if (matches!(
                        error.kind(),
                        ErrorKind::NotFound | ErrorKind::PermissionDenied | ErrorKind::WouldBlock
                    ) || error.raw_os_error() == Some(231))
                        && Instant::now() < deadline =>
                {
                    thread::sleep(REQUEST_RETRY_DELAY);
                }
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("无法连接命名管道 {}", self.pipe_path))
                }
            }
        }
    }

    fn validate_response(&self, expected_id: u64, response: &ResponseEnvelope) -> Result<()> {
        if response.version != IPC_PROTOCOL_VERSION {
            bail!("引擎 IPC 版本不匹配: {}", response.version);
        }
        if response.id != expected_id {
            bail!(
                "IPC 响应 ID 不匹配: 预期 {expected_id}，收到 {}",
                response.id
            );
        }
        if !response.ok {
            if let Some(error) = &response.error {
                bail!(
                    "{}: {}{}",
                    error.code,
                    error.message,
                    error
                        .detail
                        .as_deref()
                        .map(|detail| format!("\n{detail}"))
                        .unwrap_or_default()
                );
            }
            bail!("SMB 引擎返回未知错误");
        }
        Ok(())
    }
}

impl Drop for EngineClient {
    fn drop(&mut self) {
        if !self.process_has_exited() && !self.shutdown_sent.load(Ordering::Acquire) {
            let _ = self.shutdown();
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        if let Ok(mut child) = self.child.lock() {
            while Instant::now() < deadline {
                if child.try_wait().ok().flatten().is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(40));
            }
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        self.token.zeroize();
    }
}

#[derive(Serialize)]
struct BorrowedRequestEnvelope<'a, P: ?Sized> {
    version: u32,
    id: u64,
    token: &'a str,
    command: &'a str,
    payload: &'a P,
}

fn bootstrap_log(path: &Path) -> Result<File> {
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .context("无法打开引擎启动日志")
}

fn read_bootstrap_log(path: &Path) -> String {
    let Ok(bytes) = fs::read(path) else {
        return String::new();
    };
    let start = bytes.len().saturating_sub(STARTUP_LOG_TAIL_BYTES);
    String::from_utf8_lossy(&bytes[start..]).trim().to_owned()
}

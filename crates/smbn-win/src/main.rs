#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

#[cfg(windows)]
mod autostart;
#[cfg(windows)]
mod config_store;
#[cfg(windows)]
mod dpapi;
#[cfg(windows)]
mod engine;
mod icons;
#[cfg(windows)]
mod memory;
#[cfg(windows)]
mod paths;
#[cfg(windows)]
mod ui;
#[cfg(windows)]
mod util;

#[cfg(windows)]
fn install_panic_logger() {
    use std::backtrace::Backtrace;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    std::panic::set_hook(Box::new(|info| {
        let log_dir = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Smbn")
            .join("logs");
        let _ = fs::create_dir_all(&log_dir);

        let Ok(mut log) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("gui-panic.log"))
        else {
            return;
        };

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|value| value.as_secs_f64())
            .unwrap_or_default();
        let location = info
            .location()
            .map(|value| format!("{}:{}:{}", value.file(), value.line(), value.column()))
            .unwrap_or_else(|| "<unknown>".to_string());
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|value| (*value).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "<non-string panic payload>".to_string());

        let _ = writeln!(
            log,
            "unix_time={timestamp:.3} location={location} message={message}"
        );
        let _ = writeln!(log, "{}", Backtrace::force_capture());
        let _ = log.flush();
    }));
}

#[cfg(windows)]
fn main() {
    install_panic_logger();
    if let Err(error) = ui::run() {
        native_windows_gui::fatal_message("SMBN 启动失败", &error.to_string());
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("smbn is a Windows-only application");
}

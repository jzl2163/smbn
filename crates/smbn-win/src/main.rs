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
fn main() {
    if let Err(error) = ui::run() {
        native_windows_gui::fatal_message("SMBN 启动失败", &error.to_string());
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("smbn is a Windows-only application");
}

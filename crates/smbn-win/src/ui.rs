use crate::{autostart, config_store, dpapi, engine::EngineClient, memory, paths::AppPaths, util};
use anyhow::{anyhow, bail, Context, Result};
use native_windows_gui as nwg;
use smbn_core::{
    validate_config, AppConfig, AuthenticationMode, DiagnosticCheck, DiagnosticsResult, EngineConfig,
    EngineState, EngineStatus, IssueSeverity, ListenerConfig, LogLevel, PlainUser, SessionInfo,
    ShareConfig, Transport, UserConfig,
};
use std::cell::{Cell, RefCell};
use std::env;
use std::process::Command;
use std::rc::Rc;
use std::time::Duration;
use zeroize::Zeroize;

const WINDOW_WIDTH: i32 = 1120;
const WINDOW_HEIGHT: i32 = 800;

pub fn run() -> Result<()> {
    nwg::init().context("初始化 Win32 图形界面失败")?;
    nwg::Font::set_global_family("Segoe UI").context("设置界面字体失败")?;

    let paths = AppPaths::discover().context("初始化应用数据目录失败")?;
    let config = config_store::load(&paths).context("读取配置失败")?;
    let start_minimized = env::args().any(|arg| arg.eq_ignore_ascii_case("--minimized"));
    let (engine, engine_error) = match EngineClient::launch(&paths) {
        Ok(engine) => (Some(engine), None),
        Err(error) => (None, Some(error.to_string())),
    };

    let ui = SmbnApp::build(paths, config, engine, start_minimized)?;
    ui.inner.poll_status(true);
    if let Some(error) = engine_error {
        ui.inner.set_footer(format!("引擎尚不可用：{error}"));
    }
    if ui.inner.config.borrow().app.start_server_on_launch {
        ui.inner.start_server(false);
    }
    if start_minimized {
        ui.inner.hide_to_tray(false);
    }

    nwg::dispatch_thread_events();
    Ok(())
}

#[derive(Default)]
struct Controls {
    app_icon: nwg::Icon,
    tray_stopped_icon: nwg::Icon,
    tray_running_icon: nwg::Icon,
    window: nwg::Window,
    tray: nwg::TrayNotification,
    tray_menu: nwg::Menu,
    tray_open: nwg::MenuItem,
    tray_start: nwg::MenuItem,
    tray_stop: nwg::MenuItem,
    tray_exit: nwg::MenuItem,
    timer: nwg::AnimationTimer,

    header_state: nwg::Label,
    header_detail: nwg::Label,
    tabs: nwg::TabsContainer,
    server_tab: nwg::Tab,
    listeners_tab: nwg::Tab,
    shares_tab: nwg::Tab,
    users_tab: nwg::Tab,
    options_tab: nwg::Tab,
    monitor_tab: nwg::Tab,

    netbios_label: nwg::Label,
    netbios_input: nwg::TextInput,
    workgroup_label: nwg::Label,
    workgroup_input: nwg::TextInput,
    auth_label: nwg::Label,
    auth_combo: nwg::ComboBox<String>,
    smb1_check: nwg::CheckBox,
    smb2_check: nwg::CheckBox,
    smb3_check: nwg::CheckBox,
    inactivity_label: nwg::Label,
    inactivity_input: nwg::TextInput,
    allow_label: nwg::Label,
    allow_box: nwg::TextBox,
    reject_label: nwg::Label,
    reject_box: nwg::TextBox,
    server_help: nwg::Label,

    listeners_list: nwg::ListView,
    listener_id_label: nwg::Label,
    listener_id_input: nwg::TextInput,
    listener_address_label: nwg::Label,
    listener_address_input: nwg::TextInput,
    listener_port_label: nwg::Label,
    listener_port_input: nwg::TextInput,
    listener_transport_label: nwg::Label,
    listener_transport_combo: nwg::ComboBox<String>,
    listener_enabled_check: nwg::CheckBox,
    listener_nbns_check: nwg::CheckBox,
    listener_new_button: nwg::Button,
    listener_apply_button: nwg::Button,
    listener_delete_button: nwg::Button,
    listener_help: nwg::Label,

    shares_list: nwg::ListView,
    share_id_label: nwg::Label,
    share_id_input: nwg::TextInput,
    share_name_label: nwg::Label,
    share_name_input: nwg::TextInput,
    share_path_label: nwg::Label,
    share_path_input: nwg::TextInput,
    share_comment_label: nwg::Label,
    share_comment_input: nwg::TextInput,
    share_read_label: nwg::Label,
    share_read_box: nwg::TextBox,
    share_write_label: nwg::Label,
    share_write_box: nwg::TextBox,
    share_enabled_check: nwg::CheckBox,
    share_hidden_check: nwg::CheckBox,
    share_readonly_check: nwg::CheckBox,
    share_new_button: nwg::Button,
    share_apply_button: nwg::Button,
    share_delete_button: nwg::Button,

    users_list: nwg::ListView,
    user_id_label: nwg::Label,
    user_id_input: nwg::TextInput,
    user_name_label: nwg::Label,
    user_name_input: nwg::TextInput,
    user_password_label: nwg::Label,
    user_password_input: nwg::TextInput,
    user_enabled_check: nwg::CheckBox,
    user_new_button: nwg::Button,
    user_apply_button: nwg::Button,
    user_delete_button: nwg::Button,
    user_help: nwg::Label,

    startup_check: nwg::CheckBox,
    start_server_check: nwg::CheckBox,
    minimize_tray_check: nwg::CheckBox,
    close_tray_check: nwg::CheckBox,
    light_mode_check: nwg::CheckBox,
    confirm_exit_check: nwg::CheckBox,
    log_level_label: nwg::Label,
    log_level_combo: nwg::ComboBox<String>,
    log_size_label: nwg::Label,
    log_size_input: nwg::TextInput,
    log_files_label: nwg::Label,
    log_files_input: nwg::TextInput,
    log_tail_label: nwg::Label,
    log_tail_input: nwg::TextInput,
    open_data_button: nwg::Button,
    options_help: nwg::Label,

    monitor_summary: nwg::Label,
    sessions_list: nwg::ListView,
    refresh_button: nwg::Button,
    terminate_button: nwg::Button,
    diagnostics_button: nwg::Button,
    trim_button: nwg::Button,
    diagnostics_box: nwg::TextBox,
    log_box: nwg::TextBox,

    save_button: nwg::Button,
    start_button: nwg::Button,
    stop_button: nwg::Button,
    hide_button: nwg::Button,
    exit_button: nwg::Button,
    footer_status: nwg::Label,
}

struct SmbnApp {
    controls: Controls,
    paths: AppPaths,
    config: RefCell<AppConfig>,
    engine: RefCell<Option<EngineClient>>,
    status: RefCell<EngineStatus>,
    sessions: RefCell<Vec<SessionInfo>>,
    hidden: Cell<bool>,
    closing: Cell<bool>,
    tick: Cell<u32>,
}

struct SmbnUi {
    inner: Rc<SmbnApp>,
    handlers: RefCell<Vec<nwg::EventHandler>>,
}

include!("ui_build_shell.inc.rs");
include!("ui_build_tabs.inc.rs");
include!("ui_load.inc.rs");
include!("ui_runtime.inc.rs");
include!("ui_editors.inc.rs");
include!("ui_events.inc.rs");

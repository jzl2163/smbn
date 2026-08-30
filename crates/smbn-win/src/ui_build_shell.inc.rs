impl SmbnApp {
    fn build(
        paths: AppPaths,
        config: AppConfig,
        engine: Option<EngineClient>,
        start_minimized: bool,
    ) -> Result<SmbnUi> {
        let mut c = Controls::default();
        Self::build_resources(&mut c, &paths)?;
        Self::build_shell(&mut c, start_minimized)?;
        Self::build_server_tab(&mut c)?;
        Self::build_listeners_tab(&mut c)?;
        Self::build_shares_tab(&mut c)?;
        Self::build_users_tab(&mut c)?;
        Self::build_options_tab(&mut c)?;
        Self::build_monitor_tab(&mut c)?;
        Self::configure_lists(&c);

        let app = Rc::new(Self {
            controls: c,
            paths,
            config: RefCell::new(config),
            engine: RefCell::new(engine),
            status: RefCell::new(EngineStatus::default()),
            sessions: RefCell::new(Vec::new()),
            hidden: Cell::new(false),
            closing: Cell::new(false),
            tick: Cell::new(0),
        });
        app.load_config_into_controls();

        let ui = SmbnUi { inner: app, handlers: RefCell::new(Vec::new()) };
        ui.bind_events();
        Ok(ui)
    }

    fn build_resources(c: &mut Controls, paths: &AppPaths) -> Result<()> {
        let (app_icon, stopped_icon, running_icon) = crate::icons::ensure_runtime_icons(&paths.data_dir)?;
        nwg::Icon::builder().source_file(Some(&app_icon.to_string_lossy())).build(&mut c.app_icon)?;
        nwg::Icon::builder().source_file(Some(&stopped_icon.to_string_lossy())).build(&mut c.tray_stopped_icon)?;
        nwg::Icon::builder().source_file(Some(&running_icon.to_string_lossy())).build(&mut c.tray_running_icon)?;
        Ok(())
    }

    fn build_shell(c: &mut Controls, start_minimized: bool) -> Result<()> {
        let flags = if start_minimized {
            nwg::WindowFlags::WINDOW | nwg::WindowFlags::MINIMIZE_BOX
        } else {
            nwg::WindowFlags::WINDOW | nwg::WindowFlags::MINIMIZE_BOX | nwg::WindowFlags::VISIBLE
        };
        nwg::Window::builder()
            .flags(flags)
            .size((WINDOW_WIDTH, WINDOW_HEIGHT))
            .position((120, 80))
            .title("SMBN - SMBLibrary 服务器管理器")
            .icon(Some(&c.app_icon))
            .build(&mut c.window)?;

        nwg::TrayNotification::builder()
            .parent(&c.window)
            .icon(Some(&c.tray_stopped_icon))
            .tip(Some("SMBN - 服务未运行"))
            .build(&mut c.tray)?;
        nwg::Menu::builder().popup(true).parent(&c.window).build(&mut c.tray_menu)?;
        nwg::MenuItem::builder().text("打开控制台").parent(&c.tray_menu).build(&mut c.tray_open)?;
        nwg::MenuItem::builder().text("启动 SMB 服务").parent(&c.tray_menu).build(&mut c.tray_start)?;
        nwg::MenuItem::builder().text("停止 SMB 服务").parent(&c.tray_menu).build(&mut c.tray_stop)?;
        nwg::MenuItem::builder().text("退出").parent(&c.tray_menu).build(&mut c.tray_exit)?;

        nwg::Label::builder().text("●  未运行").position((18, 12)).size((180, 26)).parent(&c.window).build(&mut c.header_state)?;
        nwg::Label::builder().text("正在读取引擎状态……").position((205, 12)).size((880, 26)).parent(&c.window).build(&mut c.header_detail)?;
        nwg::TabsContainer::builder().position((14, 45)).size((1080, 650)).parent(&c.window).build(&mut c.tabs)?;
        nwg::Tab::builder().text("服务器").parent(&c.tabs).build(&mut c.server_tab)?;
        nwg::Tab::builder().text("监听器").parent(&c.tabs).build(&mut c.listeners_tab)?;
        nwg::Tab::builder().text("共享").parent(&c.tabs).build(&mut c.shares_tab)?;
        nwg::Tab::builder().text("用户").parent(&c.tabs).build(&mut c.users_tab)?;
        nwg::Tab::builder().text("应用设置").parent(&c.tabs).build(&mut c.options_tab)?;
        nwg::Tab::builder().text("监控与诊断").parent(&c.tabs).build(&mut c.monitor_tab)?;

        nwg::Button::builder().text("保存配置").position((15, 710)).size((115, 34)).parent(&c.window).build(&mut c.save_button)?;
        nwg::Button::builder().text("启动服务").position((140, 710)).size((115, 34)).parent(&c.window).build(&mut c.start_button)?;
        nwg::Button::builder().text("停止服务").position((265, 710)).size((115, 34)).parent(&c.window).build(&mut c.stop_button)?;
        nwg::Button::builder().text("隐藏到托盘").position((390, 710)).size((125, 34)).parent(&c.window).build(&mut c.hide_button)?;
        nwg::Button::builder().text("退出").position((525, 710)).size((90, 34)).parent(&c.window).build(&mut c.exit_button)?;
        nwg::Label::builder().text("就绪").position((630, 715)).size((455, 28)).parent(&c.window).build(&mut c.footer_status)?;

        nwg::AnimationTimer::builder()
            .parent(&c.window)
            .interval(Duration::from_secs(1))
            .active(true)
            .build(&mut c.timer)?;
        Ok(())
    }

    fn build_server_tab(c: &mut Controls) -> Result<()> {
        nwg::Label::builder().text("NetBIOS 名称").position((25, 25)).size((135, 25)).parent(&c.server_tab).build(&mut c.netbios_label)?;
        nwg::TextInput::builder().position((165, 22)).size((240, 28)).limit(15).parent(&c.server_tab).build(&mut c.netbios_input)?;
        nwg::Label::builder().text("工作组").position((455, 25)).size((100, 25)).parent(&c.server_tab).build(&mut c.workgroup_label)?;
        nwg::TextInput::builder().position((555, 22)).size((240, 28)).limit(15).parent(&c.server_tab).build(&mut c.workgroup_input)?;
        nwg::Label::builder().text("认证模式").position((25, 70)).size((135, 25)).parent(&c.server_tab).build(&mut c.auth_label)?;
        nwg::ComboBox::builder()
            .position((165, 66)).size((300, 180))
            .collection(vec!["独立账户（DPAPI 保存密码）".to_owned(), "Windows 集成认证".to_owned()])
            .selected_index(Some(0)).parent(&c.server_tab).build(&mut c.auth_combo)?;
        nwg::CheckBox::builder().text("启用 SMB1（不推荐）").position((25, 115)).size((200, 28)).parent(&c.server_tab).build(&mut c.smb1_check)?;
        nwg::CheckBox::builder().text("启用 SMB2").position((250, 115)).size((150, 28)).parent(&c.server_tab).build(&mut c.smb2_check)?;
        nwg::CheckBox::builder().text("启用 SMB3").position((420, 115)).size((150, 28)).parent(&c.server_tab).build(&mut c.smb3_check)?;
        nwg::Label::builder().text("空闲保活秒数（0=关闭）").position((25, 160)).size((190, 25)).parent(&c.server_tab).build(&mut c.inactivity_label)?;
        nwg::TextInput::builder().position((220, 156)).size((150, 28)).parent(&c.server_tab).build(&mut c.inactivity_input)?;
        nwg::Label::builder().text("允许的远程 CIDR（每行一个；空=允许全部）").position((25, 210)).size((450, 25)).parent(&c.server_tab).build(&mut c.allow_label)?;
        nwg::TextBox::builder().position((25, 238)).size((480, 220)).parent(&c.server_tab).build(&mut c.allow_box)?;
        nwg::Label::builder().text("拒绝的远程 CIDR（优先于允许列表）").position((540, 210)).size((420, 25)).parent(&c.server_tab).build(&mut c.reject_label)?;
        nwg::TextBox::builder().position((540, 238)).size((480, 220)).parent(&c.server_tab).build(&mut c.reject_box)?;
        nwg::Label::builder()
            .text("说明：SMBLibrary 将 SMB2 与 SMB3 分别作为功能开关；IPv6 必须使用 Direct TCP。NetBIOS 名称服务仅能绑定具体 IPv4 地址。")
            .position((25, 485)).size((995, 55)).parent(&c.server_tab).build(&mut c.server_help)?;
        Ok(())
    }

    fn build_listeners_tab(c: &mut Controls) -> Result<()> {
        nwg::ListView::builder().position((20, 18)).size((1015, 270)).parent(&c.listeners_tab).build(&mut c.listeners_list)?;
        nwg::Label::builder().text("监听器 ID").position((25, 315)).size((110, 25)).parent(&c.listeners_tab).build(&mut c.listener_id_label)?;
        nwg::TextInput::builder().position((140, 311)).size((245, 28)).parent(&c.listeners_tab).build(&mut c.listener_id_input)?;
        nwg::Label::builder().text("监听 IP").position((420, 315)).size((90, 25)).parent(&c.listeners_tab).build(&mut c.listener_address_label)?;
        nwg::TextInput::builder().position((515, 311)).size((235, 28)).parent(&c.listeners_tab).build(&mut c.listener_address_input)?;
        nwg::Label::builder().text("端口").position((780, 315)).size((60, 25)).parent(&c.listeners_tab).build(&mut c.listener_port_label)?;
        nwg::TextInput::builder().position((840, 311)).size((120, 28)).parent(&c.listeners_tab).build(&mut c.listener_port_input)?;
        nwg::Label::builder().text("传输方式").position((25, 365)).size((110, 25)).parent(&c.listeners_tab).build(&mut c.listener_transport_label)?;
        nwg::ComboBox::builder()
            .position((140, 361)).size((245, 160))
            .collection(vec!["Direct TCP".to_owned(), "NetBIOS over TCP".to_owned()])
            .selected_index(Some(0)).parent(&c.listeners_tab).build(&mut c.listener_transport_combo)?;
        nwg::CheckBox::builder().text("启用监听器").position((420, 360)).size((150, 30)).parent(&c.listeners_tab).build(&mut c.listener_enabled_check)?;
        nwg::CheckBox::builder().text("启用 UDP/137 NetBIOS 名称服务").position((590, 360)).size((300, 30)).parent(&c.listeners_tab).build(&mut c.listener_nbns_check)?;
        nwg::Button::builder().text("新建").position((25, 420)).size((105, 32)).parent(&c.listeners_tab).build(&mut c.listener_new_button)?;
        nwg::Button::builder().text("添加 / 更新").position((140, 420)).size((135, 32)).parent(&c.listeners_tab).build(&mut c.listener_apply_button)?;
        nwg::Button::builder().text("删除所选").position((285, 420)).size((120, 32)).parent(&c.listeners_tab).build(&mut c.listener_delete_button)?;
        nwg::Label::builder()
            .text("IPv4 通配地址：0.0.0.0；IPv6 通配地址：::。自定义端口可用于专用客户端，但 Windows 资源管理器的 UNC 路径不能直接写端口。")
            .position((25, 485)).size((995, 55)).parent(&c.listeners_tab).build(&mut c.listener_help)?;
        Ok(())
    }

}

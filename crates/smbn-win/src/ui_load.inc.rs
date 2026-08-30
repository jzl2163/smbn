impl SmbnApp {
    fn configure_lists(c: &Controls) {
        configure_list(&c.listeners_list, &[("启用", 60), ("ID", 170), ("地址", 210), ("端口", 75), ("传输", 145), ("NBNS", 70)]);
        configure_list(&c.shares_list, &[("启用", 60), ("共享名", 170), ("路径", 430), ("只读", 70), ("隐藏", 70), ("备注", 180)]);
        configure_list(&c.users_list, &[("启用", 70), ("账户名", 260), ("密码", 180), ("ID", 300)]);
        configure_list(&c.sessions_list, &[("监听器", 150), ("客户端", 210), ("协议", 120), ("用户", 180), ("设备", 170), ("打开文件", 90)]);
    }

    fn load_config_into_controls(&self) {
        let config = self.config.borrow().clone();
        self.controls.netbios_input.set_text(&config.server.netbios_name);
        self.controls.workgroup_input.set_text(&config.server.workgroup);
        self.controls.auth_combo.set_selection(Some(match config.server.authentication {
            AuthenticationMode::Independent => 0,
            AuthenticationMode::IntegratedWindows => 1,
        }));
        set_checked(&self.controls.smb1_check, config.server.enable_smb1);
        set_checked(&self.controls.smb2_check, config.server.enable_smb2);
        set_checked(&self.controls.smb3_check, config.server.enable_smb3);
        self.controls.inactivity_input.set_text(&config.server.inactivity_timeout_seconds.to_string());
        self.controls.allow_box.set_text(&config.server.allow_remote_subnets.join("\r\n"));
        self.controls.reject_box.set_text(&config.server.reject_remote_subnets.join("\r\n"));

        set_checked(&self.controls.startup_check, config.app.start_with_windows);
        set_checked(&self.controls.start_server_check, config.app.start_server_on_launch);
        set_checked(&self.controls.minimize_tray_check, config.app.minimize_to_tray);
        set_checked(&self.controls.close_tray_check, config.app.close_to_tray);
        set_checked(&self.controls.light_mode_check, config.app.light_mode);
        set_checked(&self.controls.confirm_exit_check, config.app.confirm_exit_while_running);
        self.controls.log_level_combo.set_selection(Some(log_level_index(config.logging.level)));
        self.controls.log_size_input.set_text(&config.logging.max_file_mib.to_string());
        self.controls.log_files_input.set_text(&config.logging.retained_files.to_string());
        self.controls.log_tail_input.set_text(&config.logging.gui_tail_lines.to_string());

        self.refresh_listener_list();
        self.refresh_share_list();
        self.refresh_user_list();
        self.clear_listener_editor();
        self.clear_share_editor();
        self.clear_user_editor();
    }
}

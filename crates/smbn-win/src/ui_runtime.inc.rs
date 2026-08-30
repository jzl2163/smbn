impl SmbnApp {
    fn commit_global_fields(&self) -> Result<()> {
        let inactivity = parse_number::<u64>(&self.controls.inactivity_input.text(), "空闲保活秒数")?;
        let max_file_mib = parse_number::<u32>(&self.controls.log_size_input.text(), "单文件日志上限")?;
        let retained_files = parse_number::<u32>(&self.controls.log_files_input.text(), "日志保留数量")?;
        let gui_tail_lines = parse_number::<usize>(&self.controls.log_tail_input.text(), "界面日志行数")?;

        let mut config = self.config.borrow_mut();
        config.server.netbios_name = self.controls.netbios_input.text().trim().to_owned();
        config.server.workgroup = self.controls.workgroup_input.text().trim().to_owned();
        config.server.authentication = if self.controls.auth_combo.selection() == Some(1) {
            AuthenticationMode::IntegratedWindows
        } else {
            AuthenticationMode::Independent
        };
        config.server.enable_smb1 = is_checked(&self.controls.smb1_check);
        config.server.enable_smb2 = is_checked(&self.controls.smb2_check);
        config.server.enable_smb3 = is_checked(&self.controls.smb3_check);
        config.server.inactivity_timeout_seconds = inactivity;
        config.server.allow_remote_subnets = util::parse_principals(&self.controls.allow_box.text());
        config.server.reject_remote_subnets = util::parse_principals(&self.controls.reject_box.text());

        config.app.start_with_windows = is_checked(&self.controls.startup_check);
        config.app.start_server_on_launch = is_checked(&self.controls.start_server_check);
        config.app.minimize_to_tray = is_checked(&self.controls.minimize_tray_check);
        config.app.close_to_tray = is_checked(&self.controls.close_tray_check);
        config.app.light_mode = is_checked(&self.controls.light_mode_check);
        config.app.confirm_exit_while_running = is_checked(&self.controls.confirm_exit_check);
        config.logging.level = index_log_level(self.controls.log_level_combo.selection().unwrap_or(2));
        config.logging.max_file_mib = max_file_mib;
        config.logging.retained_files = retained_files;
        config.logging.gui_tail_lines = gui_tail_lines;
        Ok(())
    }

    fn save_all(&self, show_success: bool) -> Result<()> {
        self.commit_global_fields()?;
        let config = self.config.borrow().clone();
        let issues = validate_config(&config);
        let errors = issues.iter().filter(|item| item.severity == IssueSeverity::Error).collect::<Vec<_>>();
        if !errors.is_empty() {
            bail!("配置存在错误：\n{}", errors.iter().map(|item| format!("• {}：{}", item.path, item.message)).collect::<Vec<_>>().join("\n"));
        }
        config_store::save(&self.paths, &config)?;
        let executable = env::current_exe().context("无法获取程序路径")?;
        autostart::set_enabled(config.app.start_with_windows, &executable)?;
        if show_success {
            let warnings = issues.iter().filter(|item| item.severity == IssueSeverity::Warning).count();
            self.set_footer(if warnings == 0 { "配置已保存".to_owned() } else { format!("配置已保存（{warnings} 条警告）") });
        }
        Ok(())
    }

    fn build_engine_config(&self, include_credentials: bool) -> Result<EngineConfig> {
        self.commit_global_fields()?;
        let config = self.config.borrow().clone();
        let errors = validate_config(&config)
            .into_iter()
            .filter(|item| item.severity == IssueSeverity::Error)
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            bail!("配置存在错误：\n{}", errors.iter().map(|item| format!("• {}：{}", item.path, item.message)).collect::<Vec<_>>().join("\n"));
        }

        let mut users = Vec::new();
        if config.server.authentication == AuthenticationMode::Independent {
            for user in config.users.iter().filter(|item| item.enabled) {
                let password = if include_credentials {
                    dpapi::unprotect(&user.protected_password)
                        .with_context(|| format!("无法解密用户 {} 的密码", user.account_name))?
                } else {
                    // Diagnostics only need a non-empty placeholder for structural validation.
                    // The real DPAPI secret never crosses the pipe for a diagnostic request.
                    "__diagnostic_redacted__".to_owned()
                };
                users.push(PlainUser { account_name: user.account_name.clone(), password });
            }
        }
        Ok(EngineConfig {
            server: config.server,
            listeners: config.listeners,
            shares: config.shares,
            users,
            logging: config.logging,
        })
    }

    fn ensure_engine(&self) -> Result<()> {
        let should_launch = self.engine.borrow().as_ref().map(EngineClient::process_has_exited).unwrap_or(true);
        if should_launch {
            *self.engine.borrow_mut() = Some(EngineClient::launch(&self.paths)?);
        }
        Ok(())
    }

    fn start_server(&self, interactive: bool) {
        let result = (|| -> Result<()> {
            self.save_all(false)?;
            let engine_config = self.build_engine_config(true)?;
            self.ensure_engine()?;
            self.engine.borrow().as_ref().ok_or_else(|| anyhow!("SMB 引擎不可用"))?.start(engine_config)?;
            Ok(())
        })();
        match result {
            Ok(()) => {
                self.set_footer("SMB 服务已启动".to_owned());
                self.poll_status(true);
            }
            Err(error) => {
                self.set_footer(format!("启动失败：{error}"));
                if interactive {
                    nwg::modal_error_message(&self.controls.window, "启动 SMB 服务失败", &error.to_string());
                }
            }
        }
    }

    fn stop_server(&self, interactive: bool) {
        let result = self.engine.borrow().as_ref().map_or(Ok(()), EngineClient::stop);
        match result {
            Ok(()) => {
                self.set_footer("SMB 服务已停止".to_owned());
                self.poll_status(true);
            }
            Err(error) => {
                self.set_footer(format!("停止失败：{error}"));
                if interactive {
                    nwg::modal_error_message(&self.controls.window, "停止 SMB 服务失败", &error.to_string());
                }
            }
        }
    }

    fn poll_status(&self, force_heavy: bool) {
        let tick = self.tick.get().wrapping_add(1);
        self.tick.set(tick);
        let hidden_light = self.hidden.get() && self.config.borrow().app.light_mode;
        if hidden_light && !force_heavy && tick % 5 != 0 {
            return;
        }

        let status_result = self.engine.borrow().as_ref().map(EngineClient::status);
        match status_result {
            Some(Ok(status)) => {
                *self.status.borrow_mut() = status.clone();
                self.render_status(&status);
                if !hidden_light && (force_heavy || self.controls.tabs.selected_tab() == 5) {
                    self.refresh_monitor_data();
                }
            }
            Some(Err(error)) => {
                let mut status = EngineStatus::default();
                status.state = EngineState::Faulted;
                status.last_error = Some(error.to_string());
                *self.status.borrow_mut() = status.clone();
                self.render_status(&status);
            }
            None => {
                let mut status = EngineStatus::default();
                status.state = EngineState::Faulted;
                status.last_error = Some("SMB 引擎尚未启动".to_owned());
                *self.status.borrow_mut() = status.clone();
                self.render_status(&status);
            }
        }
    }

    fn render_status(&self, status: &EngineStatus) {
        let running = matches!(status.state, EngineState::Starting | EngineState::Running | EngineState::Stopping);
        let state_text = match status.state {
            EngineState::Stopped => "未运行",
            EngineState::Starting => "正在启动",
            EngineState::Running => "正在运行",
            EngineState::Stopping => "正在停止",
            EngineState::Faulted => "故障",
        };
        self.controls.header_state.set_text(if running { "●  正在运行" } else { "○  未运行" });
        let detail = if status.state == EngineState::Running {
            format!(
                "监听器 {}  |  共享 {}  |  会话 {}  |  打开文件 {}  |  运行 {}",
                status.listener_count,
                status.share_count,
                status.session_count,
                status.open_file_count,
                util::format_duration(status.uptime_seconds)
            )
        } else if let Some(error) = &status.last_error {
            format!("{state_text}：{error}")
        } else {
            state_text.to_owned()
        };
        self.controls.header_detail.set_text(&detail);
        self.controls.monitor_summary.set_text(&format!(
            "状态：{state_text}    引擎：{}    SMBLibrary：{}    丢弃日志：{}",
            status.engine_version, status.smblibrary_version, status.dropped_log_entries
        ));
        self.controls.start_button.set_enabled(!running);
        self.controls.stop_button.set_enabled(running);
        self.controls.tray_start.set_enabled(!running);
        self.controls.tray_stop.set_enabled(running);
        self.controls.tray.set_icon(if running { &self.controls.tray_running_icon } else { &self.controls.tray_stopped_icon });
        self.controls.tray.set_tip(&format!(
            "SMBN - {state_text}\n监听器:{}  会话:{}  打开文件:{}",
            status.listener_count, status.session_count, status.open_file_count
        ));
    }

    fn refresh_monitor_data(&self) {
        let engine_ref = self.engine.borrow();
        let Some(engine) = engine_ref.as_ref() else { return; };
        if let Ok(sessions) = engine.sessions() {
            *self.sessions.borrow_mut() = sessions;
            self.refresh_session_list();
        }
        let maximum = self.config.borrow().logging.gui_tail_lines;
        if let Ok(logs) = engine.tail_logs(maximum) {
            self.controls.log_box.set_text(&logs.lines.join("\r\n"));
        }
    }

    fn run_diagnostics(&self) {
        let result = (|| -> Result<DiagnosticsResult> {
            let config = self.build_engine_config(false)?;
            self.ensure_engine()?;
            self.engine.borrow().as_ref().ok_or_else(|| anyhow!("SMB 引擎不可用"))?.diagnostics(&config)
        })();
        match result {
            Ok(result) => {
                self.controls.diagnostics_box.set_text(&format_diagnostics(&result.checks));
                self.set_footer(format!("诊断完成：{} 项", result.checks.len()));
            }
            Err(error) => nwg::modal_error_message(&self.controls.window, "诊断失败", &error.to_string()),
        }
    }

    fn trim_memory(&self) {
        if let Some(engine) = self.engine.borrow().as_ref() {
            let _ = engine.trim_memory();
        }
        self.controls.log_box.set_text("");
        self.controls.diagnostics_box.set_text("");
        memory::trim_working_set();
        self.set_footer("已请求 GUI 与引擎回收可释放内存".to_owned());
    }

    fn hide_to_tray(&self, show_notice: bool) {
        self.controls.window.set_visible(false);
        self.hidden.set(true);
        if self.config.borrow().app.light_mode {
            self.controls.log_box.set_text("");
            self.controls.diagnostics_box.set_text("");
            self.controls.sessions_list.clear();
            if let Some(engine) = self.engine.borrow().as_ref() {
                let _ = engine.trim_memory();
            }
            memory::trim_working_set();
        }
        if show_notice {
            self.controls.tray.show(
                "SMBN 仍在后台运行；右键托盘图标可打开、启停服务或退出。",
                Some("SMBN"),
                Some(nwg::TrayNotificationFlags::INFO_ICON | nwg::TrayNotificationFlags::QUIET),
                None,
            );
        }
    }

    fn show_window(&self) {
        self.controls.window.set_visible(true);
        self.controls.window.restore();
        self.controls.window.set_focus();
        self.hidden.set(false);
        self.poll_status(true);
    }

    fn request_exit(&self) {
        if self.closing.get() {
            return;
        }
        let running = matches!(self.status.borrow().state, EngineState::Starting | EngineState::Running | EngineState::Stopping);
        if running && self.config.borrow().app.confirm_exit_while_running {
            let params = nwg::MessageParams {
                title: "退出 SMBN",
                content: "SMB 服务仍在运行。退出会先停止服务并断开现有客户端。确定退出吗？",
                buttons: nwg::MessageButtons::YesNo,
                icons: nwg::MessageIcons::Warning,
            };
            let choice = nwg::modal_message(&self.controls.window, &params);
            if choice != nwg::MessageChoice::Yes {
                return;
            }
        }
        self.closing.set(true);
        if running {
            let _ = self.engine.borrow().as_ref().map(EngineClient::stop);
        }
        if let Some(engine) = self.engine.borrow().as_ref() {
            let _ = engine.shutdown();
        }
        nwg::stop_thread_dispatch();
    }

    fn handle_window_close(&self) {
        if self.closing.get() {
            nwg::stop_thread_dispatch();
        } else if self.config.borrow().app.close_to_tray {
            self.hide_to_tray(true);
        } else {
            self.request_exit();
        }
    }

    fn handle_window_minimize(&self) {
        if self.config.borrow().app.minimize_to_tray {
            self.hide_to_tray(false);
        }
    }

    fn show_tray_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.controls.tray_menu.popup(x, y);
    }

    fn save_clicked(&self) {
        if let Err(error) = self.save_all(true) {
            nwg::modal_error_message(&self.controls.window, "保存配置失败", &error.to_string());
        }
    }

    fn open_data_directory(&self) {
        if let Err(error) = Command::new("explorer.exe").arg(&self.paths.data_dir).spawn() {
            nwg::modal_error_message(&self.controls.window, "打开目录失败", &error.to_string());
        }
    }

}

impl SmbnApp {
    fn clear_listener_editor(&self) {
        self.controls.listener_id_input.set_text("");
        self.controls.listener_address_input.set_text("0.0.0.0");
        self.controls.listener_port_input.set_text("445");
        self.controls.listener_transport_combo.set_selection(Some(0));
        set_checked(&self.controls.listener_enabled_check, true);
        set_checked(&self.controls.listener_nbns_check, false);
    }

    fn load_selected_listener(&self) {
        let Some(index) = self.controls.listeners_list.selected_item() else { return; };
        let config = self.config.borrow();
        let Some(item) = config.listeners.get(index) else { return; };
        self.controls.listener_id_input.set_text(&item.id);
        self.controls.listener_address_input.set_text(&item.address);
        self.controls.listener_port_input.set_text(&item.port.to_string());
        self.controls.listener_transport_combo.set_selection(Some(match item.transport { Transport::DirectTcp => 0, Transport::NetbiosOverTcp => 1 }));
        set_checked(&self.controls.listener_enabled_check, item.enabled);
        set_checked(&self.controls.listener_nbns_check, item.netbios_name_service);
    }

    fn apply_listener(&self) {
        let result = (|| -> Result<()> {
            let mut id = self.controls.listener_id_input.text().trim().to_owned();
            if id.is_empty() { id = util::new_id("listener"); }
            let address = self.controls.listener_address_input.text().trim().to_owned();
            let port = parse_number::<u16>(&self.controls.listener_port_input.text(), "监听端口")?;
            let item = ListenerConfig {
                id: id.clone(),
                address,
                port,
                transport: if self.controls.listener_transport_combo.selection() == Some(1) { Transport::NetbiosOverTcp } else { Transport::DirectTcp },
                netbios_name_service: is_checked(&self.controls.listener_nbns_check),
                enabled: is_checked(&self.controls.listener_enabled_check),
            };
            let mut config = self.config.borrow_mut();
            if let Some(index) = config.listeners.iter().position(|existing| existing.id.eq_ignore_ascii_case(&id)) {
                config.listeners[index] = item;
            } else {
                config.listeners.push(item);
            }
            drop(config);
            self.refresh_listener_list();
            self.clear_listener_editor();
            Ok(())
        })();
        if let Err(error) = result {
            nwg::modal_error_message(&self.controls.window, "监听器设置无效", &error.to_string());
        }
    }

    fn delete_listener(&self) {
        let Some(index) = self.controls.listeners_list.selected_item() else { return; };
        if index < self.config.borrow().listeners.len() {
            self.config.borrow_mut().listeners.remove(index);
            self.refresh_listener_list();
            self.clear_listener_editor();
        }
    }

    fn refresh_listener_list(&self) {
        self.controls.listeners_list.set_redraw(false);
        self.controls.listeners_list.clear();
        for item in &self.config.borrow().listeners {
            let enabled = yes_no(item.enabled);
            let port = item.port.to_string();
            let transport = match item.transport { Transport::DirectTcp => "Direct TCP", Transport::NetbiosOverTcp => "NetBIOS/TCP" };
            let nbns = yes_no(item.netbios_name_service);
            self.controls.listeners_list.insert_items_row(None, &[enabled, item.id.as_str(), item.address.as_str(), port.as_str(), transport, nbns]);
        }
        self.controls.listeners_list.set_redraw(true);
    }

    fn clear_share_editor(&self) {
        self.controls.share_id_input.set_text("");
        self.controls.share_name_input.set_text("");
        self.controls.share_path_input.set_text("");
        self.controls.share_comment_input.set_text("");
        self.controls.share_read_box.set_text("Users");
        self.controls.share_write_box.set_text("Users");
        set_checked(&self.controls.share_enabled_check, true);
        set_checked(&self.controls.share_hidden_check, false);
        set_checked(&self.controls.share_readonly_check, false);
    }

    fn load_selected_share(&self) {
        let Some(index) = self.controls.shares_list.selected_item() else { return; };
        let config = self.config.borrow();
        let Some(item) = config.shares.get(index) else { return; };
        self.controls.share_id_input.set_text(&item.id);
        self.controls.share_name_input.set_text(&item.name);
        self.controls.share_path_input.set_text(&item.path);
        self.controls.share_comment_input.set_text(&item.comment);
        self.controls.share_read_box.set_text(&item.read_access.join("\r\n"));
        self.controls.share_write_box.set_text(&item.write_access.join("\r\n"));
        set_checked(&self.controls.share_enabled_check, item.enabled);
        set_checked(&self.controls.share_hidden_check, item.hidden);
        set_checked(&self.controls.share_readonly_check, item.read_only);
    }

    fn apply_share(&self) {
        let result = (|| -> Result<()> {
            let mut id = self.controls.share_id_input.text().trim().to_owned();
            if id.is_empty() { id = util::new_id("share"); }
            let name = self.controls.share_name_input.text().trim().to_owned();
            let path = self.controls.share_path_input.text().trim().to_owned();
            if name.is_empty() { bail!("共享名不能为空"); }
            if path.is_empty() { bail!("共享路径不能为空"); }
            let item = ShareConfig {
                id: id.clone(),
                name,
                path,
                comment: self.controls.share_comment_input.text().trim().to_owned(),
                enabled: is_checked(&self.controls.share_enabled_check),
                hidden: is_checked(&self.controls.share_hidden_check),
                read_only: is_checked(&self.controls.share_readonly_check),
                read_access: util::parse_principals(&self.controls.share_read_box.text()),
                write_access: util::parse_principals(&self.controls.share_write_box.text()),
            };
            let mut config = self.config.borrow_mut();
            if let Some(index) = config.shares.iter().position(|existing| existing.id.eq_ignore_ascii_case(&id)) {
                config.shares[index] = item;
            } else {
                config.shares.push(item);
            }
            drop(config);
            self.refresh_share_list();
            self.clear_share_editor();
            Ok(())
        })();
        if let Err(error) = result {
            nwg::modal_error_message(&self.controls.window, "共享设置无效", &error.to_string());
        }
    }

    fn delete_share(&self) {
        let Some(index) = self.controls.shares_list.selected_item() else { return; };
        if index < self.config.borrow().shares.len() {
            self.config.borrow_mut().shares.remove(index);
            self.refresh_share_list();
            self.clear_share_editor();
        }
    }

    fn refresh_share_list(&self) {
        self.controls.shares_list.set_redraw(false);
        self.controls.shares_list.clear();
        for item in &self.config.borrow().shares {
            self.controls.shares_list.insert_items_row(None, &[
                yes_no(item.enabled), item.name.as_str(), item.path.as_str(), yes_no(item.read_only), yes_no(item.hidden), item.comment.as_str(),
            ]);
        }
        self.controls.shares_list.set_redraw(true);
    }

    fn clear_user_editor(&self) {
        self.controls.user_id_input.set_text("");
        self.controls.user_name_input.set_text("");
        self.controls.user_password_input.set_text("");
        set_checked(&self.controls.user_enabled_check, true);
    }

    fn load_selected_user(&self) {
        let Some(index) = self.controls.users_list.selected_item() else { return; };
        let config = self.config.borrow();
        let Some(item) = config.users.get(index) else { return; };
        self.controls.user_id_input.set_text(&item.id);
        self.controls.user_name_input.set_text(&item.account_name);
        self.controls.user_password_input.set_text("");
        set_checked(&self.controls.user_enabled_check, item.enabled);
    }

    fn apply_user(&self) {
        let result = (|| -> Result<()> {
            let mut id = self.controls.user_id_input.text().trim().to_owned();
            if id.is_empty() { id = util::new_id("user"); }
            let account_name = self.controls.user_name_input.text().trim().to_owned();
            if account_name.is_empty() { bail!("账户名不能为空"); }
            let mut password = self.controls.user_password_input.text();
            let existing_password = self.config.borrow().users.iter()
                .find(|item| item.id.eq_ignore_ascii_case(&id))
                .map(|item| item.protected_password.clone());
            let protected_password = if password.is_empty() {
                existing_password.ok_or_else(|| anyhow!("新用户必须设置密码"))?
            } else {
                let protected = dpapi::protect(&password)?;
                password.zeroize();
                protected
            };
            let item = UserConfig {
                id: id.clone(),
                account_name,
                enabled: is_checked(&self.controls.user_enabled_check),
                protected_password,
            };
            let mut config = self.config.borrow_mut();
            if let Some(index) = config.users.iter().position(|existing| existing.id.eq_ignore_ascii_case(&id)) {
                config.users[index] = item;
            } else {
                config.users.push(item);
            }
            drop(config);
            self.controls.user_password_input.set_text("");
            self.refresh_user_list();
            self.clear_user_editor();
            Ok(())
        })();
        if let Err(error) = result {
            nwg::modal_error_message(&self.controls.window, "用户设置无效", &error.to_string());
        }
    }

    fn delete_user(&self) {
        let Some(index) = self.controls.users_list.selected_item() else { return; };
        if index < self.config.borrow().users.len() {
            self.config.borrow_mut().users.remove(index);
            self.refresh_user_list();
            self.clear_user_editor();
        }
    }

    fn refresh_user_list(&self) {
        self.controls.users_list.set_redraw(false);
        self.controls.users_list.clear();
        for item in &self.config.borrow().users {
            self.controls.users_list.insert_items_row(None, &[
                yes_no(item.enabled), item.account_name.as_str(), "已加密保存", item.id.as_str(),
            ]);
        }
        self.controls.users_list.set_redraw(true);
    }

    fn refresh_session_list(&self) {
        self.controls.sessions_list.set_redraw(false);
        self.controls.sessions_list.clear();
        for item in self.sessions.borrow().iter() {
            let open_files = item.open_file_count.to_string();
            self.controls.sessions_list.insert_items_row(None, &[
                item.listener_id.as_str(), item.client_endpoint.as_str(), item.dialect.as_str(), item.user_name.as_str(), item.machine_name.as_str(), open_files.as_str(),
            ]);
        }
        self.controls.sessions_list.set_redraw(true);
    }

    fn terminate_selected_session(&self) {
        let Some(index) = self.controls.sessions_list.selected_item() else { return; };
        let Some(session) = self.sessions.borrow().get(index).cloned() else { return; };
        let result = self.engine.borrow().as_ref().ok_or_else(|| anyhow!("SMB 引擎不可用"))
            .and_then(|engine| engine.terminate_session(&session.listener_id, &session.client_endpoint));
        match result {
            Ok(()) => self.poll_status(true),
            Err(error) => nwg::modal_error_message(&self.controls.window, "断开会话失败", &error.to_string()),
        }
    }

    fn set_footer(&self, text: String) {
        self.controls.footer_status.set_text(&text);
    }
}

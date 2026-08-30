impl SmbnUi {
    fn bind_events(&self) {
        let weak = Rc::downgrade(&self.inner);
        let handler = nwg::full_bind_event_handler(&self.inner.controls.window.handle, move |event, _data, handle| {
            let Some(app) = weak.upgrade() else { return; };
            use nwg::Event as E;
            match event {
                E::OnWindowClose if handle == app.controls.window => app.handle_window_close(),
                E::OnWindowMinimize if handle == app.controls.window => app.handle_window_minimize(),
                E::OnContextMenu if handle == app.controls.tray => app.show_tray_menu(),
                E::OnMousePress(nwg::MousePressEvent::MousePressLeftUp) if handle == app.controls.tray => app.show_window(),
                E::OnMenuItemSelected if handle == app.controls.tray_open => app.show_window(),
                E::OnMenuItemSelected if handle == app.controls.tray_start => app.start_server(true),
                E::OnMenuItemSelected if handle == app.controls.tray_stop => app.stop_server(true),
                E::OnMenuItemSelected if handle == app.controls.tray_exit => app.request_exit(),
                E::OnTimerTick if handle == app.controls.timer => app.poll_status(false),

                E::OnButtonClick if handle == app.controls.save_button => app.save_clicked(),
                E::OnButtonClick if handle == app.controls.start_button => app.start_server(true),
                E::OnButtonClick if handle == app.controls.stop_button => app.stop_server(true),
                E::OnButtonClick if handle == app.controls.hide_button => app.hide_to_tray(true),
                E::OnButtonClick if handle == app.controls.exit_button => app.request_exit(),

                E::OnListViewItemChanged if handle == app.controls.listeners_list => app.load_selected_listener(),
                E::OnButtonClick if handle == app.controls.listener_new_button => app.clear_listener_editor(),
                E::OnButtonClick if handle == app.controls.listener_apply_button => app.apply_listener(),
                E::OnButtonClick if handle == app.controls.listener_delete_button => app.delete_listener(),

                E::OnListViewItemChanged if handle == app.controls.shares_list => app.load_selected_share(),
                E::OnButtonClick if handle == app.controls.share_new_button => app.clear_share_editor(),
                E::OnButtonClick if handle == app.controls.share_apply_button => app.apply_share(),
                E::OnButtonClick if handle == app.controls.share_delete_button => app.delete_share(),

                E::OnListViewItemChanged if handle == app.controls.users_list => app.load_selected_user(),
                E::OnButtonClick if handle == app.controls.user_new_button => app.clear_user_editor(),
                E::OnButtonClick if handle == app.controls.user_apply_button => app.apply_user(),
                E::OnButtonClick if handle == app.controls.user_delete_button => app.delete_user(),

                E::OnButtonClick if handle == app.controls.open_data_button => app.open_data_directory(),
                E::OnButtonClick if handle == app.controls.refresh_button => app.poll_status(true),
                E::OnButtonClick if handle == app.controls.terminate_button => app.terminate_selected_session(),
                E::OnButtonClick if handle == app.controls.diagnostics_button => app.run_diagnostics(),
                E::OnButtonClick if handle == app.controls.trim_button => app.trim_memory(),
                _ => {}
            }
        });
        self.handlers.borrow_mut().push(handler);
    }
}

impl Drop for SmbnUi {
    fn drop(&mut self) {
        for handler in self.handlers.borrow_mut().drain(..) {
            nwg::unbind_event_handler(&handler);
        }
    }
}

fn configure_list(list: &nwg::ListView, columns: &[(&str, i32)]) {
    list.set_list_style(nwg::ListViewStyle::Detailed);
    list.set_headers_enabled(true);
    for (index, (text, width)) in columns.iter().enumerate() {
        list.insert_column(nwg::InsertListViewColumn {
            index: Some(index as i32),
            width: Some(*width),
            text: Some((*text).to_owned()),
            ..Default::default()
        });
    }
}

fn is_checked(control: &nwg::CheckBox) -> bool {
    control.check_state() == nwg::CheckBoxState::Checked
}

fn set_checked(control: &nwg::CheckBox, value: bool) {
    control.set_check_state(if value { nwg::CheckBoxState::Checked } else { nwg::CheckBoxState::Unchecked });
}

fn yes_no(value: bool) -> &'static str {
    if value { "是" } else { "否" }
}

fn parse_number<T>(text: &str, label: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    text.trim().parse::<T>().map_err(|error| anyhow!("{label}不是有效数字：{error}"))
}

fn log_level_index(level: LogLevel) -> usize {
    match level {
        LogLevel::Error => 0,
        LogLevel::Warning => 1,
        LogLevel::Information => 2,
        LogLevel::Debug => 3,
        LogLevel::Verbose => 4,
        LogLevel::Trace => 5,
    }
}

fn index_log_level(index: usize) -> LogLevel {
    match index {
        0 => LogLevel::Error,
        1 => LogLevel::Warning,
        3 => LogLevel::Debug,
        4 => LogLevel::Verbose,
        5 => LogLevel::Trace,
        _ => LogLevel::Information,
    }
}

fn format_diagnostics(checks: &[DiagnosticCheck]) -> String {
    checks.iter().map(|item| {
        let marker = match item.severity.as_str() {
            "error" => "[错误]",
            "warning" => "[警告]",
            _ => "[信息]",
        };
        format!("{marker} {}\r\n{}", item.name, item.message)
    }).collect::<Vec<_>>().join("\r\n\r\n")
}

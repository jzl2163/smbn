impl SmbnApp {
    fn build_shares_tab(c: &mut Controls) -> Result<()> {
        nwg::ListView::builder().position((20, 15)).size((1015, 235)).parent(&c.shares_tab).build(&mut c.shares_list)?;
        nwg::Label::builder().text("共享 ID").position((25, 275)).size((90, 25)).parent(&c.shares_tab).build(&mut c.share_id_label)?;
        nwg::TextInput::builder().position((120, 271)).size((225, 28)).parent(&c.shares_tab).build(&mut c.share_id_input)?;
        nwg::Label::builder().text("共享名").position((380, 275)).size((80, 25)).parent(&c.shares_tab).build(&mut c.share_name_label)?;
        nwg::TextInput::builder().position((465, 271)).size((220, 28)).parent(&c.shares_tab).build(&mut c.share_name_input)?;
        nwg::Label::builder().text("本地路径").position((25, 320)).size((90, 25)).parent(&c.shares_tab).build(&mut c.share_path_label)?;
        nwg::TextInput::builder().position((120, 316)).size((565, 28)).parent(&c.shares_tab).build(&mut c.share_path_input)?;
        nwg::Label::builder().text("备注").position((720, 275)).size((60, 25)).parent(&c.shares_tab).build(&mut c.share_comment_label)?;
        nwg::TextInput::builder().position((780, 271)).size((240, 28)).parent(&c.shares_tab).build(&mut c.share_comment_input)?;
        nwg::Label::builder().text("读取主体（逗号或换行分隔；Users=所有已认证用户）").position((25, 365)).size((475, 25)).parent(&c.shares_tab).build(&mut c.share_read_label)?;
        nwg::TextBox::builder().position((25, 392)).size((475, 100)).parent(&c.shares_tab).build(&mut c.share_read_box)?;
        nwg::Label::builder().text("写入主体").position((530, 365)).size((220, 25)).parent(&c.shares_tab).build(&mut c.share_write_label)?;
        nwg::TextBox::builder().position((530, 392)).size((490, 100)).parent(&c.shares_tab).build(&mut c.share_write_box)?;
        nwg::CheckBox::builder().text("启用共享").position((25, 510)).size((130, 28)).parent(&c.shares_tab).build(&mut c.share_enabled_check)?;
        nwg::CheckBox::builder().text("隐藏共享（自动追加 $）").position((175, 510)).size((210, 28)).parent(&c.shares_tab).build(&mut c.share_hidden_check)?;
        nwg::CheckBox::builder().text("只读").position((405, 510)).size((100, 28)).parent(&c.shares_tab).build(&mut c.share_readonly_check)?;
        nwg::Button::builder().text("新建").position((545, 507)).size((100, 32)).parent(&c.shares_tab).build(&mut c.share_new_button)?;
        nwg::Button::builder().text("添加 / 更新").position((655, 507)).size((135, 32)).parent(&c.shares_tab).build(&mut c.share_apply_button)?;
        nwg::Button::builder().text("删除所选").position((800, 507)).size((120, 32)).parent(&c.shares_tab).build(&mut c.share_delete_button)?;
        Ok(())
    }

    fn build_users_tab(c: &mut Controls) -> Result<()> {
        nwg::ListView::builder().position((20, 18)).size((1015, 275)).parent(&c.users_tab).build(&mut c.users_list)?;
        nwg::Label::builder().text("用户 ID").position((25, 325)).size((90, 25)).parent(&c.users_tab).build(&mut c.user_id_label)?;
        nwg::TextInput::builder().position((120, 321)).size((250, 28)).parent(&c.users_tab).build(&mut c.user_id_input)?;
        nwg::Label::builder().text("账户名").position((405, 325)).size((80, 25)).parent(&c.users_tab).build(&mut c.user_name_label)?;
        nwg::TextInput::builder().position((490, 321)).size((250, 28)).parent(&c.users_tab).build(&mut c.user_name_input)?;
        nwg::Label::builder().text("密码").position((25, 375)).size((90, 25)).parent(&c.users_tab).build(&mut c.user_password_label)?;
        nwg::TextInput::builder().position((120, 371)).size((330, 28)).password(Some('●')).parent(&c.users_tab).build(&mut c.user_password_input)?;
        nwg::CheckBox::builder().text("启用用户").position((490, 370)).size((140, 30)).parent(&c.users_tab).build(&mut c.user_enabled_check)?;
        nwg::Button::builder().text("新建").position((25, 430)).size((100, 32)).parent(&c.users_tab).build(&mut c.user_new_button)?;
        nwg::Button::builder().text("添加 / 更新").position((135, 430)).size((135, 32)).parent(&c.users_tab).build(&mut c.user_apply_button)?;
        nwg::Button::builder().text("删除所选").position((280, 430)).size((120, 32)).parent(&c.users_tab).build(&mut c.user_delete_button)?;
        nwg::Label::builder()
            .text("密码使用 Windows DPAPI CurrentUser 加密后写入配置；更新已有用户时留空表示保留原密码。Windows 集成认证模式会忽略此列表。")
            .position((25, 495)).size((995, 55)).parent(&c.users_tab).build(&mut c.user_help)?;
        Ok(())
    }

    fn build_options_tab(c: &mut Controls) -> Result<()> {
        nwg::CheckBox::builder().text("随 Windows 当前用户登录启动").position((30, 30)).size((300, 30)).parent(&c.options_tab).build(&mut c.startup_check)?;
        nwg::CheckBox::builder().text("程序启动后自动启动 SMB 服务").position((30, 75)).size((320, 30)).parent(&c.options_tab).build(&mut c.start_server_check)?;
        nwg::CheckBox::builder().text("最小化时隐藏到系统托盘").position((30, 120)).size((300, 30)).parent(&c.options_tab).build(&mut c.minimize_tray_check)?;
        nwg::CheckBox::builder().text("点击关闭按钮时隐藏到系统托盘").position((30, 165)).size((340, 30)).parent(&c.options_tab).build(&mut c.close_tray_check)?;
        nwg::CheckBox::builder().text("轻量模式：隐藏后停止日志/会话渲染并回收工作集").position((30, 210)).size((500, 30)).parent(&c.options_tab).build(&mut c.light_mode_check)?;
        nwg::CheckBox::builder().text("服务运行时退出前确认").position((30, 255)).size((300, 30)).parent(&c.options_tab).build(&mut c.confirm_exit_check)?;
        nwg::Label::builder().text("日志级别").position((565, 35)).size((100, 25)).parent(&c.options_tab).build(&mut c.log_level_label)?;
        nwg::ComboBox::builder()
            .position((670, 31)).size((260, 180))
            .collection(vec!["错误".into(), "警告".into(), "信息".into(), "调试".into(), "详细".into(), "跟踪".into()])
            .selected_index(Some(2)).parent(&c.options_tab).build(&mut c.log_level_combo)?;
        nwg::Label::builder().text("单文件上限 MiB").position((565, 85)).size((140, 25)).parent(&c.options_tab).build(&mut c.log_size_label)?;
        nwg::TextInput::builder().position((715, 81)).size((120, 28)).parent(&c.options_tab).build(&mut c.log_size_input)?;
        nwg::Label::builder().text("保留文件数").position((565, 135)).size((140, 25)).parent(&c.options_tab).build(&mut c.log_files_label)?;
        nwg::TextInput::builder().position((715, 131)).size((120, 28)).parent(&c.options_tab).build(&mut c.log_files_input)?;
        nwg::Label::builder().text("界面日志行数").position((565, 185)).size((140, 25)).parent(&c.options_tab).build(&mut c.log_tail_label)?;
        nwg::TextInput::builder().position((715, 181)).size((120, 28)).parent(&c.options_tab).build(&mut c.log_tail_input)?;
        nwg::Button::builder().text("打开数据目录").position((565, 245)).size((150, 34)).parent(&c.options_tab).build(&mut c.open_data_button)?;
        nwg::Label::builder()
            .text("轻量模式不会停止 SMB 引擎；它仅停止不可见界面的高频更新、清空大文本控件，并请求 GUI 与引擎回收可释放内存。托盘状态仍每 5 秒刷新。")
            .position((30, 335)).size((985, 75)).parent(&c.options_tab).build(&mut c.options_help)?;
        Ok(())
    }

    fn build_monitor_tab(c: &mut Controls) -> Result<()> {
        nwg::Label::builder().text("状态尚未刷新").position((20, 15)).size((1015, 30)).parent(&c.monitor_tab).build(&mut c.monitor_summary)?;
        nwg::ListView::builder().position((20, 50)).size((1015, 205)).parent(&c.monitor_tab).build(&mut c.sessions_list)?;
        nwg::Button::builder().text("立即刷新").position((20, 270)).size((110, 32)).parent(&c.monitor_tab).build(&mut c.refresh_button)?;
        nwg::Button::builder().text("断开所选会话").position((140, 270)).size((145, 32)).parent(&c.monitor_tab).build(&mut c.terminate_button)?;
        nwg::Button::builder().text("运行诊断").position((295, 270)).size((115, 32)).parent(&c.monitor_tab).build(&mut c.diagnostics_button)?;
        nwg::Button::builder().text("回收内存").position((420, 270)).size((115, 32)).parent(&c.monitor_tab).build(&mut c.trim_button)?;
        nwg::TextBox::builder().readonly(true).position((20, 315)).size((400, 230)).parent(&c.monitor_tab).build(&mut c.diagnostics_box)?;
        nwg::TextBox::builder().readonly(true).position((435, 315)).size((600, 230)).parent(&c.monitor_tab).build(&mut c.log_box)?;
        Ok(())
    }

}

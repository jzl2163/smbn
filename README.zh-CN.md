# SMBN：基于 SMBLibrary 的 Windows 原生 SMB 服务器管理器

SMBN 是一个面向 Windows 10/11 x64 的桌面程序。界面与控制层采用 Rust，使用 `native-windows-gui` 创建真正的 Win32 控件；SMB 协议引擎运行在独立的 .NET 8 进程中，直接调用 SMBLibrary 1.5.7 与 SMBLibrary.Win32 1.5.7。它不是 Electron/WebView 应用，也没有把 C# 库伪装成“纯 Rust SMB 实现”。

> 当前版本：`0.1.0`。这是完整工程源码与发布脚本，不包含预编译二进制。由于本次交付环境无法运行 Windows Rust/.NET 工具链，提交前必须在 Windows CI 或本机完成编译与运行检查。

## 1. 已实现的功能

### 原生界面与托盘

- 六个分页：服务器、监听器、共享、用户、应用设置、监控与诊断。
- 最小化或关闭窗口时可隐藏到 Windows 右下角通知区域。
- 左键托盘图标恢复窗口；右键菜单可打开界面、启动服务、停止服务或退出。
- 托盘图标按状态切换：**橙色表示服务正在启动、运行或停止；白色表示未运行或故障**。
- 鼠标悬停托盘图标可查看状态、监听器数、会话数和打开文件数。
- 服务仍在运行时退出会显示确认框；确认后先停止服务并断开客户端，再退出程序。

### SMB 服务器参数

- SMB1、SMB2、SMB3 独立开关；默认关闭 SMB1。
- Direct TCP 与 NetBIOS over TCP 两种传输。
- 多监听器；可设置监听 IPv4/IPv6 字面量、端口、传输方式和启用状态。
- Direct TCP 支持 IPv6，包括 `::` 通配监听。
- 可设置 NetBIOS 名称、工作组、连接空闲保活时间。
- 可选 UDP/137 NetBIOS 名称服务，支持自定义名称和工作组。
- IPv4/IPv6 CIDR 允许列表与拒绝列表；拒绝规则优先。
- 自定义 SMB TCP 端口通过 SMBLibrary 的受保护端口重载实现。

### 认证与共享

- 独立账户模式：密码在磁盘上使用 Windows DPAPI `CurrentUser` 加密。
- Windows 集成认证模式：通过 SMBLibrary.Win32 使用当前 Windows 安全子系统。
- 多目录共享、隐藏共享（自动追加 `$`）、只读模式、备注。
- 每个共享可分别配置读取主体和写入主体；`Users` 表示全部已认证用户。
- 会话列表显示监听器、客户端端点、协商方言、用户、客户端设备、打开文件数。
- 可主动终止所选 SMB 会话。

### 运行、日志与诊断

- GUI 与引擎进程分离；GUI 崩溃或被结束后，引擎通过父进程监护自动退出。
- 仅当前 Windows 用户可连接的命名管道，加 64 字符随机令牌和定长帧上限。
- 日志采用有界内存尾部队列和单后台写入任务，不在 SMB 工作线程中同步刷盘。
- 日志级别、单文件上限、轮转数量和界面保留行数均可设置。
- 诊断端口可绑定性、目录可访问性、管理员状态、SMB1 风险、Windows 139/445 冲突及自定义端口限制。
- 配置采用临时文件、同步落盘、备份和原子重命名流程。
- 可设置随当前 Windows 用户登录启动，以及程序启动后自动启动 SMB 服务。

## 2. 重要边界

### IPv6 与 NetBIOS

SMB Direct TCP 可以绑定 IPv4 或 IPv6。NetBIOS over TCP 和 UDP/137 名称服务是 IPv4 协议路径，因此界面与后端都会拒绝把它们配置到 IPv6 地址。UDP/137 名称服务还必须绑定具体 IPv4 地址，不能使用 `0.0.0.0`，因为程序需要确定广播地址和子网掩码。

### 自定义端口

服务端可以监听任意未被占用的 TCP 端口，但 Windows 资源管理器的普通 UNC 地址没有直接指定 SMB 端口的语法。非 139/445 端口通常需要支持端口参数的 SMB 客户端、端口代理或专用网络转发规则。

### Windows 自带 SMB 服务

Windows 的 LanmanServer/“Server”服务通常会占用 TCP 445，并可能占用 TCP 139。SMBN 不会自动停止系统服务、修改注册表绑定或创建防火墙规则；这些操作影响系统安全边界，必须由管理员明确处理。内置诊断会报告绑定失败和常见冲突。

### “全部功能”的范围

本项目覆盖 SMBLibrary 文档中与**文件共享服务器管理**直接相关的生命周期、协议版本、传输、认证、目录文件系统共享、访问检查、连接过滤、会话查询与终止、日志和名称服务能力。它不提供 SMB 客户端浏览器、打印共享、域控制器、DFS 管理、RPC 管理套件或 Windows 内核驱动；这些不属于本程序的服务器管理范围，部分也不是 SMBLibrary 的现成功能。

### SMB3 安全能力

界面中的 SMB3 开关表示允许 SMBLibrary 的 SMB3 协议路径。不要把它理解为“已强制所有 SMB3 加密、签名或企业级审计策略”。部署到不可信网络前，应独立验证客户端协商结果、网络隔离、Windows 防火墙和 SMBLibrary 当前版本的安全行为。

## 3. 架构

```text
┌──────────────────────────────────────────────────────┐
│ smbn.exe（Rust / Win32）                           │
│ 配置表单、托盘、DPAPI、启动项、状态呈现、生命周期     │
└───────────────────────┬──────────────────────────────┘
                        │ 同用户 Named Pipe
                        │ 4-byte LE length + JSON
                        │ 随机令牌；最大 8 MiB
┌───────────────────────▼──────────────────────────────┐
│ Smbn.Engine.exe（.NET 8）                          │
│ 配置验证、监听器、认证、共享 ACL、会话、日志、诊断   │
└───────────────────────┬──────────────────────────────┘
                        │
              SMBLibrary / SMBLibrary.Win32
```

详细设计见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)，安全设计见 [docs/SECURITY.md](docs/SECURITY.md)，交接说明见 [HANDOFF.md](HANDOFF.md)。

## 4. 构建要求

- Windows 10 或 Windows 11 x64。
- Visual Studio 2022 Build Tools，安装“使用 C++ 的桌面开发”和 Windows SDK。
- Rust 1.82.0；仓库中的 `rust-toolchain.toml` 会选择工具链和 `x86_64-pc-windows-msvc` 目标。
- .NET 8 SDK。
- PowerShell 7，或 Windows PowerShell 5.1（脚本避免依赖 PowerShell 7 专用语法）。

首次检查：

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\check.ps1
```

构建基线版：

```powershell
.\scripts\build.ps1 -CpuProfile baseline
```

构建 x86-64-v3 版：

```powershell
.\scripts\build.ps1 -CpuProfile x86-64-v3
```

在 Core Ultra 125H 目标机器上构建本机优化版：

```powershell
.\scripts\build.ps1 -CpuProfile native
```

生成自包含 .NET 引擎包：

```powershell
.\scripts\build.ps1 -CpuProfile baseline -SelfContained
```

输出目录：

```text
artifacts/
  package/
    smbn-baseline/
      smbn.exe
      assets/
      engine/
      README.zh-CN.md
      THIRD_PARTY_NOTICES.md
  smbn-baseline.zip
  smbn-baseline.zip.sha256
```

## 5. CPU 优化说明

- `baseline`：Rust 使用 `target-cpu=x86-64`，兼容范围最大。
- `x86-64-v3`：Rust 使用 `target-cpu=x86-64-v3`，适合支持 AVX/AVX2 等 v3 指令集的机器；不要分发给不支持该级别的旧 CPU。
- `native`：Rust 使用 `target-cpu=native`，只应在最终运行机器或同型号机器上构建。对 Core Ultra 125H，这是比硬编码某个不稳定 LLVM CPU 名称更稳妥的方式。
- SMB 热路径主要位于 .NET/SMBLibrary 引擎中。默认发布保留 JIT 和 Tiered PGO，使 .NET 在实际运行 CPU 上选择可用指令；Rust CPU 配置主要优化 GUI、IPC 和配置控制层，因此不要夸大其对文件传输吞吐的影响。

## 6. 首次运行

1. 将发布目录完整解压，不要只复制 `smbn.exe`。
2. 启动 `smbn.exe`。若绑定 139/445、访问受保护目录或调整外部防火墙规则，可能需要以管理员身份运行。
3. 在“共享”页添加至少一个已存在的绝对目录。
4. 选择认证模式：
   - 独立账户：在“用户”页添加至少一个账户和密码；
   - Windows 集成认证：用户页可留空。
5. 在“监听器”页确认地址和端口。默认配置启用 `0.0.0.0:445`，并预置但不启用 `[::]:445` Direct TCP；需要 IPv6 时可单独启用，或把 IPv4/IPv6 都改为具体接口地址后同时启用。
6. 点击“运行诊断”，先解决端口或目录错误。
7. 点击“保存配置”，再点击“启动服务”。
8. 使用另一台受信任客户端测试连接；不要首先在公共网络接口上开放服务。

## 7. 配置与数据位置

普通模式：

```text
%LOCALAPPDATA%\Smbn\
  config.json
  config.json.bak
  logs\
```

便携模式：在 `smbn.exe` 同目录创建空文件 `portable.flag`，数据会写入：

```text
<程序目录>\data\
```

独立账户密码字段只保存 DPAPI 密文，且只能由同一 Windows 用户上下文解密。启动服务时，明文密码必须短暂进入 GUI 与引擎进程内存；Rust 请求缓冲会在发送后清零，引擎会把凭据复制到可清零的字符数组并尽快丢弃传输对象中的密码字符串。由于 .NET 字符串不可变，不能保证托管堆中的每个历史副本被立即物理覆盖，详见安全文档。

## 8. 轻量模式

轻量模式开启且窗口隐藏后：

- 状态轮询从每秒降为每 5 秒一次；
- 停止刷新会话表与日志文本控件；
- 清空日志、诊断和会话控件持有的大块界面内容；
- 请求 .NET 引擎执行优化型 GC；
- 请求 Windows 回收 GUI 进程可释放的工作集页。

它不会停止 SMB 服务，也不会卸载 SMBLibrary。恢复窗口时会重新获取完整状态、会话和日志尾部。

## 9. 安全建议

- 保持 SMB1 关闭。
- 只绑定需要的接口，优先使用具体地址而不是通配地址。
- 使用 CIDR 允许列表，并在 Windows 防火墙中再次限制来源和网络配置文件。
- 独立账户使用长且唯一的密码；不要与 Windows 登录密码复用。
- 对外提供写权限前，使用专用低权限 Windows 账户运行程序，并限制共享目录的 NTFS 权限。
- 不要把 `config.json`、日志或诊断信息公开提交到仓库。
- 升级 SMBLibrary 后重新运行集成测试，尤其是认证、方言、签名、加密和会话终止行为。

## 10. 故障排查

**找不到 SMB 引擎**：发布目录必须含 `engine\Smbn.Engine.exe`，或者含框架依赖发布的 `Smbn.Engine.dll` 且系统可调用 `dotnet`。

**无法绑定 445/139**：先在“监控与诊断”运行诊断，再用 `Get-NetTCPConnection -LocalPort 445,139` 和系统服务管理器确认占用者。不要让脚本自动停用系统文件共享。

**IPv6 可以监听但名称找不到**：IPv6 Direct TCP 应使用 DNS、mDNS（由其他服务提供）或 IPv6 字面量；NBNS 仅适用于 IPv4。

**自定义端口无法从资源管理器连接**：这是 Windows UNC 客户端入口的限制，不代表服务未监听。使用支持自定义端口的 SMB 客户端或受控端口转发进行测试。

**配置文件无法解密密码**：DPAPI `CurrentUser` 密文与创建它的 Windows 用户绑定。换用户、迁移机器或损坏用户配置文件后需要在界面中重新设置密码。

## 11. 验证

仓库提供 Windows GitHub Actions 流程，执行：

- `cargo fmt --all`；
- `cargo test -p smbn-core`；
- Windows 目标 `cargo clippy -D warnings`；
- .NET restore/build 与命名管道/JSON 诊断冒烟测试；
- 基线发布打包；
- 包结构检查。

当前交付环境不是 Windows，且没有可用的 Rust/.NET 工具链和外网包下载，因此这里无法诚实地声明已完成本机编译或 Windows 运行测试。`HANDOFF.md` 明确列出了合入前必须在 Windows CI 完成的验收项。

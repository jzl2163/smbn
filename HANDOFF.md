# SMBN engineering handoff

## Delivery status

The repository contains a complete first implementation of the Rust Win32 controller, shared configuration/validation crate, .NET SMBLibrary engine, runtime icon generation, build/package scripts, Windows CI, examples and documentation.

The authoring container was Linux-based and had neither Rust nor .NET installed. Network access from the container could not resolve the official installers, so no local dependency restore, compiler pass, Windows link or runtime SMB test was possible. Treat the first successful Windows CI run as a required gate, not a formality.

## First maintainer actions

1. Confirm the Windows CI run for the import commit passes.
2. Run `.\scripts\check.ps1` on Windows 11 x64.
3. Fix any API/compiler drift without weakening warnings.
4. Run `.\scripts\build.ps1 -CpuProfile baseline`.
5. Execute the acceptance matrix below in an isolated VM before tagging the first release.

## Repository map

- `crates/smbn-core`: schema, protocol DTOs, validation and unit tests.
- `crates/smbn-win`: native GUI, tray, DPAPI, registry startup, IPC client and process lifecycle.
- `engine/Smbn.Engine`: .NET host over SMBLibrary/SMBLibrary.Win32 1.5.7.
- `scripts/check.ps1`: formatting, tests, Clippy, .NET build and engine IPC smoke test.
- `scripts/smoke-engine.ps1`: launches the built engine, validates snake_case JSON over the named pipe, runs diagnostics and shuts down cleanly.
- `scripts/build.ps1`: reproducible package variants.
- `.github/workflows/windows-ci.yml`: mandatory Windows validation and package artifact.
- `docs`: architecture, security and configuration references.

## Dependency decisions

- Rust is pinned by `rust-toolchain.toml` to 1.82.0.
- Direct Rust dependencies use exact versions except `anyhow` 1.x. Generate and commit `Cargo.lock` on the first Windows build.
- Both `SMBLibrary` and `SMBLibrary.Win32` are pinned to 1.5.7. Do not update only one package. SMBLibrary 1.5.7.1 exists, but the Win32 package and source signatures used for this implementation were aligned to 1.5.7.
- `native-windows-gui` is intentionally used instead of a WebView framework to minimize idle memory and preserve native notification-area behavior.

## API assumptions to validate in first CI

These were checked against upstream v1.5.7 source, but must still compile against the actual NuGet assemblies:

- subclass access to `SMBServer.Start(IPAddress, SMBTransportType, int, bool, bool, bool, TimeSpan?)`;
- `ConnectionRequested`, `LogEntryAdded`, `GetSessionsInformation()` and `TerminateConnection()`;
- `IndependentNTLMAuthenticationProvider(GetUserPassword)` and virtual `GetServerName()`;
- `IntegratedNTLMAuthenticationProvider`, `GSSProvider`, `NTDirectoryFileSystem`, `FileSystemShare.AccessRequested`;
- `SessionInformation` fields used by `SmbHost.GetSessionsUnsafe`;
- NetBIOS packet classes used by `ConfigurableNameServer`.

The NWG methods used by the UI were checked against upstream source (`Window.restore`, tray icon/tip mutation, popup menu, list view and timer events).

## Acceptance matrix

### Build and packaging

- Clean checkout builds with Visual Studio Build Tools, Rust 1.82 and .NET 8.
- `check.ps1` exits zero.
- Baseline and x86-64-v3 ZIPs contain GUI, engine dependencies and notices; application/tray ICOs are generated on first launch.
- Framework-dependent package gives a useful error if .NET 8 Desktop/Runtime is missing.
- Self-contained package starts on a clean Windows VM.

### Lifecycle and tray

- White icon before start; orange during Starting/Running/Stopping; white on Stopped/Faulted.
- Tooltip updates without reopening the GUI.
- Minimize and Close honor separate settings.
- Light mode reduces polling to 5 seconds and restores monitor data after reopening.
- Exit prompt appears only while active and confirmed exit stops listeners.
- Killing the GUI process causes the engine to exit through parent monitoring.

### Networking

- IPv4 specific address and `0.0.0.0` Direct TCP.
- IPv6 specific address and `::` Direct TCP.
- IPv4 NetBIOS over TCP.
- Custom TCP port with a client that accepts an explicit port.
- UDP/137 NBNS on a concrete IPv4 interface; registration/query inspected with packet capture.
- Duplicate/busy endpoints roll back every listener.
- CIDR allow/reject with IPv4, IPv6 and IPv4-mapped IPv6 clients.

### Authentication and authorization

- Independent user success/failure, mixed-case username, password update and DPAPI restart.
- Integrated Windows auth with local and domain accounts where available.
- Read-only share rejects create/write/delete.
- Exact read/write lists and `Users` behavior.
- Empty/anonymous username is not granted by `Users`.
- NTFS denial still wins even when application ACL allows.

### Protocol and interoperability

- SMB1 disabled default; test only in an isolated legacy VM.
- SMB2 and SMB3 negotiation with Windows, Samba `smbclient`, and at least one non-Windows client.
- Large file, many small files, Unicode names, long paths, rename/delete, reconnect and abrupt client loss.
- Validate signing/encryption behavior rather than inferring it from the SMB3 toggle.

### Monitoring and load

- Sessions and open-file counts update correctly.
- Terminating a session disconnects only the selected endpoint/listener.
- Log queue stays bounded under high event volume; rotation honors limits.
- 30-minute transfer test with GUI visible and hidden/light mode; record throughput, working set, CPU and handles.

## Known design limitations

- The Rust GUI makes synchronous IPC calls on its event thread. Calls are normally small/local, but an unresponsive engine can temporarily stall the window. A v0.2 improvement should move status and command IPC onto a worker thread and marshal results with `nwg::Notice`.
- The GUI uses fixed coordinates. Per-monitor DPI is declared and NWG scales controls, but resizing/reflow and accessibility review remain future work.
- Share comments are persisted for operators but are not advertised because SMBLibrary 1.5.7 `FileSystemShare` exposes no remark field.
- Custom ports are server-capable but not directly addressable in ordinary Windows Explorer UNC syntax.
- NBNS is IPv4-only by protocol/upstream design.
- DPAPI protects data at rest, not against malicious same-user memory inspection.
- This application does not manage firewall rules or disable the built-in Windows SMB server.

## Suggested v0.2 backlog

- Asynchronous GUI command worker with cancellation and command progress.
- Folder picker and interface-address enumerator.
- Structured event log export and Windows Event Log sink.
- Optional Windows service mode with a separately ACLed control pipe.
- Localization resources and UI Automation/accessibility audit.
- Signed MSIX/MSI installer and Authenticode release pipeline.
- Integration tests that launch the server on an ephemeral loopback port and use SMBLibrary client APIs.

## Release procedure

1. Update versions in workspace and engine project.
2. Update dependency notices and audit advisories.
3. Run the full acceptance matrix.
4. Build `baseline`; optionally build `x86-64-v3`. Build `native` only for a controlled machine-specific deployment.
5. Sign both executables and ZIP/checksum artifacts.
6. Publish source for the exact build and preserve LGPL notices/source availability.

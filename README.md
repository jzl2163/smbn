# SMBN

SMBN is a Windows-native SMB server manager with a Rust Win32 front end and a separate .NET 8 engine backed by [SMBLibrary](https://github.com/TalAloni/SMBLibrary).

The primary documentation is available in [README.zh-CN.md](README.zh-CN.md). Maintainers should also read [HANDOFF.md](HANDOFF.md).

## Highlights

- Native Win32 controls and Windows notification-area integration; no browser runtime.
- Separate GUI and SMB engine processes connected through a same-user authenticated named pipe.
- SMB1/SMB2/SMB3 switches, Direct TCP and NetBIOS-over-TCP listeners, custom TCP ports, IPv4 and IPv6 Direct TCP.
- Independent accounts protected at rest with Windows DPAPI, or Windows integrated authentication.
- Multiple directory shares, hidden shares, read-only mode, per-share read/write account lists.
- Live state, sessions, open-file counts, session termination, bounded asynchronous logs, diagnostics, and rotating log files.
- Optional light mode that suspends expensive hidden-window refreshes and trims reclaimable working-set pages.
- Baseline, x86-64-v3, and target-cpu=native build profiles.

## Build

On Windows 10/11 x64 with Visual Studio 2022 Build Tools, Rust 1.82, and the .NET 8 SDK:

```powershell
pwsh -File .\scripts\check.ps1
pwsh -File .\scripts\build.ps1 -CpuProfile baseline
```

The package directory is `artifacts/package/smbn-baseline/`; the archive and checksum are `artifacts/smbn-baseline.zip` and `artifacts/smbn-baseline.zip.sha256`.

## Licensing

Rust code and original project code are available under MIT OR Apache-2.0. SMBLibrary and the adapted `ConfigurableNameServer.cs` are LGPL-3.0-or-later. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

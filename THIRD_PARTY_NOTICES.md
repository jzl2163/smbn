# Third-party notices

## SMBLibrary and SMBLibrary.Win32

- Project: SMBLibrary by Tal Aloni
- Version pinned by this repository: 1.5.7
- License: GNU Lesser General Public License, version 3 or later
- Upstream: https://github.com/TalAloni/SMBLibrary

The .NET engine dynamically references the NuGet packages. `engine/Smbn.Engine/ConfigurableNameServer.cs` is adapted from `SMBLibrary.Server.NameServer` and is explicitly licensed LGPL-3.0-or-later. When redistributing binaries, include this notice and the LGPL text, do not prevent replacement/debugging of the LGPL components, and make the corresponding source and modifications available as required by the license.

## native-windows-gui

- Project: native-windows-gui by gabdube
- Version: 1.0.13
- License: MIT
- Upstream: https://github.com/gabdube/native-windows-gui

## Other Rust dependencies

See `Cargo.toml` and the generated `Cargo.lock`. Release maintainers should generate an automated license inventory (for example with `cargo-about`) and attach it to release artifacts.

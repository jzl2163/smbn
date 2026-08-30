# Architecture

## Goals

The design keeps the visible application small and native while using SMBLibrary through its supported .NET API. The process split is intentional: a GUI restart does not mix UI state with SMB protocol worker threads, and the IPC boundary makes configuration/lifecycle ownership explicit.

## Components

### `smbn-core`

Pure Rust shared types. It owns persisted configuration, validation rules, IPC DTOs, state/status models, and tests. It forbids unsafe Rust.

### `smbn-win`

Windows-only Rust executable using native-windows-gui. Responsibilities:

- Win32 controls, tabs, notification-area icon and menus;
- configuration editing and atomic persistence;
- DPAPI encryption/decryption;
- current-user startup registry entry;
- child engine launch, parent relationship, authenticated named-pipe client;
- UI-side validation, status rendering, exit confirmation and light mode.

### `Smbn.Engine`

.NET 8 child process. Responsibilities:

- second-line configuration validation;
- SMBLibrary listener construction and teardown;
- independent or integrated NTLM authentication provider;
- directory shares and authorization callbacks;
- IPv4/IPv6 connection policy;
- configurable IPv4 NBNS responder/registration;
- sessions, termination, status, diagnostics and rotating logs.

## IPC

Each GUI launch creates `\\.\pipe\smbn-<gui-pid>-<random>`. The engine creates the server with `PipeOptions.CurrentUserOnly`. A 64-character random token is passed in a child-only environment variable, removed from the engine environment after startup, and included in every request. Comparison is constant-time.

Frames are:

```text
u32 little-endian JSON byte length
UTF-8 JSON envelope
```

The maximum frame is 8 MiB. Requests are serialized by the GUI to keep command ordering deterministic. Commands are `ping`, `start`, `stop`, `status`, `sessions`, `terminate_session`, `tail_logs`, `diagnostics`, `trim_memory`, and `shutdown`.

## Lifecycle

The GUI owns the engine process. The engine watches the GUI PID and cancels itself when the parent exits. During normal exit the GUI stops an active service, sends `shutdown`, then lets the engine client wait briefly and force-terminate only if graceful shutdown did not complete.

## Listener model

Each enabled listener gets its own SMBLibrary `SMBServer`, share collection, authentication/security provider, connection policy event handler, and log event handler. This is necessary because SMBLibrary's server instance owns one socket listener. Startup is transactional: if any listener or NBNS endpoint fails, already-started listeners are disposed in reverse order.

Custom TCP ports are exposed by deriving `ConfigurableSmbServer` and calling SMBLibrary's protected `Start` overload.

## Logging

SMBLibrary log callbacks only format a line, update a bounded in-memory tail, and perform a non-blocking `Channel.TryWrite`. A single background task owns the file writer, flushes in batches, rotates files, and records queue/file failures. This prevents slow storage from serializing SMB worker threads.

## Light mode

The native window remains created but hidden. Expensive model-to-control rendering stops, large text/list content is cleared, polling is throttled, and both processes receive best-effort memory trimming. Destroying and recreating every Win32 control was rejected because it complicates state, event binding and accessibility while yielding limited benefit compared with clearing large control buffers.

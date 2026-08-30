# Security model

## Trust boundary

SMBN is a network server. Treat every SMB client, path component, credential attempt and incoming protocol message as untrusted. SMB parsing and file operation safety ultimately depend on SMBLibrary and the Windows storage adapter; keep dependencies current and test upgrades before deployment.

## IPC

- Named pipe is created with `CurrentUserOnly`.
- Pipe name contains random entropy and the GUI process ID.
- Every frame carries a random token; the engine compares UTF-8 bytes with `CryptographicOperations.FixedTimeEquals`.
- Frames have an 8 MiB hard limit.
- The token is passed through an environment variable rather than the command line and removed from the engine environment after parsing.
- This protects against accidental or cross-user control. It is not designed to resist malicious code already running as the same Windows user with process-inspection capability.

## Credentials

Persisted independent-account passwords use DPAPI CurrentUser plus application entropy. They are not portable between Windows user contexts. Plaintext is necessarily present during authentication:

1. Rust decrypts a password only immediately before building a `start` request. Diagnostics use a non-secret validation placeholder.
2. The serialized Rust request buffer is zeroized after writing to the pipe.
3. The engine copies passwords to mutable `char[]` values and clears those arrays on stop/dispose.
4. Transport model password properties are replaced with empty strings after startup or failure.

Limit: .NET `string` is immutable, JSON deserialization creates managed strings, and the CLR does not promise immediate physical overwriting. A hostile same-user debugger or memory-dump process may recover transient credentials. Do not run untrusted code in the same user session.

## Share authorization

`Users` means any non-empty authenticated username. Exact account comparisons are case-insensitive. Read and write decisions are enforced in SMBLibrary's `FileSystemShare.AccessRequested` callback. They complement, not replace, NTFS permissions. Run the engine under a Windows identity that has only the required local file rights.

## Network policy

Rejected CIDRs are evaluated first; then the allow list is applied. IPv4-mapped IPv6 addresses are normalized before matching. This is application-layer filtering, not a replacement for Windows Firewall. Firewall rules should restrict ports, profiles and remote subnets independently.

## Privilege

The manifest requests `asInvoker`; the program does not silently elevate. Low-numbered/occupied ports and protected directories may require explicit administrator launch. The program does not disable LanmanServer, rewrite SMB binding registry keys or add firewall rules.

## Reporting

Do not attach configuration files or logs containing real usernames, paths, addresses or client details to public issues without redaction. Security-sensitive reports should first be reproduced against the pinned SMBLibrary release and, where relevant, reported upstream.

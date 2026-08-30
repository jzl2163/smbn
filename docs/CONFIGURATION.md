# Configuration reference

`config.json` uses schema version 1. Unknown fields are ignored by Serde; missing fields receive defaults. A newer unsupported schema version is rejected rather than silently downgraded.

## `server`

- `netbios_name`: 1–15 ASCII letters, digits or hyphens.
- `workgroup`: 1–15 characters.
- `authentication`: `independent` or `integrated_windows`.
- `enable_smb1`, `enable_smb2`, `enable_smb3`: protocol switches. SMB3 requires SMB2.
- `inactivity_timeout_seconds`: 0 disables SMBLibrary inactivity monitoring.
- `allow_remote_subnets`: CIDR list; empty permits all addresses not rejected.
- `reject_remote_subnets`: CIDR list applied before the allow list.

## `listeners[]`

- `id`: stable unique UI/monitoring identifier.
- `address`: IPv4/IPv6 literal only; DNS names are deliberately not accepted for bind addresses.
- `port`: 1–65535.
- `transport`: `direct_tcp` or `netbios_over_tcp`.
- `netbios_name_service`: starts configurable UDP/137 NBNS; concrete IPv4 only.
- `enabled`: disabled entries remain saved but are not started.

## `shares[]`

- `id`: stable unique identifier.
- `name`: SMB share name. Hidden shares append `$` when needed.
- `path`: absolute local drive or UNC path that exists at engine startup.
- `comment`: UI metadata; SMBLibrary `FileSystemShare` 1.5.7 has no share-remark property, so it is not announced over SMB.
- `enabled`, `hidden`, `read_only`.
- `read_access`, `write_access`: case-insensitive account names or `Users`.

## `users[]`

- `id`: stable unique identifier.
- `account_name`: independent-authentication account.
- `enabled`.
- `protected_password`: Base64 DPAPI blob; edit only through the GUI.

## `app`

Startup, automatic server startup, minimize/close behavior, light mode, exit confirmation and UI language. `language` is reserved for future localization; version 0.1 renders Simplified Chinese.

## `logging`

- `level`: `error`, `warning`, `information`, `debug`, `verbose`, `trace`.
- `max_file_mib`: 1–1024.
- `retained_files`: 1–100.
- `gui_tail_lines`: 50–10000.

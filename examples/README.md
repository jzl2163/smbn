# Configuration example

`config.integrated.example.json` demonstrates Windows integrated authentication with IPv4 and IPv6 Direct TCP listeners.

Before using it:

1. Create `C:\SmbnShares\Public` and set appropriate NTFS permissions.
2. Restrict the CIDR allow list to the actual trusted networks.
3. Check whether Windows LanmanServer already owns TCP 445.
4. Copy it to `%LOCALAPPDATA%\Smbn\config.json` only while SMBN is stopped.
5. Run the built-in diagnostics before starting the service.

Independent-account configurations should be created through the GUI because `protected_password` must be a DPAPI CurrentUser blob, not plaintext or a reusable example value.

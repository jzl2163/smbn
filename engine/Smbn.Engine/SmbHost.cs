using System.IO;
using System.Net;
using System.Net.NetworkInformation;
using System.Net.Sockets;
using System.Reflection;
using System.Security.Principal;
using SMBLibrary;
using SMBLibrary.Authentication.GSSAPI;
using SMBLibrary.Authentication.NTLM;
using SMBLibrary.Server;
using SMBLibrary.Win32;
using SMBLibrary.Win32.Security;

namespace Smbn.Engine;

internal sealed class SmbHost : IDisposable
{
    private readonly object _gate = new();
    private readonly string _logDirectory;
    private readonly List<RuntimeListener> _listeners = [];
    private CredentialVault? _credentials;
    private RollingLog? _log;
    private EngineConfig? _lastConfig;
    private EngineState _state = EngineState.Stopped;
    private DateTimeOffset? _startedAt;
    private string? _lastError;

    internal SmbHost(string logDirectory)
    {
        _logDirectory = logDirectory;
    }

    internal void Start(EngineConfig config)
    {
        lock (_gate)
        {
            if (_state is EngineState.Starting or EngineState.Running or EngineState.Stopping)
            {
                throw new InvalidOperationException("SMB service is already active or changing state.");
            }
            _state = EngineState.Starting;
            _lastError = null;
        }

        try
        {
            Validate(config);
            RollingLog newLog = new(_logDirectory, config.Logging);
            CredentialVault? newCredentials = config.Server.Authentication == AuthenticationMode.Independent
                ? new CredentialVault(config.Users)
                : null;
            NetworkPolicy networkPolicy = new(config.Server.AllowRemoteSubnets, config.Server.RejectRemoteSubnets);
            List<RuntimeListener> started = [];
            HashSet<string> nbnsAddresses = new(StringComparer.OrdinalIgnoreCase);

            try
            {
                foreach (ListenerConfig listener in config.Listeners.Where(static item => item.Enabled))
                {
                    RuntimeListener runtime = StartListener(listener, config, newCredentials, networkPolicy, newLog);
                    started.Add(runtime);

                    if (listener.NetbiosNameService)
                    {
                        if (!nbnsAddresses.Add(listener.Address))
                        {
                            throw new InvalidOperationException($"Only one NetBIOS name service can bind {listener.Address}:137.");
                        }
                        IPAddress address = IPAddress.Parse(listener.Address);
                        IPAddress mask = FindSubnetMask(address)
                            ?? throw new InvalidOperationException($"Unable to determine the IPv4 subnet mask for {address}.");
                        ConfigurableNameServer nameServer = new(address, mask, config.Server.NetbiosName, config.Server.Workgroup);
                        nameServer.Start();
                        runtime.NameServer = nameServer;
                        newLog.Write(listener.Id, $"NetBIOS name service started on {address}:137 as {config.Server.NetbiosName}.");
                    }
                }
            }
            catch
            {
                foreach (RuntimeListener listener in started.AsEnumerable().Reverse())
                {
                    listener.Dispose();
                }
                newCredentials?.Dispose();
                newLog.Dispose();
                throw;
            }

            lock (_gate)
            {
                _log?.Dispose();
                _credentials?.Dispose();
                _listeners.AddRange(started);
                _credentials = newCredentials;
                _log = newLog;
                _lastConfig = config;
                _startedAt = DateTimeOffset.UtcNow;
                _state = EngineState.Running;
                _log.Write("engine", $"SMB service started with {_listeners.Count} listener(s).");
            }

            ClearTransportPasswords(config);
        }
        catch (Exception exception)
        {
            lock (_gate)
            {
                _state = EngineState.Faulted;
                _lastError = exception.Message;
            }
            ClearTransportPasswords(config);
            throw;
        }
    }

    internal void Stop()
    {
        List<RuntimeListener> listeners;
        lock (_gate)
        {
            if (_state == EngineState.Stopped)
            {
                return;
            }
            _state = EngineState.Stopping;
            listeners = _listeners.ToList();
            _listeners.Clear();
        }

        List<Exception> failures = [];
        foreach (RuntimeListener listener in listeners.AsEnumerable().Reverse())
        {
            try { listener.Dispose(); }
            catch (Exception exception) { failures.Add(exception); }
        }

        lock (_gate)
        {
            _credentials?.Dispose();
            _credentials = null;
            _startedAt = null;
            if (failures.Count == 0)
            {
                _state = EngineState.Stopped;
                _lastError = null;
                _log?.Write("engine", "SMB service stopped.");
            }
            else
            {
                _state = EngineState.Faulted;
                _lastError = string.Join("; ", failures.Select(static item => item.Message));
                _log?.Write("engine", $"SMB service stopped with errors: {_lastError}");
            }
        }

        if (failures.Count > 0)
        {
            throw new AggregateException("One or more listeners failed to stop cleanly.", failures);
        }
    }

    internal EngineStatus GetStatus()
    {
        lock (_gate)
        {
            List<SessionInfo> sessions = GetSessionsUnsafe();
            return new EngineStatus
            {
                State = _state,
                StartedAtUtc = _startedAt?.ToString("O"),
                UptimeSeconds = _startedAt.HasValue
                    ? (ulong)Math.Max(0, (DateTimeOffset.UtcNow - _startedAt.Value).TotalSeconds)
                    : 0,
                ListenerCount = _listeners.Count,
                ShareCount = _state == EngineState.Running ? _lastConfig?.Shares.Count(static item => item.Enabled) ?? 0 : 0,
                UserCount = _state == EngineState.Running ? (_lastConfig?.Users.Count ?? 0) : 0,
                SessionCount = sessions.Count,
                OpenFileCount = sessions.Sum(static item => item.OpenFileCount),
                LastError = _lastError,
                EngineVersion = Assembly.GetExecutingAssembly().GetName().Version?.ToString() ?? "unknown",
                SmblibraryVersion = typeof(SMBLibrary.Server.SMBServer).Assembly.GetName().Version?.ToString() ?? "unknown",
                DroppedLogEntries = _log?.DroppedEntries ?? 0,
            };
        }
    }

    internal List<SessionInfo> GetSessions()
    {
        lock (_gate)
        {
            return GetSessionsUnsafe();
        }
    }

    internal bool TerminateSession(string listenerId, string clientEndpoint)
    {
        lock (_gate)
        {
            RuntimeListener? listener = _listeners.FirstOrDefault(item =>
                string.Equals(item.Id, listenerId, StringComparison.OrdinalIgnoreCase));
            if (listener is null)
            {
                return false;
            }
            SessionInformation? session = listener.Server.GetSessionsInformation().FirstOrDefault(item =>
                string.Equals(item.ClientEndPoint.ToString(), clientEndpoint, StringComparison.OrdinalIgnoreCase));
            if (session is null)
            {
                return false;
            }
            listener.Server.TerminateConnection(session.ClientEndPoint);
            _log?.Write("engine", $"Terminated session {clientEndpoint} on {listenerId}.");
            return true;
        }
    }

    internal LogTail TailLogs(int maximum) => _log?.Tail(maximum) ?? new LogTail();

    internal DiagnosticsResult RunDiagnostics(EngineConfig? candidate)
    {
        EngineConfig config = candidate ?? _lastConfig ?? new EngineConfig();
        DiagnosticsResult result = new();

        void Add(string severity, string name, string message) =>
            result.Checks.Add(new DiagnosticCheck { Severity = severity, Name = name, Message = message });

        try
        {
            Validate(config);
            Add("information", "configuration", "Configuration structure is valid.");
        }
        catch (Exception exception)
        {
            Add("error", "configuration", exception.Message);
        }

        bool running;
        lock (_gate) { running = _state == EngineState.Running; }
        foreach (ListenerConfig listener in config.Listeners.Where(static item => item.Enabled))
        {
            if (!IPAddress.TryParse(listener.Address, out IPAddress? address))
            {
                Add("error", $"listener:{listener.Id}", "The address is not a valid IPv4 or IPv6 literal.");
                continue;
            }
            if (running)
            {
                Add("information", $"listener:{listener.Id}", "Port probe skipped because the SMB service is running.");
                continue;
            }
            try
            {
                using Socket socket = new(address.AddressFamily, SocketType.Stream, ProtocolType.Tcp);
                socket.ExclusiveAddressUse = true;
                socket.Bind(new IPEndPoint(address, listener.Port));
                Add("information", $"listener:{listener.Id}", $"{FormatEndpoint(address, listener.Port)} is available.");
            }
            catch (SocketException exception)
            {
                Add("error", $"listener:{listener.Id}", $"Cannot bind {FormatEndpoint(address, listener.Port)}: {exception.Message}");
            }
        }

        foreach (ShareConfig share in config.Shares.Where(static item => item.Enabled))
        {
            if (!Directory.Exists(share.Path))
            {
                Add("error", $"share:{share.Name}", $"Directory does not exist: {share.Path}");
                continue;
            }
            try
            {
                _ = Directory.EnumerateFileSystemEntries(share.Path).Take(1).ToArray();
                Add("information", $"share:{share.Name}", "Directory is readable by the current account.");
            }
            catch (Exception exception)
            {
                Add("error", $"share:{share.Name}", $"Directory access failed: {exception.Message}");
            }
        }

        if (config.Server.EnableSmb1)
        {
            Add("warning", "protocol:smb1", "SMB1 is enabled. Keep it disabled unless a legacy client strictly requires it.");
        }
        if (config.Listeners.Any(static item => item.Enabled && item.Port is 139 or 445))
        {
            Add("warning", "windows-port-conflict", "Windows LanmanServer may already own TCP 139/445. Stop or rebind that service, or choose another port.");
        }
        if (config.Listeners.Any(static item => item.Enabled && item.Port is not (139 or 445)))
        {
            Add("warning", "custom-port", "Windows Explorer does not provide a UNC syntax for an arbitrary SMB port; use a client that supports a custom port or a port-forwarding rule.");
        }
        if (!IsElevated())
        {
            Add("warning", "privilege", "The engine is not elevated. Low ports, firewall changes, or protected directories may require administrator rights.");
        }
        Add("information", "firewall", "Ensure the selected TCP ports and optional UDP/137 are allowed only on the intended Windows Firewall profiles.");
        return result;
    }

    internal static void TrimMemory()
    {
        GC.Collect(GC.MaxGeneration, GCCollectionMode.Optimized, blocking: false, compacting: false);
    }

    private RuntimeListener StartListener(
        ListenerConfig listener,
        EngineConfig config,
        CredentialVault? credentials,
        NetworkPolicy policy,
        RollingLog log)
    {
        IPAddress address = IPAddress.Parse(listener.Address);
        NTLMAuthenticationProviderBase authentication = config.Server.Authentication switch
        {
            AuthenticationMode.IntegratedWindows => new IntegratedNTLMAuthenticationProvider(),
            AuthenticationMode.Independent => new NamedIndependentAuthenticationProvider(
                (credentials ?? throw new InvalidOperationException("Independent authentication has no credential store.")).GetPassword,
                config.Server.NetbiosName),
            _ => throw new ArgumentOutOfRangeException(nameof(config.Server.Authentication)),
        };

        SMBShareCollection shares = BuildShares(config.Shares);
        GSSProvider securityProvider = new(authentication);
        ConfigurableSmbServer server = new(shares, securityProvider);
        server.ConnectionRequested += (_, args) =>
        {
            args.Accept = policy.Allows(args.IPEndPoint.Address);
            if (!args.Accept)
            {
                log.Write(listener.Id, $"Rejected connection from {args.IPEndPoint} by network policy.");
            }
        };
        server.LogEntryAdded += (_, entry) => log.WriteSmbLibrary(listener.Id, entry);

        SMBTransportType transport = listener.Transport switch
        {
            Transport.DirectTcp => SMBTransportType.DirectTCPTransport,
            Transport.NetbiosOverTcp => SMBTransportType.NetBiosOverTCP,
            _ => throw new ArgumentOutOfRangeException(nameof(listener.Transport)),
        };
        TimeSpan? inactivity = config.Server.InactivityTimeoutSeconds == 0
            ? null
            : TimeSpan.FromSeconds(Math.Min(config.Server.InactivityTimeoutSeconds, 31_536_000UL));

        server.StartConfigured(
            address,
            transport,
            listener.Port,
            config.Server.EnableSmb1,
            config.Server.EnableSmb2,
            config.Server.EnableSmb3,
            inactivity);
        log.Write(listener.Id, $"Listening on {FormatEndpoint(address, listener.Port)} via {listener.Transport}.");
        return new RuntimeListener(listener.Id, server);
    }

    private static SMBShareCollection BuildShares(IEnumerable<ShareConfig> settings)
    {
        SMBShareCollection shares = new();
        foreach (ShareConfig item in settings.Where(static share => share.Enabled))
        {
            string wireName = item.Hidden && !item.Name.EndsWith('$') ? item.Name + "$" : item.Name;
            HashSet<string> readers = new(item.ReadAccess, StringComparer.OrdinalIgnoreCase);
            HashSet<string> writers = new(item.WriteAccess, StringComparer.OrdinalIgnoreCase);
            FileSystemShare share = new(wireName, new NTDirectoryFileSystem(item.Path));
            share.AccessRequested += (_, args) =>
            {
                bool authenticated = !string.IsNullOrWhiteSpace(args.UserName);
                bool mayRead = authenticated && (readers.Contains("Users") || readers.Contains(args.UserName));
                bool mayWrite = authenticated && !item.ReadOnly && (writers.Contains("Users") || writers.Contains(args.UserName));
                args.Allow = args.RequestedAccess switch
                {
                    FileAccess.Read => mayRead,
                    FileAccess.Write => mayWrite,
                    _ => mayRead && mayWrite,
                };
            };
            shares.Add(share);
        }
        return shares;
    }

    private List<SessionInfo> GetSessionsUnsafe()
    {
        List<SessionInfo> result = [];
        foreach (RuntimeListener listener in _listeners)
        {
            try
            {
                foreach (SessionInformation session in listener.Server.GetSessionsInformation())
                {
                    result.Add(new SessionInfo
                    {
                        ListenerId = listener.Id,
                        ClientEndpoint = session.ClientEndPoint.ToString(),
                        Dialect = session.Dialect.ToString(),
                        UserName = session.UserName ?? string.Empty,
                        MachineName = session.MachineName ?? string.Empty,
                        OpenFileCount = session.OpenFiles?.Count ?? 0,
                        CreatedAtUtc = session.CreationDT.ToUniversalTime().ToString("O"),
                    });
                }
            }
            catch (Exception exception)
            {
                _log?.Write("engine", $"Session query failed for {listener.Id}: {exception.Message}");
            }
        }
        return result;
    }

    private static void Validate(EngineConfig config)
    {
        if (config.Listeners.Count(static item => item.Enabled) == 0)
            throw new ArgumentException("At least one listener must be enabled.");
        if (config.Shares.Count(static item => item.Enabled) == 0)
            throw new ArgumentException("At least one share must be enabled.");
        if (!config.Server.EnableSmb1 && !config.Server.EnableSmb2 && !config.Server.EnableSmb3)
            throw new ArgumentException("At least one SMB protocol family must be enabled.");
        if (config.Server.EnableSmb3 && !config.Server.EnableSmb2)
            throw new ArgumentException("SMB2 must be enabled when SMB3 is enabled.");
        if (config.Server.Authentication == AuthenticationMode.Independent && config.Users.Count == 0)
            throw new ArgumentException("Independent authentication requires at least one user.");
        if (string.IsNullOrWhiteSpace(config.Server.NetbiosName) || config.Server.NetbiosName.Length > 15)
            throw new ArgumentException("NetBIOS name must contain 1 to 15 characters.");
        if (config.Server.NetbiosName.Any(static c => !(char.IsAsciiLetterOrDigit(c) || c == '-')))
            throw new ArgumentException("NetBIOS name may contain only ASCII letters, digits, and hyphens.");
        if (string.IsNullOrWhiteSpace(config.Server.Workgroup) || config.Server.Workgroup.Length > 15)
            throw new ArgumentException("Workgroup name must contain 1 to 15 characters.");
        if (config.Logging.MaxFileMib is < 1 or > 1024 || config.Logging.RetainedFiles is < 1 or > 100 || config.Logging.GuiTailLines is < 50 or > 10_000)
            throw new ArgumentException("Logging limits are outside the supported range.");
        if (config.Server.Authentication == AuthenticationMode.Independent)
        {
            HashSet<string> accounts = new(StringComparer.OrdinalIgnoreCase);
            foreach (PlainUser user in config.Users)
            {
                if (string.IsNullOrWhiteSpace(user.AccountName) || string.IsNullOrEmpty(user.Password) || !accounts.Add(user.AccountName))
                    throw new ArgumentException("Independent accounts must have unique non-empty names and passwords.");
            }
        }

        HashSet<string> listenerIds = new(StringComparer.OrdinalIgnoreCase);
        HashSet<string> endpoints = new(StringComparer.OrdinalIgnoreCase);
        foreach (ListenerConfig listener in config.Listeners.Where(static item => item.Enabled))
        {
            if (string.IsNullOrWhiteSpace(listener.Id) || !listenerIds.Add(listener.Id))
                throw new ArgumentException("Listener IDs must be non-empty and unique.");
            if (listener.Port == 0) throw new ArgumentException($"Listener {listener.Id} has port 0.");
            IPAddress address = IPAddress.Parse(listener.Address);
            string endpointKey = $"{address.AddressFamily}|{address}|{listener.Port}|{listener.Transport}";
            if (!endpoints.Add(endpointKey))
                throw new ArgumentException($"Listener {listener.Id} duplicates an existing endpoint.");
            if (address.AddressFamily == AddressFamily.InterNetworkV6 &&
                (listener.Transport == Transport.NetbiosOverTcp || listener.NetbiosNameService))
                throw new ArgumentException($"Listener {listener.Id}: NetBIOS transport and name service are IPv4-only.");
            if (listener.NetbiosNameService && (address.AddressFamily != AddressFamily.InterNetwork || IPAddress.Equals(address, IPAddress.Any)))
                throw new ArgumentException($"Listener {listener.Id}: NetBIOS name service requires a concrete IPv4 address.");
        }
        HashSet<string> shareNames = new(StringComparer.OrdinalIgnoreCase);
        foreach (ShareConfig share in config.Shares.Where(static item => item.Enabled))
        {
            if (string.IsNullOrWhiteSpace(share.Name) || share.Name.Length > 80)
                throw new ArgumentException("Share names must contain 1 to 80 characters.");
            if (share.Name.IndexOfAny(['\\', '/', '[', ']', ':', ';', '|', '=', ',', '+', '*', '?', '<', '>', '"']) >= 0)
                throw new ArgumentException($"Share name contains an invalid character: {share.Name}");
            string wireName = share.Hidden && !share.Name.EndsWith('$') ? share.Name + "$" : share.Name;
            if (!shareNames.Add(wireName)) throw new ArgumentException($"Duplicate share name: {wireName}");
            if (!Directory.Exists(share.Path)) throw new DirectoryNotFoundException($"Share directory does not exist: {share.Path}");
        }
        _ = new NetworkPolicy(config.Server.AllowRemoteSubnets, config.Server.RejectRemoteSubnets);
    }

    private static IPAddress? FindSubnetMask(IPAddress address)
    {
        foreach (NetworkInterface adapter in NetworkInterface.GetAllNetworkInterfaces())
        {
            foreach (UnicastIPAddressInformation unicast in adapter.GetIPProperties().UnicastAddresses)
            {
                if (unicast.Address.Equals(address))
                {
                    return unicast.IPv4Mask;
                }
            }
        }
        return null;
    }

    private static bool IsElevated()
    {
        using WindowsIdentity identity = WindowsIdentity.GetCurrent();
        return new WindowsPrincipal(identity).IsInRole(WindowsBuiltInRole.Administrator);
    }

    private static string FormatEndpoint(IPAddress address, ushort port) =>
        address.AddressFamily == AddressFamily.InterNetworkV6 ? $"[{address}]:{port}" : $"{address}:{port}";

    private static void ClearTransportPasswords(EngineConfig config)
    {
        foreach (PlainUser user in config.Users)
        {
            user.Password = string.Empty;
        }
    }

    public void Dispose()
    {
        try { Stop(); } catch { }
        _credentials?.Dispose();
        _log?.Dispose();
    }

    private sealed class RuntimeListener : IDisposable
    {
        internal RuntimeListener(string id, ConfigurableSmbServer server)
        {
            Id = id;
            Server = server;
        }
        internal string Id { get; }
        internal ConfigurableSmbServer Server { get; }
        internal ConfigurableNameServer? NameServer { get; set; }
        public void Dispose()
        {
            Exception? first = null;
            try { NameServer?.Dispose(); }
            catch (Exception exception) { first = exception; }
            try { Server.Stop(); }
            catch (Exception exception) { first ??= exception; }
            if (first is not null) throw first;
        }
    }

    private sealed class ConfigurableSmbServer : SMBLibrary.Server.SMBServer
    {
        internal ConfigurableSmbServer(SMBShareCollection shares, GSSProvider securityProvider)
            : base(shares, securityProvider) { }

        internal void StartConfigured(
            IPAddress address,
            SMBTransportType transport,
            int port,
            bool enableSmb1,
            bool enableSmb2,
            bool enableSmb3,
            TimeSpan? inactivityTimeout) =>
            Start(address, transport, port, enableSmb1, enableSmb2, enableSmb3, inactivityTimeout);
    }

    private sealed class NamedIndependentAuthenticationProvider : IndependentNTLMAuthenticationProvider
    {
        private readonly string _serverName;
        internal NamedIndependentAuthenticationProvider(GetUserPassword getPassword, string serverName)
            : base(getPassword)
        {
            _serverName = serverName;
        }
        public override string GetServerName() => _serverName;
    }

    private sealed class CredentialVault : IDisposable
    {
        private readonly Dictionary<string, char[]> _passwords;

        internal CredentialVault(IEnumerable<PlainUser> users)
        {
            _passwords = users.ToDictionary(
                static user => user.AccountName,
                static user => user.Password.ToCharArray(),
                StringComparer.OrdinalIgnoreCase);
        }

        internal string GetPassword(string userName) =>
            _passwords.TryGetValue(userName, out char[]? password) ? new string(password) : null!;

        public void Dispose()
        {
            foreach (char[] password in _passwords.Values)
            {
                Array.Clear(password, 0, password.Length);
            }
            _passwords.Clear();
        }
    }
}

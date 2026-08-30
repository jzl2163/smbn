namespace Smbn.Engine;

internal sealed class EngineConfig
{
    public ServerConfig Server { get; set; } = new();
    public List<ListenerConfig> Listeners { get; set; } = [];
    public List<ShareConfig> Shares { get; set; } = [];
    public List<PlainUser> Users { get; set; } = [];
    public LoggingConfig Logging { get; set; } = new();
}

internal sealed class ServerConfig
{
    public string NetbiosName { get; set; } = "SMBN";
    public string Workgroup { get; set; } = "WORKGROUP";
    public AuthenticationMode Authentication { get; set; } = AuthenticationMode.Independent;
    public bool EnableSmb1 { get; set; }
    public bool EnableSmb2 { get; set; } = true;
    public bool EnableSmb3 { get; set; } = true;
    public ulong InactivityTimeoutSeconds { get; set; } = 300;
    public List<string> RejectRemoteSubnets { get; set; } = [];
    public List<string> AllowRemoteSubnets { get; set; } = [];
}

internal enum AuthenticationMode
{
    Independent,
    IntegratedWindows,
}

internal sealed class ListenerConfig
{
    public string Id { get; set; } = string.Empty;
    public string Address { get; set; } = "0.0.0.0";
    public ushort Port { get; set; } = 445;
    public Transport Transport { get; set; } = Transport.DirectTcp;
    public bool NetbiosNameService { get; set; }
    public bool Enabled { get; set; } = true;
}

internal enum Transport
{
    DirectTcp,
    NetbiosOverTcp,
}

internal sealed class ShareConfig
{
    public string Id { get; set; } = string.Empty;
    public string Name { get; set; } = string.Empty;
    public string Path { get; set; } = string.Empty;
    public string Comment { get; set; } = string.Empty;
    public bool Enabled { get; set; } = true;
    public bool Hidden { get; set; }
    public bool ReadOnly { get; set; }
    public List<string> ReadAccess { get; set; } = [];
    public List<string> WriteAccess { get; set; } = [];
}

internal sealed class PlainUser
{
    public string AccountName { get; set; } = string.Empty;
    public string Password { get; set; } = string.Empty;
}

internal sealed class LoggingConfig
{
    public LogLevel Level { get; set; } = LogLevel.Information;
    public uint MaxFileMib { get; set; } = 8;
    public uint RetainedFiles { get; set; } = 4;
    public ulong GuiTailLines { get; set; } = 500;
}

internal enum LogLevel
{
    Error,
    Warning,
    Information,
    Debug,
    Verbose,
    Trace,
}

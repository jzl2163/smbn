using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Smbn.Engine;

internal static class IpcProtocol
{
    internal const uint Version = 1;
    internal const int MaxMessageBytes = 8 * 1024 * 1024;

    internal static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DictionaryKeyPolicy = JsonNamingPolicy.SnakeCaseLower,
        PropertyNameCaseInsensitive = true,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
        Converters = { new JsonStringEnumConverter(JsonNamingPolicy.SnakeCaseLower) },
    };

    internal static async Task<RequestEnvelope?> ReadRequestAsync(Stream stream, CancellationToken cancellationToken)
    {
        byte[] lengthBuffer = new byte[sizeof(int)];
        int read = await ReadExactlyOrEofAsync(stream, lengthBuffer, cancellationToken).ConfigureAwait(false);
        if (read == 0)
        {
            return null;
        }

        int length = BinaryPrimitives.ReadInt32LittleEndian(lengthBuffer);
        if (length <= 0 || length > MaxMessageBytes)
        {
            throw new InvalidDataException($"IPC frame length {length} is invalid.");
        }

        byte[] payload = GC.AllocateUninitializedArray<byte>(length);
        try
        {
            await stream.ReadExactlyAsync(payload, cancellationToken).ConfigureAwait(false);
            return JsonSerializer.Deserialize<RequestEnvelope>(payload, JsonOptions)
                ?? throw new InvalidDataException("IPC request is empty.");
        }
        finally
        {
            CryptographicOperations.ZeroMemory(payload);
        }
    }

    internal static async Task WriteResponseAsync(Stream stream, ResponseEnvelope response, CancellationToken cancellationToken)
    {
        byte[] payload = JsonSerializer.SerializeToUtf8Bytes(response, JsonOptions);
        if (payload.Length > MaxMessageBytes)
        {
            throw new InvalidDataException("IPC response exceeds the configured limit.");
        }

        byte[] lengthBuffer = new byte[sizeof(int)];
        BinaryPrimitives.WriteInt32LittleEndian(lengthBuffer, payload.Length);
        await stream.WriteAsync(lengthBuffer, cancellationToken).ConfigureAwait(false);
        await stream.WriteAsync(payload, cancellationToken).ConfigureAwait(false);
        await stream.FlushAsync(cancellationToken).ConfigureAwait(false);
    }

    private static async Task<int> ReadExactlyOrEofAsync(Stream stream, Memory<byte> buffer, CancellationToken cancellationToken)
    {
        int total = 0;
        while (total < buffer.Length)
        {
            int count = await stream.ReadAsync(buffer[total..], cancellationToken).ConfigureAwait(false);
            if (count == 0)
            {
                if (total == 0)
                {
                    return 0;
                }
                throw new EndOfStreamException("IPC frame header ended unexpectedly.");
            }
            total += count;
        }
        return total;
    }
}

internal sealed class RequestEnvelope
{
    public uint Version { get; set; }
    public ulong Id { get; set; }
    public string Token { get; set; } = string.Empty;
    public string Command { get; set; } = string.Empty;
    public JsonElement Payload { get; set; }
}

internal sealed class ResponseEnvelope
{
    public uint Version { get; set; } = IpcProtocol.Version;
    public ulong Id { get; set; }
    public bool Ok { get; set; }
    public object? Payload { get; set; }
    public EngineError? Error { get; set; }

    public static ResponseEnvelope Success(ulong id, object? payload = null) =>
        new() { Id = id, Ok = true, Payload = payload ?? new { } };

    public static ResponseEnvelope Failure(ulong id, string code, string message, string? detail = null) =>
        new() { Id = id, Ok = false, Error = new EngineError { Code = code, Message = message, Detail = detail } };
}

internal sealed class EngineError
{
    public string Code { get; set; } = string.Empty;
    public string Message { get; set; } = string.Empty;
    public string? Detail { get; set; }
}

internal sealed class StartPayload
{
    public EngineConfig Config { get; set; } = new();
}

internal enum EngineState
{
    Stopped,
    Starting,
    Running,
    Stopping,
    Faulted,
}

internal sealed class EngineStatus
{
    public EngineState State { get; set; }
    public string? StartedAtUtc { get; set; }
    public ulong UptimeSeconds { get; set; }
    public int ListenerCount { get; set; }
    public int ShareCount { get; set; }
    public int UserCount { get; set; }
    public int SessionCount { get; set; }
    public int OpenFileCount { get; set; }
    public string? LastError { get; set; }
    public string EngineVersion { get; set; } = string.Empty;
    public string SmblibraryVersion { get; set; } = string.Empty;
    public ulong DroppedLogEntries { get; set; }
}

internal sealed class SessionInfo
{
    public string ListenerId { get; set; } = string.Empty;
    public string ClientEndpoint { get; set; } = string.Empty;
    public string Dialect { get; set; } = string.Empty;
    public string UserName { get; set; } = string.Empty;
    public string MachineName { get; set; } = string.Empty;
    public int OpenFileCount { get; set; }
    public string CreatedAtUtc { get; set; } = string.Empty;
}

internal sealed class DiagnosticsResult
{
    public List<DiagnosticCheck> Checks { get; set; } = [];
}

internal sealed class DiagnosticCheck
{
    public string Severity { get; set; } = "information";
    public string Name { get; set; } = string.Empty;
    public string Message { get; set; } = string.Empty;
}

internal sealed class LogTail
{
    public List<string> Lines { get; set; } = [];
    public bool Truncated { get; set; }
}

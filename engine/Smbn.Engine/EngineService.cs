using System.IO.Pipes;
using System.Runtime;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;

namespace Smbn.Engine;

internal sealed class EngineService : IAsyncDisposable
{
    private readonly string _pipeName;
    private readonly byte[] _expectedToken;
    private readonly SmbHost _host;
    private readonly CancellationTokenSource _shutdown = new();
    private volatile bool _shutdownRequested;

    internal EngineService(string pipeName, string token, string logDirectory)
    {
        _pipeName = ValidatePipeName(pipeName);
        _expectedToken = Encoding.UTF8.GetBytes(token);
        _host = new SmbHost(logDirectory);
    }

    internal async Task RunAsync(CancellationToken externalCancellation)
    {
        using CancellationTokenSource linked = CancellationTokenSource.CreateLinkedTokenSource(_shutdown.Token, externalCancellation);
        CancellationToken cancellationToken = linked.Token;

        while (!cancellationToken.IsCancellationRequested)
        {
            await using NamedPipeServerStream pipe = new(
                _pipeName,
                PipeDirection.InOut,
                NamedPipeServerStream.MaxAllowedServerInstances,
                PipeTransmissionMode.Byte,
                PipeOptions.Asynchronous | PipeOptions.CurrentUserOnly | PipeOptions.WriteThrough,
                64 * 1024,
                64 * 1024);

            try
            {
                await pipe.WaitForConnectionAsync(cancellationToken).ConfigureAwait(false);
                RequestEnvelope? request = await IpcProtocol.ReadRequestAsync(pipe, cancellationToken).ConfigureAwait(false);
                if (request is null)
                {
                    continue;
                }
                ResponseEnvelope response = Handle(request);
                await IpcProtocol.WriteResponseAsync(pipe, response, CancellationToken.None).ConfigureAwait(false);
                if (_shutdownRequested)
                {
                    _shutdown.Cancel();
                }
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                break;
            }
            catch (IOException)
            {
                // A GUI process can disappear mid-request; accept the next connection.
            }
            catch (Exception exception)
            {
                try
                {
                    if (pipe.IsConnected)
                    {
                        await IpcProtocol.WriteResponseAsync(
                            pipe,
                            ResponseEnvelope.Failure(0, "ipc_failure", "IPC request failed.", exception.ToString()),
                            CancellationToken.None).ConfigureAwait(false);
                    }
                }
                catch { }
            }
        }
    }

    private ResponseEnvelope Handle(RequestEnvelope request)
    {
        if (request.Version != IpcProtocol.Version)
        {
            return ResponseEnvelope.Failure(request.Id, "protocol_version", $"Unsupported IPC protocol version {request.Version}.");
        }
        if (!TokenMatches(request.Token))
        {
            return ResponseEnvelope.Failure(request.Id, "unauthorized", "The IPC authentication token is invalid.");
        }

        try
        {
            switch (request.Command)
            {
                case "ping":
                    return ResponseEnvelope.Success(request.Id, new { engine_version = typeof(EngineService).Assembly.GetName().Version?.ToString() ?? "unknown" });

                case "start":
                {
                    StartPayload payload = Deserialize<StartPayload>(request.Payload);
                    _host.Start(payload.Config);
                    SetLatencyMode(GCLatencyMode.SustainedLowLatency);
                    return ResponseEnvelope.Success(request.Id);
                }

                case "stop":
                    _host.Stop();
                    SetLatencyMode(GCLatencyMode.Interactive);
                    return ResponseEnvelope.Success(request.Id);

                case "status":
                    return ResponseEnvelope.Success(request.Id, _host.GetStatus());

                case "sessions":
                    return ResponseEnvelope.Success(request.Id, _host.GetSessions());

                case "terminate_session":
                {
                    TerminateSessionPayload payload = Deserialize<TerminateSessionPayload>(request.Payload);
                    bool terminated = _host.TerminateSession(payload.ListenerId, payload.ClientEndpoint);
                    return terminated
                        ? ResponseEnvelope.Success(request.Id)
                        : ResponseEnvelope.Failure(request.Id, "session_not_found", "The selected SMB session no longer exists.");
                }

                case "tail_logs":
                {
                    TailLogsPayload payload = Deserialize<TailLogsPayload>(request.Payload);
                    return ResponseEnvelope.Success(request.Id, _host.TailLogs(Math.Clamp(payload.Maximum, 1, 10_000)));
                }

                case "diagnostics":
                {
                    DiagnosticsPayload? payload = request.Payload.ValueKind is JsonValueKind.Undefined or JsonValueKind.Null
                        ? null
                        : Deserialize<DiagnosticsPayload>(request.Payload);
                    return ResponseEnvelope.Success(request.Id, _host.RunDiagnostics(payload?.Config));
                }

                case "trim_memory":
                    SmbHost.TrimMemory();
                    return ResponseEnvelope.Success(request.Id);

                case "shutdown":
                    try { _host.Stop(); } catch { }
                    _shutdownRequested = true;
                    return ResponseEnvelope.Success(request.Id);

                default:
                    return ResponseEnvelope.Failure(request.Id, "unknown_command", $"Unknown command: {request.Command}");
            }
        }
        catch (JsonException exception)
        {
            return ResponseEnvelope.Failure(request.Id, "invalid_payload", "The request payload is invalid.", exception.Message);
        }
        catch (Exception exception)
        {
            return ResponseEnvelope.Failure(request.Id, "engine_error", exception.Message, exception.ToString());
        }
    }


    private static void SetLatencyMode(GCLatencyMode mode)
    {
        try { GCSettings.LatencyMode = mode; }
        catch (InvalidOperationException) { }
    }

    private static T Deserialize<T>(JsonElement payload) where T : class =>
        payload.Deserialize<T>(IpcProtocol.JsonOptions)
        ?? throw new JsonException($"Unable to deserialize {typeof(T).Name}.");

    private bool TokenMatches(string candidate)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(candidate);
        try
        {
            return bytes.Length == _expectedToken.Length && CryptographicOperations.FixedTimeEquals(bytes, _expectedToken);
        }
        finally
        {
            CryptographicOperations.ZeroMemory(bytes);
        }
    }

    private static string ValidatePipeName(string pipeName)
    {
        if (string.IsNullOrWhiteSpace(pipeName) || pipeName.Length > 180 || pipeName.Any(static c => !(char.IsAsciiLetterOrDigit(c) || c is '-' or '_' or '.')))
        {
            throw new ArgumentException("Invalid pipe name.", nameof(pipeName));
        }
        return pipeName;
    }

    public async ValueTask DisposeAsync()
    {
        _shutdown.Cancel();
        _host.Dispose();
        CryptographicOperations.ZeroMemory(_expectedToken);
        _shutdown.Dispose();
        await ValueTask.CompletedTask;
    }

    private sealed class TerminateSessionPayload
    {
        public string ListenerId { get; set; } = string.Empty;
        public string ClientEndpoint { get; set; } = string.Empty;
    }

    private sealed class TailLogsPayload
    {
        public int Maximum { get; set; } = 500;
    }

    private sealed class DiagnosticsPayload
    {
        public EngineConfig? Config { get; set; }
    }
}

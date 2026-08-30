using System.Diagnostics;

namespace Smbn.Engine;

internal static class Program
{
    private static async Task<int> Main(string[] args)
    {
        try
        {
            Arguments options = Arguments.Parse(args);
            Directory.CreateDirectory(options.LogDirectory);

            using CancellationTokenSource lifetime = new();
            Console.CancelKeyPress += (_, eventArgs) =>
            {
                eventArgs.Cancel = true;
                lifetime.Cancel();
            };
            AppDomain.CurrentDomain.ProcessExit += (_, _) => lifetime.Cancel();

            Task? parentWatcher = options.ParentProcessId > 0
                ? WatchParentAsync(options.ParentProcessId, lifetime)
                : null;

            await using EngineService service = new(options.PipeName, options.Token, options.LogDirectory);
            await service.RunAsync(lifetime.Token).ConfigureAwait(false);
            if (parentWatcher is not null)
            {
                lifetime.Cancel();
                await parentWatcher.ConfigureAwait(false);
            }
            return 0;
        }
        catch (OperationCanceledException)
        {
            return 0;
        }
        catch (Exception exception)
        {
            Console.Error.WriteLine(exception);
            return 1;
        }
    }

    private static async Task WatchParentAsync(int processId, CancellationTokenSource lifetime)
    {
        try
        {
            using Process parent = Process.GetProcessById(processId);
            await parent.WaitForExitAsync(lifetime.Token).ConfigureAwait(false);
            lifetime.Cancel();
        }
        catch (ArgumentException)
        {
            lifetime.Cancel();
        }
        catch (InvalidOperationException)
        {
            lifetime.Cancel();
        }
        catch (OperationCanceledException)
        {
        }
    }

    private sealed class Arguments
    {
        internal required string PipeName { get; init; }
        internal required string Token { get; init; }
        internal required string LogDirectory { get; init; }
        internal int ParentProcessId { get; init; }

        internal static Arguments Parse(string[] args)
        {
            Dictionary<string, string> values = new(StringComparer.OrdinalIgnoreCase);
            for (int index = 0; index < args.Length; index++)
            {
                string current = args[index];
                if (!current.StartsWith("--", StringComparison.Ordinal) || index + 1 >= args.Length)
                {
                    throw new ArgumentException($"Invalid argument: {current}");
                }
                values[current[2..]] = args[++index];
            }

            string pipe = Required(values, "pipe");
            string token = Environment.GetEnvironmentVariable("SMBN_IPC_TOKEN")
                ?? throw new ArgumentException("Missing SMBN_IPC_TOKEN environment variable.");
            Environment.SetEnvironmentVariable("SMBN_IPC_TOKEN", null);
            string logDirectory = Path.GetFullPath(Required(values, "log-dir"));
            int parent = values.TryGetValue("parent", out string? parentValue) && int.TryParse(parentValue, out int parsed)
                ? parsed
                : 0;
            return new Arguments { PipeName = pipe, Token = token, LogDirectory = logDirectory, ParentProcessId = parent };
        }

        private static string Required(Dictionary<string, string> values, string name) =>
            values.TryGetValue(name, out string? value) && !string.IsNullOrWhiteSpace(value)
                ? value
                : throw new ArgumentException($"Missing --{name}.");
    }
}

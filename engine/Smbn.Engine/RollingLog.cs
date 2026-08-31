using System.Reflection;
using System.Text;
using System.Threading.Channels;

namespace Smbn.Engine;

/// <summary>
/// Keeps a bounded in-memory tail and writes files on one background task so SMB worker
/// threads never block on disk I/O. The channel is deliberately bounded.
/// </summary>
internal sealed class RollingLog : IDisposable
{
    private readonly object _tailGate = new();
    private readonly Queue<string> _tail;
    private readonly string _directory;
    private readonly int _tailCapacity;
    private readonly long _maxBytes;
    private readonly int _retainedFiles;
    private readonly LogLevel _minimumLevel;
    private readonly Channel<string> _fileQueue;
    private readonly Task _writerTask;
    private StreamWriter? _writer;
    private long _dropped;
    private int _disposed;

    internal RollingLog(string directory, LoggingConfig config)
    {
        _directory = directory;
        _tailCapacity = Math.Clamp((int)Math.Min(config.GuiTailLines, (ulong)int.MaxValue), 50, 10_000);
        _maxBytes = Math.Clamp((long)config.MaxFileMib, 1, 1024) * 1024L * 1024L;
        _retainedFiles = Math.Clamp((int)config.RetainedFiles, 1, 100);
        _minimumLevel = config.Level;
        _tail = new Queue<string>(_tailCapacity);
        _fileQueue = Channel.CreateBounded<string>(new BoundedChannelOptions(4096)
        {
            SingleReader = true,
            SingleWriter = false,
            FullMode = BoundedChannelFullMode.Wait,
            AllowSynchronousContinuations = false,
        });
        Directory.CreateDirectory(_directory);
        OpenWriter();
        _writerTask = Task.Run(WriterLoopAsync);
    }

    internal ulong DroppedEntries => (ulong)Math.Max(0, Interlocked.Read(ref _dropped));

    internal void Write(string source, string message, LogLevel level = LogLevel.Information)
    {
        if (Volatile.Read(ref _disposed) != 0 || (int)level > (int)_minimumLevel)
        {
            return;
        }

        string line = $"{DateTimeOffset.UtcNow:O} [{level}] [{source}] {Sanitize(message)}";
        lock (_tailGate)
        {
            if (_tail.Count >= _tailCapacity)
            {
                _tail.Dequeue();
                Interlocked.Increment(ref _dropped);
            }
            _tail.Enqueue(line);
        }

        if (!_fileQueue.Writer.TryWrite(line))
        {
            Interlocked.Increment(ref _dropped);
        }
    }

    internal void WriteSmbLibrary(string source, object entry)
    {
        try
        {
            string? severity = ReadMember(entry, "Severity") ?? ReadMember(entry, "LogLevel");
            Write(source, entry.ToString() ?? "SMBLibrary log entry", ParseLevel(severity));
        }
        catch
        {
            Write(source, "SMBLibrary emitted an unreadable log entry.", LogLevel.Warning);
        }
    }

    internal LogTail Tail(int maximum)
    {
        lock (_tailGate)
        {
            int take = Math.Clamp(maximum, 1, _tailCapacity);
            string[] all = _tail.ToArray();
            int skip = Math.Max(0, all.Length - take);
            return new LogTail
            {
                Lines = all.Skip(skip).ToList(),
                Truncated = DroppedEntries > 0 || skip > 0,
            };
        }
    }

    private async Task WriterLoopAsync()
    {
        try
        {
            while (await _fileQueue.Reader.WaitToReadAsync().ConfigureAwait(false))
            {
                while (_fileQueue.Reader.TryRead(out string? line))
                {
                    try
                    {
                        EnsureWriter();
                        _writer!.WriteLine(line);
                    }
                    catch
                    {
                        Interlocked.Increment(ref _dropped);
                        CloseWriter();
                    }
                }
                try { _writer?.Flush(); }
                catch
                {
                    Interlocked.Increment(ref _dropped);
                    CloseWriter();
                }
            }
        }
        finally
        {
            try { _writer?.Flush(); } catch { }
            CloseWriter();
        }
    }

    private void EnsureWriter()
    {
        if (_writer is null)
        {
            OpenWriter();
        }
        if (_writer!.BaseStream.Length >= _maxBytes)
        {
            CloseWriter();
            Rotate();
            OpenWriter();
        }
    }

    private void OpenWriter()
    {
        string path = Path.Combine(_directory, "smbn-engine.log");
        _writer = new StreamWriter(
            new FileStream(path, FileMode.Append, FileAccess.Write, FileShare.ReadWrite),
            new UTF8Encoding(false))
        {
            AutoFlush = false,
        };
    }

    private void CloseWriter()
    {
        try { _writer?.Dispose(); } catch { }
        _writer = null;
    }

    private void Rotate()
    {
        string basePath = Path.Combine(_directory, "smbn-engine.log");
        string oldest = $"{basePath}.{_retainedFiles}";
        if (File.Exists(oldest))
        {
            File.Delete(oldest);
        }
        for (int index = _retainedFiles - 1; index >= 1; index--)
        {
            string source = $"{basePath}.{index}";
            string destination = $"{basePath}.{index + 1}";
            if (File.Exists(source))
            {
                File.Move(source, destination, true);
            }
        }
        if (File.Exists(basePath))
        {
            File.Move(basePath, $"{basePath}.1", true);
        }
    }

    private static string? ReadMember(object value, string name)
    {
        Type type = value.GetType();
        const BindingFlags flags = BindingFlags.Instance | BindingFlags.Public;
        return type.GetProperty(name, flags)?.GetValue(value)?.ToString()
            ?? type.GetField(name, flags)?.GetValue(value)?.ToString();
    }

    private static LogLevel ParseLevel(string? value)
    {
        if (string.IsNullOrWhiteSpace(value)) return LogLevel.Information;
        if (value.Contains("error", StringComparison.OrdinalIgnoreCase) || value.Contains("critical", StringComparison.OrdinalIgnoreCase)) return LogLevel.Error;
        if (value.Contains("warn", StringComparison.OrdinalIgnoreCase)) return LogLevel.Warning;
        if (value.Contains("trace", StringComparison.OrdinalIgnoreCase)) return LogLevel.Trace;
        if (value.Contains("verbose", StringComparison.OrdinalIgnoreCase)) return LogLevel.Verbose;
        if (value.Contains("debug", StringComparison.OrdinalIgnoreCase)) return LogLevel.Debug;
        return LogLevel.Information;
    }

    private static string Sanitize(string value) => value.Replace('\r', ' ').Replace('\n', ' ');

    public void Dispose()
    {
        if (Interlocked.Exchange(ref _disposed, 1) != 0)
        {
            return;
        }
        _fileQueue.Writer.TryComplete();
        try { _writerTask.GetAwaiter().GetResult(); }
        catch { CloseWriter(); }
    }
}

using System.Diagnostics;

namespace ModelMerger.Core.Merging;

internal interface IRustWorkerProcess : IAsyncDisposable
{
    bool HasExited { get; }

    Task WriteLineAsync(string line, CancellationToken cancellationToken);

    Task<string?> ReadLineAsync(CancellationToken cancellationToken);

    Task WaitForExitAsync(CancellationToken cancellationToken);

    void Kill();
}

internal sealed class ProcessRustWorker : IRustWorkerProcess
{
    private readonly Process _process;

    private ProcessRustWorker(Process process)
    {
        _process = process;
        _ = process.StandardError.ReadToEndAsync();
    }

    public bool HasExited => _process.HasExited;

    public static ProcessRustWorker Start(string executablePath)
    {
        if (!File.Exists(executablePath))
        {
            throw new FileNotFoundException(
                "Rust merge worker was not found. Build or install model-merger-worker.exe.",
                executablePath);
        }

        var process = Process.Start(new ProcessStartInfo
        {
            FileName = executablePath,
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true
        }) ?? throw new InvalidOperationException("Unable to start Rust merge worker.");
        return new ProcessRustWorker(process);
    }

    public async Task WriteLineAsync(string line, CancellationToken cancellationToken)
    {
        await _process.StandardInput.WriteLineAsync(line.AsMemory(), cancellationToken)
            .ConfigureAwait(false);
        await _process.StandardInput.FlushAsync(cancellationToken).ConfigureAwait(false);
    }

    public Task<string?> ReadLineAsync(CancellationToken cancellationToken) =>
        _process.StandardOutput.ReadLineAsync(cancellationToken).AsTask();

    public Task WaitForExitAsync(CancellationToken cancellationToken) =>
        _process.WaitForExitAsync(cancellationToken);

    public void Kill()
    {
        if (!_process.HasExited)
        {
            _process.Kill(entireProcessTree: true);
        }
    }

    public ValueTask DisposeAsync()
    {
        _process.Dispose();
        return ValueTask.CompletedTask;
    }
}

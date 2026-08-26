namespace ModelMerger.Core.Merging;

internal static class RustWorkerLifetime
{
    private static readonly TimeSpan ShutdownTimeout = TimeSpan.FromSeconds(2);

    public static async Task CancelAndDisposeAsync(
        IRustWorkerProcess process,
        string cancelCommand)
    {
        try
        {
            if (!process.HasExited)
            {
                await process.WriteLineAsync(cancelCommand, CancellationToken.None)
                    .ConfigureAwait(false);
            }
        }
        catch (Exception exception) when (
            exception is IOException or InvalidOperationException or ObjectDisposedException)
        {
            // The stop path below terminates a worker that no longer accepts input.
        }

        await StopAndDisposeAsync(process).ConfigureAwait(false);
    }

    public static async Task StopAndDisposeAsync(IRustWorkerProcess process)
    {
        try
        {
            if (!process.HasExited)
            {
                using var timeout = new CancellationTokenSource(ShutdownTimeout);
                try
                {
                    await process.WaitForExitAsync(timeout.Token).ConfigureAwait(false);
                }
                catch (OperationCanceledException) when (timeout.IsCancellationRequested)
                {
                    process.Kill();
                }
            }
        }
        finally
        {
            await process.DisposeAsync().ConfigureAwait(false);
        }
    }
}

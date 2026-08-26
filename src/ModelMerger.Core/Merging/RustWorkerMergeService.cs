namespace ModelMerger.Core.Merging;

public sealed class RustWorkerMergeService : IModelMergeService
{
    private readonly Func<IRustWorkerProcess> _processFactory;
    private readonly IMergeOutputClaims _directInvocationClaims = MergeOutputClaims.Shared;

    public RustWorkerMergeService()
        : this(Path.Combine(AppContext.BaseDirectory, "model-merger-worker.exe"))
    {
    }

    public RustWorkerMergeService(string workerExecutablePath)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(workerExecutablePath);
        var fullPath = Path.GetFullPath(workerExecutablePath);
        _processFactory = () => ProcessRustWorker.Start(fullPath);
    }

    internal RustWorkerMergeService(Func<IRustWorkerProcess> processFactory)
    {
        _processFactory = processFactory ?? throw new ArgumentNullException(nameof(processFactory));
    }

    public async Task<IPreparedMergeOperation> PrepareAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        cancellationToken.ThrowIfCancellationRequested();
        if (request.InputFiles is null)
        {
            throw new MergeValidationException(
            [
                new MergeValidationError(
                    MergeValidationErrorCode.InvalidPartCount,
                    "A merge requires 2 to 15 model parts.")
            ]);
        }
        var process = _processFactory();
        try
        {
            await process.WriteLineAsync(
                RustWorkerProtocol.CreatePrepareCommand(request),
                cancellationToken).ConfigureAwait(false);
            var outputPath = await RustWorkerProtocol.ReadPreparedAsync(
                process,
                progress,
                cancellationToken).ConfigureAwait(false);
            return new RustPreparedMergeOperation(process, outputPath);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            await RustWorkerLifetime.CancelAndDisposeAsync(process, RustWorkerProtocol.CancelCommand)
                .ConfigureAwait(false);
            throw;
        }
        catch
        {
            await RustWorkerLifetime.StopAndDisposeAsync(process).ConfigureAwait(false);
            throw;
        }
    }

    public async Task<MergeResult> MergeAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        var prepared = await PrepareAsync(request, progress, cancellationToken).ConfigureAwait(false);
        try
        {
            using var outputClaim = _directInvocationClaims.Claim(prepared.OutputPath);
            return await prepared.ExecuteAsync(progress, cancellationToken).ConfigureAwait(false);
        }
        finally
        {
            if (prepared is IAsyncDisposable asyncDisposable)
            {
                await asyncDisposable.DisposeAsync().ConfigureAwait(false);
            }
        }
    }

    private sealed class RustPreparedMergeOperation(
        IRustWorkerProcess process,
        string outputPath) : IPreparedMergeOperation, IAsyncDisposable
    {
        private int _executed;
        private int _disposed;

        public string OutputPath { get; } = outputPath;

        public async Task<MergeResult> ExecuteAsync(
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default)
        {
            if (Interlocked.Exchange(ref _executed, 1) != 0)
            {
                throw new InvalidOperationException("A prepared merge operation can only be executed once.");
            }
            ObjectDisposedException.ThrowIf(Volatile.Read(ref _disposed) != 0, this);

            try
            {
                await process.WriteLineAsync(
                    RustWorkerProtocol.ExecuteCommand,
                    cancellationToken).ConfigureAwait(false);
                var result = await RustWorkerProtocol.ReadResultAsync(
                    process,
                    progress,
                    cancellationToken).ConfigureAwait(false);
                await StopOnceAsync(cancel: false).ConfigureAwait(false);
                return result;
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                await StopOnceAsync(cancel: true).ConfigureAwait(false);
                throw;
            }
            catch
            {
                await StopOnceAsync(cancel: false).ConfigureAwait(false);
                throw;
            }
        }

        public ValueTask DisposeAsync() => new(StopOnceAsync(cancel: true));

        private async Task StopOnceAsync(bool cancel)
        {
            if (Interlocked.Exchange(ref _disposed, 1) != 0)
            {
                return;
            }

            if (cancel)
            {
                await RustWorkerLifetime.CancelAndDisposeAsync(
                    process,
                    RustWorkerProtocol.CancelCommand).ConfigureAwait(false);
            }
            else
            {
                await RustWorkerLifetime.StopAndDisposeAsync(process).ConfigureAwait(false);
            }
        }
    }
}

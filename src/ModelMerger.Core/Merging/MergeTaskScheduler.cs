using System.Collections.Concurrent;

namespace ModelMerger.Core.Merging;

public enum MergeTaskState
{
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled
}

public sealed class MergeTaskHandle
{
    private readonly CancellationTokenSource _cancellation = new();
    private readonly TaskCompletionSource<MergeResult> _completion =
        new(TaskCreationOptions.RunContinuationsAsynchronously);
    private int _state = (int)MergeTaskState.Queued;

    internal MergeTaskHandle(MergeRequest request)
    {
        Request = request;
    }

    public Guid Id { get; } = Guid.NewGuid();

    public MergeRequest Request { get; }

    public MergeTaskState State => (MergeTaskState)Volatile.Read(ref _state);

    public Task<MergeResult> Completion => _completion.Task;

    internal CancellationToken CancellationToken => _cancellation.Token;

    public void Cancel()
    {
        if (State is MergeTaskState.Succeeded or MergeTaskState.Failed or MergeTaskState.Cancelled)
        {
            return;
        }

        try
        {
            _cancellation.Cancel();
        }
        catch (ObjectDisposedException)
        {
            // A terminal transition won the race; cancellation is already unnecessary.
        }
    }

    internal void MarkRunning() => TransitionTo(MergeTaskState.Running);

    internal void MarkSucceeded(MergeResult result)
    {
        TransitionTo(MergeTaskState.Succeeded);
        _completion.TrySetResult(result);
        _cancellation.Dispose();
    }

    internal void MarkFailed(Exception exception)
    {
        TransitionTo(MergeTaskState.Failed);
        _completion.TrySetException(exception);
        _cancellation.Dispose();
    }

    internal void MarkCancelled()
    {
        TransitionTo(MergeTaskState.Cancelled);
        _completion.TrySetCanceled(_cancellation.Token);
        _cancellation.Dispose();
    }

    private void TransitionTo(MergeTaskState state)
    {
        Interlocked.Exchange(ref _state, (int)state);
    }
}

public interface IMergeTaskScheduler : IDisposable
{
    int MaximumConcurrency { get; }

    MergeTaskHandle Schedule(MergeRequest request, IProgress<MergeProgress>? progress = null);
}

public sealed class MergeTaskScheduler : IMergeTaskScheduler, IDisposable
{
    private readonly IModelMergeService _mergeService;
    private readonly SemaphoreSlim _slots;
    private readonly IMergeOutputClaims _outputClaims = MergeOutputClaims.Shared;
    private readonly object _lifecycleGate = new();
    private readonly Dictionary<Guid, MergeTaskHandle> _tasks = [];
    private int _disposed;
    private int _slotsDisposed;

    public MergeTaskScheduler(int maximumConcurrency = 2)
        : this(new ModelMergeService(), maximumConcurrency)
    {
    }

    public MergeTaskScheduler(IModelMergeService mergeService, int maximumConcurrency = 2)
    {
        ArgumentNullException.ThrowIfNull(mergeService);
        if (maximumConcurrency < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(maximumConcurrency));
        }

        _mergeService = mergeService;
        _slots = new SemaphoreSlim(maximumConcurrency, maximumConcurrency);
        MaximumConcurrency = maximumConcurrency;
    }

    public int MaximumConcurrency { get; }

    public MergeTaskHandle Schedule(MergeRequest request, IProgress<MergeProgress>? progress = null)
    {
        ArgumentNullException.ThrowIfNull(request);
        MergeTaskHandle handle;
        lock (_lifecycleGate)
        {
            ObjectDisposedException.ThrowIf(_disposed != 0, this);
            handle = new MergeTaskHandle(request);
            _tasks.Add(handle.Id, handle);
        }

        _ = ExecuteAsync(handle, progress);
        return handle;
    }

    public void Dispose()
    {
        MergeTaskHandle[] tasks;
        var disposeSlots = false;
        lock (_lifecycleGate)
        {
            if (_disposed != 0)
            {
                return;
            }

            _disposed = 1;
            tasks = _tasks.Values.ToArray();
            disposeSlots = TryReserveSlotDisposalWhenIdle();
        }

        foreach (var task in tasks)
        {
            task.Cancel();
        }

        if (disposeSlots)
        {
            _slots.Dispose();
        }
    }

    private async Task ExecuteAsync(MergeTaskHandle handle, IProgress<MergeProgress>? progress)
    {
        var enteredSlot = false;
        try
        {
            await _slots.WaitAsync(handle.CancellationToken).ConfigureAwait(false);
            enteredSlot = true;
            handle.MarkRunning();
            var prepared = await _mergeService
                .PrepareAsync(handle.Request, progress, handle.CancellationToken)
                .ConfigureAwait(false);
            MergeResult result;
            using (_outputClaims.Claim(prepared.OutputPath))
            {
                result = await prepared
                    .ExecuteAsync(progress, handle.CancellationToken)
                    .ConfigureAwait(false);
            }

            handle.MarkSucceeded(result);
        }
        catch (OperationCanceledException) when (handle.CancellationToken.IsCancellationRequested)
        {
            handle.MarkCancelled();
        }
        catch (Exception exception)
        {
            handle.MarkFailed(exception);
        }
        finally
        {
            var disposeSlots = false;
            if (enteredSlot)
            {
                _slots.Release();
            }

            lock (_lifecycleGate)
            {
                _tasks.Remove(handle.Id);
                disposeSlots = TryReserveSlotDisposalWhenIdle();
            }

            if (disposeSlots)
            {
                _slots.Dispose();
            }
        }
    }

    private bool TryReserveSlotDisposalWhenIdle()
    {
        if (_disposed == 0 || _tasks.Count > 0 || _slotsDisposed != 0)
        {
            return false;
        }

        _slotsDisposed = 1;
        return true;
    }
}

public sealed class MergeOutputConflictException(string outputPath)
    : IOException($"Another merge task is already writing to {outputPath}.")
{
    public string OutputPath { get; } = outputPath;
}

internal interface IMergeOutputClaims
{
    IDisposable Claim(string outputPath);
}

internal sealed class MergeOutputClaims : IMergeOutputClaims
{
    public static IMergeOutputClaims Shared { get; } = new MergeOutputClaims();

    private readonly ConcurrentDictionary<string, byte> _claimedPaths =
        new(StringComparer.OrdinalIgnoreCase);

    public IDisposable Claim(string outputPath)
    {
        var normalizedPath = Path.GetFullPath(outputPath);
        if (!_claimedPaths.TryAdd(normalizedPath, 0))
        {
            throw new MergeOutputConflictException(normalizedPath);
        }

        return new OutputClaim(_claimedPaths, normalizedPath);
    }

    private sealed class OutputClaim(
        ConcurrentDictionary<string, byte> claimedPaths,
        string outputPath) : IDisposable
    {
        private int _disposed;

        public void Dispose()
        {
            if (Interlocked.Exchange(ref _disposed, 1) == 0)
            {
                claimedPaths.TryRemove(outputPath, out _);
            }
        }
    }
}

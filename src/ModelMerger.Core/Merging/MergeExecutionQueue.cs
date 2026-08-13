namespace ModelMerger.Core.Merging;

public interface IMergeExecutionQueue
{
    Task<MergeResult> EnqueueAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default);
}

public sealed class MergeExecutionQueue : IMergeExecutionQueue, IDisposable
{
    private readonly IModelMergeService _mergeService;
    private readonly SemaphoreSlim _slots;

    public MergeExecutionQueue(IModelMergeService mergeService, int maximumConcurrency = 2)
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

    public async Task<MergeResult> EnqueueAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        await _slots.WaitAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            return await _mergeService.MergeAsync(request, progress, cancellationToken).ConfigureAwait(false);
        }
        finally
        {
            _slots.Release();
        }
    }

    public void Dispose() => _slots.Dispose();
}

using ModelMerger.Core.Merging;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class MergeExecutionQueueTests
{
    [Fact]
    public async Task EnqueueAsync_WithThreeGroups_RunsNoMoreThanTwoAtOnce()
    {
        var fake = new CountingMergeService();
        var queue = new MergeExecutionQueue(fake, maximumConcurrency: 2);
        var requests = Enumerable.Range(1, 3)
            .Select(index => new MergeRequest(["a.cast", "b.cast"], ".", $"group-{index}.cast"))
            .ToArray();

        var tasks = requests.Select(request => queue.EnqueueAsync(request)).ToArray();
        await fake.WaitUntilStartedAsync(2);

        Assert.Equal(2, fake.MaximumObservedConcurrency);
        Assert.Equal(2, fake.ActiveCount);

        fake.ReleaseAll();
        await Task.WhenAll(tasks);
        Assert.Equal(2, fake.MaximumObservedConcurrency);
    }

    private sealed class CountingMergeService : IModelMergeService
    {
        private readonly SemaphoreSlim _release = new(0);
        private readonly TaskCompletionSource _twoStarted = new(TaskCreationOptions.RunContinuationsAsynchronously);
        private int _activeCount;
        private int _maximumObservedConcurrency;
        private int _startedCount;

        public int ActiveCount => Volatile.Read(ref _activeCount);

        public int MaximumObservedConcurrency => Volatile.Read(ref _maximumObservedConcurrency);

        public async Task<MergeResult> MergeAsync(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default)
        {
            var active = Interlocked.Increment(ref _activeCount);
            UpdateMaximum(active);
            if (Interlocked.Increment(ref _startedCount) >= 2)
            {
                _twoStarted.TrySetResult();
            }

            try
            {
                await _release.WaitAsync(cancellationToken);
                return new MergeResult(request.OutputFileName!, "root", 2, 0, 0, []);
            }
            finally
            {
                Interlocked.Decrement(ref _activeCount);
            }
        }

        public async Task WaitUntilStartedAsync(int count)
        {
            if (count != 2)
            {
                throw new ArgumentOutOfRangeException(nameof(count));
            }

            await _twoStarted.Task.WaitAsync(TimeSpan.FromSeconds(5));
        }

        public void ReleaseAll() => _release.Release(3);

        private void UpdateMaximum(int value)
        {
            int current;
            do
            {
                current = Volatile.Read(ref _maximumObservedConcurrency);
                if (current >= value)
                {
                    return;
                }
            }
            while (Interlocked.CompareExchange(ref _maximumObservedConcurrency, value, current) != current);
        }
    }
}

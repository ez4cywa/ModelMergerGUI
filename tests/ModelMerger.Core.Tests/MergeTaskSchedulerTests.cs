using ModelMerger.Core.Merging;
using PhilLibX;
using PhilLibX.Mathematics;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class MergeTaskSchedulerTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"ModelMergerSchedulerTests-{Guid.NewGuid():N}");

    public MergeTaskSchedulerTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public async Task Schedule_WithThreeTasks_RunsNoMoreThanTwoAndTracksLifecycle()
    {
        var mergeService = new CountingMergeService();
        using var scheduler = new MergeTaskScheduler(mergeService, maximumConcurrency: 2);

        var handles = Enumerable.Range(1, 3)
            .Select(index => scheduler.Schedule(CreateRequest(index)))
            .ToArray();
        await mergeService.WaitUntilStartedAsync(2);

        Assert.Equal(2, mergeService.MaximumObservedConcurrency);
        Assert.Equal(MergeTaskState.Queued, handles[2].State);

        mergeService.ReleaseAll();
        await Task.WhenAll(handles.Select(handle => handle.Completion));

        Assert.All(handles, handle => Assert.Equal(MergeTaskState.Succeeded, handle.State));
        Assert.Equal(2, mergeService.MaximumObservedConcurrency);
    }

    [Fact]
    public async Task Cancel_WhileQueued_DoesNotStartTheTask()
    {
        var mergeService = new CountingMergeService();
        using var scheduler = new MergeTaskScheduler(mergeService, maximumConcurrency: 1);
        var running = scheduler.Schedule(CreateRequest(1));
        await mergeService.WaitUntilStartedAsync(1);
        var queued = scheduler.Schedule(CreateRequest(2));

        queued.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => queued.Completion);
        Assert.Equal(MergeTaskState.Cancelled, queued.State);
        Assert.Equal(1, mergeService.StartedCount);
        mergeService.ReleaseAll();
        await running.Completion;
    }

    [Fact]
    public async Task Cancel_WhileRunning_CancelsOperationAndReleasesSlot()
    {
        var mergeService = new CountingMergeService();
        using var scheduler = new MergeTaskScheduler(mergeService, maximumConcurrency: 1);
        var running = scheduler.Schedule(CreateRequest(1));
        await mergeService.WaitUntilStartedAsync(1);

        running.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => running.Completion);
        Assert.Equal(MergeTaskState.Cancelled, running.State);
        Assert.Equal(0, mergeService.ActiveCount);

        var replacement = scheduler.Schedule(CreateRequest(2));
        mergeService.ReleaseAll();
        await replacement.Completion;
        Assert.Equal(MergeTaskState.Succeeded, replacement.State);
        Assert.Equal(2, mergeService.StartedCount);
    }

    [Fact]
    public async Task Dispose_RacingWithSchedule_EitherRejectsOrCancelsTheTask()
    {
        for (var iteration = 0; iteration < 25; iteration++)
        {
            var scheduler = new MergeTaskScheduler(new CountingMergeService(), maximumConcurrency: 1);
            using var start = new Barrier(2);
            MergeTaskHandle? handle = null;
            Exception? scheduleException = null;
            var schedule = Task.Run(() =>
            {
                start.SignalAndWait();
                try
                {
                    handle = scheduler.Schedule(CreateRequest(iteration));
                }
                catch (Exception exception)
                {
                    scheduleException = exception;
                }
            });
            var dispose = Task.Run(() =>
            {
                start.SignalAndWait();
                scheduler.Dispose();
            });

            await Task.WhenAll(schedule, dispose);
            if (handle is null)
            {
                Assert.IsType<ObjectDisposedException>(scheduleException);
                continue;
            }

            await Assert.ThrowsAnyAsync<OperationCanceledException>(
                () => handle.Completion.WaitAsync(TimeSpan.FromSeconds(10)));
            Assert.Equal(MergeTaskState.Cancelled, handle.State);
        }
    }

    [Fact]
    public async Task Schedule_WithSameOutputPath_RejectsSecondTaskBeforeMeshMerge()
    {
        var bodyPath = Path.Combine(_directory, "body.cast");
        var headPath = Path.Combine(_directory, "head.cast");
        CreateBodyModel().Save(bodyPath);
        CreateHeadModel().Save(headPath);
        using var scheduler = new MergeTaskScheduler(maximumConcurrency: 2);
        using var firstEnteredMerge = new ManualResetEventSlim();
        using var releaseFirst = new ManualResetEventSlim();
        var firstProgress = new CallbackProgress<MergeProgress>(progress =>
        {
            if (progress.Stage == MergeStage.Merging)
            {
                firstEnteredMerge.Set();
                releaseFirst.Wait(TimeSpan.FromSeconds(10));
            }
        });
        var request = new MergeRequest([bodyPath, headPath], _directory, "shared-output.cast");

        var first = scheduler.Schedule(request, firstProgress);
        Assert.True(firstEnteredMerge.Wait(TimeSpan.FromSeconds(10)));
        var second = scheduler.Schedule(request);

        var exception = await Assert.ThrowsAsync<MergeOutputConflictException>(() => second.Completion);
        Assert.Equal(Path.Combine(_directory, "shared-output.cast"), exception.OutputPath);
        Assert.Equal(MergeTaskState.Failed, second.State);
        releaseFirst.Set();
        await first.Completion;
    }

    [Fact]
    public async Task Schedule_AcrossSchedulersWithSameOutput_UsesSharedClaimBeforeExecute()
    {
        var outputPath = Path.Combine(_directory, "prepared-shared.cast");
        var mergeService = new PreparedBlockingMergeService(outputPath);
        using var firstScheduler = new MergeTaskScheduler(mergeService, maximumConcurrency: 1);
        using var secondScheduler = new MergeTaskScheduler(mergeService, maximumConcurrency: 1);
        var first = firstScheduler.Schedule(CreateRequest(1));
        await mergeService.WaitUntilExecuteStartedAsync();

        var second = secondScheduler.Schedule(CreateRequest(2));

        var exception = await Assert.ThrowsAsync<MergeOutputConflictException>(() => second.Completion);
        Assert.Equal(outputPath, exception.OutputPath);
        Assert.Equal(1, mergeService.ExecuteStartedCount);
        mergeService.Release();
        await first.Completion;

        var retryAfterCompletion = secondScheduler.Schedule(CreateRequest(3));
        await retryAfterCompletion.Completion;
        Assert.Equal(2, mergeService.ExecuteStartedCount);
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }

    private MergeRequest CreateRequest(int index) => new(
        [$"part-a-{index}.cast", $"part-b-{index}.cast"],
        _directory,
        $"result-{index}.cast");

    private static Model CreateBodyModel()
    {
        var model = new Model("body");
        model.Bones.Add(new Model.Bone("root"));
        model.Bones.Add(new Model.Bone("attach", 0, new Vector3(0, 0, 1), IdentityRotation()));
        model.Meshes.Add(CreateTriangleMesh(0));
        model.GenerateGlobalBoneData();
        return model;
    }

    private static Model CreateHeadModel()
    {
        var model = new Model("head");
        model.Bones.Add(new Model.Bone("attach"));
        model.Bones.Add(new Model.Bone("head", 0, new Vector3(0, 0, 1), IdentityRotation()));
        model.Meshes.Add(CreateTriangleMesh(1));
        model.GenerateGlobalBoneData();
        return model;
    }

    private static Model.Mesh CreateTriangleMesh(float z)
    {
        var mesh = new Model.Mesh(3, 1);
        mesh.Vertices.Add(new Model.Vertex(new Vector3(0, 0, z), new Vector3(0, 0, 1)));
        mesh.Vertices.Add(new Model.Vertex(new Vector3(1, 0, z), new Vector3(0, 0, 1)));
        mesh.Vertices.Add(new Model.Vertex(new Vector3(0, 1, z), new Vector3(0, 0, 1)));
        mesh.Faces.Add(new Model.Face(0, 1, 2));
        return mesh;
    }

    private static Quaternion IdentityRotation() => new(0, 0, 0, 1);

    private sealed class CallbackProgress<T>(Action<T> callback) : IProgress<T>
    {
        public void Report(T value) => callback(value);
    }

    private sealed class CountingMergeService : IModelMergeService
    {
        private readonly SemaphoreSlim _release = new(0);
        private readonly TaskCompletionSource _startedSignal = new(TaskCreationOptions.RunContinuationsAsynchronously);
        private int _activeCount;
        private int _maximumObservedConcurrency;
        private int _requiredStartedCount;
        private int _startedCount;

        public int MaximumObservedConcurrency => Volatile.Read(ref _maximumObservedConcurrency);

        public int StartedCount => Volatile.Read(ref _startedCount);

        public int ActiveCount => Volatile.Read(ref _activeCount);

        public Task<IPreparedMergeOperation> PrepareAsync(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var outputPath = Path.Combine(request.OutputDirectory, request.OutputFileName!);
            return Task.FromResult<IPreparedMergeOperation>(new Operation(this, request, outputPath));
        }

        public async Task<MergeResult> MergeAsync(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default)
        {
            var active = Interlocked.Increment(ref _activeCount);
            UpdateMaximum(active);
            if (Interlocked.Increment(ref _startedCount) >= Volatile.Read(ref _requiredStartedCount))
            {
                _startedSignal.TrySetResult();
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
            Volatile.Write(ref _requiredStartedCount, count);
            if (StartedCount >= count)
            {
                return;
            }

            await _startedSignal.Task.WaitAsync(TimeSpan.FromSeconds(10));
        }

        public void ReleaseAll() => _release.Release(3);

        private void UpdateMaximum(int value)
        {
            while (true)
            {
                var current = Volatile.Read(ref _maximumObservedConcurrency);
                if (value <= current || Interlocked.CompareExchange(ref _maximumObservedConcurrency, value, current) == current)
                {
                    return;
                }
            }
        }

        private sealed class Operation(
            CountingMergeService owner,
            MergeRequest request,
            string outputPath) : IPreparedMergeOperation
        {
            public string OutputPath { get; } = Path.GetFullPath(outputPath);

            public Task<MergeResult> ExecuteAsync(
                IProgress<MergeProgress>? progress = null,
                CancellationToken cancellationToken = default) =>
                owner.MergeAsync(request, progress, cancellationToken);
        }
    }

    private sealed class PreparedBlockingMergeService(string outputPath) : IModelMergeService
    {
        private readonly TaskCompletionSource _executeStarted =
            new(TaskCreationOptions.RunContinuationsAsynchronously);
        private readonly TaskCompletionSource _release =
            new(TaskCreationOptions.RunContinuationsAsynchronously);
        private readonly string _outputPath = Path.GetFullPath(outputPath);
        private int _executeStartedCount;

        public int ExecuteStartedCount => Volatile.Read(ref _executeStartedCount);

        public Task<IPreparedMergeOperation> PrepareAsync(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default) =>
            Task.FromResult<IPreparedMergeOperation>(new Operation(this, request));

        public Task<MergeResult> MergeAsync(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default) => throw new NotSupportedException();

        public Task WaitUntilExecuteStartedAsync() =>
            _executeStarted.Task.WaitAsync(TimeSpan.FromSeconds(10));

        public void Release() => _release.TrySetResult();

        private sealed class Operation(PreparedBlockingMergeService owner, MergeRequest request)
            : IPreparedMergeOperation
        {
            public string OutputPath => owner._outputPath;

            public async Task<MergeResult> ExecuteAsync(
                IProgress<MergeProgress>? progress = null,
                CancellationToken cancellationToken = default)
            {
                Interlocked.Increment(ref owner._executeStartedCount);
                owner._executeStarted.TrySetResult();
                await owner._release.Task.WaitAsync(cancellationToken);
                return new MergeResult(OutputPath, "root", request.InputFiles.Count, 0, 0, []);
            }
        }
    }
}

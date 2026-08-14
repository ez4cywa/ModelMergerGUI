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
    }
}

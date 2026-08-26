using System.Text.Json;
using ModelMerger.Core.Merging;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class RustWorkerMergeServiceTests
{
    [Fact]
    public async Task PrepareAndExecute_MapsNdjsonProtocolToExistingMergeContracts()
    {
        var worker = new FakeRustWorkerProcess(
        [
            """{"protocol":1,"event":"progress","stage":"validating","current":0,"total":1,"item":null}""",
            """{"protocol":1,"event":"prepared","output_path":"C:\\output\\combined.cast"}""",
            """{"protocol":1,"event":"progress","stage":"merging","current":0,"total":1,"item":"head"}""",
            """{"protocol":1,"event":"result","output_path":"C:\\output\\combined.cast","root_model_name":"body","part_count":2,"bone_count":3,"mesh_count":2,"warnings":[{"code":"no_attachment_bone","model_name":"prop","root_model_name":"body"}]}"""
        ]);
        var progress = new List<MergeProgress>();
        var service = new RustWorkerMergeService(() => worker);

        var prepared = await service.PrepareAsync(
            new MergeRequest(
                [@"C:\models\body.cast", @"C:\models\head.cast"],
                @"C:\output",
                "combined.cast"),
            new CallbackProgress<MergeProgress>(progress.Add));
        var result = await prepared.ExecuteAsync(
            new CallbackProgress<MergeProgress>(progress.Add));

        Assert.Equal(@"C:\output\combined.cast", prepared.OutputPath);
        Assert.Equal("body", result.RootModelName);
        Assert.Equal(3, result.BoneCount);
        Assert.Equal(2, result.MeshCount);
        var warning = Assert.Single(result.Warnings);
        Assert.Equal(MergeWarningCode.NoAttachmentBone, warning.Code);
        Assert.Contains(progress, item => item.Code == MergeProgressCode.ValidatingRequest);
        Assert.Contains(progress, item =>
            item.Code == MergeProgressCode.MergingModel && item.Subject == "head");

        Assert.Equal(2, worker.WrittenLines.Count);
        using var prepareDocument = JsonDocument.Parse(worker.WrittenLines[0]);
        Assert.Equal("prepare", prepareDocument.RootElement.GetProperty("command").GetString());
        Assert.Equal("automatic", prepareDocument.RootElement
            .GetProperty("request")
            .GetProperty("root_selection_mode")
            .GetString());
        using var executeDocument = JsonDocument.Parse(worker.WrittenLines[1]);
        Assert.Equal("execute", executeDocument.RootElement.GetProperty("command").GetString());
    }

    [Fact]
    public async Task ExecuteAsync_WhenCancelled_SendsCancelAndStopsWorker()
    {
        var worker = new FakeRustWorkerProcess(
        [
            """{"protocol":1,"event":"prepared","output_path":"C:\\output\\cancelled.cast"}"""
        ], blockWhenEmpty: true);
        var service = new RustWorkerMergeService(() => worker);
        var prepared = await service.PrepareAsync(new MergeRequest(
            [@"C:\models\body.cast", @"C:\models\head.cast"],
            @"C:\output"));
        using var cancellation = new CancellationTokenSource();

        var execution = prepared.ExecuteAsync(cancellationToken: cancellation.Token);
        await worker.ReadBlocked.Task.WaitAsync(TimeSpan.FromSeconds(2));
        cancellation.Cancel();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => execution);
        Assert.Contains(worker.WrittenLines, line =>
            JsonDocument.Parse(line).RootElement.GetProperty("command").GetString() == "cancel");
        Assert.True(worker.HasExited);
    }

    [Fact]
    public async Task PrepareAsync_MapsStructuredValidationCodeAndPath()
    {
        var worker = new FakeRustWorkerProcess(
        [
            """{"protocol":1,"event":"error","code":"validation","validation_code":"output_already_exists","message":"output already exists","path":"C:\\output\\existing.cast"}"""
        ]);
        var service = new RustWorkerMergeService(() => worker);

        var exception = await Assert.ThrowsAsync<MergeValidationException>(() => service.PrepareAsync(
            new MergeRequest(
                [@"C:\models\body.cast", @"C:\models\head.cast"],
                @"C:\output")));

        var error = Assert.Single(exception.Errors);
        Assert.Equal(MergeValidationErrorCode.OutputAlreadyExists, error.Code);
        Assert.Equal(@"C:\output\existing.cast", error.FilePath);
    }

    [Fact]
    public async Task PrepareAsync_MapsUnreadableCastToExistingReadException()
    {
        var worker = new FakeRustWorkerProcess(
        [
            """{"protocol":1,"event":"error","code":"model_read","message":"invalid Cast magic","path":"C:\\models\\broken.cast","format":"Cast"}"""
        ]);
        var service = new RustWorkerMergeService(() => worker);

        var exception = await Assert.ThrowsAsync<ModelPartReadException>(() => service.PrepareAsync(
            new MergeRequest(
                [@"C:\models\valid.cast", @"C:\models\broken.cast"],
                @"C:\output")));

        Assert.Equal(@"C:\models\broken.cast", exception.FilePath);
        Assert.Equal("Cast", exception.FormatName);
    }

    [Fact]
    public async Task MergeAsync_ConcurrentDirectCallsClaimOutputAndDisposeRejectedWorker()
    {
        const string prepared =
            """{"protocol":1,"event":"prepared","output_path":"C:\\output\\shared.cast"}""";
        var firstWorker = new FakeRustWorkerProcess([prepared], blockWhenEmpty: true);
        var secondWorker = new FakeRustWorkerProcess([prepared], blockWhenEmpty: true);
        var firstService = new RustWorkerMergeService(() => firstWorker);
        var secondService = new RustWorkerMergeService(() => secondWorker);
        var request = new MergeRequest(
            [@"C:\models\body.cast", @"C:\models\head.cast"],
            @"C:\output");
        using var firstCancellation = new CancellationTokenSource();

        var first = firstService.MergeAsync(request, cancellationToken: firstCancellation.Token);
        await firstWorker.ReadBlocked.Task.WaitAsync(TimeSpan.FromSeconds(2));

        var conflict = await Assert.ThrowsAsync<MergeOutputConflictException>(() =>
            secondService.MergeAsync(request));

        Assert.Equal(@"C:\output\shared.cast", conflict.OutputPath);
        Assert.True(secondWorker.HasExited);
        Assert.Contains(secondWorker.WrittenLines, line =>
            JsonDocument.Parse(line).RootElement.GetProperty("command").GetString() == "cancel");
        firstCancellation.Cancel();
        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => first);
    }

    private sealed class FakeRustWorkerProcess(
        IEnumerable<string> output,
        bool blockWhenEmpty = false) : IRustWorkerProcess
    {
        private readonly Queue<string> _output = new(output);

        public List<string> WrittenLines { get; } = [];

        public TaskCompletionSource ReadBlocked { get; } = new(
            TaskCreationOptions.RunContinuationsAsynchronously);

        public bool HasExited { get; private set; }

        public Task WriteLineAsync(string line, CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            WrittenLines.Add(line);
            using var document = JsonDocument.Parse(line);
            if (document.RootElement.GetProperty("command").GetString() == "cancel")
            {
                HasExited = true;
            }
            return Task.CompletedTask;
        }

        public async Task<string?> ReadLineAsync(CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (_output.Count == 0)
            {
                if (blockWhenEmpty)
                {
                    ReadBlocked.TrySetResult();
                    await Task.Delay(Timeout.InfiniteTimeSpan, cancellationToken);
                }

                HasExited = true;
                return null;
            }

            return _output.Dequeue();
        }

        public Task WaitForExitAsync(CancellationToken cancellationToken)
        {
            HasExited = true;
            return Task.CompletedTask;
        }

        public void Kill()
        {
            HasExited = true;
        }

        public ValueTask DisposeAsync()
        {
            HasExited = true;
            return ValueTask.CompletedTask;
        }
    }

    private sealed class CallbackProgress<T>(Action<T> callback) : IProgress<T>
    {
        public void Report(T value) => callback(value);
    }
}

using ModelMerger.Core.Merging;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class AdaptiveModelMergeServiceTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"AdaptiveModelMergeServiceTests-{Guid.NewGuid():N}");

    public AdaptiveModelMergeServiceTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Theory]
    [InlineData(1023, "csharp")]
    [InlineData(1024, "rust")]
    public async Task PrepareAsync_RoutesByTotalInputBytes(long totalBytes, string expectedEngine)
    {
        var first = Path.Combine(_directory, "first.cast");
        var second = Path.Combine(_directory, "second.cast");
        await File.WriteAllBytesAsync(first, new byte[checked((int)(totalBytes / 2))]);
        await File.WriteAllBytesAsync(second, new byte[checked((int)(totalBytes - totalBytes / 2))]);
        var csharp = new RecordingMergeService("csharp");
        var rust = new RecordingMergeService("rust");
        var service = new AdaptiveModelMergeService(csharp, rust, rustThresholdBytes: 1024);

        var prepared = await service.PrepareAsync(new MergeRequest([first, second], _directory));

        Assert.Equal(expectedEngine, prepared.OutputPath);
        Assert.Equal(expectedEngine == "csharp" ? 1 : 0, csharp.PrepareCount);
        Assert.Equal(expectedEngine == "rust" ? 1 : 0, rust.PrepareCount);
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }

    private sealed class RecordingMergeService(string name) : IModelMergeService
    {
        public int PrepareCount { get; private set; }

        public Task<IPreparedMergeOperation> PrepareAsync(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default)
        {
            PrepareCount++;
            return Task.FromResult<IPreparedMergeOperation>(new Prepared(name));
        }

        public async Task<MergeResult> MergeAsync(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default)
        {
            var prepared = await PrepareAsync(request, progress, cancellationToken);
            return await prepared.ExecuteAsync(progress, cancellationToken);
        }

        private sealed class Prepared(string name) : IPreparedMergeOperation
        {
            public string OutputPath => name;

            public Task<MergeResult> ExecuteAsync(
                IProgress<MergeProgress>? progress = null,
                CancellationToken cancellationToken = default) =>
                Task.FromResult(new MergeResult(name, name, 2, 0, 0, []));
        }
    }
}

using ModelMerger.Core.Preview;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class AdaptiveModelPreviewServiceTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"AdaptivePreviewTests-{Guid.NewGuid():N}");

    public AdaptiveModelPreviewServiceTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public void DefaultThreshold_KeepsLargeFilesOnRustPath()
    {
        Assert.Equal(2 * 1024 * 1024, AdaptiveModelPreviewService.DefaultRustThresholdBytes);
    }

    [Fact]
    public async Task LoadAsync_RoutesOnlyLargeExistingCastFilesToRust()
    {
        var smallPath = CreateFile("small.cast", 9);
        var largePath = CreateFile("large.cast", 10);
        var csharp = new RecordingPreviewService("csharp");
        var rust = new RecordingPreviewService("rust");
        var service = new AdaptiveModelPreviewService(csharp, rust, rustThresholdBytes: 10);

        var small = await service.LoadAsync(smallPath);
        var large = await service.LoadAsync(largePath);

        Assert.Equal("csharp", small.ModelName);
        Assert.Equal("rust", large.ModelName);
        Assert.Equal([smallPath], csharp.Paths);
        Assert.Equal([largePath], rust.Paths);
    }

    [Fact]
    public async Task LoadAsync_RoutesSmallCastWith32BitFaceIndicesToRust()
    {
        var path = Path.Combine(_directory, "uint32.cast");
        WriteCastWith32BitFaceIndices(path);
        Assert.True(new FileInfo(path).Length < AdaptiveModelPreviewService.DefaultRustThresholdBytes);
        var csharp = new RecordingPreviewService("csharp");
        var rust = new RecordingPreviewService("rust");
        var service = new AdaptiveModelPreviewService(
            csharp,
            rust,
            AdaptiveModelPreviewService.DefaultRustThresholdBytes);

        var preview = await service.LoadAsync(path);

        Assert.Equal("rust", preview.ModelName);
        Assert.Empty(csharp.Paths);
        Assert.Equal([path], rust.Paths);
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }

    private string CreateFile(string name, int length)
    {
        var path = Path.Combine(_directory, name);
        File.WriteAllBytes(path, new byte[length]);
        return path;
    }

    private static void WriteCastWith32BitFaceIndices(string path)
    {
        using var stream = File.Create(path);
        using var writer = new BinaryWriter(stream);
        writer.Write("cast"u8);
        writer.Write(1u);
        writer.Write(1u);
        writer.Write(0u);

        const uint meshLength = 24 + 8 + 1 + 12;
        const uint modelLength = 24 + meshLength;
        const uint rootLength = 24 + modelLength;
        WriteNodeHeader("root"u8, rootLength, propertyCount: 0, childCount: 1);
        WriteNodeHeader("modl"u8, modelLength, propertyCount: 0, childCount: 1);
        WriteNodeHeader("mesh"u8, meshLength, propertyCount: 1, childCount: 0);
        writer.Write(new byte[] { (byte)'i', 0 });
        writer.Write((ushort)1);
        writer.Write(3u);
        writer.Write((byte)'f');
        writer.Write(0u);
        writer.Write(65_535u);
        writer.Write(65_536u);
        return;

        void WriteNodeHeader(
            ReadOnlySpan<byte> identifier,
            uint nodeLength,
            uint propertyCount,
            uint childCount)
        {
            writer.Write(identifier);
            writer.Write(nodeLength);
            writer.Write(0ul);
            writer.Write(propertyCount);
            writer.Write(childCount);
        }
    }

    private sealed class RecordingPreviewService(string name) : IModelPreviewService
    {
        public List<string> Paths { get; } = [];

        public Task<ModelPreviewData> LoadAsync(
            string filePath,
            int triangleLimit = ModelPreviewService.DefaultTriangleLimit,
            CancellationToken cancellationToken = default)
        {
            Paths.Add(filePath);
            return Task.FromResult(new ModelPreviewData(
                filePath,
                name,
                1,
                3,
                1,
                1,
                false,
                new PreviewBounds(new PreviewPoint3(), new PreviewPoint3(1, 1, 0)),
                [new PreviewMeshData([], [], [])]));
        }
    }
}

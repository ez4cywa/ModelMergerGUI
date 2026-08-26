using ModelMerger.Core.Merging;
using ModelMerger.Core.Preview;
using System.Text.Json;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class RustWorkerModelPreviewServiceTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"RustWorkerPreviewTests-{Guid.NewGuid():N}");

    public RustWorkerModelPreviewServiceTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public async Task LoadAsync_ReadsCompactWorkerPayloadThroughPreviewInterface()
    {
        var payloadPath = Path.Combine(_directory, "preview.bin");
        WritePayload(payloadPath);
        var sourcePath = Path.Combine(_directory, "source.cast");
        await File.WriteAllBytesAsync(sourcePath, []);
        var result = JsonSerializer.Serialize(new
        {
            protocol = 1,
            @event = "preview_result",
            file_path = sourcePath,
            model_name = "source",
            source_mesh_count = 1,
            source_vertex_count = 3,
            source_triangle_count = 1,
            displayed_triangle_count = 1,
            is_simplified = false,
            bounds = new
            {
                minimum = new[] { 0f, 0f, 0f },
                maximum = new[] { 1f, 1f, 0f }
            },
            payload_path = payloadPath,
            payload_length = new FileInfo(payloadPath).Length
        });
        var process = new FakeRustWorkerProcess([result]);
        var service = new RustWorkerModelPreviewService(() => process);

        var preview = await service.LoadAsync(sourcePath, triangleLimit: 42);

        Assert.Equal("source", preview.ModelName);
        Assert.Equal(1, preview.SourceMeshCount);
        Assert.Equal(3, preview.SourceVertexCount);
        Assert.Equal(1, preview.SourceTriangleCount);
        Assert.Equal(1, preview.DisplayedTriangleCount);
        Assert.False(preview.IsSimplified);
        var mesh = Assert.Single(preview.Meshes);
        Assert.Equal(
            [new PreviewPoint3(0, 0, 0), new PreviewPoint3(1, 0, 0), new PreviewPoint3(0, 1, 0)],
            mesh.Positions);
        Assert.Equal([0, 1, 2], mesh.TriangleIndices);
        Assert.Contains(process.WrittenLines, line =>
        {
            using var command = JsonDocument.Parse(line);
            return command.RootElement.GetProperty("command").GetString() == "preview" &&
                   command.RootElement.GetProperty("request").GetProperty("triangle_limit").GetInt32() == 42;
        });
        Assert.Contains(process.WrittenLines, line =>
            JsonDocument.Parse(line).RootElement.GetProperty("command").GetString() == "release_preview");
        Assert.True(process.IsDisposed);
    }

    [Fact]
    public void PayloadCodec_CancelsWithinALargeSingleMesh()
    {
        var payloadPath = Path.Combine(_directory, "large-preview.bin");
        var points = Enumerable.Range(0, 10_000)
            .Select(index => new PreviewPoint3(index, 0, 0))
            .ToArray();
        using (var writer = new BinaryWriter(File.Create(payloadPath)))
        {
            RustPreviewPayloadCodec.Write(
                writer,
                [new PreviewMeshData(points, points, [0, 1, 2])]);
        }
        var descriptor = CreateDescriptor(payloadPath, displayedTriangleCount: 1);
        using var cancellation = new CancellationTokenSource();

        Assert.Throws<OperationCanceledException>(() => RustPreviewPayloadCodec.Read(
            descriptor,
            cancellation.Token,
            valuesRead =>
            {
                if (valuesRead >= 4_096)
                {
                    cancellation.Cancel();
                }
            }));
    }

    [Fact]
    public async Task LoadAsync_WhenCancelledAfterWorkerResult_SendsCancelAndDisposesWorker()
    {
        var payloadPath = Path.Combine(_directory, "cancel-preview.bin");
        WritePayload(payloadPath);
        var sourcePath = Path.Combine(_directory, "cancel.cast");
        await File.WriteAllBytesAsync(sourcePath, []);
        using var cancellation = new CancellationTokenSource();
        var process = new FakeRustWorkerProcess(
            [CreateResultJson(sourcePath, payloadPath)],
            () => cancellation.Cancel());
        var service = new RustWorkerModelPreviewService(() => process);

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() =>
            service.LoadAsync(sourcePath, cancellationToken: cancellation.Token));

        Assert.Contains(process.WrittenLines, line =>
            JsonDocument.Parse(line).RootElement.GetProperty("command").GetString() == "cancel");
        Assert.True(process.IsDisposed);
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }

    private static void WritePayload(string path)
    {
        using var writer = new BinaryWriter(File.Create(path));
        RustPreviewPayloadCodec.Write(
            writer,
            [new PreviewMeshData(
                [new(0, 0, 0), new(1, 0, 0), new(0, 1, 0)],
                [new(0, 0, 1), new(0, 0, 1), new(0, 0, 1)],
                [0, 1, 2])]);
    }

    private static RustPreviewDescriptor CreateDescriptor(
        string payloadPath,
        int displayedTriangleCount) =>
        new(
            "source.cast",
            "source",
            1,
            3,
            displayedTriangleCount,
            displayedTriangleCount,
            false,
            new PreviewPoint3(),
            new PreviewPoint3(1, 1, 0),
            payloadPath,
            new FileInfo(payloadPath).Length);

    private static string CreateResultJson(string sourcePath, string payloadPath) =>
        JsonSerializer.Serialize(new
        {
            protocol = 1,
            @event = "preview_result",
            file_path = sourcePath,
            model_name = "source",
            source_mesh_count = 1,
            source_vertex_count = 3,
            source_triangle_count = 1,
            displayed_triangle_count = 1,
            is_simplified = false,
            bounds = new
            {
                minimum = new[] { 0f, 0f, 0f },
                maximum = new[] { 1f, 1f, 0f }
            },
            payload_path = payloadPath,
            payload_length = new FileInfo(payloadPath).Length
        });

    private sealed class FakeRustWorkerProcess(
        IEnumerable<string> output,
        Action? afterRead = null) : IRustWorkerProcess
    {
        private readonly Queue<string> _output = new(output);

        public bool HasExited { get; private set; }

        public bool IsDisposed { get; private set; }

        public List<string> WrittenLines { get; } = [];

        public Task WriteLineAsync(string line, CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            WrittenLines.Add(line);
            using var command = JsonDocument.Parse(line);
            if (command.RootElement.GetProperty("command").GetString() == "release_preview")
            {
                HasExited = true;
            }
            return Task.CompletedTask;
        }

        public Task<string?> ReadLineAsync(CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            var line = _output.Count == 0 ? null : _output.Dequeue();
            afterRead?.Invoke();
            return Task.FromResult(line);
        }

        public Task WaitForExitAsync(CancellationToken cancellationToken)
        {
            cancellationToken.ThrowIfCancellationRequested();
            HasExited = true;
            return Task.CompletedTask;
        }

        public void Kill() => HasExited = true;

        public ValueTask DisposeAsync()
        {
            IsDisposed = true;
            return ValueTask.CompletedTask;
        }
    }
}

using ModelMerger.Core.Preview;
using ModelMerger.Core.Merging;
using PhilLibX;
using PhilLibX.Mathematics;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class ModelPreviewServiceTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"ModelMergerPreviewTests-{Guid.NewGuid():N}");

    public ModelPreviewServiceTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public async Task LoadAsync_WithSyntheticCast_ReturnsRenderableGeometryAndSourceStatistics()
    {
        var path = Path.Combine(_directory, "preview.cast");
        var model = new Model("preview");
        model.Meshes.Add(CreateTriangleMesh(0));
        model.Meshes.Add(CreateTriangleMesh(1));
        model.Save(path);
        var service = new ModelPreviewService();

        var preview = await service.LoadAsync(path);

        Assert.Equal("preview", preview.ModelName);
        Assert.Equal(2, preview.SourceMeshCount);
        Assert.Equal(6, preview.SourceVertexCount);
        Assert.Equal(2, preview.SourceTriangleCount);
        Assert.Equal(2, preview.DisplayedTriangleCount);
        Assert.False(preview.IsSimplified);
        Assert.Equal(2, preview.Meshes.Count);
        Assert.All(preview.Meshes, mesh => Assert.Equal(3, mesh.TriangleIndices.Count));
    }

    [Fact]
    public async Task LoadAsync_WhenTriangleLimitIsLowerThanSource_SamplesAcrossTheModel()
    {
        var path = Path.Combine(_directory, "large.cast");
        var model = new Model("large");
        model.Meshes.Add(CreateIndependentTriangles(20));
        model.Save(path);
        var service = new ModelPreviewService();

        var preview = await service.LoadAsync(path, triangleLimit: 5);

        Assert.Equal(20, preview.SourceTriangleCount);
        Assert.Equal(5, preview.DisplayedTriangleCount);
        Assert.True(preview.IsSimplified);
        Assert.Equal(15, preview.Meshes.Single().TriangleIndices.Count);
    }

    [Fact]
    public void SamplingMath_WithLargeModels_DoesNotOverflow()
    {
        var quotas = ModelPreviewService.AllocateTriangleQuotas([90_000, 90_000], 75_000);
        var ordinals = ModelPreviewService.SelectTriangleOrdinals(90_000, 75_000);

        Assert.Equal([37_500, 37_500], quotas);
        Assert.Equal(75_000, ordinals.Count);
        Assert.Equal(0, ordinals.Min());
        Assert.Equal(89_998, ordinals.Max());
    }

    [Fact]
    public async Task LoadAsync_WhenAnInvalidTrianglePrecedesAValidTriangle_FillsThePreviewQuota()
    {
        var path = Path.Combine(_directory, "partially-valid.cast");
        var model = new Model("partially-valid");
        var mesh = new Model.Mesh(6, 2);
        mesh.Vertices.Add(new Model.Vertex(new Vector3(float.NaN, 0, 0), new Vector3(0, 0, 1)));
        mesh.Vertices.Add(new Model.Vertex(new Vector3(1, 0, 0), new Vector3(0, 0, 1)));
        mesh.Vertices.Add(new Model.Vertex(new Vector3(0, 1, 0), new Vector3(0, 0, 1)));
        mesh.Vertices.Add(new Model.Vertex(new Vector3(0, 0, 1), new Vector3(0, 0, 1)));
        mesh.Vertices.Add(new Model.Vertex(new Vector3(1, 0, 1), new Vector3(0, 0, 1)));
        mesh.Vertices.Add(new Model.Vertex(new Vector3(0, 1, 1), new Vector3(0, 0, 1)));
        mesh.Faces.Add(new Model.Face(0, 1, 2));
        mesh.Faces.Add(new Model.Face(3, 4, 5));
        model.Meshes.Add(mesh);
        model.Save(path);

        var preview = await new ModelPreviewService().LoadAsync(path, triangleLimit: 1);

        Assert.Equal(2, preview.SourceTriangleCount);
        Assert.Equal(1, preview.DisplayedTriangleCount);
        Assert.True(preview.IsSimplified);
        Assert.Equal(3, preview.Meshes.Single().TriangleIndices.Count);
    }

    [Fact]
    public async Task LoadAsync_WithUnreadableCast_ReturnsStructuredPreviewError()
    {
        var path = Path.Combine(_directory, "broken.cast");
        await File.WriteAllTextAsync(path, "not a Cast model");
        var service = new ModelPreviewService();

        var exception = await Assert.ThrowsAsync<ModelPreviewException>(() => service.LoadAsync(path));

        Assert.Equal(ModelPreviewErrorCode.UnreadableModel, exception.Code);
        Assert.Equal(path, exception.FilePath);
    }

    [Fact]
    public async Task LoadAsync_WithMergedOutput_ReturnsEveryMergedMesh()
    {
        var firstPath = Path.Combine(_directory, "merge-first.cast");
        var secondPath = Path.Combine(_directory, "merge-second.cast");
        var first = new Model("merge-first");
        first.Meshes.Add(CreateTriangleMesh(0));
        first.Save(firstPath);
        var second = new Model("merge-second");
        second.Meshes.Add(CreateTriangleMesh(1));
        second.Save(secondPath);
        var result = await new ModelMergeService().MergeAsync(
            new MergeRequest([firstPath, secondPath], _directory, "merged-preview.cast"));

        var preview = await new ModelPreviewService().LoadAsync(result.OutputPath);

        Assert.Equal(2, preview.SourceMeshCount);
        Assert.Equal(2, preview.DisplayedTriangleCount);
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
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

    private static Model.Mesh CreateIndependentTriangles(int count)
    {
        var mesh = new Model.Mesh(count * 3, count);
        for (var index = 0; index < count; index++)
        {
            var x = index * 2f;
            mesh.Vertices.Add(new Model.Vertex(new Vector3(x, 0, 0), new Vector3(0, 0, 1)));
            mesh.Vertices.Add(new Model.Vertex(new Vector3(x + 1, 0, 0), new Vector3(0, 0, 1)));
            mesh.Vertices.Add(new Model.Vertex(new Vector3(x, 1, 0), new Vector3(0, 0, 1)));
            mesh.Faces.Add(new Model.Face(index * 3, index * 3 + 1, index * 3 + 2));
        }

        return mesh;
    }
}

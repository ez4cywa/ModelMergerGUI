using ModelMerger.Core.Merging;
using PhilLibX;
using PhilLibX.Mathematics;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class ModelMergeServiceIntegrationTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"ModelMergerIntegrationTests-{Guid.NewGuid():N}");

    public ModelMergeServiceIntegrationTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public async Task MergeAsync_WithConnectedSyntheticCastParts_WritesReadableMergedModel()
    {
        var bodyPath = Path.Combine(_directory, "body.cast");
        var headPath = Path.Combine(_directory, "head.cast");
        CreateBodyModel().Save(bodyPath);
        CreateHeadModel().Save(headPath);
        var service = new ModelMergeService();

        var result = await service.MergeAsync(
            new MergeRequest([bodyPath, headPath], _directory, "combined.cast"));

        Assert.Equal("body", result.RootModelName);
        Assert.Equal(2, result.PartCount);
        Assert.Equal(3, result.BoneCount);
        Assert.Equal(2, result.MeshCount);
        Assert.True(File.Exists(result.OutputPath));

        var castFile = Cast.CastFile.Load(result.OutputPath);
        var castModel = castFile.RootNodes[0].ChildrenOfType<Cast.Model>().Single();
        Assert.Equal(3, castModel.Skeleton()!.Bones().Count);
        Assert.Equal(2, castModel.Meshes().Count);
    }

    [Fact]
    public async Task MergeAsync_WhenCancelled_RemovesTemporaryOutput()
    {
        var bodyPath = Path.Combine(_directory, "cancel-body.cast");
        var headPath = Path.Combine(_directory, "cancel-head.cast");
        CreateBodyModel().Save(bodyPath);
        CreateHeadModel().Save(headPath);
        using var cancellation = new CancellationTokenSource();
        var progress = new CallbackProgress<MergeProgress>(value =>
        {
            if (value.Stage == MergeStage.Merging)
            {
                cancellation.Cancel();
            }
        });
        var service = new ModelMergeService();

        await Assert.ThrowsAnyAsync<OperationCanceledException>(() => service.MergeAsync(
            new MergeRequest([bodyPath, headPath], _directory, "cancelled.cast"),
            progress,
            cancellation.Token));

        Assert.False(File.Exists(Path.Combine(_directory, "cancelled.cast")));
        Assert.Empty(Directory.GetFiles(_directory, "*.tmp.cast"));
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }

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
}

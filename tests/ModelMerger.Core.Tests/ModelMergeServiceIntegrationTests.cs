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
    public async Task MergeAsync_ReportsStructuredProgressWithSubjects()
    {
        var bodyPath = Path.Combine(_directory, "progress-body.cast");
        var headPath = Path.Combine(_directory, "progress-head.cast");
        CreateBodyModel().Save(bodyPath);
        CreateHeadModel().Save(headPath);
        var reported = new List<MergeProgress>();
        var service = new ModelMergeService();

        await service.MergeAsync(
            new MergeRequest([bodyPath, headPath], _directory, "progress.cast"),
            new CallbackProgress<MergeProgress>(reported.Add));

        Assert.Contains(reported, item =>
            item.Code == MergeProgressCode.LoadingFile && item.Subject == "progress-body.cast");
        Assert.Contains(reported, item =>
            item.Code == MergeProgressCode.SavedFile && item.Subject == "progress.cast");
    }

    [Fact]
    public async Task MergeAsync_ReportsStructuredWarningsWithModelNames()
    {
        var bodyPath = Path.Combine(_directory, "warning-body.cast");
        var propPath = Path.Combine(_directory, "warning-prop.cast");
        CreateBodyModel().Save(bodyPath);
        var prop = new Model("prop");
        prop.Bones.Add(new Model.Bone("unrelated"));
        prop.Save(propPath);
        var service = new ModelMergeService();

        var result = await service.MergeAsync(
            new MergeRequest([bodyPath, propPath], _directory, "warning.cast"));

        var warning = Assert.Single(result.Warnings);
        Assert.Equal(MergeWarningCode.NoAttachmentBone, warning.Code);
        Assert.Equal("warning-prop", warning.ModelName);
        Assert.Equal("warning-body", warning.RootModelName);
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
            if (value.Stage == MergeStage.Saving)
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

    [Fact]
    public async Task MergeAsync_WithCorruptCast_IdentifiesTheUnreadableFile()
    {
        var validPath = Path.Combine(_directory, "valid.cast");
        var corruptPath = Path.Combine(_directory, "broken.cast");
        CreateBodyModel().Save(validPath);
        await File.WriteAllTextAsync(corruptPath, "not a Cast model");
        var service = new ModelMergeService();

        var exception = await Assert.ThrowsAsync<ModelPartReadException>(() => service.MergeAsync(
            new MergeRequest([validPath, corruptPath], _directory, "corrupt-result.cast")));

        Assert.Equal(corruptPath, exception.FilePath);
        Assert.Equal("Cast", exception.FormatName);
        Assert.Contains("broken.cast", exception.Message, StringComparison.OrdinalIgnoreCase);
        Assert.Contains("valid or readable Cast", exception.Message, StringComparison.OrdinalIgnoreCase);
        Assert.False(File.Exists(Path.Combine(_directory, "corrupt-result.cast")));
    }

    [Fact]
    public async Task MergeAsync_WithExistingDefaultOutput_StopsBeforeMergingMeshes()
    {
        var bodyPath = Path.Combine(_directory, "body.cast");
        var headPath = Path.Combine(_directory, "head.cast");
        var existingOutput = Path.Combine(_directory, "body.cast");
        CreateBodyModel().Save(bodyPath);
        CreateHeadModel().Save(headPath);
        var reportedStages = new List<MergeStage>();
        var progress = new CallbackProgress<MergeProgress>(value => reportedStages.Add(value.Stage));
        var service = new ModelMergeService();

        var exception = await Assert.ThrowsAsync<MergeValidationException>(() => service.MergeAsync(
            new MergeRequest([bodyPath, headPath], _directory),
            progress));

        Assert.Contains(exception.Errors, error => error.Code == MergeValidationErrorCode.OutputAlreadyExists);
        Assert.DoesNotContain(MergeStage.Merging, reportedStages);
        Assert.True(File.Exists(existingOutput));
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

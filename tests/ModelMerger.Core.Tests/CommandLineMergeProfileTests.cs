using ModelMerger.Core.Merging;
using PhilLibX;
using SELib;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class CommandLineMergeProfileTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"ModelMergerCliProfileTests-{Guid.NewGuid():N}");

    public CommandLineMergeProfileTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public async Task MergeAsync_CommandLineProfile_AcceptsSingleSeModelAndWritesCast()
    {
        var inputPath = Path.Combine(_directory, "single.semodel");
        var input = new SEModel();
        input.Bones.Add(new SEModelBone { BoneName = "root" });
        input.Write(inputPath);
        var mergeService = ModelMergeService.CreateForCommandLine();

        var result = await mergeService.MergeAsync(new MergeRequest(
            [inputPath],
            _directory,
            "single-output.cast",
            Overwrite: true));

        Assert.Equal(1, result.PartCount);
        Assert.Equal(1, result.BoneCount);
        Assert.True(File.Exists(result.OutputPath));
        var castFile = Cast.CastFile.Load(result.OutputPath);
        var castModel = castFile.RootNodes[0].ChildrenOfType<Cast.Model>().Single();
        Assert.Single(castModel.Skeleton()!.Bones());
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }
}

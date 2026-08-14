using ModelMerger.Core.Merging;
using ModelMerger.Core.Planning;
using ModelMerger.Core.Selection;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class MergeGroupPlanTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"ModelMergerPlanTests-{Guid.NewGuid():N}");

    public MergeGroupPlanTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public void AddParts_CreatesReadyRequestWithAutomaticOutput()
    {
        var first = CreateCastFile("source", "body.cast");
        var second = CreateCastFile("source", "head.cast");
        var plan = new MergeGroupPlan();

        var firstResult = plan.AddPart(first);
        var secondResult = plan.AddPart(second);
        var request = plan.CreateRequest();

        Assert.Equal(AddPartStatus.Added, firstResult.Status);
        Assert.Equal(AddPartStatus.Added, secondResult.Status);
        Assert.Equal(2, plan.State.PartCount);
        Assert.True(plan.State.IsReady);
        Assert.False(plan.State.HasExplicitOutputDirectory);
        Assert.Equal(Path.Combine(Path.GetDirectoryName(first)!, "Merged Models"), plan.State.OutputDirectory);
        Assert.Equal([first, second], request.InputFiles);
        Assert.Equal(plan.State.OutputDirectory, request.OutputDirectory);
    }

    [Fact]
    public void ExplicitOutputDirectory_SurvivesReplacementAndClear()
    {
        var first = CreateCastFile("source-a", "body.cast");
        var replacement = CreateCastFile("source-b", "replacement.cast");
        var chosenOutput = Path.Combine(_directory, "chosen-output");
        Directory.CreateDirectory(chosenOutput);
        var plan = new MergeGroupPlan();
        plan.AddPart(first);

        plan.ChooseOutputDirectory(chosenOutput);
        plan.ReplacePart(0, replacement);
        plan.ClearParts();

        Assert.True(plan.State.HasExplicitOutputDirectory);
        Assert.Equal(chosenOutput, plan.State.OutputDirectory);
        Assert.Empty(plan.State.PartFiles);
    }

    [Fact]
    public void RemovingFirstPart_RecalculatesAutomaticOutputFromNewFirstPart()
    {
        var first = CreateCastFile("source-a", "body.cast");
        var second = CreateCastFile("source-b", "head.cast");
        var plan = new MergeGroupPlan();
        plan.AddPart(first);
        plan.AddPart(second);

        plan.RemovePart(0);

        Assert.Equal(Path.Combine(Path.GetDirectoryName(second)!, "Merged Models"), plan.State.OutputDirectory);
        Assert.False(plan.State.HasExplicitOutputDirectory);
    }

    [Fact]
    public void ReplacingManualRoot_KeepsTheReplacementAsManualRoot()
    {
        var first = CreateCastFile("source", "body.cast");
        var second = CreateCastFile("source", "head.cast");
        var replacement = CreateCastFile("source", "replacement.cast");
        var plan = new MergeGroupPlan();
        plan.AddPart(first);
        plan.AddPart(second);
        plan.SetManualRoot(1);

        var result = plan.ReplacePart(1, replacement);

        Assert.Equal(AddPartStatus.Added, result.Status);
        Assert.Equal(RootSelectionMode.Manual, plan.State.RootSelectionMode);
        Assert.Equal(replacement, plan.State.ManualRootFile);
        Assert.Equal(replacement, plan.CreateRequest().ManualRootFile);
    }

    [Fact]
    public void SelectedPartDeletedAfterSelection_MakesPlanNotReady()
    {
        var first = CreateCastFile("source", "body.cast");
        var second = CreateCastFile("source", "head.cast");
        var plan = new MergeGroupPlan();
        plan.AddPart(first);
        plan.AddPart(second);

        File.Delete(second);

        Assert.False(plan.State.IsReady);
        Assert.Throws<InvalidOperationException>(() => plan.CreateRequest());
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }

    private string CreateCastFile(string folder, string name)
    {
        var directory = Path.Combine(_directory, folder);
        Directory.CreateDirectory(directory);
        var path = Path.Combine(directory, name);
        File.WriteAllText(path, string.Empty);
        return path;
    }
}

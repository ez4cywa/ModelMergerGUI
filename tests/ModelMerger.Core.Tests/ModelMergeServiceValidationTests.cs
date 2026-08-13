using ModelMerger.Core.Merging;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class ModelMergeServiceValidationTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"ModelMergerValidationTests-{Guid.NewGuid():N}");

    public ModelMergeServiceValidationTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public async Task MergeAsync_WithOnlyOnePart_RejectsTheRequestBeforeLoading()
    {
        var input = Path.Combine(_directory, "only.cast");
        File.WriteAllText(input, string.Empty);
        var request = new MergeRequest([input], _directory, "merged.cast");
        var service = new ModelMergeService();

        var exception = await Assert.ThrowsAsync<MergeValidationException>(
            () => service.MergeAsync(request));

        Assert.Contains(exception.Errors, error => error.Code == MergeValidationErrorCode.InvalidPartCount);
        Assert.False(File.Exists(Path.Combine(_directory, "merged.cast")));
    }

    [Theory]
    [InlineData(0)]
    [InlineData(17)]
    public async Task MergeAsync_WithPartCountOutsideTwoToSixteen_RejectsTheRequest(int count)
    {
        var inputs = Enumerable.Range(1, count)
            .Select(index =>
            {
                var path = Path.Combine(_directory, $"part-{count}-{index}.cast");
                File.WriteAllText(path, string.Empty);
                return path;
            })
            .ToArray();
        var service = new ModelMergeService();

        var exception = await Assert.ThrowsAsync<MergeValidationException>(() => service.MergeAsync(
            new MergeRequest(inputs, _directory, $"merged-{count}.cast")));

        Assert.Contains(exception.Errors, error => error.Code == MergeValidationErrorCode.InvalidPartCount);
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }
}

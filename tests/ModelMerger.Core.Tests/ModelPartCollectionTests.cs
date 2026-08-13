using ModelMerger.Core.Selection;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class ModelPartCollectionTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"ModelMergerTests-{Guid.NewGuid():N}");

    public ModelPartCollectionTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public void Add_ValidCastFilesOneAtATime_StopsAtSixteen()
    {
        var parts = new ModelPartCollection();

        for (var index = 1; index <= ModelPartCollection.MaximumParts; index++)
        {
            var path = CreateFile($"part-{index:00}.cast");
            var result = parts.TryAdd(path);

            Assert.Equal(AddPartStatus.Added, result.Status);
            Assert.Equal(index, parts.Count);
        }

        var rejected = parts.TryAdd(CreateFile("part-17.cast"));

        Assert.Equal(AddPartStatus.CollectionFull, rejected.Status);
        Assert.Equal(ModelPartCollection.MaximumParts, parts.Count);
    }

    [Fact]
    public void Replace_WithValidCast_ChangesOnlyChosenSlot()
    {
        var parts = new ModelPartCollection();
        var first = CreateFile("first.cast");
        var second = CreateFile("second.cast");
        var replacement = CreateFile("replacement.cast");
        parts.TryAdd(first);
        parts.TryAdd(second);

        var result = parts.TryReplace(0, replacement);

        Assert.Equal(AddPartStatus.Added, result.Status);
        Assert.Equal(replacement, parts.Paths[0]);
        Assert.Equal(second, parts.Paths[1]);
    }

    [Fact]
    public void Add_DuplicateCastFile_DoesNotConsumeAnotherSlot()
    {
        var parts = new ModelPartCollection();
        var file = CreateFile("same.cast");
        parts.TryAdd(file);

        var result = parts.TryAdd(file.ToUpperInvariant());

        Assert.Equal(AddPartStatus.Duplicate, result.Status);
        Assert.Equal(1, parts.Count);
    }

    [Fact]
    public void Add_NonCastFile_IsRejected()
    {
        var parts = new ModelPartCollection();
        var file = CreateFile("part.txt");

        var result = parts.TryAdd(file);

        Assert.Equal(AddPartStatus.NotCastFile, result.Status);
        Assert.Empty(parts.Paths);
    }

    [Fact]
    public void Add_MissingCastFile_IsRejected()
    {
        var parts = new ModelPartCollection();
        var missingFile = Path.Combine(_directory, "missing.cast");

        var result = parts.TryAdd(missingFile);

        Assert.Equal(AddPartStatus.FileNotFound, result.Status);
        Assert.Empty(parts.Paths);
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }

    private string CreateFile(string name)
    {
        var path = Path.Combine(_directory, name);
        File.WriteAllText(path, string.Empty);
        return path;
    }
}

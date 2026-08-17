namespace ModelMerger.Core.Selection;

public enum AddPartStatus
{
    Added,
    InvalidPath,
    FileNotFound,
    NotCastFile,
    Duplicate,
    CollectionFull
}

public sealed record AddPartResult(AddPartStatus Status, string? FilePath = null);

public sealed class ModelPartCollection
{
    public const int MinimumParts = 2;
    public const int MaximumParts = 15;

    private readonly List<string> _paths = [];

    public int Count => _paths.Count;

    public IReadOnlyList<string> Paths => _paths;

    public bool CanMerge => Count >= MinimumParts;

    public AddPartResult TryAdd(string? filePath)
    {
        if (_paths.Count >= MaximumParts)
        {
            return new AddPartResult(AddPartStatus.CollectionFull);
        }

        if (string.IsNullOrWhiteSpace(filePath))
        {
            return new AddPartResult(AddPartStatus.InvalidPath);
        }

        string fullPath;
        try
        {
            fullPath = Path.GetFullPath(filePath);
        }
        catch (Exception exception) when (exception is ArgumentException or NotSupportedException or PathTooLongException)
        {
            return new AddPartResult(AddPartStatus.InvalidPath);
        }

        if (!string.Equals(Path.GetExtension(fullPath), ".cast", StringComparison.OrdinalIgnoreCase))
        {
            return new AddPartResult(AddPartStatus.NotCastFile, fullPath);
        }

        if (!File.Exists(fullPath))
        {
            return new AddPartResult(AddPartStatus.FileNotFound, fullPath);
        }

        if (_paths.Contains(fullPath, StringComparer.OrdinalIgnoreCase))
        {
            return new AddPartResult(AddPartStatus.Duplicate, fullPath);
        }

        _paths.Add(fullPath);
        return new AddPartResult(AddPartStatus.Added, fullPath);
    }

    public bool RemoveAt(int index)
    {
        if (index < 0 || index >= _paths.Count)
        {
            return false;
        }

        _paths.RemoveAt(index);
        return true;
    }

    public AddPartResult TryReplace(int index, string? filePath)
    {
        if (index < 0 || index >= _paths.Count || string.IsNullOrWhiteSpace(filePath))
        {
            return new AddPartResult(AddPartStatus.InvalidPath);
        }

        string fullPath;
        try
        {
            fullPath = Path.GetFullPath(filePath);
        }
        catch (Exception exception) when (exception is ArgumentException or NotSupportedException or PathTooLongException)
        {
            return new AddPartResult(AddPartStatus.InvalidPath);
        }

        if (!string.Equals(Path.GetExtension(fullPath), ".cast", StringComparison.OrdinalIgnoreCase))
        {
            return new AddPartResult(AddPartStatus.NotCastFile, fullPath);
        }

        if (!File.Exists(fullPath))
        {
            return new AddPartResult(AddPartStatus.FileNotFound, fullPath);
        }

        if (_paths.Where((_, itemIndex) => itemIndex != index).Contains(fullPath, StringComparer.OrdinalIgnoreCase))
        {
            return new AddPartResult(AddPartStatus.Duplicate, fullPath);
        }

        _paths[index] = fullPath;
        return new AddPartResult(AddPartStatus.Added, fullPath);
    }

    public void Clear() => _paths.Clear();
}

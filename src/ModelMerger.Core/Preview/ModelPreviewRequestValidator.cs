namespace ModelMerger.Core.Preview;

internal static class ModelPreviewRequestValidator
{
    public static string Validate(string? filePath, int triangleLimit)
    {
        if (triangleLimit < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(triangleLimit));
        }

        string normalizedPath;
        try
        {
            normalizedPath = !string.IsNullOrWhiteSpace(filePath)
                ? Path.GetFullPath(filePath)
                : throw new ModelPreviewException(ModelPreviewErrorCode.InvalidPath, filePath);
        }
        catch (ModelPreviewException)
        {
            throw;
        }
        catch (Exception exception) when (
            exception is ArgumentException or NotSupportedException or PathTooLongException)
        {
            throw new ModelPreviewException(ModelPreviewErrorCode.InvalidPath, filePath, exception);
        }

        if (!File.Exists(normalizedPath))
        {
            throw new ModelPreviewException(ModelPreviewErrorCode.MissingFile, normalizedPath);
        }
        if (!string.Equals(Path.GetExtension(normalizedPath), ".cast", StringComparison.OrdinalIgnoreCase))
        {
            throw new ModelPreviewException(ModelPreviewErrorCode.UnsupportedFormat, normalizedPath);
        }
        return normalizedPath;
    }
}

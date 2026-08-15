namespace ModelMerger.Core.Preview;

public readonly record struct PreviewPoint3(float X, float Y, float Z);

public sealed record PreviewMeshData(
    IReadOnlyList<PreviewPoint3> Positions,
    IReadOnlyList<PreviewPoint3> Normals,
    IReadOnlyList<int> TriangleIndices);

public sealed record PreviewBounds(PreviewPoint3 Minimum, PreviewPoint3 Maximum);

public sealed record ModelPreviewData(
    string FilePath,
    string ModelName,
    int SourceMeshCount,
    int SourceVertexCount,
    int SourceTriangleCount,
    int DisplayedTriangleCount,
    bool IsSimplified,
    PreviewBounds Bounds,
    IReadOnlyList<PreviewMeshData> Meshes);

public enum ModelPreviewErrorCode
{
    InvalidPath,
    MissingFile,
    UnsupportedFormat,
    UnreadableModel,
    NoGeometry
}

public sealed class ModelPreviewException(
    ModelPreviewErrorCode code,
    string? filePath,
    Exception? innerException = null)
    : Exception($"Unable to preview model: {code}.", innerException)
{
    public ModelPreviewErrorCode Code { get; } = code;

    public string? FilePath { get; } = filePath;
}

public interface IModelPreviewService
{
    Task<ModelPreviewData> LoadAsync(
        string filePath,
        int triangleLimit = ModelPreviewService.DefaultTriangleLimit,
        CancellationToken cancellationToken = default);
}

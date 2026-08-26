using ModelMerger.Core.Merging;

namespace ModelMerger.Core.Preview;

public sealed class AdaptiveModelPreviewService : IModelPreviewService
{
    public const long DefaultRustThresholdBytes = 2 * 1024 * 1024;
    private readonly IModelPreviewService _csharpService;
    private readonly IModelPreviewService? _rustService;
    private readonly long _rustThresholdBytes;

    public AdaptiveModelPreviewService()
        : this(
            new ModelPreviewService(),
            CreateInstalledRustService(),
            DefaultRustThresholdBytes)
    {
    }

    internal AdaptiveModelPreviewService(
        IModelPreviewService csharpService,
        IModelPreviewService? rustService,
        long rustThresholdBytes)
    {
        ArgumentNullException.ThrowIfNull(csharpService);
        if (rustThresholdBytes < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(rustThresholdBytes));
        }
        _csharpService = csharpService;
        _rustService = rustService;
        _rustThresholdBytes = rustThresholdBytes;
    }

    public Task<ModelPreviewData> LoadAsync(
        string filePath,
        int triangleLimit = ModelPreviewService.DefaultTriangleLimit,
        CancellationToken cancellationToken = default) =>
        Select(filePath).LoadAsync(filePath, triangleLimit, cancellationToken);

    private IModelPreviewService Select(string filePath)
    {
        if (_rustService is null)
        {
            return _csharpService;
        }
        try
        {
            if (new FileInfo(filePath).Length >= _rustThresholdBytes ||
                CastPreviewRouteProbe.Uses32BitFaceIndices(filePath))
            {
                return _rustService;
            }
        }
        catch (Exception exception) when (
            exception is IOException or UnauthorizedAccessException or ArgumentException or
            NotSupportedException or InvalidDataException or OverflowException)
        {
            // The C# adapter preserves the established validation contract for invalid inputs.
        }
        return _csharpService;
    }

    private static IModelPreviewService? CreateInstalledRustService()
    {
        var workerPath = ModelMergeServiceFactory.GetWorkerPath();
        return File.Exists(workerPath) ? new RustWorkerModelPreviewService(workerPath) : null;
    }
}

public static class ModelPreviewServiceFactory
{
    public static IModelPreviewService CreateDefault()
    {
        var engine = Environment.GetEnvironmentVariable("MODEL_MERGER_ENGINE");
        return engine?.Trim().ToLowerInvariant() switch
        {
            "csharp" => new ModelPreviewService(),
            "rust" => new RustWorkerModelPreviewService(ModelMergeServiceFactory.GetWorkerPath()),
            _ => new AdaptiveModelPreviewService()
        };
    }
}

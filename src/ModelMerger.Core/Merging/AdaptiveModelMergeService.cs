namespace ModelMerger.Core.Merging;

public sealed class AdaptiveModelMergeService : IModelMergeService
{
    public const long DefaultRustThresholdBytes = 4 * 1024 * 1024;
    private readonly IModelMergeService _csharpService;
    private readonly IModelMergeService? _rustService;
    private readonly long _rustThresholdBytes;

    public AdaptiveModelMergeService()
        : this(
            new ModelMergeService(),
            CreateInstalledRustService(),
            DefaultRustThresholdBytes)
    {
    }

    internal AdaptiveModelMergeService(
        IModelMergeService csharpService,
        IModelMergeService? rustService,
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

    public Task<IPreparedMergeOperation> PrepareAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        return Select(request).PrepareAsync(request, progress, cancellationToken);
    }

    public Task<MergeResult> MergeAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        return Select(request).MergeAsync(request, progress, cancellationToken);
    }

    private IModelMergeService Select(MergeRequest request)
    {
        if (_rustService is null)
        {
            return _csharpService;
        }

        long totalBytes = 0;
        try
        {
            foreach (var input in request.InputFiles)
            {
                totalBytes = checked(totalBytes + new FileInfo(input).Length);
                if (totalBytes >= _rustThresholdBytes)
                {
                    return _rustService;
                }
            }
        }
        catch (Exception exception) when (
            exception is IOException or UnauthorizedAccessException or ArgumentException or
            NotSupportedException or OverflowException)
        {
            // The C# engine preserves the established validation contract for invalid inputs.
        }

        return _csharpService;
    }

    private static IModelMergeService? CreateInstalledRustService()
    {
        var workerPath = ModelMergeServiceFactory.GetWorkerPath();
        return File.Exists(workerPath) ? new RustWorkerMergeService(workerPath) : null;
    }
}

public static class ModelMergeServiceFactory
{
    private const string EngineEnvironmentVariable = "MODEL_MERGER_ENGINE";
    private const string WorkerEnvironmentVariable = "MODEL_MERGER_RUST_WORKER";

    public static IModelMergeService CreateDefault()
    {
        var engine = Environment.GetEnvironmentVariable(EngineEnvironmentVariable);
        return engine?.Trim().ToLowerInvariant() switch
        {
            "csharp" => new ModelMergeService(),
            "rust" => new RustWorkerMergeService(GetWorkerPath()),
            _ => new AdaptiveModelMergeService()
        };
    }

    internal static string GetWorkerPath()
    {
        var configured = Environment.GetEnvironmentVariable(WorkerEnvironmentVariable);
        return string.IsNullOrWhiteSpace(configured)
            ? Path.Combine(AppContext.BaseDirectory, "model-merger-worker.exe")
            : Path.GetFullPath(configured);
    }
}

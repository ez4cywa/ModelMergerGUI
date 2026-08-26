using ModelMerger.Core.Merging;

namespace ModelMerger.Core.Preview;

public sealed class RustWorkerModelPreviewService : IModelPreviewService
{
    private readonly Func<IRustWorkerProcess> _processFactory;

    public RustWorkerModelPreviewService(string executablePath)
        : this(() => ProcessRustWorker.Start(executablePath))
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(executablePath);
    }

    internal RustWorkerModelPreviewService(Func<IRustWorkerProcess> processFactory)
    {
        ArgumentNullException.ThrowIfNull(processFactory);
        _processFactory = processFactory;
    }

    public async Task<ModelPreviewData> LoadAsync(
        string filePath,
        int triangleLimit = ModelPreviewService.DefaultTriangleLimit,
        CancellationToken cancellationToken = default)
    {
        var normalizedPath = ModelPreviewRequestValidator.Validate(filePath, triangleLimit);
        cancellationToken.ThrowIfCancellationRequested();
        var process = _processFactory();
        try
        {
            await process.WriteLineAsync(
                RustPreviewProtocol.CreateCommand(normalizedPath, triangleLimit),
                cancellationToken).ConfigureAwait(false);
            var descriptor = await RustPreviewProtocol
                .ReadResultAsync(process, cancellationToken)
                .ConfigureAwait(false);
            var meshes = await Task.Run(
                () => RustPreviewPayloadCodec.Read(descriptor, cancellationToken),
                cancellationToken).ConfigureAwait(false);
            await process.WriteLineAsync(
                RustPreviewProtocol.ReleaseCommand,
                CancellationToken.None).ConfigureAwait(false);
            await RustWorkerLifetime.StopAndDisposeAsync(process).ConfigureAwait(false);
            return new ModelPreviewData(
                descriptor.FilePath,
                descriptor.ModelName,
                descriptor.SourceMeshCount,
                descriptor.SourceVertexCount,
                descriptor.SourceTriangleCount,
                descriptor.DisplayedTriangleCount,
                descriptor.IsSimplified,
                new PreviewBounds(descriptor.Minimum, descriptor.Maximum),
                meshes);
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            await RustWorkerLifetime.CancelAndDisposeAsync(
                process,
                RustPreviewProtocol.CancelCommand).ConfigureAwait(false);
            throw;
        }
        catch
        {
            await RustWorkerLifetime.CancelAndDisposeAsync(
                process,
                RustPreviewProtocol.CancelCommand).ConfigureAwait(false);
            throw;
        }
    }
}

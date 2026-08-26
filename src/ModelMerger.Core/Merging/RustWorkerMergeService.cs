using System.Diagnostics;
using System.Text.Json;

namespace ModelMerger.Core.Merging;

public sealed class RustWorkerMergeService : IModelMergeService
{
    private const int ProtocolVersion = 1;
    private static readonly TimeSpan ShutdownTimeout = TimeSpan.FromSeconds(2);
    private readonly Func<IRustWorkerProcess> _processFactory;

    public RustWorkerMergeService()
        : this(Path.Combine(AppContext.BaseDirectory, "model-merger-worker.exe"))
    {
    }

    public RustWorkerMergeService(string workerExecutablePath)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(workerExecutablePath);
        var fullPath = Path.GetFullPath(workerExecutablePath);
        _processFactory = () => ProcessRustWorker.Start(fullPath);
    }

    internal RustWorkerMergeService(Func<IRustWorkerProcess> processFactory)
    {
        _processFactory = processFactory ?? throw new ArgumentNullException(nameof(processFactory));
    }

    public async Task<IPreparedMergeOperation> PrepareAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(request);
        cancellationToken.ThrowIfCancellationRequested();
        var process = _processFactory();
        try
        {
            await process.WriteLineAsync(
                JsonSerializer.Serialize(new
                {
                    protocol = ProtocolVersion,
                    command = "prepare",
                    request = new
                    {
                        input_files = request.InputFiles,
                        output_directory = request.OutputDirectory,
                        output_file_name = request.OutputFileName,
                        root_selection_mode = request.RootSelectionMode == RootSelectionMode.Manual
                            ? "manual"
                            : "automatic",
                        manual_root_file = request.ManualRootFile,
                        overwrite = request.Overwrite
                    }
                }),
                cancellationToken).ConfigureAwait(false);
            var prepared = await ReadUntilAsync(
                process,
                "prepared",
                progress,
                cancellationToken).ConfigureAwait(false);
            return new RustPreparedMergeOperation(
                process,
                prepared.GetProperty("output_path").GetString()
                    ?? throw new InvalidDataException("Rust worker omitted output_path."));
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            await CancelAndDisposeAsync(process).ConfigureAwait(false);
            throw;
        }
        catch
        {
            await StopAndDisposeAsync(process).ConfigureAwait(false);
            throw;
        }
    }

    public async Task<MergeResult> MergeAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default)
    {
        var prepared = await PrepareAsync(request, progress, cancellationToken).ConfigureAwait(false);
        return await prepared.ExecuteAsync(progress, cancellationToken).ConfigureAwait(false);
    }

    private static async Task<JsonElement> ReadUntilAsync(
        IRustWorkerProcess process,
        string expectedEvent,
        IProgress<MergeProgress>? progress,
        CancellationToken cancellationToken)
    {
        while (true)
        {
            var line = await process.ReadLineAsync(cancellationToken).ConfigureAwait(false);
            if (line is null)
            {
                throw new InvalidDataException(
                    $"Rust merge worker exited before reporting '{expectedEvent}'.");
            }

            using var document = JsonDocument.Parse(line);
            var root = document.RootElement;
            if (!root.TryGetProperty("protocol", out var protocol) ||
                protocol.GetInt32() != ProtocolVersion)
            {
                throw new InvalidDataException("Rust merge worker used an unsupported protocol version.");
            }

            var eventName = root.GetProperty("event").GetString();
            if (eventName == "progress")
            {
                progress?.Report(ParseProgress(root));
                continue;
            }

            if (eventName == "error")
            {
                throw ParseWorkerError(root, cancellationToken);
            }

            if (eventName != expectedEvent)
            {
                throw new InvalidDataException(
                    $"Rust merge worker reported unexpected event '{eventName}'.");
            }

            return root.Clone();
        }
    }

    private static MergeProgress ParseProgress(JsonElement root)
    {
        var stageName = root.GetProperty("stage").GetString();
        var (stage, code) = stageName switch
        {
            "validating" => (MergeStage.Validating, MergeProgressCode.ValidatingRequest),
            "loading" => (MergeStage.Loading, MergeProgressCode.LoadingFile),
            "selecting_root" => (MergeStage.SelectingRoot, MergeProgressCode.SelectingRootModel),
            "merging" => (MergeStage.Merging, MergeProgressCode.MergingModel),
            "saving" => (MergeStage.Saving, MergeProgressCode.SavingFile),
            "verifying" => (MergeStage.Verifying, MergeProgressCode.VerifyingCast),
            "completed" => (MergeStage.Completed, MergeProgressCode.SavedFile),
            _ => throw new InvalidDataException($"Unknown Rust worker progress stage '{stageName}'.")
        };
        var item = root.TryGetProperty("item", out var itemElement) &&
                   itemElement.ValueKind == JsonValueKind.String
            ? itemElement.GetString()
            : null;
        return new MergeProgress(
            stage,
            root.GetProperty("current").GetInt32(),
            root.GetProperty("total").GetInt32(),
            code,
            item);
    }

    private static Exception ParseWorkerError(JsonElement root, CancellationToken cancellationToken)
    {
        var code = root.GetProperty("code").GetString();
        var message = root.GetProperty("message").GetString() ?? "Rust merge worker failed.";
        if (code == "cancelled")
        {
            return new OperationCanceledException(message, cancellationToken);
        }

        if (code == "validation")
        {
            var validationCode = root.TryGetProperty("validation_code", out var validationElement)
                ? validationElement.GetString() switch
                {
                    "invalid_part_count" => MergeValidationErrorCode.InvalidPartCount,
                    "invalid_path" => MergeValidationErrorCode.InvalidPath,
                    "missing_file" => MergeValidationErrorCode.MissingFile,
                    "unsupported_extension" => MergeValidationErrorCode.UnsupportedExtension,
                    "duplicate_file" => MergeValidationErrorCode.DuplicateFile,
                    "invalid_output_directory" => MergeValidationErrorCode.InvalidOutputDirectory,
                    "invalid_output_file_name" => MergeValidationErrorCode.InvalidOutputFileName,
                    "output_already_exists" => MergeValidationErrorCode.OutputAlreadyExists,
                    "manual_root_not_selected" => MergeValidationErrorCode.ManualRootNotSelected,
                    _ => MergeValidationErrorCode.InvalidPath
                }
                : MergeValidationErrorCode.InvalidPath;
            var path = root.TryGetProperty("path", out var pathElement) &&
                       pathElement.ValueKind == JsonValueKind.String
                ? pathElement.GetString()
                : null;
            return new MergeValidationException(
            [
                new MergeValidationError(validationCode, message, path)
            ]);
        }

        if (code == "model_read")
        {
            var path = root.GetProperty("path").GetString() ?? string.Empty;
            var format = root.TryGetProperty("format", out var formatElement)
                ? formatElement.GetString() ?? "Cast"
                : "Cast";
            return new ModelPartReadException(path, format, new InvalidDataException(message));
        }

        return new InvalidDataException(message);
    }

    private static MergeResult ParseResult(JsonElement root)
    {
        var warnings = new List<MergeWarning>();
        if (root.TryGetProperty("warnings", out var warningArray))
        {
            foreach (var warning in warningArray.EnumerateArray())
            {
                var code = warning.GetProperty("code").GetString() switch
                {
                    "no_attachment_bone" => MergeWarningCode.NoAttachmentBone,
                    "unconnected_hierarchy" => MergeWarningCode.UnconnectedHierarchy,
                    var value => throw new InvalidDataException(
                        $"Unknown Rust worker warning code '{value}'.")
                };
                warnings.Add(new MergeWarning(
                    code,
                    warning.GetProperty("model_name").GetString() ?? string.Empty,
                    warning.GetProperty("root_model_name").GetString() ?? string.Empty));
            }
        }

        return new MergeResult(
            root.GetProperty("output_path").GetString()
                ?? throw new InvalidDataException("Rust worker omitted result output_path."),
            root.GetProperty("root_model_name").GetString()
                ?? throw new InvalidDataException("Rust worker omitted root_model_name."),
            root.GetProperty("part_count").GetInt32(),
            root.GetProperty("bone_count").GetInt32(),
            root.GetProperty("mesh_count").GetInt32(),
            warnings);
    }

    private static async Task CancelAndDisposeAsync(IRustWorkerProcess process)
    {
        try
        {
            if (!process.HasExited)
            {
                await process.WriteLineAsync(
                    JsonSerializer.Serialize(new { protocol = ProtocolVersion, command = "cancel" }),
                    CancellationToken.None).ConfigureAwait(false);
            }
        }
        catch (Exception exception) when (exception is IOException or InvalidOperationException)
        {
            // The fallback below terminates a worker that stopped accepting input.
        }

        await StopAndDisposeAsync(process).ConfigureAwait(false);
    }

    private static async Task StopAndDisposeAsync(IRustWorkerProcess process)
    {
        if (!process.HasExited)
        {
            using var timeout = new CancellationTokenSource(ShutdownTimeout);
            try
            {
                await process.WaitForExitAsync(timeout.Token).ConfigureAwait(false);
            }
            catch (OperationCanceledException) when (timeout.IsCancellationRequested)
            {
                process.Kill();
            }
        }

        await process.DisposeAsync().ConfigureAwait(false);
    }

    private sealed class RustPreparedMergeOperation(
        IRustWorkerProcess process,
        string outputPath) : IPreparedMergeOperation
    {
        private int _executed;

        public string OutputPath { get; } = outputPath;

        public async Task<MergeResult> ExecuteAsync(
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default)
        {
            if (Interlocked.Exchange(ref _executed, 1) != 0)
            {
                throw new InvalidOperationException("A prepared merge operation can only be executed once.");
            }

            try
            {
                await process.WriteLineAsync(
                    JsonSerializer.Serialize(new { protocol = ProtocolVersion, command = "execute" }),
                    cancellationToken).ConfigureAwait(false);
                var result = await ReadUntilAsync(
                    process,
                    "result",
                    progress,
                    cancellationToken).ConfigureAwait(false);
                await StopAndDisposeAsync(process).ConfigureAwait(false);
                return ParseResult(result);
            }
            catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
            {
                await CancelAndDisposeAsync(process).ConfigureAwait(false);
                throw;
            }
            catch
            {
                await StopAndDisposeAsync(process).ConfigureAwait(false);
                throw;
            }
        }
    }
}

internal interface IRustWorkerProcess : IAsyncDisposable
{
    bool HasExited { get; }

    Task WriteLineAsync(string line, CancellationToken cancellationToken);

    Task<string?> ReadLineAsync(CancellationToken cancellationToken);

    Task WaitForExitAsync(CancellationToken cancellationToken);

    void Kill();
}

internal sealed class ProcessRustWorker : IRustWorkerProcess
{
    private readonly Process _process;

    private ProcessRustWorker(Process process)
    {
        _process = process;
        _ = process.StandardError.ReadToEndAsync();
    }

    public bool HasExited => _process.HasExited;

    public static ProcessRustWorker Start(string executablePath)
    {
        if (!File.Exists(executablePath))
        {
            throw new FileNotFoundException(
                "Rust merge worker was not found. Build or install model-merger-worker.exe.",
                executablePath);
        }

        var process = Process.Start(new ProcessStartInfo
        {
            FileName = executablePath,
            UseShellExecute = false,
            RedirectStandardInput = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true
        }) ?? throw new InvalidOperationException("Unable to start Rust merge worker.");
        return new ProcessRustWorker(process);
    }

    public async Task WriteLineAsync(string line, CancellationToken cancellationToken)
    {
        await _process.StandardInput.WriteLineAsync(line.AsMemory(), cancellationToken)
            .ConfigureAwait(false);
        await _process.StandardInput.FlushAsync(cancellationToken).ConfigureAwait(false);
    }

    public Task<string?> ReadLineAsync(CancellationToken cancellationToken) =>
        _process.StandardOutput.ReadLineAsync(cancellationToken).AsTask();

    public Task WaitForExitAsync(CancellationToken cancellationToken) =>
        _process.WaitForExitAsync(cancellationToken);

    public void Kill()
    {
        if (!_process.HasExited)
        {
            _process.Kill(entireProcessTree: true);
        }
    }

    public ValueTask DisposeAsync()
    {
        _process.Dispose();
        return ValueTask.CompletedTask;
    }
}

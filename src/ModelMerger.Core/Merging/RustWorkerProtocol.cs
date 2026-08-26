using System.Text.Json;

namespace ModelMerger.Core.Merging;

internal static class RustWorkerProtocol
{
    private const int Version = 1;

    public static string ExecuteCommand { get; } = SerializeCommand("execute");

    public static string CancelCommand { get; } = SerializeCommand("cancel");

    public static string CreatePrepareCommand(MergeRequest request) =>
        JsonSerializer.Serialize(new
        {
            protocol = Version,
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
        });

    public static async Task<string> ReadPreparedAsync(
        IRustWorkerProcess process,
        IProgress<MergeProgress>? progress,
        CancellationToken cancellationToken)
    {
        var prepared = await ReadUntilAsync(
            process,
            "prepared",
            progress,
            cancellationToken).ConfigureAwait(false);
        return prepared.GetProperty("output_path").GetString()
            ?? throw new InvalidDataException("Rust worker omitted output_path.");
    }

    public static async Task<MergeResult> ReadResultAsync(
        IRustWorkerProcess process,
        IProgress<MergeProgress>? progress,
        CancellationToken cancellationToken)
    {
        var result = await ReadUntilAsync(
            process,
            "result",
            progress,
            cancellationToken).ConfigureAwait(false);
        return ParseResult(result);
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
            if (!root.TryGetProperty("protocol", out var protocol) || protocol.GetInt32() != Version)
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
                ? ParseValidationCode(validationElement.GetString())
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

    private static MergeValidationErrorCode ParseValidationCode(string? code) => code switch
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
    };

    private static string SerializeCommand(string command) =>
        JsonSerializer.Serialize(new { protocol = Version, command });
}

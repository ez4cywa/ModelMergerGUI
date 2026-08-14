namespace ModelMerger.Core.Merging;

public enum RootSelectionMode
{
    Automatic,
    Manual
}

public sealed record MergeRequest(
    IReadOnlyList<string> InputFiles,
    string OutputDirectory,
    string? OutputFileName = null,
    RootSelectionMode RootSelectionMode = RootSelectionMode.Automatic,
    string? ManualRootFile = null,
    bool Overwrite = false);

public enum MergeStage
{
    Validating,
    Loading,
    SelectingRoot,
    Merging,
    Saving,
    Verifying,
    Completed
}

public enum MergeProgressCode
{
    ValidatingRequest,
    LoadingFile,
    SelectingRootModel,
    MergingModel,
    SavingFile,
    VerifyingCast,
    SavedFile
}

public sealed record MergeProgress(
    MergeStage Stage,
    int Current,
    int Total,
    MergeProgressCode Code,
    string? Subject = null)
{
    public double Percentage => Total <= 0 ? 0 : Math.Clamp(Current * 100d / Total, 0, 100);
}

public enum MergeWarningCode
{
    NoAttachmentBone,
    UnconnectedHierarchy
}

public sealed record MergeWarning(
    MergeWarningCode Code,
    string ModelName,
    string RootModelName);

public sealed record MergeResult(
    string OutputPath,
    string RootModelName,
    int PartCount,
    int BoneCount,
    int MeshCount,
    IReadOnlyList<MergeWarning> Warnings);

public interface IModelMergeService
{
    Task<IPreparedMergeOperation> PrepareAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default);

    Task<MergeResult> MergeAsync(
        MergeRequest request,
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default);
}

public interface IPreparedMergeOperation
{
    string OutputPath { get; }

    Task<MergeResult> ExecuteAsync(
        IProgress<MergeProgress>? progress = null,
        CancellationToken cancellationToken = default);
}

public enum MergeValidationErrorCode
{
    InvalidPartCount,
    InvalidPath,
    MissingFile,
    UnsupportedExtension,
    DuplicateFile,
    InvalidOutputDirectory,
    InvalidOutputFileName,
    OutputAlreadyExists,
    ManualRootNotSelected
}

public sealed record MergeValidationError(MergeValidationErrorCode Code, string Message, string? FilePath = null);

public sealed class MergeValidationException(IReadOnlyList<MergeValidationError> errors)
    : Exception("The merge request is invalid.")
{
    public IReadOnlyList<MergeValidationError> Errors { get; } = errors;
}

public sealed class ModelPartReadException(
    string filePath,
    string formatName,
    Exception innerException)
    : Exception(
        $"Unable to read {Path.GetFileName(filePath)}. The file is not a valid or readable {formatName} model. {innerException.Message}",
        innerException)
{
    public string FilePath { get; } = filePath;

    public string FormatName { get; } = formatName;
}

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

public sealed record MergeProgress(
    MergeStage Stage,
    int Current,
    int Total,
    string Message)
{
    public double Percentage => Total <= 0 ? 0 : Math.Clamp(Current * 100d / Total, 0, 100);
}

public sealed record MergeResult(
    string OutputPath,
    string RootModelName,
    int PartCount,
    int BoneCount,
    int MeshCount,
    IReadOnlyList<string> Warnings);

public interface IModelMergeService
{
    Task<MergeResult> MergeAsync(
        MergeRequest request,
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

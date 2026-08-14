using ModelMerger.Core.Merging;
using ModelMerger.Core.Selection;

namespace ModelMerger.Core.Planning;

public sealed record MergeGroupPlanState(
    IReadOnlyList<string> PartFiles,
    string OutputDirectory,
    string OutputFileName,
    bool HasExplicitOutputDirectory,
    RootSelectionMode RootSelectionMode,
    string? ManualRootFile,
    string? RecentInputDirectory)
{
    public int PartCount => PartFiles.Count;

    public bool IsReady =>
        PartCount >= ModelPartCollection.MinimumParts &&
        PartFiles.All(path =>
            File.Exists(path) && string.Equals(Path.GetExtension(path), ".cast", StringComparison.OrdinalIgnoreCase)) &&
        !string.IsNullOrWhiteSpace(OutputDirectory) &&
        (RootSelectionMode == RootSelectionMode.Automatic ||
         ManualRootFile is not null && PartFiles.Contains(ManualRootFile, StringComparer.OrdinalIgnoreCase));
}

public sealed class MergeGroupPlan
{
    private readonly ModelPartCollection _parts = new();
    private string _outputDirectory = string.Empty;
    private string _outputFileName = string.Empty;
    private string? _manualRootFile;
    private string? _recentInputDirectory;
    private bool _hasExplicitOutputDirectory;
    private RootSelectionMode _rootSelectionMode;

    public MergeGroupPlan(
        string? preferredOutputDirectory = null,
        RootSelectionMode defaultRootMode = RootSelectionMode.Automatic)
    {
        _rootSelectionMode = defaultRootMode;
        if (Directory.Exists(preferredOutputDirectory))
        {
            _outputDirectory = Path.GetFullPath(preferredOutputDirectory!);
            _hasExplicitOutputDirectory = true;
        }
    }

    public MergeGroupPlanState State => new(
        _parts.Paths.ToArray(),
        _outputDirectory,
        _outputFileName,
        _hasExplicitOutputDirectory,
        _rootSelectionMode,
        _manualRootFile,
        _recentInputDirectory);

    public AddPartResult AddPart(string? filePath)
    {
        var result = _parts.TryAdd(filePath);
        if (result.Status == AddPartStatus.Added)
        {
            RememberInputDirectory(result.FilePath);
            NormalizeAfterPartsChanged();
        }

        return result;
    }

    public AddPartResult ReplacePart(int index, string? filePath)
    {
        var replacesManualRoot = index >= 0 &&
                                 index < _parts.Count &&
                                 string.Equals(_parts.Paths[index], _manualRootFile, StringComparison.OrdinalIgnoreCase);
        var result = _parts.TryReplace(index, filePath);
        if (result.Status == AddPartStatus.Added)
        {
            if (replacesManualRoot)
            {
                _manualRootFile = result.FilePath;
            }

            RememberInputDirectory(result.FilePath);
            NormalizeAfterPartsChanged();
        }

        return result;
    }

    public bool RemovePart(int index)
    {
        if (!_parts.RemoveAt(index))
        {
            return false;
        }

        NormalizeAfterPartsChanged();
        return true;
    }

    public void ClearParts()
    {
        _parts.Clear();
        _manualRootFile = null;
        NormalizeAfterPartsChanged();
    }

    public void ChooseOutputDirectory(string? directory)
    {
        if (string.IsNullOrWhiteSpace(directory))
        {
            _hasExplicitOutputDirectory = false;
            RecalculateAutomaticOutputDirectory();
            return;
        }

        _outputDirectory = directory.Trim();
        _hasExplicitOutputDirectory = true;
    }

    public void UseAutomaticOutputDirectory()
    {
        _hasExplicitOutputDirectory = false;
        RecalculateAutomaticOutputDirectory();
    }

    public void SetOutputFileName(string? fileName)
    {
        _outputFileName = fileName ?? string.Empty;
    }

    public void SetRootMode(RootSelectionMode mode)
    {
        _rootSelectionMode = mode;
        NormalizeManualRoot();
    }

    public bool SetManualRoot(int index)
    {
        if (index < 0 || index >= _parts.Count)
        {
            return false;
        }

        _manualRootFile = _parts.Paths[index];
        _rootSelectionMode = RootSelectionMode.Manual;
        return true;
    }

    public void ResetPreferences(string? preferredOutputDirectory, RootSelectionMode defaultRootMode)
    {
        _rootSelectionMode = defaultRootMode;
        _outputFileName = string.Empty;
        if (Directory.Exists(preferredOutputDirectory))
        {
            _outputDirectory = Path.GetFullPath(preferredOutputDirectory!);
            _hasExplicitOutputDirectory = true;
        }
        else
        {
            _hasExplicitOutputDirectory = false;
            RecalculateAutomaticOutputDirectory();
        }

        NormalizeManualRoot();
    }

    public MergeRequest CreateRequest(bool overwrite = false)
    {
        var state = State;
        if (!state.IsReady)
        {
            throw new InvalidOperationException("The model merge plan is not ready.");
        }

        return new MergeRequest(
            state.PartFiles,
            state.OutputDirectory,
            string.IsNullOrWhiteSpace(state.OutputFileName) ? null : state.OutputFileName.Trim(),
            state.RootSelectionMode,
            state.RootSelectionMode == RootSelectionMode.Manual ? state.ManualRootFile : null,
            overwrite);
    }

    private void NormalizeAfterPartsChanged()
    {
        NormalizeManualRoot();
        if (!_hasExplicitOutputDirectory)
        {
            RecalculateAutomaticOutputDirectory();
        }
    }

    private void NormalizeManualRoot()
    {
        if (_manualRootFile is not null &&
            !_parts.Paths.Contains(_manualRootFile, StringComparer.OrdinalIgnoreCase))
        {
            _manualRootFile = null;
        }

        if (_rootSelectionMode == RootSelectionMode.Manual && _manualRootFile is null && _parts.Count > 0)
        {
            _manualRootFile = _parts.Paths[0];
        }
    }

    private void RecalculateAutomaticOutputDirectory()
    {
        _outputDirectory = _parts.Count == 0
            ? string.Empty
            : Path.Combine(Path.GetDirectoryName(_parts.Paths[0])!, "Merged Models");
    }

    private void RememberInputDirectory(string? filePath)
    {
        var directory = string.IsNullOrWhiteSpace(filePath) ? null : Path.GetDirectoryName(filePath);
        if (Directory.Exists(directory))
        {
            _recentInputDirectory = directory;
        }
    }
}

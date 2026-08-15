using ModelMerger.Core.Merging;
using ModelMerger.Core.Planning;
using ModelMerger.Core.Selection;
using ModelMerger.Gui.Commands;
using ModelMerger.Gui.Localization;
using ModelMerger.Gui.Services;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Diagnostics;
using System.Text;
using System.Windows.Input;

namespace ModelMerger.Gui.ViewModels;

internal sealed class MergeGroupViewModel : ViewModelBase, IDisposable
{
    private readonly IMergeTaskScheduler _scheduler;
    private readonly IUserDialogService _dialogs;
    private readonly IModelPreviewDialogService _previewDialogs;
    private readonly ILanguageCatalog _language;
    private readonly MergeGroupPlan _plan;
    private readonly List<LogEntry> _log = [];
    private readonly RelayCommand _addNextCommand;
    private readonly RelayCommand _addOrReplaceCommand;
    private readonly RelayCommand _removePartCommand;
    private readonly RelayCommand _clearCommand;
    private readonly RelayCommand _browseOutputCommand;
    private readonly RelayCommand _setManualRootCommand;
    private readonly RelayCommand _cancelCommand;
    private readonly RelayCommand _openOutputCommand;
    private readonly RelayCommand _previewPartCommand;
    private readonly RelayCommand _previewOutputCommand;
    private readonly AsyncRelayCommand _mergeCommand;
    private MergeTaskHandle? _mergeTask;
    private LocalizedText _status = LocalizedText.FromKey(LanguageKeys.StatusInitial);
    private string? _lastOutputPath;
    private bool _isBusy;
    private bool _isExpanded = true;
    private bool _lastKnownPartFilesValid = true;
    private double _progressValue;

    public MergeGroupViewModel(
        int number,
        IMergeTaskScheduler scheduler,
        IUserDialogService dialogs,
        string? preferredOutputDirectory,
        RootSelectionMode defaultRootMode,
        ILanguageCatalog? languageCatalog = null,
        IModelPreviewDialogService? previewDialogs = null)
    {
        Number = number;
        _scheduler = scheduler;
        _dialogs = dialogs;
        _language = languageCatalog ?? LanguageCatalog.Current;
        _previewDialogs = previewDialogs ?? new ModelPreviewDialogService(_language);
        _language.PropertyChanged += Language_PropertyChanged;
        _plan = new MergeGroupPlan(preferredOutputDirectory, defaultRootMode);
        Slots = new ObservableCollection<PartSlotViewModel>(
            Enumerable.Range(0, ModelPartCollection.MaximumParts)
                .Select(index => new PartSlotViewModel(index, _language)));

        _addNextCommand = new RelayCommand(_ => AddNextPart(), _ => !IsBusy && PartCount < ModelPartCollection.MaximumParts);
        _addOrReplaceCommand = new RelayCommand(AddOrReplacePart, parameter => !IsBusy && parameter is PartSlotViewModel);
        _removePartCommand = new RelayCommand(RemovePart, parameter => !IsBusy && parameter is PartSlotViewModel slot && slot.IsOccupied);
        _clearCommand = new RelayCommand(_ => ClearParts(), _ => !IsBusy && PartCount > 0);
        _browseOutputCommand = new RelayCommand(_ => BrowseOutputDirectory(), _ => !IsBusy);
        _setManualRootCommand = new RelayCommand(SetManualRoot, parameter => !IsBusy && parameter is PartSlotViewModel slot && slot.IsOccupied);
        _cancelCommand = new RelayCommand(_ => Cancel(), _ => IsBusy);
        _openOutputCommand = new RelayCommand(_ => OpenLastOutput(), _ => LastOutputPath is not null && File.Exists(LastOutputPath));
        _previewPartCommand = new RelayCommand(
            PreviewPart,
            parameter => parameter is PartSlotViewModel { FilePath: not null } slot && File.Exists(slot.FilePath));
        _previewOutputCommand = new RelayCommand(
            _ => PreviewOutput(),
            _ => LastOutputPath is not null && File.Exists(LastOutputPath));
        _mergeCommand = new AsyncRelayCommand(MergeAsync, () => CanMerge);
    }

    public event EventHandler? StateChanged;

    public event EventHandler<string>? OutputDirectoryChosen;

    public int Number { get; }

    public string Name => _language.Format(LanguageKeys.GroupName, Number);

    public ObservableCollection<PartSlotViewModel> Slots { get; }

    public ICommand AddNextCommand => _addNextCommand;

    public ICommand AddOrReplaceCommand => _addOrReplaceCommand;

    public ICommand RemovePartCommand => _removePartCommand;

    public ICommand ClearCommand => _clearCommand;

    public ICommand BrowseOutputCommand => _browseOutputCommand;

    public ICommand SetManualRootCommand => _setManualRootCommand;

    public ICommand CancelCommand => _cancelCommand;

    public ICommand OpenOutputCommand => _openOutputCommand;

    public ICommand PreviewPartCommand => _previewPartCommand;

    public ICommand PreviewOutputCommand => _previewOutputCommand;

    public ICommand MergeCommand => _mergeCommand;

    public int PartCount => _plan.State.PartCount;

    public string PartCountText => $"{PartCount} / {ModelPartCollection.MaximumParts}";

    public string SummaryText => IsBusy
        ? _language.Format(LanguageKeys.SummaryProcessing, PartCountText)
        : CanMerge
            ? _language.Format(LanguageKeys.SummaryReady, PartCountText)
            : _language.Format(LanguageKeys.SummaryNeedTwo, PartCountText);

    public bool CanMerge => _plan.State.IsReady && !IsBusy;

    public bool IsEditable => !IsBusy;

    public bool IsExpanded
    {
        get => _isExpanded;
        set => SetProperty(ref _isExpanded, value);
    }

    public string OutputDirectory
    {
        get => _plan.State.OutputDirectory;
        set
        {
            var previous = _plan.State.OutputDirectory;
            _plan.ChooseOutputDirectory(value);
            if (string.Equals(previous, _plan.State.OutputDirectory, StringComparison.Ordinal))
            {
                return;
            }

            RaisePropertyChanged();
            if (_plan.State.HasExplicitOutputDirectory && !string.IsNullOrWhiteSpace(value))
            {
                OutputDirectoryChosen?.Invoke(this, value);
            }

            OnMergeStateChanged();
        }
    }

    public string OutputFileName
    {
        get => _plan.State.OutputFileName;
        set
        {
            var previous = _plan.State.OutputFileName;
            _plan.SetOutputFileName(value);
            if (!string.Equals(previous, _plan.State.OutputFileName, StringComparison.Ordinal))
            {
                RaisePropertyChanged();
            }
        }
    }

    public RootSelectionMode RootSelectionMode => _plan.State.RootSelectionMode;

    public bool IsAutomaticRoot
    {
        get => RootSelectionMode == RootSelectionMode.Automatic;
        set
        {
            if (value)
            {
                SetRootMode(RootSelectionMode.Automatic);
            }
        }
    }

    public bool IsManualRootMode
    {
        get => RootSelectionMode == RootSelectionMode.Manual;
        set
        {
            if (value)
            {
                SetRootMode(RootSelectionMode.Manual);
            }
        }
    }

    public bool IsBusy
    {
        get => _isBusy;
        private set
        {
            if (SetProperty(ref _isBusy, value))
            {
                RaisePropertyChanged(nameof(IsEditable));
                OnMergeStateChanged();
            }
        }
    }

    public double ProgressValue
    {
        get => _progressValue;
        private set => SetProperty(ref _progressValue, value);
    }

    public string StatusMessage => _status.Render(_language);

    public string LogText
    {
        get
        {
            var result = new StringBuilder();
            foreach (var entry in _log)
            {
                result.Append('[')
                    .Append(entry.Timestamp.ToString("HH:mm:ss"))
                    .Append("] ")
                    .AppendLine(entry.Message.Render(_language));
            }

            return result.ToString();
        }
    }

    public string? LastOutputPath
    {
        get => _lastOutputPath;
        private set
        {
            if (SetProperty(ref _lastOutputPath, value))
            {
                _openOutputCommand.RaiseCanExecuteChanged();
                _previewOutputCommand.RaiseCanExecuteChanged();
            }
        }
    }

    public void AddDroppedFiles(IEnumerable<string> files)
    {
        if (IsBusy)
        {
            return;
        }

        var rejected = new List<string>();
        var added = 0;
        foreach (var file in files)
        {
            var result = _plan.AddPart(file);
            if (result.Status == AddPartStatus.Added)
            {
                added++;
            }
            else
            {
                rejected.Add($"{Path.GetFileName(file)}: {Describe(result.Status)}");
            }
        }

        RefreshSlots();
        IsExpanded = true;
        if (rejected.Count > 0)
        {
            _dialogs.ShowInformation(
                Text(LanguageKeys.DroppedPartialTitle, Name),
                Text(LanguageKeys.DroppedPartialBody, added, string.Join(Environment.NewLine, rejected)));
        }
    }

    public async Task MergeAsync()
    {
        if (!CanMerge)
        {
            return;
        }

        var overwrite = ConfirmKnownOverwrite();
        if (overwrite is null)
        {
            return;
        }

        var request = _plan.CreateRequest(overwrite.Value);
        IsBusy = true;
        IsExpanded = true;
        ProgressValue = 0;
        LastOutputPath = null;
        SetStatus(LanguageKeys.QueueWaiting);
        AppendLog(LanguageKeys.QueueLog, Name, PartCount);
        var progress = new Progress<MergeProgress>(OnProgress);
        try
        {
            MergeResult result;
            try
            {
                _mergeTask = _scheduler.Schedule(request, progress);
                result = await _mergeTask.Completion;
            }
            catch (MergeValidationException exception) when (
                exception.Errors.Any(error => error.Code == MergeValidationErrorCode.OutputAlreadyExists))
            {
                var output = exception.Errors.First(error => error.Code == MergeValidationErrorCode.OutputAlreadyExists).FilePath;
                if (!_dialogs.Confirm(
                        Text(LanguageKeys.OverwriteTitle),
                        Text(LanguageKeys.OverwritePrompt, Name, output)))
                {
                    SetStatus(LanguageKeys.OverwriteCancelled);
                    return;
                }

                _mergeTask = _scheduler.Schedule(request with { Overwrite = true }, progress);
                result = await _mergeTask.Completion;
            }

            LastOutputPath = result.OutputPath;
            ProgressValue = 100;
            SetStatus(LanguageKeys.MergeCompletedStatus, Path.GetFileName(result.OutputPath));
            AppendLog(LanguageKeys.MergeCompletedLog, result.BoneCount, result.MeshCount);
            var warningText = result.Warnings.Count > 0
                ? $"\n\n{Text(LanguageKeys.WarningsHeading)}\n{string.Join(Environment.NewLine, result.Warnings.Select(Describe))}"
                : string.Empty;
            _dialogs.ShowInformation(
                Text(LanguageKeys.MergeCompletedTitle, Name),
                Text(LanguageKeys.MergeCompletedBody, result.OutputPath, result.BoneCount, result.MeshCount, warningText));
        }
        catch (OperationCanceledException)
        {
            SetStatus(LanguageKeys.CancelledStatus);
            AppendLog(LanguageKeys.CancelledLog);
        }
        catch (MergeValidationException exception)
        {
            var messages = exception.Errors.Select(Describe).ToArray();
            var message = string.Join(Environment.NewLine, messages);
            SetStatus(LanguageKeys.InvalidRequestStatus);
            foreach (var item in exception.Errors)
            {
                AppendLog(DescribeText(item));
            }

            _dialogs.ShowError(Text(LanguageKeys.InvalidRequestTitle, Name), message);
        }
        catch (MergeOutputConflictException exception)
        {
            SetStatus(LanguageKeys.OutputConflictStatus);
            AppendLog(LanguageKeys.OutputConflictBody, exception.OutputPath);
            _dialogs.ShowError(
                Text(LanguageKeys.OutputConflictTitle, Name),
                Text(LanguageKeys.OutputConflictBody, exception.OutputPath));
        }
        catch (ModelPartReadException exception)
        {
            SetStatus(LanguageKeys.MergeFailedStatus);
            AppendLog(LanguageKeys.ModelPartReadError, exception.FilePath, exception.FormatName);
            _dialogs.ShowError(
                Text(LanguageKeys.MergeFailedTitle, Name),
                Text(LanguageKeys.ModelPartReadError, exception.FilePath, exception.FormatName));
        }
        catch (Exception exception)
        {
            SetStatus(LanguageKeys.MergeFailedStatus);
            AppendLiteralLog(exception.ToString());
            _dialogs.ShowError(Text(LanguageKeys.MergeFailedTitle, Name), exception.Message);
        }
        finally
        {
            _mergeTask = null;
            IsBusy = false;
        }
    }

    public void Cancel() => _mergeTask?.Cancel();

    public void RefreshFileValidity()
    {
        if (IsBusy)
        {
            return;
        }

        foreach (var slot in Slots)
        {
            slot.RefreshValidity();
        }

        _previewPartCommand.RaiseCanExecuteChanged();
        _openOutputCommand.RaiseCanExecuteChanged();
        _previewOutputCommand.RaiseCanExecuteChanged();

        var filesValid = ArePartFilesValid(_plan.State.PartFiles);
        if (filesValid == _lastKnownPartFilesValid)
        {
            return;
        }

        _lastKnownPartFilesValid = filesValid;
        OnMergeStateChanged();
    }

    public void ResetPreferences(string? preferredOutputDirectory, RootSelectionMode defaultRootMode)
    {
        _plan.ResetPreferences(preferredOutputDirectory, defaultRootMode);
        RaisePropertyChanged(nameof(OutputDirectory));
        RaisePropertyChanged(nameof(OutputFileName));
        RaisePropertyChanged(nameof(RootSelectionMode));
        RaisePropertyChanged(nameof(IsAutomaticRoot));
        RaisePropertyChanged(nameof(IsManualRootMode));
        RefreshRootMarkers();
        OnMergeStateChanged();
    }

    public void Dispose()
    {
        _language.PropertyChanged -= Language_PropertyChanged;
    }

    private void AddNextPart()
    {
        var file = _dialogs.PickCastFile(_plan.State.RecentInputDirectory);
        if (file is not null)
        {
            HandleAddResult(_plan.AddPart(file));
        }
    }

    private void AddOrReplacePart(object? parameter)
    {
        if (parameter is not PartSlotViewModel slot)
        {
            return;
        }

        var initialDirectory = slot.IsOccupied ? Path.GetDirectoryName(slot.FilePath) : _plan.State.RecentInputDirectory;
        var file = _dialogs.PickCastFile(initialDirectory);
        if (file is null)
        {
            return;
        }

        HandleAddResult(slot.IsOccupied ? _plan.ReplacePart(slot.Index, file) : _plan.AddPart(file));
    }

    private void HandleAddResult(AddPartResult result)
    {
        if (result.Status != AddPartStatus.Added)
        {
            _dialogs.ShowInformation(Text(LanguageKeys.AddFailedTitle), Describe(result.Status));
            return;
        }

        RefreshSlots();
        SetStatus(LanguageKeys.AddedStatus, PartCount);
    }

    private void RemovePart(object? parameter)
    {
        if (parameter is PartSlotViewModel slot && _plan.RemovePart(slot.Index))
        {
            RefreshSlots();
            SetStatus(LanguageKeys.RemovedStatus, PartCountText);
        }
    }

    private void ClearParts()
    {
        _plan.ClearParts();
        LastOutputPath = null;
        RefreshSlots();
        SetStatus(LanguageKeys.ClearedStatus);
        ProgressValue = 0;
    }

    private void BrowseOutputDirectory()
    {
        var selected = _dialogs.PickOutputFolder(OutputDirectory);
        if (selected is not null)
        {
            OutputDirectory = selected;
        }
    }

    private void SetManualRoot(object? parameter)
    {
        if (parameter is not PartSlotViewModel { FilePath: not null } slot)
        {
            return;
        }

        if (_plan.SetManualRoot(slot.Index))
        {
            RaisePropertyChanged(nameof(RootSelectionMode));
            RaisePropertyChanged(nameof(IsAutomaticRoot));
            RaisePropertyChanged(nameof(IsManualRootMode));
            RefreshRootMarkers();
            SetStatus(LanguageKeys.ManualRootStatus, slot.FileName);
        }
    }

    private void SetRootMode(RootSelectionMode mode)
    {
        if (RootSelectionMode == mode)
        {
            return;
        }

        _plan.SetRootMode(mode);
        RaisePropertyChanged(nameof(RootSelectionMode));
        RaisePropertyChanged(nameof(IsAutomaticRoot));
        RaisePropertyChanged(nameof(IsManualRootMode));
        RefreshRootMarkers();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    private bool? ConfirmKnownOverwrite()
    {
        if (string.IsNullOrWhiteSpace(OutputFileName))
        {
            return false;
        }

        var fileName = Path.HasExtension(OutputFileName) ? OutputFileName : $"{OutputFileName}.cast";
        var outputPath = Path.Combine(OutputDirectory, fileName);
        if (!File.Exists(outputPath))
        {
            return false;
        }

        return _dialogs.Confirm(
            Text(LanguageKeys.OverwriteTitle),
            Text(LanguageKeys.OverwritePrompt, Name, outputPath))
            ? true
            : null;
    }

    private void OnProgress(MergeProgress progress)
    {
        ProgressValue = progress.Percentage;
        var message = DescribeText(progress);
        SetStatus(message);
        AppendLog(message);
    }

    private void RefreshSlots()
    {
        var state = _plan.State;
        for (var index = 0; index < Slots.Count; index++)
        {
            Slots[index].FilePath = index < state.PartCount ? state.PartFiles[index] : null;
        }

        _lastKnownPartFilesValid = ArePartFilesValid(state.PartFiles);

        RefreshRootMarkers();
        RaisePropertyChanged(nameof(PartCount));
        RaisePropertyChanged(nameof(PartCountText));
        RaisePropertyChanged(nameof(OutputDirectory));
        RaisePropertyChanged(nameof(RootSelectionMode));
        RaisePropertyChanged(nameof(IsAutomaticRoot));
        RaisePropertyChanged(nameof(IsManualRootMode));
        OnMergeStateChanged();
    }

    private static bool ArePartFilesValid(IEnumerable<string> partFiles) =>
        partFiles.All(path =>
            File.Exists(path) &&
            string.Equals(Path.GetExtension(path), ".cast", StringComparison.OrdinalIgnoreCase));

    private void RefreshRootMarkers()
    {
        var state = _plan.State;
        foreach (var slot in Slots)
        {
            slot.IsManualRoot = state.RootSelectionMode == RootSelectionMode.Manual &&
                                slot.FilePath is not null &&
                                string.Equals(slot.FilePath, state.ManualRootFile, StringComparison.OrdinalIgnoreCase);
        }
    }

    private void OpenLastOutput()
    {
        var outputPath = LastOutputPath;
        if (outputPath is null || !File.Exists(outputPath))
        {
            return;
        }

        var startInfo = new ProcessStartInfo { FileName = "explorer.exe", UseShellExecute = true };
        startInfo.ArgumentList.Add("/select,");
        startInfo.ArgumentList.Add(outputPath);
        Process.Start(startInfo);
    }

    private void PreviewPart(object? parameter)
    {
        if (parameter is PartSlotViewModel { FilePath: not null } slot && File.Exists(slot.FilePath))
        {
            _previewDialogs.Show(slot.FilePath);
        }
    }

    private void PreviewOutput()
    {
        if (LastOutputPath is not null && File.Exists(LastOutputPath))
        {
            _previewDialogs.Show(LastOutputPath);
        }
    }

    private void OnMergeStateChanged()
    {
        RaisePropertyChanged(nameof(CanMerge));
        RaisePropertyChanged(nameof(SummaryText));
        RefreshCommandStates();
        StateChanged?.Invoke(this, EventArgs.Empty);
    }

    private void RefreshCommandStates()
    {
        _addNextCommand.RaiseCanExecuteChanged();
        _addOrReplaceCommand.RaiseCanExecuteChanged();
        _removePartCommand.RaiseCanExecuteChanged();
        _clearCommand.RaiseCanExecuteChanged();
        _browseOutputCommand.RaiseCanExecuteChanged();
        _setManualRootCommand.RaiseCanExecuteChanged();
        _cancelCommand.RaiseCanExecuteChanged();
        _mergeCommand.RaiseCanExecuteChanged();
        _previewPartCommand.RaiseCanExecuteChanged();
        _previewOutputCommand.RaiseCanExecuteChanged();
    }

    private void SetStatus(string key, params object?[] arguments) =>
        SetStatus(LocalizedText.FromKey(key, arguments));

    private void SetStatus(LocalizedText status)
    {
        _status = status;
        RaisePropertyChanged(nameof(StatusMessage));
    }

    private void AppendLog(string key, params object?[] arguments) =>
        AppendLog(LocalizedText.FromKey(key, arguments));

    private void AppendLiteralLog(string message) => AppendLog(LocalizedText.FromLiteral(message));

    private void AppendLog(LocalizedText message)
    {
        _log.Add(new LogEntry(DateTime.Now, message));
        RaisePropertyChanged(nameof(LogText));
    }

    private string Text(string key, params object?[] arguments) => _language.Format(key, arguments);

    private string Describe(AddPartStatus status) => Text(status switch
    {
        AddPartStatus.InvalidPath => LanguageKeys.AddPartInvalidPath,
        AddPartStatus.FileNotFound => LanguageKeys.AddPartMissing,
        AddPartStatus.NotCastFile => LanguageKeys.AddPartNotCast,
        AddPartStatus.Duplicate => LanguageKeys.AddPartDuplicate,
        AddPartStatus.CollectionFull => LanguageKeys.AddPartFull,
        _ => LanguageKeys.AddPartSucceeded
    });

    private string Describe(MergeValidationError error) => DescribeText(error).Render(_language);

    private LocalizedText DescribeText(MergeValidationError error) => error.Code switch
    {
        MergeValidationErrorCode.InvalidPartCount => LocalizedText.FromKey(LanguageKeys.ValidationInvalidPartCount),
        MergeValidationErrorCode.InvalidPath => LocalizedText.FromKey(LanguageKeys.ValidationInvalidPath, error.FilePath ?? string.Empty),
        MergeValidationErrorCode.MissingFile => LocalizedText.FromKey(LanguageKeys.ValidationMissingFile, error.FilePath ?? string.Empty),
        MergeValidationErrorCode.UnsupportedExtension => LocalizedText.FromKey(LanguageKeys.ValidationUnsupportedExtension, error.FilePath ?? string.Empty),
        MergeValidationErrorCode.DuplicateFile => LocalizedText.FromKey(LanguageKeys.ValidationDuplicateFile, error.FilePath ?? string.Empty),
        MergeValidationErrorCode.InvalidOutputDirectory => LocalizedText.FromKey(LanguageKeys.ValidationInvalidOutputDirectory),
        MergeValidationErrorCode.InvalidOutputFileName => LocalizedText.FromKey(LanguageKeys.ValidationInvalidOutputFileName),
        MergeValidationErrorCode.OutputAlreadyExists => LocalizedText.FromKey(LanguageKeys.ValidationOutputAlreadyExists, error.FilePath ?? string.Empty),
        MergeValidationErrorCode.ManualRootNotSelected => LocalizedText.FromKey(LanguageKeys.ValidationManualRootNotSelected),
        _ => LocalizedText.FromLiteral(error.Message)
    };

    private string Describe(MergeWarning warning) => Text(
        warning.Code == MergeWarningCode.NoAttachmentBone
            ? LanguageKeys.WarningNoAttachmentBone
            : LanguageKeys.WarningUnconnectedHierarchy,
        warning.ModelName,
        warning.RootModelName);

    private static LocalizedText DescribeText(MergeProgress progress) => progress.Code switch
    {
        MergeProgressCode.ValidatingRequest => LocalizedText.FromKey(LanguageKeys.ProgressValidating),
        MergeProgressCode.LoadingFile => LocalizedText.FromKey(LanguageKeys.ProgressLoading, progress.Subject ?? string.Empty),
        MergeProgressCode.SelectingRootModel => LocalizedText.FromKey(LanguageKeys.ProgressSelectingRoot),
        MergeProgressCode.MergingModel => LocalizedText.FromKey(LanguageKeys.ProgressMerging, progress.Subject ?? string.Empty),
        MergeProgressCode.SavingFile => LocalizedText.FromKey(LanguageKeys.ProgressSaving, progress.Subject ?? string.Empty),
        MergeProgressCode.VerifyingCast => LocalizedText.FromKey(LanguageKeys.ProgressVerifying),
        MergeProgressCode.SavedFile => LocalizedText.FromKey(LanguageKeys.ProgressCompleted, progress.Subject ?? string.Empty),
        _ => LocalizedText.FromKey(LanguageKeys.ProgressGeneric)
    };

    private void Language_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName is not nameof(ILanguageCatalog.Language) and not "Item[]")
        {
            return;
        }

        RaisePropertyChanged(nameof(Name));
        RaisePropertyChanged(nameof(SummaryText));
        RaisePropertyChanged(nameof(StatusMessage));
        RaisePropertyChanged(nameof(LogText));
        foreach (var slot in Slots)
        {
            slot.RefreshLanguage();
        }
    }

    private sealed record LogEntry(DateTime Timestamp, LocalizedText Message);
}

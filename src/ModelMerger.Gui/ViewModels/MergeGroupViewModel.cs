using ModelMerger.Core.Merging;
using ModelMerger.Core.Planning;
using ModelMerger.Core.Selection;
using ModelMerger.Gui.Commands;
using ModelMerger.Gui.Services;
using System.Collections.ObjectModel;
using System.Diagnostics;
using System.Text;
using System.Windows.Input;

namespace ModelMerger.Gui.ViewModels;

internal sealed class MergeGroupViewModel : ViewModelBase
{
    private readonly IMergeTaskScheduler _scheduler;
    private readonly IUserDialogService _dialogs;
    private readonly MergeGroupPlan _plan;
    private readonly StringBuilder _log = new();
    private readonly RelayCommand _addNextCommand;
    private readonly RelayCommand _addOrReplaceCommand;
    private readonly RelayCommand _removePartCommand;
    private readonly RelayCommand _clearCommand;
    private readonly RelayCommand _browseOutputCommand;
    private readonly RelayCommand _setManualRootCommand;
    private readonly RelayCommand _cancelCommand;
    private readonly RelayCommand _openOutputCommand;
    private readonly AsyncRelayCommand _mergeCommand;
    private MergeTaskHandle? _mergeTask;
    private string _statusMessage = "添加 2 至 16 个 Cast 部件";
    private string _logText = string.Empty;
    private string? _lastOutputPath;
    private bool _isBusy;
    private bool _isExpanded = true;
    private double _progressValue;

    public MergeGroupViewModel(
        int number,
        IMergeTaskScheduler scheduler,
        IUserDialogService dialogs,
        string? preferredOutputDirectory,
        RootSelectionMode defaultRootMode)
    {
        Number = number;
        _scheduler = scheduler;
        _dialogs = dialogs;
        _plan = new MergeGroupPlan(preferredOutputDirectory, defaultRootMode);
        Slots = new ObservableCollection<PartSlotViewModel>(
            Enumerable.Range(0, ModelPartCollection.MaximumParts).Select(index => new PartSlotViewModel(index)));

        _addNextCommand = new RelayCommand(_ => AddNextPart(), _ => !IsBusy && PartCount < ModelPartCollection.MaximumParts);
        _addOrReplaceCommand = new RelayCommand(AddOrReplacePart, parameter => !IsBusy && parameter is PartSlotViewModel);
        _removePartCommand = new RelayCommand(RemovePart, parameter => !IsBusy && parameter is PartSlotViewModel slot && slot.IsOccupied);
        _clearCommand = new RelayCommand(_ => ClearParts(), _ => !IsBusy && PartCount > 0);
        _browseOutputCommand = new RelayCommand(_ => BrowseOutputDirectory(), _ => !IsBusy);
        _setManualRootCommand = new RelayCommand(SetManualRoot, parameter => !IsBusy && parameter is PartSlotViewModel slot && slot.IsOccupied);
        _cancelCommand = new RelayCommand(_ => Cancel(), _ => IsBusy);
        _openOutputCommand = new RelayCommand(_ => OpenLastOutput(), _ => LastOutputPath is not null && File.Exists(LastOutputPath));
        _mergeCommand = new AsyncRelayCommand(MergeAsync, () => CanMerge);
    }

    public event EventHandler? StateChanged;

    public event EventHandler<string>? OutputDirectoryChosen;

    public int Number { get; }

    public string Name => $"模型组 {Number}";

    public ObservableCollection<PartSlotViewModel> Slots { get; }

    public ICommand AddNextCommand => _addNextCommand;

    public ICommand AddOrReplaceCommand => _addOrReplaceCommand;

    public ICommand RemovePartCommand => _removePartCommand;

    public ICommand ClearCommand => _clearCommand;

    public ICommand BrowseOutputCommand => _browseOutputCommand;

    public ICommand SetManualRootCommand => _setManualRootCommand;

    public ICommand CancelCommand => _cancelCommand;

    public ICommand OpenOutputCommand => _openOutputCommand;

    public ICommand MergeCommand => _mergeCommand;

    public int PartCount => _plan.State.PartCount;

    public string PartCountText => $"{PartCount} / {ModelPartCollection.MaximumParts}";

    public string SummaryText => IsBusy
        ? $"{PartCountText} · 处理中"
        : CanMerge
            ? $"{PartCountText} · 已就绪"
            : $"{PartCountText} · 至少需要 2 个部件";

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

    public string StatusMessage
    {
        get => _statusMessage;
        private set => SetProperty(ref _statusMessage, value);
    }

    public string LogText
    {
        get => _logText;
        private set => SetProperty(ref _logText, value);
    }

    public string? LastOutputPath
    {
        get => _lastOutputPath;
        private set
        {
            if (SetProperty(ref _lastOutputPath, value))
            {
                _openOutputCommand.RaiseCanExecuteChanged();
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
                rejected.Add($"{Path.GetFileName(file)}：{Describe(result.Status)}");
            }
        }

        RefreshSlots();
        IsExpanded = true;
        if (rejected.Count > 0)
        {
            _dialogs.ShowInformation(
                $"{Name}：部分文件未添加",
                $"已添加 {added} 个部件。\n\n{string.Join(Environment.NewLine, rejected)}");
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

        var request = BuildRequest(overwrite.Value);

        IsBusy = true;
        IsExpanded = true;
        ProgressValue = 0;
        LastOutputPath = null;
        StatusMessage = "等待可用的并发处理位置";
        AppendLog($"{Name} 已加入队列，共 {PartCount} 个部件");
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
                if (!_dialogs.Confirm("覆盖输出文件", $"{Name} 的文件已存在：\n{output}\n\n是否覆盖？"))
                {
                    StatusMessage = "已取消覆盖";
                    return;
                }

                _mergeTask = _scheduler.Schedule(request with { Overwrite = true }, progress);
                result = await _mergeTask.Completion;
            }

            LastOutputPath = result.OutputPath;
            ProgressValue = 100;
            StatusMessage = $"完成：{Path.GetFileName(result.OutputPath)}";
            AppendLog($"合并完成：{result.BoneCount} 根骨骼，{result.MeshCount} 个网格");
            var warningText = result.Warnings.Count > 0
                ? $"\n\n警告：\n{string.Join(Environment.NewLine, result.Warnings)}"
                : string.Empty;
            _dialogs.ShowInformation(
                $"{Name} 合并完成",
                $"已保存到：\n{result.OutputPath}\n\n骨骼：{result.BoneCount}    网格：{result.MeshCount}{warningText}");
        }
        catch (OperationCanceledException)
        {
            StatusMessage = "已取消，临时文件已清理";
            AppendLog("用户取消了该组任务");
        }
        catch (MergeValidationException exception)
        {
            var message = string.Join(Environment.NewLine, exception.Errors.Select(error => error.Message));
            StatusMessage = "合并请求无效";
            AppendLog(message);
            _dialogs.ShowError($"{Name} 无法开始合并", message);
        }
        catch (MergeOutputConflictException exception)
        {
            StatusMessage = "输出路径正被其他模型组占用";
            AppendLog(exception.Message);
            _dialogs.ShowError($"{Name} 输出冲突", $"另一个模型组正在写入：\n{exception.OutputPath}\n\n请等待其完成，或选择不同的输出文件名。");
        }
        catch (Exception exception)
        {
            StatusMessage = "合并失败";
            AppendLog(exception.ToString());
            _dialogs.ShowError($"{Name} 合并失败", exception.Message);
        }
        finally
        {
            _mergeTask = null;
            IsBusy = false;
        }
    }

    public void Cancel() => _mergeTask?.Cancel();

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
            _dialogs.ShowInformation("无法添加部件", Describe(result.Status));
            return;
        }

        RefreshSlots();
        StatusMessage = $"已添加 {PartCount} 个部件";
    }

    private void RemovePart(object? parameter)
    {
        if (parameter is PartSlotViewModel slot && _plan.RemovePart(slot.Index))
        {
            RefreshSlots();
            StatusMessage = $"已移除部件，当前 {PartCountText}";
        }
    }

    private void ClearParts()
    {
        _plan.ClearParts();
        LastOutputPath = null;
        RefreshSlots();
        StatusMessage = "部件列表已清空";
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
            StatusMessage = $"已将 {slot.FileName} 设为根模型";
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

    private MergeRequest BuildRequest(bool overwrite)
    {
        return _plan.CreateRequest(overwrite);
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

        return _dialogs.Confirm("覆盖输出文件", $"{Name} 的文件已存在：\n{outputPath}\n\n是否覆盖？") ? true : null;
    }

    private void OnProgress(MergeProgress progress)
    {
        ProgressValue = progress.Percentage;
        StatusMessage = $"{Describe(progress.Stage)}：{progress.Message}";
        AppendLog(StatusMessage);
    }

    private void RefreshSlots()
    {
        var state = _plan.State;
        for (var index = 0; index < Slots.Count; index++)
        {
            Slots[index].FilePath = index < state.PartCount ? state.PartFiles[index] : null;
        }

        RefreshRootMarkers();
        RaisePropertyChanged(nameof(PartCount));
        RaisePropertyChanged(nameof(PartCountText));
        RaisePropertyChanged(nameof(OutputDirectory));
        RaisePropertyChanged(nameof(RootSelectionMode));
        RaisePropertyChanged(nameof(IsAutomaticRoot));
        RaisePropertyChanged(nameof(IsManualRootMode));
        OnMergeStateChanged();
    }

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
    }


    private void AppendLog(string message)
    {
        _log.Append('[').Append(DateTime.Now.ToString("HH:mm:ss")).Append("] ").AppendLine(message);
        LogText = _log.ToString();
    }

    private static string Describe(AddPartStatus status) => status switch
    {
        AddPartStatus.InvalidPath => "文件路径无效",
        AddPartStatus.FileNotFound => "文件不存在",
        AddPartStatus.NotCastFile => "仅支持 .cast 文件",
        AddPartStatus.Duplicate => "该部件已添加",
        AddPartStatus.CollectionFull => "该组已达到 16 个部件上限",
        _ => "已添加"
    };

    private static string Describe(MergeStage stage) => stage switch
    {
        MergeStage.Validating => "正在检查",
        MergeStage.Loading => "正在读取",
        MergeStage.SelectingRoot => "正在识别根模型",
        MergeStage.Merging => "正在合并",
        MergeStage.Saving => "正在保存",
        MergeStage.Verifying => "正在验证",
        MergeStage.Completed => "已完成",
        _ => "处理中"
    };
}

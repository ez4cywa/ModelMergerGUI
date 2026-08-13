using ModelMerger.Core.Merging;
using ModelMerger.Core.Settings;
using ModelMerger.Gui.Commands;
using ModelMerger.Gui.Services;
using System.Collections.ObjectModel;
using System.Windows.Input;

namespace ModelMerger.Gui.ViewModels;

internal sealed class MainWindowViewModel : ViewModelBase, IDisposable
{
    private const int MaximumConcurrentGroups = 2;

    private readonly ISettingsStore _settingsStore;
    private readonly IUserDialogService _dialogs;
    private readonly MergeExecutionQueue _executionQueue;
    private readonly RelayCommand _addGroupCommand;
    private readonly RelayCommand _removeGroupCommand;
    private readonly RelayCommand _cancelAllCommand;
    private readonly AsyncRelayCommand _mergeAllCommand;
    private readonly AsyncRelayCommand _restoreDefaultsCommand;
    private int _nextGroupNumber = 1;
    private string? _preferredOutputDirectory;
    private bool _rememberOutputDirectory;
    private bool _isMergingAll;
    private string _workspaceStatus = "创建模型组并添加部件";
    private RootSelectionMode _defaultRootMode = RootSelectionMode.Automatic;

    public MainWindowViewModel(ISettingsStore settingsStore, IUserDialogService dialogs)
    {
        _settingsStore = settingsStore;
        _dialogs = dialogs;
        _executionQueue = new MergeExecutionQueue(new ModelMergeService(), MaximumConcurrentGroups);
        Groups = [];
        _addGroupCommand = new RelayCommand(_ => AddGroup());
        _removeGroupCommand = new RelayCommand(RemoveGroup, parameter =>
            Groups.Count > 1 && parameter is MergeGroupViewModel { IsBusy: false });
        _cancelAllCommand = new RelayCommand(_ => CancelAll(), _ => IsAnyBusy);
        _mergeAllCommand = new AsyncRelayCommand(MergeAllAsync, () => ReadyGroupCount > 0 && !IsMergingAll);
        _restoreDefaultsCommand = new AsyncRelayCommand(RestoreDefaultsAsync, () => !IsAnyBusy);
    }

    public event EventHandler? DefaultsRestored;

    public ObservableCollection<MergeGroupViewModel> Groups { get; }

    public ICommand AddGroupCommand => _addGroupCommand;

    public ICommand RemoveGroupCommand => _removeGroupCommand;

    public ICommand CancelAllCommand => _cancelAllCommand;

    public ICommand MergeAllCommand => _mergeAllCommand;

    public ICommand RestoreDefaultsCommand => _restoreDefaultsCommand;

    public int GroupCount => Groups.Count;

    public int ReadyGroupCount => Groups.Count(group => group.CanMerge);

    public int RunningGroupCount => Groups.Count(group => group.IsBusy);

    public bool IsAnyBusy => RunningGroupCount > 0;

    public bool IsMergingAll
    {
        get => _isMergingAll;
        private set
        {
            if (SetProperty(ref _isMergingAll, value))
            {
                RefreshWorkspaceState();
            }
        }
    }

    public bool RememberOutputDirectory
    {
        get => _rememberOutputDirectory;
        set => SetProperty(ref _rememberOutputDirectory, value);
    }

    public string WorkspaceStatus
    {
        get => _workspaceStatus;
        private set => SetProperty(ref _workspaceStatus, value);
    }

    public string ConcurrencyText => $"最多 {MaximumConcurrentGroups} 组并行";

    public WindowBounds? SavedWindowBounds { get; private set; }

    public async Task InitializeAsync()
    {
        var settings = await _settingsStore.LoadAsync();
        RememberOutputDirectory = settings.RememberOutputDirectory;
        _preferredOutputDirectory = settings.RememberOutputDirectory && Directory.Exists(settings.PreferredOutputDirectory)
            ? settings.PreferredOutputDirectory
            : null;
        _defaultRootMode = settings.RootSelectionMode;
        SavedWindowBounds = settings.WindowBounds;
        AddGroup();
        RefreshWorkspaceState();
    }

    public async Task SaveSettingsAsync(WindowBounds? windowBounds, bool showConfirmation = true)
    {
        try
        {
            var rootMode = Groups.FirstOrDefault()?.RootSelectionMode ?? _defaultRootMode;
            var settings = new AppSettings
            {
                PreferredOutputDirectory = RememberOutputDirectory ? _preferredOutputDirectory : null,
                RememberOutputDirectory = RememberOutputDirectory,
                RootSelectionMode = rootMode,
                WindowBounds = windowBounds
            };
            await _settingsStore.SaveAsync(settings);
            SavedWindowBounds = windowBounds;
            _defaultRootMode = rootMode;
            if (showConfirmation)
            {
                WorkspaceStatus = "设置已保存；部件文件路径不会被记录";
                _dialogs.ShowInformation("保存设置", "已保存输出目录、首组根模型模式和窗口位置。\n不会保存各组的模型文件路径。");
            }
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            if (showConfirmation)
            {
                _dialogs.ShowError("无法保存设置", exception.Message);
            }
        }
    }

    public void Dispose() => _executionQueue.Dispose();

    private void AddGroup()
    {
        var group = new MergeGroupViewModel(
            _nextGroupNumber++,
            _executionQueue,
            _dialogs,
            _preferredOutputDirectory,
            _defaultRootMode);
        group.StateChanged += Group_StateChanged;
        group.OutputDirectoryChosen += Group_OutputDirectoryChosen;
        Groups.Add(group);
        RaisePropertyChanged(nameof(GroupCount));
        RefreshWorkspaceState();
    }

    private void RemoveGroup(object? parameter)
    {
        if (parameter is not MergeGroupViewModel group || Groups.Count <= 1 || group.IsBusy)
        {
            return;
        }

        group.StateChanged -= Group_StateChanged;
        group.OutputDirectoryChosen -= Group_OutputDirectoryChosen;
        Groups.Remove(group);
        RaisePropertyChanged(nameof(GroupCount));
        RefreshWorkspaceState();
    }

    private async Task MergeAllAsync()
    {
        var readyGroups = Groups.Where(group => group.CanMerge).ToArray();
        if (readyGroups.Length == 0)
        {
            return;
        }

        IsMergingAll = true;
        WorkspaceStatus = $"已启动 {readyGroups.Length} 个组；{ConcurrencyText}";
        try
        {
            await Task.WhenAll(readyGroups.Select(group => group.MergeAsync()));
        }
        finally
        {
            IsMergingAll = false;
            WorkspaceStatus = $"批量处理结束，共检查 {readyGroups.Length} 个组";
        }
    }

    private void CancelAll()
    {
        foreach (var group in Groups.Where(group => group.IsBusy))
        {
            group.Cancel();
        }

        WorkspaceStatus = "正在取消所有运行和等待中的组";
    }

    private async Task RestoreDefaultsAsync()
    {
        var defaults = new AppSettings();
        await _settingsStore.SaveAsync(defaults);
        RememberOutputDirectory = false;
        _preferredOutputDirectory = null;
        _defaultRootMode = RootSelectionMode.Automatic;
        foreach (var group in Groups)
        {
            group.ResetPreferences(null, RootSelectionMode.Automatic);
        }

        SavedWindowBounds = null;
        WorkspaceStatus = "已恢复默认设置；现有模型组和部件未被清除";
        DefaultsRestored?.Invoke(this, EventArgs.Empty);
    }

    private void Group_StateChanged(object? sender, EventArgs e) => RefreshWorkspaceState();

    private void Group_OutputDirectoryChosen(object? sender, string outputDirectory)
    {
        _preferredOutputDirectory = outputDirectory;
    }

    private void RefreshWorkspaceState()
    {
        RaisePropertyChanged(nameof(ReadyGroupCount));
        RaisePropertyChanged(nameof(RunningGroupCount));
        RaisePropertyChanged(nameof(IsAnyBusy));
        if (!IsMergingAll)
        {
            WorkspaceStatus = IsAnyBusy
                ? $"正在处理 {RunningGroupCount} 个组；{ConcurrencyText}"
                : $"共 {GroupCount} 个组，{ReadyGroupCount} 个已就绪";
        }

        _addGroupCommand.RaiseCanExecuteChanged();
        _removeGroupCommand.RaiseCanExecuteChanged();
        _cancelAllCommand.RaiseCanExecuteChanged();
        _mergeAllCommand.RaiseCanExecuteChanged();
        _restoreDefaultsCommand.RaiseCanExecuteChanged();
    }
}

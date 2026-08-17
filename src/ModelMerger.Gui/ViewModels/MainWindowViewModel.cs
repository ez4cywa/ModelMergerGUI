using ModelMerger.Core.Merging;
using ModelMerger.Core.Settings;
using ModelMerger.Gui.Commands;
using ModelMerger.Gui.Localization;
using ModelMerger.Gui.Services;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.Globalization;
using System.Windows.Input;

namespace ModelMerger.Gui.ViewModels;

internal sealed class MainWindowViewModel : ViewModelBase, IDisposable
{
    private readonly ISettingsStore _settingsStore;
    private readonly IUserDialogService _dialogs;
    private readonly IMergeTaskScheduler _scheduler;
    private readonly ILanguageCatalog _language;
    private readonly IModelPreviewDialogService _previewDialogs;
    private readonly RelayCommand _addGroupCommand;
    private readonly RelayCommand _removeGroupCommand;
    private readonly RelayCommand _cancelAllCommand;
    private readonly AsyncRelayCommand _mergeAllCommand;
    private readonly AsyncRelayCommand _restoreDefaultsCommand;
    private int _nextGroupNumber = 1;
    private string? _preferredOutputDirectory;
    private bool _rememberOutputDirectory;
    private bool _isMergingAll;
    private LocalizedText _workspaceStatus = LocalizedText.FromKey(LanguageKeys.WorkspaceInitial);
    private RootSelectionMode _defaultRootMode = RootSelectionMode.Automatic;

    public MainWindowViewModel(
        ISettingsStore settingsStore,
        IUserDialogService dialogs,
        IMergeTaskScheduler scheduler,
        ILanguageCatalog? languageCatalog = null,
        IModelPreviewDialogService? previewDialogs = null)
    {
        _settingsStore = settingsStore;
        _dialogs = dialogs;
        _scheduler = scheduler;
        _language = languageCatalog ?? LanguageCatalog.Current;
        _previewDialogs = previewDialogs ?? new ModelPreviewDialogService(_language);
        _language.PropertyChanged += Language_PropertyChanged;
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

    public IReadOnlyList<LanguageOption> LanguageOptions => _language.AvailableLanguages;

    public ICommand AddGroupCommand => _addGroupCommand;

    public ICommand RemoveGroupCommand => _removeGroupCommand;

    public ICommand CancelAllCommand => _cancelAllCommand;

    public ICommand MergeAllCommand => _mergeAllCommand;

    public ICommand RestoreDefaultsCommand => _restoreDefaultsCommand;

    public string AppTitle => Text(LanguageKeys.AppTitle);

    public string AppSubtitle => Text(LanguageKeys.AppSubtitle);

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

    public AppLanguage SelectedLanguage
    {
        get => _language.Language;
        set => _language.SetLanguage(value);
    }

    public string WorkspaceStatus => _workspaceStatus.Render(_language);

    public string ConcurrencyText => Text(LanguageKeys.Concurrency, _scheduler.MaximumConcurrency);

    public WindowBounds? SavedWindowBounds { get; private set; }

    public async Task InitializeAsync()
    {
        var settings = await _settingsStore.LoadAsync();
        _language.SetLanguage(settings.UiLanguage ?? _language.Language);
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
                UiLanguage = SelectedLanguage,
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
                SetWorkspaceStatus(LanguageKeys.SettingsSavedStatus);
                _dialogs.ShowInformation(
                    Text(LanguageKeys.SettingsSavedTitle),
                    Text(LanguageKeys.SettingsSavedMessage));
            }
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            if (showConfirmation)
            {
                _dialogs.ShowError(Text(LanguageKeys.SettingsSaveFailedTitle), exception.Message);
            }
        }
    }

    public void RefreshFileValidity()
    {
        foreach (var group in Groups)
        {
            group.RefreshFileValidity();
        }
    }

    public void Dispose()
    {
        _language.PropertyChanged -= Language_PropertyChanged;
        foreach (var group in Groups)
        {
            group.Dispose();
        }

        _scheduler.Dispose();
    }

    private void AddGroup()
    {
        var group = new MergeGroupViewModel(
            _nextGroupNumber++,
            _scheduler,
            _dialogs,
            _preferredOutputDirectory,
            _defaultRootMode,
            _language,
            _previewDialogs);
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
        group.Dispose();
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
        SetWorkspaceStatus(LanguageKeys.BatchStarted, readyGroups.Length, ConcurrencyText);
        try
        {
            await Task.WhenAll(readyGroups.Select(group => group.MergeAsync()));
        }
        finally
        {
            IsMergingAll = false;
            SetWorkspaceStatus(LanguageKeys.BatchFinished, readyGroups.Length);
        }
    }

    private void CancelAll()
    {
        foreach (var group in Groups.Where(group => group.IsBusy))
        {
            group.Cancel();
        }

        SetWorkspaceStatus(LanguageKeys.CancelAllStatus);
    }

    private async Task RestoreDefaultsAsync()
    {
        try
        {
            await _settingsStore.SaveAsync(new AppSettings());
        }
        catch (Exception exception) when (exception is IOException or UnauthorizedAccessException)
        {
            _dialogs.ShowError(Text(LanguageKeys.RestoreFailedTitle), exception.Message);
            return;
        }

        RememberOutputDirectory = false;
        _preferredOutputDirectory = null;
        _defaultRootMode = RootSelectionMode.Automatic;
        _language.SetLanguage(LanguageCatalog.ResolveInitialLanguage(CultureInfo.CurrentUICulture));
        foreach (var group in Groups)
        {
            group.ResetPreferences(null, RootSelectionMode.Automatic);
        }

        SavedWindowBounds = null;
        SetWorkspaceStatus(LanguageKeys.RestoreDoneStatus);
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
            if (IsAnyBusy)
            {
                SetWorkspaceStatus(LanguageKeys.WorkspaceProcessing, RunningGroupCount, ConcurrencyText);
            }
            else
            {
                SetWorkspaceStatus(LanguageKeys.WorkspaceSummary, GroupCount, ReadyGroupCount);
            }
        }

        _addGroupCommand.RaiseCanExecuteChanged();
        _removeGroupCommand.RaiseCanExecuteChanged();
        _cancelAllCommand.RaiseCanExecuteChanged();
        _mergeAllCommand.RaiseCanExecuteChanged();
        _restoreDefaultsCommand.RaiseCanExecuteChanged();
    }

    private void SetWorkspaceStatus(string key, params object?[] arguments)
    {
        _workspaceStatus = LocalizedText.FromKey(key, arguments);
        RaisePropertyChanged(nameof(WorkspaceStatus));
    }

    private string Text(string key, params object?[] arguments) => _language.Format(key, arguments);

    private void Language_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName is not nameof(ILanguageCatalog.Language) and not "Item[]")
        {
            return;
        }

        RaisePropertyChanged(nameof(SelectedLanguage));
        RaisePropertyChanged(nameof(AppTitle));
        RaisePropertyChanged(nameof(AppSubtitle));
        RaisePropertyChanged(nameof(WorkspaceStatus));
        RaisePropertyChanged(nameof(ConcurrencyText));
    }

}

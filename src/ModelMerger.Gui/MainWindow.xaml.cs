using ModelMerger.Core.Merging;
using ModelMerger.Core.Settings;
using ModelMerger.Gui.Localization;
using ModelMerger.Gui.Services;
using ModelMerger.Gui.ViewModels;
using System.ComponentModel;
using System.Windows;
using System.Windows.Threading;

namespace ModelMerger.Gui;

public partial class MainWindow : Window
{
    private readonly ILanguageCatalog _language = LanguageCatalog.Current;
    private readonly DispatcherTimer _fileValidityTimer = new() { Interval = TimeSpan.FromSeconds(2) };
    private readonly MainWindowViewModel _viewModel;
    private bool _initialized;
    private bool _closingAfterSave;
    private bool _savingBeforeClose;

    public MainWindow()
        : this(JsonSettingsStore.CreateDefault())
    {
    }

    internal MainWindow(ISettingsStore settingsStore)
    {
        InitializeComponent();
        _viewModel = new MainWindowViewModel(
            settingsStore,
            new UserDialogService(_language),
            new MergeTaskScheduler(maximumConcurrency: 2),
            _language);
        _viewModel.DefaultsRestored += (_, _) => ResetWindowBounds();
        _fileValidityTimer.Tick += (_, _) => _viewModel.RefreshFileValidity();
        DataContext = _viewModel;
    }

    private async void Window_Loaded(object sender, RoutedEventArgs e)
    {
        if (_initialized)
        {
            return;
        }

        _initialized = true;
        await _viewModel.InitializeAsync();
        ApplySavedWindowBounds(_viewModel.SavedWindowBounds);
        _fileValidityTimer.Start();
    }

    private void Group_DragOver(object sender, DragEventArgs e)
    {
        var group = (sender as FrameworkElement)?.DataContext as MergeGroupViewModel;
        e.Effects = group is null || group.IsBusy || !e.Data.GetDataPresent(DataFormats.FileDrop)
            ? DragDropEffects.None
            : DragDropEffects.Copy;
        e.Handled = true;
    }

    private void Group_Drop(object sender, DragEventArgs e)
    {
        if ((sender as FrameworkElement)?.DataContext is MergeGroupViewModel group &&
            e.Data.GetData(DataFormats.FileDrop) is string[] files)
        {
            group.AddDroppedFiles(files);
        }

        e.Handled = true;
    }

    private async void SaveSettings_Click(object sender, RoutedEventArgs e)
    {
        await _viewModel.SaveSettingsAsync(GetWindowBounds());
    }

    private async void Window_Closing(object? sender, CancelEventArgs e)
    {
        if (_closingAfterSave)
        {
            _fileValidityTimer.Stop();
            _viewModel.Dispose();
            return;
        }

        if (_viewModel.IsAnyBusy)
        {
            e.Cancel = true;
            MessageBox.Show(
                _language[LanguageKeys.CloseBusyMessage],
                _language[LanguageKeys.CloseBusyTitle],
                MessageBoxButton.OK,
                MessageBoxImage.Information);
            return;
        }

        e.Cancel = true;
        if (_savingBeforeClose)
        {
            return;
        }

        _savingBeforeClose = true;
        IsEnabled = false;
        await _viewModel.SaveSettingsAsync(GetWindowBounds(), showConfirmation: false);
        if (_viewModel.IsAnyBusy)
        {
            IsEnabled = true;
            _savingBeforeClose = false;
            MessageBox.Show(
                _language[LanguageKeys.CloseRaceMessage],
                _language[LanguageKeys.CloseBusyTitle],
                MessageBoxButton.OK,
                MessageBoxImage.Information);
            return;
        }

        _closingAfterSave = true;
        _ = Dispatcher.BeginInvoke(DispatcherPriority.Normal, new Action(Close));
    }

    private WindowBounds GetWindowBounds()
    {
        var restore = RestoreBounds;
        return new WindowBounds(restore.Left, restore.Top, restore.Width, restore.Height);
    }

    private void ApplySavedWindowBounds(WindowBounds? bounds)
    {
        if (bounds is null)
        {
            return;
        }

        Left = bounds.Left;
        Top = bounds.Top;
        Width = bounds.Width;
        Height = bounds.Height;
        EnsureWindowIsVisible();
    }

    private void EnsureWindowIsVisible()
    {
        var workArea = SystemParameters.WorkArea;
        Width = Math.Min(Width, workArea.Width);
        Height = Math.Min(Height, workArea.Height);
        Left = Math.Clamp(Left, workArea.Left, Math.Max(workArea.Left, workArea.Right - Width));
        Top = Math.Clamp(Top, workArea.Top, Math.Max(workArea.Top, workArea.Bottom - Height));
    }

    private void ResetWindowBounds()
    {
        Width = 1280;
        Height = 900;
        WindowStartupLocation = WindowStartupLocation.CenterScreen;
    }
}

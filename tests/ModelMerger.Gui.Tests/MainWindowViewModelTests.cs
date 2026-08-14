using ModelMerger.Core.Merging;
using ModelMerger.Core.Settings;
using ModelMerger.Gui.Localization;
using ModelMerger.Gui.Services;
using ModelMerger.Gui.ViewModels;
using Xunit;

namespace ModelMerger.Gui.Tests;

public sealed class MainWindowViewModelTests
{
    [Fact]
    public async Task InitializeAndSave_UsesAndPersistsSelectedLanguage()
    {
        var store = new MemorySettingsStore(new AppSettings { UiLanguage = AppLanguage.English });
        var catalog = new LanguageCatalog(AppLanguage.ChineseSimplified);
        using var viewModel = new MainWindowViewModel(
            store,
            new SilentDialogs(),
            new UnusedTaskScheduler(),
            catalog);

        await viewModel.InitializeAsync();

        Assert.Equal(AppLanguage.English, viewModel.SelectedLanguage);
        Assert.Equal("Cast Model Merger", viewModel.AppTitle);
        viewModel.SelectedLanguage = AppLanguage.ChineseSimplified;
        Assert.Equal("Cast 模型合并器", viewModel.AppTitle);
        await viewModel.SaveSettingsAsync(windowBounds: null, showConfirmation: false);
        Assert.Equal(AppLanguage.ChineseSimplified, store.Saved!.UiLanguage);
    }

    private sealed class MemorySettingsStore(AppSettings settings) : ISettingsStore
    {
        public AppSettings? Saved { get; private set; }

        public Task<AppSettings> LoadAsync(CancellationToken cancellationToken = default) =>
            Task.FromResult(settings);

        public Task SaveAsync(AppSettings value, CancellationToken cancellationToken = default)
        {
            Saved = value;
            return Task.CompletedTask;
        }
    }

    private sealed class SilentDialogs : IUserDialogService
    {
        public string? PickCastFile(string? initialDirectory = null) => null;

        public string? PickOutputFolder(string? initialDirectory = null) => null;

        public bool Confirm(string title, string message) => false;

        public void ShowInformation(string title, string message)
        {
        }

        public void ShowError(string title, string message)
        {
        }
    }

    private sealed class UnusedTaskScheduler : IMergeTaskScheduler
    {
        public int MaximumConcurrency => 2;

        public MergeTaskHandle Schedule(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null) => throw new NotSupportedException();

        public void Dispose()
        {
        }
    }
}

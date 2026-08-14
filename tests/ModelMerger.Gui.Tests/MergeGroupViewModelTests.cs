using ModelMerger.Core.Merging;
using ModelMerger.Core.Settings;
using ModelMerger.Gui.Localization;
using ModelMerger.Gui.Services;
using ModelMerger.Gui.ViewModels;
using System.IO;
using Xunit;

namespace ModelMerger.Gui.Tests;

public sealed class MergeGroupViewModelTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"ModelMergerGuiTests-{Guid.NewGuid():N}");

    public MergeGroupViewModelTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public void AddNext_AfterFirstPart_StartsInFirstPartsDirectory()
    {
        var first = CreateCastFile("first.cast");
        var second = CreateCastFile("second.cast");
        var dialogs = new RecordingDialogService(first, second);
        var viewModel = new MergeGroupViewModel(
            1,
            new UnusedTaskScheduler(),
            dialogs,
            preferredOutputDirectory: null,
            RootSelectionMode.Automatic);

        viewModel.AddNextCommand.Execute(null);
        viewModel.AddNextCommand.Execute(null);

        Assert.Null(dialogs.InitialDirectories[0]);
        Assert.Equal(_directory, dialogs.InitialDirectories[1]);
        Assert.Equal(2, viewModel.PartCount);
    }

    [Fact]
    public void LanguageChange_ReRendersExistingGroupStatus()
    {
        var catalog = new LanguageCatalog(AppLanguage.ChineseSimplified);
        var viewModel = new MergeGroupViewModel(
            1,
            new UnusedTaskScheduler(),
            new RecordingDialogService(),
            preferredOutputDirectory: null,
            RootSelectionMode.Automatic,
            catalog);

        Assert.Equal("添加 2 至 16 个 Cast 部件", viewModel.StatusMessage);

        catalog.SetLanguage(AppLanguage.English);

        Assert.Equal("Add 2 to 16 Cast parts", viewModel.StatusMessage);
    }

    [Fact]
    public async Task LanguageChange_ReRendersExistingRunLog()
    {
        var first = CreateCastFile("log-first.cast");
        var second = CreateCastFile("log-second.cast");
        var catalog = new LanguageCatalog(AppLanguage.ChineseSimplified);
        using var scheduler = new MergeTaskScheduler(new CompletedMergeService(), 1);
        using var viewModel = new MergeGroupViewModel(
            1,
            scheduler,
            new RecordingDialogService(first, second),
            preferredOutputDirectory: null,
            RootSelectionMode.Automatic,
            catalog);
        viewModel.AddNextCommand.Execute(null);
        viewModel.AddNextCommand.Execute(null);

        await viewModel.MergeAsync();

        Assert.Contains("已加入队列", viewModel.LogText);
        catalog.SetLanguage(AppLanguage.English);
        Assert.DoesNotContain("已加入队列", viewModel.LogText);
        Assert.Contains("was queued", viewModel.LogText);
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }

    private string CreateCastFile(string name)
    {
        var path = Path.Combine(_directory, name);
        File.WriteAllText(path, string.Empty);
        return path;
    }

    private sealed class RecordingDialogService(params string[] selectedFiles) : IUserDialogService
    {
        private readonly Queue<string> _selectedFiles = new(selectedFiles);

        public List<string?> InitialDirectories { get; } = [];

        public string? PickCastFile(string? initialDirectory = null)
        {
            InitialDirectories.Add(initialDirectory);
            return _selectedFiles.Dequeue();
        }

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
        public int MaximumConcurrency => 1;

        public MergeTaskHandle Schedule(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null) => throw new NotSupportedException();

        public void Dispose()
        {
        }
    }

    private sealed class CompletedMergeService : IModelMergeService
    {
        public Task<MergeResult> MergeAsync(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default)
        {
            return Task.FromResult(new MergeResult(
                Path.Combine(request.OutputDirectory, "merged.cast"),
                "root",
                request.InputFiles.Count,
                1,
                1,
                []));
        }
    }
}

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

        Assert.Equal("添加 2 至 15 个 Cast 部件", viewModel.StatusMessage);

        catalog.SetLanguage(AppLanguage.English);

        Assert.Equal("Add 2 to 15 Cast parts", viewModel.StatusMessage);
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

    [Fact]
    public void RefreshFileValidity_WhenSelectedFileWasDeleted_DisablesMergeAndMarksSlot()
    {
        var first = CreateCastFile("validity-first.cast");
        var second = CreateCastFile("validity-second.cast");
        using var viewModel = new MergeGroupViewModel(
            1,
            new UnusedTaskScheduler(),
            new RecordingDialogService(first, second),
            preferredOutputDirectory: null,
            RootSelectionMode.Automatic,
            new LanguageCatalog(AppLanguage.English));
        viewModel.AddNextCommand.Execute(null);
        viewModel.AddNextCommand.Execute(null);
        Assert.True(viewModel.CanMerge);

        File.Delete(second);
        viewModel.RefreshFileValidity();

        Assert.False(viewModel.CanMerge);
        Assert.True(viewModel.Slots[1].IsInvalid);
    }

    [Fact]
    public void RefreshFileValidity_WhenOneOfTwoMissingFilesReturns_ReenablesItsPreview()
    {
        var first = CreateCastFile("restored-first.cast");
        var second = CreateCastFile("restored-second.cast");
        using var viewModel = new MergeGroupViewModel(
            1,
            new UnusedTaskScheduler(),
            new RecordingDialogService(first, second),
            preferredOutputDirectory: null,
            RootSelectionMode.Automatic,
            new LanguageCatalog(AppLanguage.English));
        viewModel.AddNextCommand.Execute(null);
        viewModel.AddNextCommand.Execute(null);
        File.Delete(first);
        File.Delete(second);
        viewModel.RefreshFileValidity();
        Assert.False(viewModel.PreviewPartCommand.CanExecute(viewModel.Slots[0]));

        File.WriteAllText(first, string.Empty);
        viewModel.RefreshFileValidity();

        Assert.True(viewModel.PreviewPartCommand.CanExecute(viewModel.Slots[0]));
        Assert.False(viewModel.CanMerge);
    }

    [Fact]
    public async Task PreviewCommands_OpenSelectedPartAndMergedOutput()
    {
        var first = CreateCastFile("preview-first.cast");
        var second = CreateCastFile("preview-second.cast");
        var previews = new RecordingPreviewDialogService();
        using var scheduler = new MergeTaskScheduler(new CompletedMergeService(), 1);
        using var viewModel = new MergeGroupViewModel(
            1,
            scheduler,
            new RecordingDialogService(first, second),
            preferredOutputDirectory: null,
            RootSelectionMode.Automatic,
            new LanguageCatalog(AppLanguage.English),
            previews);
        viewModel.AddNextCommand.Execute(null);
        viewModel.AddNextCommand.Execute(null);

        viewModel.PreviewPartCommand.Execute(viewModel.Slots[0]);
        await viewModel.MergeAsync();
        viewModel.PreviewOutputCommand.Execute(null);

        Assert.Equal(first, previews.Paths[0]);
        var mergedPath = Path.Combine(_directory, "Merged Models", "merged.cast");
        Assert.Equal(mergedPath, previews.Paths[1]);

        File.Delete(mergedPath);
        viewModel.RefreshFileValidity();
        Assert.False(viewModel.PreviewOutputCommand.CanExecute(null));
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

    private sealed class RecordingPreviewDialogService : IModelPreviewDialogService
    {
        public List<string> Paths { get; } = [];

        public void Show(string filePath) => Paths.Add(filePath);
    }

    private sealed class CompletedMergeService : IModelMergeService
    {
        public Task<IPreparedMergeOperation> PrepareAsync(
            MergeRequest request,
            IProgress<MergeProgress>? progress = null,
            CancellationToken cancellationToken = default)
        {
            cancellationToken.ThrowIfCancellationRequested();
            return Task.FromResult<IPreparedMergeOperation>(new Operation(request));
        }

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

        private sealed class Operation(MergeRequest request) : IPreparedMergeOperation
        {
            public string OutputPath { get; } = Path.Combine(request.OutputDirectory, "merged.cast");

            public Task<MergeResult> ExecuteAsync(
                IProgress<MergeProgress>? progress = null,
                CancellationToken cancellationToken = default)
            {
                cancellationToken.ThrowIfCancellationRequested();
                Directory.CreateDirectory(Path.GetDirectoryName(OutputPath)!);
                File.WriteAllText(OutputPath, string.Empty);
                return Task.FromResult(new MergeResult(
                    OutputPath,
                    "root",
                    request.InputFiles.Count,
                    1,
                    1,
                    []));
            }
        }
    }
}

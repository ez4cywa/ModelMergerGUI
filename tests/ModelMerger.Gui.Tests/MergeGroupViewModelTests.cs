using ModelMerger.Core.Merging;
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
}

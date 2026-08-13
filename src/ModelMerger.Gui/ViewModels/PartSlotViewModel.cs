namespace ModelMerger.Gui.ViewModels;

internal sealed class PartSlotViewModel(int index) : ViewModelBase
{
    private string? _filePath;
    private bool _isManualRoot;

    public int Index { get; } = index;

    public int Number => Index + 1;

    public string NumberText => Number.ToString("00");

    public string? FilePath
    {
        get => _filePath;
        set
        {
            if (SetProperty(ref _filePath, value))
            {
                RaisePropertyChanged(nameof(IsOccupied));
                RaisePropertyChanged(nameof(FileName));
                RaisePropertyChanged(nameof(DirectoryName));
                RaisePropertyChanged(nameof(AccessibleName));
            }
        }
    }

    public bool IsOccupied => !string.IsNullOrWhiteSpace(FilePath);

    public string FileName => IsOccupied ? Path.GetFileName(FilePath) ?? string.Empty : "添加部件";

    public string DirectoryName => IsOccupied ? Path.GetDirectoryName(FilePath) ?? string.Empty : "点击选择 .cast 文件";

    public string AccessibleName => IsOccupied ? $"部件 {Number}: {FileName}" : $"空部件槽位 {Number}";

    public bool IsManualRoot
    {
        get => _isManualRoot;
        set => SetProperty(ref _isManualRoot, value);
    }
}

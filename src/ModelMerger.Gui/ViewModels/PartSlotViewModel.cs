using ModelMerger.Gui.Localization;

namespace ModelMerger.Gui.ViewModels;

internal sealed class PartSlotViewModel(int index, ILanguageCatalog language) : ViewModelBase
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

    public string FileName => IsOccupied ? Path.GetFileName(FilePath) ?? string.Empty : language[LanguageKeys.AddPart];

    public string DirectoryName => IsOccupied ? Path.GetDirectoryName(FilePath) ?? string.Empty : language[LanguageKeys.ClickCastFile];

    public string AccessibleName => IsOccupied
        ? language.Format(LanguageKeys.PartAccessible, Number, FileName)
        : language.Format(LanguageKeys.EmptySlotAccessible, Number);

    public bool IsManualRoot
    {
        get => _isManualRoot;
        set => SetProperty(ref _isManualRoot, value);
    }

    public void RefreshLanguage()
    {
        RaisePropertyChanged(nameof(FileName));
        RaisePropertyChanged(nameof(DirectoryName));
        RaisePropertyChanged(nameof(AccessibleName));
    }
}

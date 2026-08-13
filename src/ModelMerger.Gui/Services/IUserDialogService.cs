namespace ModelMerger.Gui.Services;

internal interface IUserDialogService
{
    string? PickCastFile(string? initialDirectory = null);

    string? PickOutputFolder(string? initialDirectory = null);

    bool Confirm(string title, string message);

    void ShowInformation(string title, string message);

    void ShowError(string title, string message);
}

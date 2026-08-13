using Microsoft.Win32;
using System.Windows;

namespace ModelMerger.Gui.Services;

internal sealed class UserDialogService : IUserDialogService
{
    public string? PickCastFile(string? initialDirectory = null)
    {
        var dialog = new OpenFileDialog
        {
            Title = "选择 Cast 模型部件",
            Filter = "Cast 模型 (*.cast)|*.cast",
            Multiselect = false,
            CheckFileExists = true,
            InitialDirectory = Directory.Exists(initialDirectory) ? initialDirectory : null
        };
        return dialog.ShowDialog() == true ? dialog.FileName : null;
    }

    public string? PickOutputFolder(string? initialDirectory = null)
    {
        var dialog = new OpenFolderDialog
        {
            Title = "选择合并模型的输出文件夹",
            Multiselect = false,
            InitialDirectory = Directory.Exists(initialDirectory) ? initialDirectory : null
        };
        return dialog.ShowDialog() == true ? dialog.FolderName : null;
    }

    public bool Confirm(string title, string message)
    {
        return MessageBox.Show(message, title, MessageBoxButton.YesNo, MessageBoxImage.Question) == MessageBoxResult.Yes;
    }

    public void ShowInformation(string title, string message)
    {
        MessageBox.Show(message, title, MessageBoxButton.OK, MessageBoxImage.Information);
    }

    public void ShowError(string title, string message)
    {
        MessageBox.Show(message, title, MessageBoxButton.OK, MessageBoxImage.Error);
    }
}

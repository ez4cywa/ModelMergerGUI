using ModelMerger.Core.Preview;
using ModelMerger.Gui.Localization;
using System.Windows;

namespace ModelMerger.Gui.Services;

internal sealed class ModelPreviewDialogService(ILanguageCatalog language) : IModelPreviewDialogService
{
    private readonly IModelPreviewService _previewService = ModelPreviewServiceFactory.CreateDefault();

    public void Show(string filePath)
    {
        var window = new ModelPreviewWindow(filePath, _previewService, language);
        if (Application.Current?.MainWindow is { IsVisible: true } owner)
        {
            window.Owner = owner;
        }

        window.Show();
    }
}

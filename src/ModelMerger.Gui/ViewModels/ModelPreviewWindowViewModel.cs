using ModelMerger.Core.Preview;
using ModelMerger.Gui.Localization;
using System.ComponentModel;

namespace ModelMerger.Gui.ViewModels;

internal sealed class ModelPreviewWindowViewModel : ViewModelBase, IDisposable
{
    private readonly IModelPreviewService _previewService;
    private readonly ILanguageCatalog _language;
    private ModelPreviewData? _preview;
    private ModelPreviewErrorCode? _errorCode;
    private bool _isLoading = true;

    public ModelPreviewWindowViewModel(
        string filePath,
        IModelPreviewService previewService,
        ILanguageCatalog language)
    {
        FilePath = filePath;
        _previewService = previewService;
        _language = language;
        _language.PropertyChanged += Language_PropertyChanged;
    }

    public string FilePath { get; }

    public string FileName => Path.GetFileName(FilePath);

    public string WindowTitle => _language.Format(LanguageKeys.PreviewWindowTitle, FileName);

    public string Header => _language[LanguageKeys.PreviewHeader];

    public string LoadingText => _language[LanguageKeys.PreviewLoading];

    public string Instructions => _language[LanguageKeys.PreviewInstructions];

    public bool IsLoading
    {
        get => _isLoading;
        private set
        {
            if (SetProperty(ref _isLoading, value))
            {
                RaisePropertyChanged(nameof(HasPreview));
            }
        }
    }

    public bool HasError => _errorCode is not null;

    public bool HasPreview => _preview is not null && !IsLoading;

    public string ErrorTitle => _language[LanguageKeys.PreviewErrorTitle];

    public string ErrorMessage => _errorCode switch
    {
        ModelPreviewErrorCode.InvalidPath => _language[LanguageKeys.PreviewErrorInvalidPath],
        ModelPreviewErrorCode.MissingFile => _language[LanguageKeys.PreviewErrorMissingFile],
        ModelPreviewErrorCode.UnsupportedFormat => _language[LanguageKeys.PreviewErrorUnsupportedFormat],
        ModelPreviewErrorCode.NoGeometry => _language[LanguageKeys.PreviewErrorNoGeometry],
        _ => _language[LanguageKeys.PreviewErrorUnreadableModel]
    };

    public string Statistics => _preview is null
        ? string.Empty
        : _language.Format(
            LanguageKeys.PreviewStats,
            _preview.SourceMeshCount,
            _preview.SourceVertexCount,
            _preview.SourceTriangleCount);

    public string DisplayStatistics => _preview is null
        ? string.Empty
        : _language.Format(
            LanguageKeys.PreviewDisplayedStats,
            _preview.DisplayedTriangleCount,
            _preview.SourceTriangleCount);

    public string SimplificationNotice => _language[LanguageKeys.PreviewSimplified];

    public bool IsSimplified => _preview?.IsSimplified == true;

    public async Task<ModelPreviewData?> LoadAsync(CancellationToken cancellationToken)
    {
        try
        {
            _preview = await _previewService.LoadAsync(FilePath, cancellationToken: cancellationToken);
            RaisePreviewPropertiesChanged();
            return _preview;
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            IsLoading = false;
            return null;
        }
        catch (ModelPreviewException exception)
        {
            _errorCode = exception.Code;
            IsLoading = false;
            RaisePreviewPropertiesChanged();
            return null;
        }
        catch
        {
            _errorCode = ModelPreviewErrorCode.UnreadableModel;
            IsLoading = false;
            RaisePreviewPropertiesChanged();
            return null;
        }
    }

    public void CompleteRendering()
    {
        IsLoading = false;
    }

    public void SetRenderingError()
    {
        _preview = null;
        _errorCode = ModelPreviewErrorCode.UnreadableModel;
        IsLoading = false;
        RaisePreviewPropertiesChanged();
    }

    public void Dispose()
    {
        _language.PropertyChanged -= Language_PropertyChanged;
    }

    private void RaisePreviewPropertiesChanged()
    {
        RaisePropertyChanged(nameof(HasError));
        RaisePropertyChanged(nameof(HasPreview));
        RaisePropertyChanged(nameof(ErrorMessage));
        RaisePropertyChanged(nameof(Statistics));
        RaisePropertyChanged(nameof(DisplayStatistics));
        RaisePropertyChanged(nameof(IsSimplified));
    }

    private void Language_PropertyChanged(object? sender, PropertyChangedEventArgs e)
    {
        if (e.PropertyName is not nameof(ILanguageCatalog.Language) and not "Item[]")
        {
            return;
        }

        RaisePropertyChanged(nameof(WindowTitle));
        RaisePropertyChanged(nameof(Header));
        RaisePropertyChanged(nameof(LoadingText));
        RaisePropertyChanged(nameof(Instructions));
        RaisePropertyChanged(nameof(ErrorTitle));
        RaisePropertyChanged(nameof(ErrorMessage));
        RaisePropertyChanged(nameof(Statistics));
        RaisePropertyChanged(nameof(DisplayStatistics));
        RaisePropertyChanged(nameof(SimplificationNotice));
    }
}

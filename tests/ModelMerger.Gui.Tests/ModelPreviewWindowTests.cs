using ModelMerger.Core.Preview;
using ModelMerger.Core.Settings;
using ModelMerger.Gui.Localization;
using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Xunit;

namespace ModelMerger.Gui.Tests;

public sealed class ModelPreviewWindowTests
{
    [Fact]
    public async Task ModelPreviewSceneBuilder_BuildsFrozenCrossThreadGeometry()
    {
        var scene = await Task.Run(() =>
            ModelPreviewSceneBuilder.Build(CreatePreview(), CancellationToken.None));

        Assert.True(scene.IsFrozen);
        Assert.Equal(4, scene.Children.Count);
    }

    [Fact]
    public void ModelPreviewSceneBuilder_WithCancelledToken_StopsBeforeBuildingGeometry()
    {
        using var cancellation = new CancellationTokenSource();
        cancellation.Cancel();

        Assert.Throws<OperationCanceledException>(() =>
            ModelPreviewSceneBuilder.Build(CreatePreview(), cancellation.Token));
    }

    [Fact]
    public void ModelPreviewWindow_RendersGeometryInAllFiveLanguages()
    {
        var originalLanguage = LanguageCatalog.Current.Language;
        Exception? failure = null;
        var renderedLanguages = new List<AppLanguage>();
        var thread = new Thread(() =>
        {
            try
            {
                foreach (var language in SupportedLanguageTestData.All)
                {
                    LanguageCatalog.Current.SetLanguage(language);
                    var window = new ModelPreviewWindow(
                        "preview-sample.cast",
                        new CompletedPreviewService(CreatePreview()),
                        LanguageCatalog.Current)
                    {
                        Left = -20000,
                        Top = -20000,
                        Width = 960,
                        Height = 720,
                        ShowActivated = false,
                        ShowInTaskbar = false
                    };
                    window.Show();
                    PumpDispatcher(TimeSpan.FromMilliseconds(100));
                    window.UpdateLayout();

                    Assert.Equal(1, window.RenderedMeshCount);
                    var bitmap = Render(window);
                    Assert.True(bitmap.PixelWidth >= 720);
                    Assert.True(bitmap.PixelHeight >= 520);
                    SaveWhenRequested(bitmap, language);
                    renderedLanguages.Add(language);
                    window.Close();
                    PumpDispatcher(TimeSpan.FromMilliseconds(30));
                }

                LanguageCatalog.Current.SetLanguage(originalLanguage);
            }
            catch (Exception exception)
            {
                LanguageCatalog.Current.SetLanguage(originalLanguage);
                failure = exception;
            }
            finally
            {
                Dispatcher.CurrentDispatcher.BeginInvokeShutdown(DispatcherPriority.Background);
            }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();

        Assert.True(thread.Join(TimeSpan.FromSeconds(15)), "WPF model preview test timed out.");
        Assert.Null(failure);
        Assert.Equal(SupportedLanguageTestData.All, renderedLanguages);
    }

    private static ModelPreviewData CreatePreview()
    {
        var positions = new[]
        {
            new PreviewPoint3(-1, 0, 0),
            new PreviewPoint3(1, 0, 0),
            new PreviewPoint3(0, 0, 2)
        };
        var normals = new[]
        {
            new PreviewPoint3(0, -1, 0),
            new PreviewPoint3(0, -1, 0),
            new PreviewPoint3(0, -1, 0)
        };
        return new ModelPreviewData(
            "preview-sample.cast",
            "preview-sample",
            1,
            3,
            2,
            1,
            true,
            new PreviewBounds(new PreviewPoint3(-1, 0, 0), new PreviewPoint3(1, 0, 2)),
            [new PreviewMeshData(positions, normals, [0, 1, 2])]);
    }

    private static void PumpDispatcher(TimeSpan duration)
    {
        var frame = new DispatcherFrame();
        var timer = new DispatcherTimer(DispatcherPriority.Background)
        {
            Interval = duration
        };
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            frame.Continue = false;
        };
        timer.Start();
        Dispatcher.PushFrame(frame);
    }

    private static RenderTargetBitmap Render(FrameworkElement element)
    {
        var bitmap = new RenderTargetBitmap(
            (int)Math.Ceiling(element.ActualWidth),
            (int)Math.Ceiling(element.ActualHeight),
            96,
            96,
            PixelFormats.Pbgra32);
        bitmap.Render(element);
        return bitmap;
    }

    private static void SaveWhenRequested(BitmapSource bitmap, AppLanguage language)
    {
        var outputDirectory = Environment.GetEnvironmentVariable("MODEL_MERGER_SCREENSHOT_DIR");
        if (string.IsNullOrWhiteSpace(outputDirectory))
        {
            return;
        }

        Directory.CreateDirectory(outputDirectory);
        var suffix = SupportedLanguageTestData.GetFileSuffix(language);
        using var stream = File.Create(Path.Combine(outputDirectory, $"model-preview-{suffix}.png"));
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        encoder.Save(stream);
    }

    private sealed class CompletedPreviewService(ModelPreviewData preview) : IModelPreviewService
    {
        public Task<ModelPreviewData> LoadAsync(
            string filePath,
            int triangleLimit = ModelPreviewService.DefaultTriangleLimit,
            CancellationToken cancellationToken = default)
        {
            cancellationToken.ThrowIfCancellationRequested();
            return Task.FromResult(preview with { FilePath = filePath });
        }
    }
}

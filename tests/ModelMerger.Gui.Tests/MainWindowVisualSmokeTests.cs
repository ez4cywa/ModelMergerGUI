using ModelMerger.Core.Settings;
using ModelMerger.Gui.Localization;
using System.IO;
using System.Windows;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Xunit;

namespace ModelMerger.Gui.Tests;

public sealed class MainWindowVisualSmokeTests
{
    [Fact]
    public void MainWindow_RendersInChineseAndEnglish()
    {
        var originalLanguage = LanguageCatalog.Current.Language;
        Exception? failure = null;
        var renderedLanguages = new List<AppLanguage>();
        var thread = new Thread(() =>
        {
            try
            {
                var window = new MainWindow
                {
                    Left = -20000,
                    Top = -20000,
                    Width = 1280,
                    Height = 900,
                    ShowActivated = false,
                    ShowInTaskbar = false
                };
                window.Show();

                var timer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
                timer.Tick += (_, _) =>
                {
                    timer.Stop();
                    window.UpdateLayout();
                    _ = Render(window);
                    foreach (var language in new[] { AppLanguage.ChineseSimplified, AppLanguage.English })
                    {
                        LanguageCatalog.Current.SetLanguage(language);
                        Dispatcher.CurrentDispatcher.Invoke(
                            DispatcherPriority.ContextIdle,
                            new Action(() => { }));
                        window.UpdateLayout();
                        var bitmap = Render(window);
                        Assert.True(bitmap.PixelWidth >= 800);
                        Assert.True(bitmap.PixelHeight >= 600);
                        SaveWhenRequested(bitmap, language);
                        renderedLanguages.Add(language);
                    }

                    LanguageCatalog.Current.SetLanguage(originalLanguage);
                    (window.DataContext as IDisposable)?.Dispose();
                    window.Hide();
                    Dispatcher.CurrentDispatcher.BeginInvokeShutdown(DispatcherPriority.Background);
                };
                timer.Start();
                Dispatcher.Run();
            }
            catch (Exception exception)
            {
                LanguageCatalog.Current.SetLanguage(originalLanguage);
                failure = exception;
                Dispatcher.CurrentDispatcher.BeginInvokeShutdown(DispatcherPriority.Send);
            }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();

        Assert.True(thread.Join(TimeSpan.FromSeconds(15)), "WPF visual smoke test timed out.");
        Assert.Null(failure);
        Assert.Equal([AppLanguage.ChineseSimplified, AppLanguage.English], renderedLanguages);
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
        var suffix = language == AppLanguage.English ? "en" : "zh";
        using var stream = File.Create(Path.Combine(outputDirectory, $"main-window-{suffix}.png"));
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        encoder.Save(stream);
    }
}

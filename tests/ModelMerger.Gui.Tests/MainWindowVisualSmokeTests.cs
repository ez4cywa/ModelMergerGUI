using ModelMerger.Core.Settings;
using ModelMerger.Gui.Localization;
using ModelMerger.Gui.ViewModels;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Threading;
using Xunit;

namespace ModelMerger.Gui.Tests;

public sealed class MainWindowVisualSmokeTests
{
    [Fact]
    public void MainWindow_RendersInAllFiveLanguages()
    {
        var originalLanguage = LanguageCatalog.Current.Language;
        Exception? failure = null;
        var renderedLanguages = new List<AppLanguage>();
        var thread = new Thread(() =>
        {
            var previewPart = Path.Combine(Path.GetTempPath(), $"model-merger-visual-{Guid.NewGuid():N}.cast");
            try
            {
                File.WriteAllBytes(previewPart, []);
                var timer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(1) };
                timer.Tick += (_, _) =>
                {
                    timer.Stop();
                    foreach (var language in SupportedLanguageTestData.All)
                    {
                        LanguageCatalog.Current.SetLanguage(language);
                        var window = new MainWindow(new MemorySettingsStore())
                        {
                            Left = -20000,
                            Top = -20000,
                            ShowActivated = false,
                            ShowInTaskbar = false
                        };
                        window.Show();
                        Dispatcher.CurrentDispatcher.Invoke(
                            DispatcherPriority.ContextIdle,
                            new Action(() => { }));
                        window.UpdateLayout();
                        PumpDispatcher(TimeSpan.FromMilliseconds(100));
                        var viewModel = Assert.IsType<MainWindowViewModel>(window.DataContext);
                        var group = Assert.Single(viewModel.Groups);
                        Assert.Equal(15, group.Slots.Count);
                        group.AddDroppedFiles([previewPart]);
                        window.UpdateLayout();
                        PumpDispatcher(TimeSpan.FromMilliseconds(50));
                        Assert.Equal(1240, (int)Math.Round(window.ActualWidth));
                        Assert.Equal(820, (int)Math.Round(window.ActualHeight));
                        var root = Assert.IsType<Grid>(window.Content);
                        Assert.True(
                            root.ActualWidth <= window.ActualWidth,
                            $"The {language} root layout overflows horizontally: {root.ActualWidth} > {window.ActualWidth}.");
                        var groupsScroller = Assert.IsType<ScrollViewer>(root.Children[1]);
                        var groups = Assert.IsType<ItemsControl>(groupsScroller.Content);
                        Assert.True(
                            groups.ActualWidth <= groupsScroller.ViewportWidth + 1,
                            $"The {language} groups overflow horizontally: {groups.ActualWidth} > {groupsScroller.ViewportWidth}.");
                        var groupContainer = Assert.IsType<ContentPresenter>(
                            groups.ItemContainerGenerator.ContainerFromIndex(0));
                        Assert.True(
                            groupContainer.DesiredSize.Width <= groupsScroller.ViewportWidth + 1,
                            $"The {language} group card requests more width than the viewport: " +
                            $"{groupContainer.DesiredSize.Width} > {groupsScroller.ViewportWidth}.");
                        Assert.Contains(
                            language == AppLanguage.ChineseSimplified ? "MiSans" : "Segoe UI",
                            window.FontFamily.Source,
                            StringComparison.OrdinalIgnoreCase);
                        _ = Render(window);
                        var bitmap = Render(window);
                        Assert.True(bitmap.PixelWidth >= 800);
                        Assert.True(bitmap.PixelHeight >= 600);
                        SaveWhenRequested(bitmap, language, "main-window");

                        window.Width = window.MinWidth;
                        window.Height = window.MinHeight;
                        window.UpdateLayout();
                        PumpDispatcher(TimeSpan.FromMilliseconds(50));
                        Assert.Equal(1000, (int)Math.Round(window.ActualWidth));
                        Assert.Equal(680, (int)Math.Round(window.ActualHeight));
                        Assert.True(root.ActualWidth <= window.ActualWidth);
                        Assert.True(groups.ActualWidth <= groupsScroller.ViewportWidth + 1);
                        var compactBitmap = Render(window);
                        Assert.Equal(1000, compactBitmap.PixelWidth);
                        Assert.Equal(680, compactBitmap.PixelHeight);
                        SaveWhenRequested(compactBitmap, language, "main-window-compact");
                        renderedLanguages.Add(language);
                        window.Close();
                        Dispatcher.CurrentDispatcher.Invoke(
                            DispatcherPriority.ContextIdle,
                            new Action(() => { }));
                    }

                    LanguageCatalog.Current.SetLanguage(originalLanguage);
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
            finally
            {
                File.Delete(previewPart);
            }
        });
        thread.SetApartmentState(ApartmentState.STA);
        thread.Start();

        Assert.True(thread.Join(TimeSpan.FromSeconds(15)), "WPF visual smoke test timed out.");
        Assert.Null(failure);
        Assert.Equal(SupportedLanguageTestData.All, renderedLanguages);
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

    private static void SaveWhenRequested(BitmapSource bitmap, AppLanguage language, string prefix)
    {
        var outputDirectory = Environment.GetEnvironmentVariable("MODEL_MERGER_SCREENSHOT_DIR");
        if (string.IsNullOrWhiteSpace(outputDirectory))
        {
            return;
        }

        Directory.CreateDirectory(outputDirectory);
        var suffix = SupportedLanguageTestData.GetFileSuffix(language);
        using var stream = File.Create(Path.Combine(outputDirectory, $"{prefix}-{suffix}.png"));
        var encoder = new PngBitmapEncoder();
        encoder.Frames.Add(BitmapFrame.Create(bitmap));
        encoder.Save(stream);
    }

    private sealed class MemorySettingsStore : ISettingsStore
    {
        public Task<AppSettings> LoadAsync(CancellationToken cancellationToken = default) =>
            Task.FromResult(new AppSettings());

        public Task SaveAsync(AppSettings settings, CancellationToken cancellationToken = default) =>
            Task.CompletedTask;
    }
}

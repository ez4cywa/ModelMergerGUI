using ModelMerger.Core.Merging;
using ModelMerger.Core.Settings;
using Xunit;

namespace ModelMerger.Core.Tests;

public sealed class JsonSettingsStoreTests : IDisposable
{
    private readonly string _directory = Path.Combine(
        Path.GetTempPath(),
        $"ModelMergerSettingsTests-{Guid.NewGuid():N}");

    public JsonSettingsStoreTests()
    {
        Directory.CreateDirectory(_directory);
    }

    [Fact]
    public async Task SaveThenLoad_RoundTripsApprovedUserPreferences()
    {
        var settingsPath = Path.Combine(_directory, "settings.json");
        var outputPath = Path.Combine(_directory, "output");
        Directory.CreateDirectory(outputPath);
        var store = new JsonSettingsStore(settingsPath);
        var expected = new AppSettings
        {
            PreferredOutputDirectory = outputPath,
            RememberOutputDirectory = true,
            UiLanguage = AppLanguage.English,
            RootSelectionMode = RootSelectionMode.Manual,
            WindowBounds = new WindowBounds(100, 120, 1120, 760)
        };

        await store.SaveAsync(expected);
        var loaded = await store.LoadAsync();

        Assert.Equal(expected, loaded);
    }

    [Fact]
    public async Task Load_WithCorruptJson_FallsBackToDefaults()
    {
        var settingsPath = Path.Combine(_directory, "settings.json");
        await File.WriteAllTextAsync(settingsPath, "{ definitely not json");
        var store = new JsonSettingsStore(settingsPath);

        var loaded = await store.LoadAsync();

        Assert.Equal(new AppSettings(), loaded);
    }

    [Fact]
    public async Task Load_WithMissingRememberedOutputFolder_DisablesThePreference()
    {
        var settingsPath = Path.Combine(_directory, "settings.json");
        await File.WriteAllTextAsync(
            settingsPath,
            """
            {
              "schemaVersion": 1,
              "preferredOutputDirectory": "Z:\\folder-that-does-not-exist",
              "rememberOutputDirectory": true,
              "rootSelectionMode": 0
            }
            """);
        var store = new JsonSettingsStore(settingsPath);

        var loaded = await store.LoadAsync();

        Assert.False(loaded.RememberOutputDirectory);
        Assert.Null(loaded.PreferredOutputDirectory);
    }

    public void Dispose()
    {
        Directory.Delete(_directory, recursive: true);
    }
}

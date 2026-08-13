using System.Text.Json;

namespace ModelMerger.Core.Settings;

public sealed class JsonSettingsStore(string settingsPath) : ISettingsStore
{
    private static readonly JsonSerializerOptions SerializerOptions = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = true
    };

    public string SettingsPath { get; } = Path.GetFullPath(settingsPath);

    public static JsonSettingsStore CreateDefault()
    {
        var localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        return new JsonSettingsStore(Path.Combine(localAppData, "CastModelMerger", "settings.json"));
    }

    public async Task<AppSettings> LoadAsync(CancellationToken cancellationToken = default)
    {
        if (!File.Exists(SettingsPath))
        {
            return new AppSettings();
        }

        try
        {
            await using var stream = File.OpenRead(SettingsPath);
            var settings = await JsonSerializer.DeserializeAsync<AppSettings>(
                stream,
                SerializerOptions,
                cancellationToken).ConfigureAwait(false);
            return Sanitize(settings ?? new AppSettings());
        }
        catch (Exception exception) when (exception is JsonException or IOException or UnauthorizedAccessException)
        {
            return new AppSettings();
        }
    }

    public async Task SaveAsync(AppSettings settings, CancellationToken cancellationToken = default)
    {
        ArgumentNullException.ThrowIfNull(settings);
        var directory = Path.GetDirectoryName(SettingsPath)
            ?? throw new InvalidOperationException("The settings path does not have a parent directory.");
        Directory.CreateDirectory(directory);
        var temporaryPath = $"{SettingsPath}.{Guid.NewGuid():N}.tmp";
        try
        {
            await using (var stream = new FileStream(
                             temporaryPath,
                             FileMode.CreateNew,
                             FileAccess.Write,
                             FileShare.None,
                             4096,
                             FileOptions.Asynchronous))
            {
                await JsonSerializer.SerializeAsync(
                    stream,
                    Sanitize(settings),
                    SerializerOptions,
                    cancellationToken).ConfigureAwait(false);
                await stream.FlushAsync(cancellationToken).ConfigureAwait(false);
            }

            File.Move(temporaryPath, SettingsPath, overwrite: true);
        }
        finally
        {
            if (File.Exists(temporaryPath))
            {
                File.Delete(temporaryPath);
            }
        }
    }

    private static AppSettings Sanitize(AppSettings settings)
    {
        var outputDirectory = settings.PreferredOutputDirectory;
        var rememberOutput = settings.RememberOutputDirectory;
        if (rememberOutput && (string.IsNullOrWhiteSpace(outputDirectory) || !Directory.Exists(outputDirectory)))
        {
            outputDirectory = null;
            rememberOutput = false;
        }

        var rootMode = Enum.IsDefined(settings.RootSelectionMode)
            ? settings.RootSelectionMode
            : Merging.RootSelectionMode.Automatic;
        var bounds = IsValid(settings.WindowBounds) ? settings.WindowBounds : null;
        return settings with
        {
            SchemaVersion = 1,
            PreferredOutputDirectory = outputDirectory,
            RememberOutputDirectory = rememberOutput,
            RootSelectionMode = rootMode,
            WindowBounds = bounds
        };
    }

    private static bool IsValid(WindowBounds? bounds)
    {
        return bounds is not null &&
               double.IsFinite(bounds.Left) &&
               double.IsFinite(bounds.Top) &&
               double.IsFinite(bounds.Width) &&
               double.IsFinite(bounds.Height) &&
               bounds.Width >= 800 &&
               bounds.Height >= 600;
    }
}

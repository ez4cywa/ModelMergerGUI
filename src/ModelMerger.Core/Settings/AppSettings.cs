using ModelMerger.Core.Merging;

namespace ModelMerger.Core.Settings;

public enum AppLanguage
{
    ChineseSimplified = 0,
    English = 1,
    French = 2,
    Russian = 3,
    Spanish = 4
}

public sealed record WindowBounds(double Left, double Top, double Width, double Height);

public sealed record AppSettings
{
    public int SchemaVersion { get; init; } = 2;

    public AppLanguage? UiLanguage { get; init; }

    public string? PreferredOutputDirectory { get; init; }

    public bool RememberOutputDirectory { get; init; }

    public RootSelectionMode RootSelectionMode { get; init; } = RootSelectionMode.Automatic;

    public WindowBounds? WindowBounds { get; init; }
}

public interface ISettingsStore
{
    Task<AppSettings> LoadAsync(CancellationToken cancellationToken = default);

    Task SaveAsync(AppSettings settings, CancellationToken cancellationToken = default);
}

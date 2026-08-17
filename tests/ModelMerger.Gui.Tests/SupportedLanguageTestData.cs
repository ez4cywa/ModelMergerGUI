using ModelMerger.Core.Settings;
using ModelMerger.Gui.Localization;

namespace ModelMerger.Gui.Tests;

internal static class SupportedLanguageTestData
{
    public static IReadOnlyList<AppLanguage> All { get; } = LanguageCatalog.SupportedLanguages
        .Select(language => language.Value)
        .ToArray();

    public static string GetFileSuffix(AppLanguage language) => LanguageCatalog.SupportedLanguages
        .Single(option => option.Value == language)
        .FileSuffix;
}

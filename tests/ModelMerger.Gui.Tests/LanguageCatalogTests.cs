using ModelMerger.Core.Settings;
using ModelMerger.Gui.Localization;
using Xunit;

namespace ModelMerger.Gui.Tests;

public sealed class LanguageCatalogTests
{
    [Fact]
    public void SetLanguage_ProvidesCompleteResourcesForAllFiveLanguages()
    {
        var catalog = new LanguageCatalog(AppLanguage.ChineseSimplified);
        var expectedNewGroupText = new Dictionary<AppLanguage, string>
        {
            [AppLanguage.ChineseSimplified] = "新建模型组",
            [AppLanguage.English] = "New group",
            [AppLanguage.French] = "Nouveau groupe",
            [AppLanguage.Russian] = "Новая группа",
            [AppLanguage.Spanish] = "Nuevo grupo"
        };

        foreach (var language in SupportedLanguageTestData.All)
        {
            catalog.SetLanguage(language);
            Assert.Equal(expectedNewGroupText[language], catalog[LanguageKeys.NewGroup]);
            foreach (var key in LanguageKeys.All)
            {
                Assert.False(string.IsNullOrWhiteSpace(catalog[key]));
            }
        }
    }

    [Fact]
    public void Format_UsesEquivalentPlaceholdersInAllFiveLanguages()
    {
        var catalog = new LanguageCatalog(AppLanguage.ChineseSimplified);
        var expected = new Dictionary<AppLanguage, string>
        {
            [AppLanguage.ChineseSimplified] = "模型组 3",
            [AppLanguage.English] = "Group 3",
            [AppLanguage.French] = "Groupe 3",
            [AppLanguage.Russian] = "Группа 3",
            [AppLanguage.Spanish] = "Grupo 3"
        };

        foreach (var language in SupportedLanguageTestData.All)
        {
            catalog.SetLanguage(language);
            Assert.Equal(expected[language], catalog.Format(LanguageKeys.GroupName, 3));
        }
    }

    [Theory]
    [InlineData("zh-CN", AppLanguage.ChineseSimplified)]
    [InlineData("en-US", AppLanguage.English)]
    [InlineData("fr-FR", AppLanguage.French)]
    [InlineData("ru-RU", AppLanguage.Russian)]
    [InlineData("es-ES", AppLanguage.Spanish)]
    [InlineData("de-DE", AppLanguage.ChineseSimplified)]
    public void ResolveInitialLanguage_RecognizesTheFiveSupportedLanguages(
        string cultureName,
        AppLanguage expected)
    {
        Assert.Equal(expected, LanguageCatalog.ResolveInitialLanguage(new System.Globalization.CultureInfo(cultureName)));
    }

    [Fact]
    public void InterfaceFontFamily_UsesMiSansOnlyForChinese()
    {
        var catalog = new LanguageCatalog(AppLanguage.ChineseSimplified);

        Assert.Contains("MiSans", catalog.InterfaceFontFamily.Source, StringComparison.OrdinalIgnoreCase);

        foreach (var language in SupportedLanguageTestData.All.Where(language => language != AppLanguage.ChineseSimplified))
        {
            catalog.SetLanguage(language);
            Assert.Equal("Segoe UI", catalog.InterfaceFontFamily.Source);
        }
    }
}

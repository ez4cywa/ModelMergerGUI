using ModelMerger.Core.Settings;
using ModelMerger.Gui.Localization;
using Xunit;

namespace ModelMerger.Gui.Tests;

public sealed class LanguageCatalogTests
{
    [Fact]
    public void SetLanguage_ImmediatelySwitchesAllRegisteredText()
    {
        var catalog = new LanguageCatalog(AppLanguage.ChineseSimplified);

        Assert.Equal("新建模型组", catalog[LanguageKeys.NewGroup]);

        catalog.SetLanguage(AppLanguage.English);

        Assert.Equal("New group", catalog[LanguageKeys.NewGroup]);
        foreach (var key in LanguageKeys.All)
        {
            Assert.False(string.IsNullOrWhiteSpace(catalog[key]));
        }
    }

    [Fact]
    public void Format_UsesEquivalentPlaceholdersInBothLanguages()
    {
        var catalog = new LanguageCatalog(AppLanguage.ChineseSimplified);

        Assert.Equal("模型组 3", catalog.Format(LanguageKeys.GroupName, 3));

        catalog.SetLanguage(AppLanguage.English);

        Assert.Equal("Group 3", catalog.Format(LanguageKeys.GroupName, 3));
    }
}

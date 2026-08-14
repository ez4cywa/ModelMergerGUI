namespace ModelMerger.Gui.Localization;

internal sealed record LocalizedText(
    string? Key,
    string? Literal,
    IReadOnlyList<object?> Arguments)
{
    public static LocalizedText FromKey(string key, params object?[] arguments) =>
        new(key, null, arguments);

    public static LocalizedText FromLiteral(string value) =>
        new(null, value, []);

    public string Render(ILanguageCatalog catalog) =>
        Key is null ? Literal ?? string.Empty : catalog.Format(Key, Arguments.ToArray());
}

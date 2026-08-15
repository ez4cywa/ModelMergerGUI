using ModelMerger.Core.Settings;
using System.ComponentModel;
using System.Globalization;
using System.Text.RegularExpressions;

namespace ModelMerger.Gui.Localization;

public interface ILanguageCatalog : INotifyPropertyChanged
{
    AppLanguage Language { get; }

    string this[string key] { get; }

    string Format(string key, params object?[] arguments);

    void SetLanguage(AppLanguage language);
}

public sealed partial class LanguageCatalog : ILanguageCatalog
{
    private static readonly IReadOnlyDictionary<AppLanguage, IReadOnlyDictionary<string, string>> Resources =
        CreateResources();
    private AppLanguage _language;

    static LanguageCatalog()
    {
        ValidateResources(Resources);
    }

    public LanguageCatalog(AppLanguage language)
    {
        _language = language;
    }

    public static LanguageCatalog Current { get; } = new(ResolveInitialLanguage(CultureInfo.CurrentUICulture));

    public event PropertyChangedEventHandler? PropertyChanged;

    public AppLanguage Language => _language;

    public string this[string key] => Resources[_language].TryGetValue(key, out var value)
        ? value
        : throw new KeyNotFoundException($"Unknown language key: {key}");

    public string Format(string key, params object?[] arguments) =>
        string.Format(GetCulture(_language), this[key], arguments);

    public void SetLanguage(AppLanguage language)
    {
        if (!Enum.IsDefined(language))
        {
            throw new ArgumentOutOfRangeException(nameof(language));
        }

        if (_language == language)
        {
            return;
        }

        _language = language;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Language)));
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs("Item[]"));
    }

    public static AppLanguage ResolveInitialLanguage(CultureInfo culture) =>
        string.Equals(culture.TwoLetterISOLanguageName, "en", StringComparison.OrdinalIgnoreCase)
            ? AppLanguage.English
            : AppLanguage.ChineseSimplified;

    private static CultureInfo GetCulture(AppLanguage language) => CultureInfo.GetCultureInfo(
        language == AppLanguage.English ? "en-US" : "zh-CN");

    private static void ValidateResources(
        IReadOnlyDictionary<AppLanguage, IReadOnlyDictionary<string, string>> resources)
    {
        foreach (var language in Enum.GetValues<AppLanguage>())
        {
            if (!resources.TryGetValue(language, out var entries))
            {
                throw new InvalidOperationException($"Missing resources for {language}.");
            }

            var missing = LanguageKeys.All.Where(key => !entries.ContainsKey(key)).ToArray();
            if (missing.Length > 0)
            {
                throw new InvalidOperationException($"{language} is missing: {string.Join(", ", missing)}");
            }
        }

        foreach (var key in LanguageKeys.All)
        {
            var chinesePlaceholders = PlaceholderPattern().Matches(resources[AppLanguage.ChineseSimplified][key])
                .Select(match => match.Groups[1].Value)
                .ToArray();
            var englishPlaceholders = PlaceholderPattern().Matches(resources[AppLanguage.English][key])
                .Select(match => match.Groups[1].Value)
                .ToArray();
            if (!chinesePlaceholders.SequenceEqual(englishPlaceholders))
            {
                throw new InvalidOperationException($"Placeholder mismatch for {key}.");
            }
        }
    }

    private static IReadOnlyDictionary<AppLanguage, IReadOnlyDictionary<string, string>> CreateResources() =>
        new Dictionary<AppLanguage, IReadOnlyDictionary<string, string>>
        {
            [AppLanguage.ChineseSimplified] = Chinese(),
            [AppLanguage.English] = English()
        };

    private static IReadOnlyDictionary<string, string> Chinese() => new Dictionary<string, string>
    {
        [LanguageKeys.AppTitle] = "Cast 模型合并器",
        [LanguageKeys.AppSubtitle] = "按模型组管理和并发合并 Cast 部件；每组支持 2–16 个部件",
        [LanguageKeys.Language] = "界面语言",
        [LanguageKeys.NewGroup] = "新建模型组",
        [LanguageKeys.SaveSettings] = "保存设置",
        [LanguageKeys.RestoreDefaults] = "恢复默认",
        [LanguageKeys.StartGroup] = "开始本组",
        [LanguageKeys.Cancel] = "取消",
        [LanguageKeys.DeleteGroup] = "删除组",
        [LanguageKeys.ModelParts] = "模型部件",
        [LanguageKeys.ModelPartsHint] = "点击空槽逐个添加，或将多个 .cast 文件拖到本组",
        [LanguageKeys.AddNext] = "添加下一个",
        [LanguageKeys.Clear] = "清空",
        [LanguageKeys.RootBadge] = "根",
        [LanguageKeys.FileMissingBadge] = "文件丢失",
        [LanguageKeys.SetAsRoot] = "设为根",
        [LanguageKeys.Remove] = "移除",
        [LanguageKeys.GroupSettings] = "本组设置",
        [LanguageKeys.RootModel] = "根模型",
        [LanguageKeys.Automatic] = "自动识别",
        [LanguageKeys.Manual] = "手动指定",
        [LanguageKeys.ManualRootHint] = "手动模式下，在部件槽点击“设为根”",
        [LanguageKeys.OutputFolder] = "输出文件夹",
        [LanguageKeys.OutputFolderHelp] = "选择本模型组的合并输出文件夹",
        [LanguageKeys.Browse] = "浏览",
        [LanguageKeys.OutputFileNameOptional] = "输出文件名（可选）",
        [LanguageKeys.OutputFileNameHelp] = "可选；留空时使用根模型名称",
        [LanguageKeys.OutputFileHint] = "留空时使用本组根模型名称",
        [LanguageKeys.GroupStatus] = "本组状态",
        [LanguageKeys.RunLog] = "运行日志",
        [LanguageKeys.OpenGroupOutput] = "打开本组输出",
        [LanguageKeys.Attribution] = " · 基于 Scobalula / echo000 ModelMerger · MIT License",
        [LanguageKeys.RememberOutput] = "记住最近选择的输出目录",
        [LanguageKeys.RememberOutputHint] = "保存设置后，作为新建模型组的默认输出目录",
        [LanguageKeys.CancelAll] = "取消全部",
        [LanguageKeys.MergeAllReady] = "合并全部就绪组",
        [LanguageKeys.PickCastTitle] = "选择 Cast 模型部件",
        [LanguageKeys.CastFilter] = "Cast 模型 (*.cast)|*.cast",
        [LanguageKeys.PickOutputTitle] = "选择合并模型的输出文件夹",
        [LanguageKeys.WorkspaceInitial] = "创建模型组并添加部件",
        [LanguageKeys.Concurrency] = "最多 {0} 组并行",
        [LanguageKeys.SettingsSavedStatus] = "设置已保存；部件文件路径不会被记录",
        [LanguageKeys.SettingsSavedTitle] = "保存设置",
        [LanguageKeys.SettingsSavedMessage] = "已保存界面语言、输出目录、首组根模型模式和窗口位置。\n不会保存各组的模型文件路径。",
        [LanguageKeys.SettingsSaveFailedTitle] = "无法保存设置",
        [LanguageKeys.BatchStarted] = "已启动 {0} 个组；{1}",
        [LanguageKeys.BatchFinished] = "批量处理结束，共检查 {0} 个组",
        [LanguageKeys.CancelAllStatus] = "正在取消所有运行和等待中的组",
        [LanguageKeys.RestoreFailedTitle] = "无法恢复默认设置",
        [LanguageKeys.RestoreDoneStatus] = "已恢复默认设置；现有模型组和部件未被清除",
        [LanguageKeys.WorkspaceProcessing] = "正在处理 {0} 个组；{1}",
        [LanguageKeys.WorkspaceSummary] = "共 {0} 个组，{1} 个已就绪",
        [LanguageKeys.GroupName] = "模型组 {0}",
        [LanguageKeys.SummaryProcessing] = "{0} · 处理中",
        [LanguageKeys.SummaryReady] = "{0} · 已就绪",
        [LanguageKeys.SummaryNeedTwo] = "{0} · 至少需要 2 个部件",
        [LanguageKeys.StatusInitial] = "添加 2 至 16 个 Cast 部件",
        [LanguageKeys.DroppedPartialTitle] = "{0}：部分文件未添加",
        [LanguageKeys.DroppedPartialBody] = "已添加 {0} 个部件。\n\n{1}",
        [LanguageKeys.AddPartInvalidPath] = "文件路径无效",
        [LanguageKeys.AddPartMissing] = "文件不存在",
        [LanguageKeys.AddPartNotCast] = "仅支持 .cast 文件",
        [LanguageKeys.AddPartDuplicate] = "该部件已添加",
        [LanguageKeys.AddPartFull] = "该组已达到 16 个部件上限",
        [LanguageKeys.AddPartSucceeded] = "已添加",
        [LanguageKeys.AddedStatus] = "已添加 {0} 个部件",
        [LanguageKeys.QueueWaiting] = "等待可用的并发处理位置",
        [LanguageKeys.QueueLog] = "{0} 已加入队列，共 {1} 个部件",
        [LanguageKeys.OverwriteTitle] = "覆盖输出文件",
        [LanguageKeys.OverwritePrompt] = "{0} 的文件已存在：\n{1}\n\n是否覆盖？",
        [LanguageKeys.OverwriteCancelled] = "已取消覆盖",
        [LanguageKeys.MergeCompletedStatus] = "完成：{0}",
        [LanguageKeys.MergeCompletedLog] = "合并完成：{0} 根骨骼，{1} 个网格",
        [LanguageKeys.WarningsHeading] = "警告：",
        [LanguageKeys.MergeCompletedTitle] = "{0} 合并完成",
        [LanguageKeys.MergeCompletedBody] = "已保存到：\n{0}\n\n骨骼：{1}    网格：{2}{3}",
        [LanguageKeys.CancelledStatus] = "已取消，临时文件已清理",
        [LanguageKeys.CancelledLog] = "用户取消了该组任务",
        [LanguageKeys.InvalidRequestStatus] = "合并请求无效",
        [LanguageKeys.InvalidRequestTitle] = "{0} 无法开始合并",
        [LanguageKeys.OutputConflictStatus] = "输出路径正被其他模型组占用",
        [LanguageKeys.OutputConflictTitle] = "{0} 输出冲突",
        [LanguageKeys.OutputConflictBody] = "另一个模型组正在写入：\n{0}\n\n请等待其完成，或选择不同的输出文件名。",
        [LanguageKeys.MergeFailedStatus] = "合并失败",
        [LanguageKeys.MergeFailedTitle] = "{0} 合并失败",
        [LanguageKeys.AddFailedTitle] = "无法添加部件",
        [LanguageKeys.RemovedStatus] = "已移除部件，当前 {0}",
        [LanguageKeys.ClearedStatus] = "部件列表已清空",
        [LanguageKeys.ManualRootStatus] = "已将 {0} 设为根模型",
        [LanguageKeys.AddPart] = "添加部件",
        [LanguageKeys.ClickCastFile] = "点击选择 .cast 文件",
        [LanguageKeys.PartAccessible] = "部件 {0}: {1}",
        [LanguageKeys.EmptySlotAccessible] = "空部件槽位 {0}",
        [LanguageKeys.CloseBusyTitle] = "正在合并",
        [LanguageKeys.CloseBusyMessage] = "请先取消所有正在运行或等待的合并任务，再关闭窗口。",
        [LanguageKeys.CloseRaceMessage] = "已有合并任务启动，请先取消任务再关闭。",
        [LanguageKeys.ProgressValidating] = "正在检查合并请求",
        [LanguageKeys.ProgressLoading] = "正在读取 {0}",
        [LanguageKeys.ProgressSelectingRoot] = "正在识别根模型",
        [LanguageKeys.ProgressMerging] = "正在合并 {0}",
        [LanguageKeys.ProgressSaving] = "正在保存 {0}",
        [LanguageKeys.ProgressVerifying] = "正在验证保存的 Cast 模型",
        [LanguageKeys.ProgressCompleted] = "已保存 {0}",
        [LanguageKeys.ProgressGeneric] = "处理中",
        [LanguageKeys.ValidationInvalidPartCount] = "每个模型组需要 2 至 16 个部件。",
        [LanguageKeys.ValidationInvalidPath] = "模型部件路径无效：{0}",
        [LanguageKeys.ValidationMissingFile] = "模型部件不存在：{0}",
        [LanguageKeys.ValidationUnsupportedExtension] = "GUI 仅支持 .cast 模型部件：{0}",
        [LanguageKeys.ValidationDuplicateFile] = "同一部件不能重复添加：{0}",
        [LanguageKeys.ValidationInvalidOutputDirectory] = "请选择有效的输出文件夹。",
        [LanguageKeys.ValidationInvalidOutputFileName] = "输出文件名无效，且必须使用 .cast 扩展名。",
        [LanguageKeys.ValidationOutputAlreadyExists] = "输出文件已存在：{0}",
        [LanguageKeys.ValidationManualRootNotSelected] = "手动根模型必须是本组已选部件。",
        [LanguageKeys.WarningNoAttachmentBone] = "{0} 与 {1} 没有共同的连接骨骼；已在不重新定位的情况下合并。请检查根模型选择和连接骨骼名称。",
        [LanguageKeys.WarningUnconnectedHierarchy] = "{0} 无法连接到当前层级；已在不重新定位的情况下合并。请尝试手动选择根模型或修正骨骼层级。",
        [LanguageKeys.ModelPartReadError] = "无法读取模型部件：{0}\n文件格式：{1}\n请重新导出模型，或替换损坏的文件。",
        [LanguageKeys.Preview] = "预览",
        [LanguageKeys.PreviewMerged] = "预览合并模型",
        [LanguageKeys.PreviewWindowTitle] = "{0} · 模型预览",
        [LanguageKeys.PreviewHeader] = "模型预览",
        [LanguageKeys.PreviewLoading] = "正在后台读取并准备模型…",
        [LanguageKeys.PreviewInstructions] = "按住鼠标左键拖动旋转，滚轮缩放；也可使用按钮或键盘方向键、+、−。",
        [LanguageKeys.PreviewRotateLeft] = "向左旋转",
        [LanguageKeys.PreviewRotateRight] = "向右旋转",
        [LanguageKeys.PreviewZoomIn] = "放大",
        [LanguageKeys.PreviewZoomOut] = "缩小",
        [LanguageKeys.PreviewResetView] = "重置视角",
        [LanguageKeys.PreviewClose] = "关闭",
        [LanguageKeys.PreviewStats] = "{0} 个网格 · {1:N0} 个顶点 · {2:N0} 个三角面",
        [LanguageKeys.PreviewDisplayedStats] = "当前显示 {0:N0} / {1:N0} 个三角面",
        [LanguageKeys.PreviewSimplified] = "为保持预览流畅，画面已自动抽样；源文件和合并输出不会被修改。",
        [LanguageKeys.PreviewErrorTitle] = "无法预览模型",
        [LanguageKeys.PreviewErrorInvalidPath] = "模型路径无效，请重新选择文件。",
        [LanguageKeys.PreviewErrorMissingFile] = "模型文件已不存在，请重新选择或重新执行合并。",
        [LanguageKeys.PreviewErrorUnsupportedFormat] = "预览仅支持 .cast 模型。",
        [LanguageKeys.PreviewErrorUnreadableModel] = "无法读取此 Cast 模型。请重新导出模型或替换损坏的文件。",
        [LanguageKeys.PreviewErrorNoGeometry] = "此模型没有可显示的网格几何体。"
    };

    private static IReadOnlyDictionary<string, string> English() => new Dictionary<string, string>
    {
        [LanguageKeys.AppTitle] = "Cast Model Merger",
        [LanguageKeys.AppSubtitle] = "Manage and merge Cast parts in concurrent groups; 2–16 parts per group",
        [LanguageKeys.Language] = "Language",
        [LanguageKeys.NewGroup] = "New group",
        [LanguageKeys.SaveSettings] = "Save settings",
        [LanguageKeys.RestoreDefaults] = "Restore defaults",
        [LanguageKeys.StartGroup] = "Start group",
        [LanguageKeys.Cancel] = "Cancel",
        [LanguageKeys.DeleteGroup] = "Delete group",
        [LanguageKeys.ModelParts] = "Model parts",
        [LanguageKeys.ModelPartsHint] = "Select empty slots one at a time, or drop multiple .cast files into this group",
        [LanguageKeys.AddNext] = "Add next",
        [LanguageKeys.Clear] = "Clear",
        [LanguageKeys.RootBadge] = "Root",
        [LanguageKeys.FileMissingBadge] = "Missing",
        [LanguageKeys.SetAsRoot] = "Set root",
        [LanguageKeys.Remove] = "Remove",
        [LanguageKeys.GroupSettings] = "Group settings",
        [LanguageKeys.RootModel] = "Root model",
        [LanguageKeys.Automatic] = "Automatic",
        [LanguageKeys.Manual] = "Manual",
        [LanguageKeys.ManualRootHint] = "In manual mode, select “Set root” on a part slot",
        [LanguageKeys.OutputFolder] = "Output folder",
        [LanguageKeys.OutputFolderHelp] = "Choose the merged output folder for this group",
        [LanguageKeys.Browse] = "Browse",
        [LanguageKeys.OutputFileNameOptional] = "Output file name (optional)",
        [LanguageKeys.OutputFileNameHelp] = "Optional; leave blank to use the root model name",
        [LanguageKeys.OutputFileHint] = "Leave blank to use this group's root model name",
        [LanguageKeys.GroupStatus] = "Group status",
        [LanguageKeys.RunLog] = "Run log",
        [LanguageKeys.OpenGroupOutput] = "Open group output",
        [LanguageKeys.Attribution] = " · Based on Scobalula / echo000 ModelMerger · MIT License",
        [LanguageKeys.RememberOutput] = "Remember the last output folder",
        [LanguageKeys.RememberOutputHint] = "After saving, use it as the default output folder for new groups",
        [LanguageKeys.CancelAll] = "Cancel all",
        [LanguageKeys.MergeAllReady] = "Merge all ready groups",
        [LanguageKeys.PickCastTitle] = "Select a Cast model part",
        [LanguageKeys.CastFilter] = "Cast models (*.cast)|*.cast",
        [LanguageKeys.PickOutputTitle] = "Select the merged model output folder",
        [LanguageKeys.WorkspaceInitial] = "Create a group and add model parts",
        [LanguageKeys.Concurrency] = "Up to {0} groups in parallel",
        [LanguageKeys.SettingsSavedStatus] = "Settings saved; model part paths are never stored",
        [LanguageKeys.SettingsSavedTitle] = "Save settings",
        [LanguageKeys.SettingsSavedMessage] = "Saved the interface language, output folder, first-group root mode, and window position.\nModel file paths are never saved.",
        [LanguageKeys.SettingsSaveFailedTitle] = "Unable to save settings",
        [LanguageKeys.BatchStarted] = "Started {0} groups; {1}",
        [LanguageKeys.BatchFinished] = "Batch complete; checked {0} groups",
        [LanguageKeys.CancelAllStatus] = "Cancelling all running and queued groups",
        [LanguageKeys.RestoreFailedTitle] = "Unable to restore defaults",
        [LanguageKeys.RestoreDoneStatus] = "Defaults restored; existing groups and parts were kept",
        [LanguageKeys.WorkspaceProcessing] = "Processing {0} groups; {1}",
        [LanguageKeys.WorkspaceSummary] = "{0} groups, {1} ready",
        [LanguageKeys.GroupName] = "Group {0}",
        [LanguageKeys.SummaryProcessing] = "{0} · Processing",
        [LanguageKeys.SummaryReady] = "{0} · Ready",
        [LanguageKeys.SummaryNeedTwo] = "{0} · At least 2 parts required",
        [LanguageKeys.StatusInitial] = "Add 2 to 16 Cast parts",
        [LanguageKeys.DroppedPartialTitle] = "{0}: Some files were not added",
        [LanguageKeys.DroppedPartialBody] = "Added {0} parts.\n\n{1}",
        [LanguageKeys.AddPartInvalidPath] = "The file path is invalid",
        [LanguageKeys.AddPartMissing] = "The file does not exist",
        [LanguageKeys.AddPartNotCast] = "Only .cast files are supported",
        [LanguageKeys.AddPartDuplicate] = "This part has already been added",
        [LanguageKeys.AddPartFull] = "This group has reached the 16-part limit",
        [LanguageKeys.AddPartSucceeded] = "Added",
        [LanguageKeys.AddedStatus] = "Added {0} parts",
        [LanguageKeys.QueueWaiting] = "Waiting for an available processing slot",
        [LanguageKeys.QueueLog] = "{0} was queued with {1} parts",
        [LanguageKeys.OverwriteTitle] = "Overwrite output file",
        [LanguageKeys.OverwritePrompt] = "A file for {0} already exists:\n{1}\n\nOverwrite it?",
        [LanguageKeys.OverwriteCancelled] = "Overwrite cancelled",
        [LanguageKeys.MergeCompletedStatus] = "Complete: {0}",
        [LanguageKeys.MergeCompletedLog] = "Merge complete: {0} bones, {1} meshes",
        [LanguageKeys.WarningsHeading] = "Warnings:",
        [LanguageKeys.MergeCompletedTitle] = "{0} merge complete",
        [LanguageKeys.MergeCompletedBody] = "Saved to:\n{0}\n\nBones: {1}    Meshes: {2}{3}",
        [LanguageKeys.CancelledStatus] = "Cancelled; temporary files were cleaned up",
        [LanguageKeys.CancelledLog] = "The user cancelled this group task",
        [LanguageKeys.InvalidRequestStatus] = "The merge request is invalid",
        [LanguageKeys.InvalidRequestTitle] = "{0} cannot start",
        [LanguageKeys.OutputConflictStatus] = "The output path is being used by another group",
        [LanguageKeys.OutputConflictTitle] = "{0} output conflict",
        [LanguageKeys.OutputConflictBody] = "Another group is writing to:\n{0}\n\nWait for it to finish or choose a different output file name.",
        [LanguageKeys.MergeFailedStatus] = "Merge failed",
        [LanguageKeys.MergeFailedTitle] = "{0} merge failed",
        [LanguageKeys.AddFailedTitle] = "Unable to add part",
        [LanguageKeys.RemovedStatus] = "Part removed; now {0}",
        [LanguageKeys.ClearedStatus] = "The part list was cleared",
        [LanguageKeys.ManualRootStatus] = "Set {0} as the root model",
        [LanguageKeys.AddPart] = "Add part",
        [LanguageKeys.ClickCastFile] = "Select a .cast file",
        [LanguageKeys.PartAccessible] = "Part {0}: {1}",
        [LanguageKeys.EmptySlotAccessible] = "Empty part slot {0}",
        [LanguageKeys.CloseBusyTitle] = "Merge in progress",
        [LanguageKeys.CloseBusyMessage] = "Cancel every running or queued merge task before closing the window.",
        [LanguageKeys.CloseRaceMessage] = "A merge task has started. Cancel it before closing the window.",
        [LanguageKeys.ProgressValidating] = "Validating the merge request",
        [LanguageKeys.ProgressLoading] = "Loading {0}",
        [LanguageKeys.ProgressSelectingRoot] = "Selecting the root model",
        [LanguageKeys.ProgressMerging] = "Merging {0}",
        [LanguageKeys.ProgressSaving] = "Saving {0}",
        [LanguageKeys.ProgressVerifying] = "Verifying the saved Cast model",
        [LanguageKeys.ProgressCompleted] = "Saved {0}",
        [LanguageKeys.ProgressGeneric] = "Processing",
        [LanguageKeys.ValidationInvalidPartCount] = "Each group requires 2 to 16 parts.",
        [LanguageKeys.ValidationInvalidPath] = "The model part path is invalid: {0}",
        [LanguageKeys.ValidationMissingFile] = "The model part does not exist: {0}",
        [LanguageKeys.ValidationUnsupportedExtension] = "The GUI only supports .cast model parts: {0}",
        [LanguageKeys.ValidationDuplicateFile] = "The same part cannot be added twice: {0}",
        [LanguageKeys.ValidationInvalidOutputDirectory] = "Choose a valid output folder.",
        [LanguageKeys.ValidationInvalidOutputFileName] = "The output file name is invalid and must use the .cast extension.",
        [LanguageKeys.ValidationOutputAlreadyExists] = "The output file already exists: {0}",
        [LanguageKeys.ValidationManualRootNotSelected] = "The manual root must be one of this group's selected parts.",
        [LanguageKeys.WarningNoAttachmentBone] = "{0} shares no attachment bone with {1}; it was merged without repositioning. Check the root selection and attachment bone names.",
        [LanguageKeys.WarningUnconnectedHierarchy] = "{0} could not connect to the current hierarchy; it was merged without repositioning. Try selecting the root manually or fix the bone hierarchy.",
        [LanguageKeys.ModelPartReadError] = "Unable to read model part: {0}\nFormat: {1}\nRe-export the model or replace the damaged file.",
        [LanguageKeys.Preview] = "Preview",
        [LanguageKeys.PreviewMerged] = "Preview merged model",
        [LanguageKeys.PreviewWindowTitle] = "{0} · Model preview",
        [LanguageKeys.PreviewHeader] = "Model preview",
        [LanguageKeys.PreviewLoading] = "Loading and preparing the model in the background…",
        [LanguageKeys.PreviewInstructions] = "Drag with the left mouse button to rotate and use the wheel to zoom; buttons and arrow, +, − keys also work.",
        [LanguageKeys.PreviewRotateLeft] = "Rotate left",
        [LanguageKeys.PreviewRotateRight] = "Rotate right",
        [LanguageKeys.PreviewZoomIn] = "Zoom in",
        [LanguageKeys.PreviewZoomOut] = "Zoom out",
        [LanguageKeys.PreviewResetView] = "Reset view",
        [LanguageKeys.PreviewClose] = "Close",
        [LanguageKeys.PreviewStats] = "{0} meshes · {1:N0} vertices · {2:N0} triangles",
        [LanguageKeys.PreviewDisplayedStats] = "Displaying {0:N0} / {1:N0} triangles",
        [LanguageKeys.PreviewSimplified] = "The view was sampled to stay responsive; the source file and merged output are unchanged.",
        [LanguageKeys.PreviewErrorTitle] = "Unable to preview model",
        [LanguageKeys.PreviewErrorInvalidPath] = "The model path is invalid. Select the file again.",
        [LanguageKeys.PreviewErrorMissingFile] = "The model file no longer exists. Select it again or rerun the merge.",
        [LanguageKeys.PreviewErrorUnsupportedFormat] = "Preview supports .cast models only.",
        [LanguageKeys.PreviewErrorUnreadableModel] = "This Cast model could not be read. Re-export it or replace the damaged file.",
        [LanguageKeys.PreviewErrorNoGeometry] = "This model has no mesh geometry to display."
    };

    [GeneratedRegex(@"\{(\d+)(?:[^}]*)\}")]
    private static partial Regex PlaceholderPattern();
}

use crate::AppLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    AppTitle,
    AppSubtitle,
    Language,
    NewGroup,
    SaveSettings,
    RestoreDefaults,
    Group,
    ModelParts,
    ModelPartsHint,
    AddNext,
    Clear,
    RootModel,
    Automatic,
    Manual,
    OutputFolder,
    Browse,
    OutputFileName,
    GroupStatus,
    RunLog,
    Preview,
    PreviewMerged,
    StartGroup,
    Cancel,
    DeleteGroup,
    MergeAllReady,
    CancelAll,
    RememberOutput,
    AddPart,
    Replace,
    Remove,
    SetAsRoot,
    RootBadge,
    FileMissing,
    SelectCast,
    SelectOutput,
    SettingsSaved,
    SettingsSaveFailed,
    DefaultsRestored,
    Overwrite,
    OverwriteQuestion,
    Yes,
    No,
    Close,
    PreviewLoading,
    PreviewInstructions,
    RotateLeft,
    RotateRight,
    ZoomIn,
    ZoomOut,
    ResetView,
    PreviewSimplified,
    PreviewFailed,
    PreviewWorkerStopped,
    Meshes,
    Triangles,
    Actions,
    MiSansMissing,
    NeedTwoToFifteen,
    Ready,
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    OutputConflict,
    MergeFailed,
    Completed,
    Concurrency,
    AddPartInvalidPath,
    AddPartMissing,
    AddPartNotCast,
    AddPartDuplicate,
    AddPartFull,
    ValidationInvalidPartCount,
    ValidationInvalidPath,
    ValidationMissingFile,
    ValidationUnsupportedExtension,
    ValidationDuplicateFile,
    ValidationInvalidOutputDirectory,
    ValidationInvalidOutputFileName,
    ValidationOutputAlreadyExists,
    ValidationManualRootNotSelected,
    WarningNoAttachmentBone,
    WarningUnconnectedHierarchy,
    ProgressValidating,
    ProgressLoading,
    ProgressSelectingRoot,
    ProgressMerging,
    ProgressSaving,
    ProgressVerifying,
    ProgressCompleted,
    QueueWaiting,
    MergeCompletedLog,
    CancelledLog,
    ModelPartReadError,
    Attribution,
}

impl TextKey {
    pub const ALL: &'static [Self] = &[
        Self::AppTitle,
        Self::AppSubtitle,
        Self::Language,
        Self::NewGroup,
        Self::SaveSettings,
        Self::RestoreDefaults,
        Self::Group,
        Self::ModelParts,
        Self::ModelPartsHint,
        Self::AddNext,
        Self::Clear,
        Self::RootModel,
        Self::Automatic,
        Self::Manual,
        Self::OutputFolder,
        Self::Browse,
        Self::OutputFileName,
        Self::GroupStatus,
        Self::RunLog,
        Self::Preview,
        Self::PreviewMerged,
        Self::StartGroup,
        Self::Cancel,
        Self::DeleteGroup,
        Self::MergeAllReady,
        Self::CancelAll,
        Self::RememberOutput,
        Self::AddPart,
        Self::Replace,
        Self::Remove,
        Self::SetAsRoot,
        Self::RootBadge,
        Self::FileMissing,
        Self::SelectCast,
        Self::SelectOutput,
        Self::SettingsSaved,
        Self::SettingsSaveFailed,
        Self::DefaultsRestored,
        Self::Overwrite,
        Self::OverwriteQuestion,
        Self::Yes,
        Self::No,
        Self::Close,
        Self::PreviewLoading,
        Self::PreviewInstructions,
        Self::RotateLeft,
        Self::RotateRight,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::ResetView,
        Self::PreviewSimplified,
        Self::PreviewFailed,
        Self::PreviewWorkerStopped,
        Self::Meshes,
        Self::Triangles,
        Self::Actions,
        Self::MiSansMissing,
        Self::NeedTwoToFifteen,
        Self::Ready,
        Self::Queued,
        Self::Running,
        Self::Succeeded,
        Self::Failed,
        Self::Cancelled,
        Self::OutputConflict,
        Self::MergeFailed,
        Self::Completed,
        Self::Concurrency,
        Self::AddPartInvalidPath,
        Self::AddPartMissing,
        Self::AddPartNotCast,
        Self::AddPartDuplicate,
        Self::AddPartFull,
        Self::ValidationInvalidPartCount,
        Self::ValidationInvalidPath,
        Self::ValidationMissingFile,
        Self::ValidationUnsupportedExtension,
        Self::ValidationDuplicateFile,
        Self::ValidationInvalidOutputDirectory,
        Self::ValidationInvalidOutputFileName,
        Self::ValidationOutputAlreadyExists,
        Self::ValidationManualRootNotSelected,
        Self::WarningNoAttachmentBone,
        Self::WarningUnconnectedHierarchy,
        Self::ProgressValidating,
        Self::ProgressLoading,
        Self::ProgressSelectingRoot,
        Self::ProgressMerging,
        Self::ProgressSaving,
        Self::ProgressVerifying,
        Self::ProgressCompleted,
        Self::QueueWaiting,
        Self::MergeCompletedLog,
        Self::CancelledLog,
        Self::ModelPartReadError,
        Self::Attribution,
    ];
}

#[derive(Debug, Clone, Copy)]
pub struct Catalog {
    language: AppLanguage,
}

impl Catalog {
    pub fn new(language: AppLanguage) -> Self {
        Self { language }
    }

    pub fn language(&self) -> AppLanguage {
        self.language
    }

    pub fn language_name(&self) -> &'static str {
        match self.language {
            AppLanguage::ChineseSimplified => "中文",
            AppLanguage::English => "English",
            AppLanguage::French => "Français",
            AppLanguage::Russian => "Русский",
            AppLanguage::Spanish => "Español",
        }
    }

    pub fn text(&self, key: TextKey) -> &'static str {
        match self.language {
            AppLanguage::ChineseSimplified => chinese(key),
            AppLanguage::English => english(key),
            AppLanguage::French => french(key),
            AppLanguage::Russian => russian(key),
            AppLanguage::Spanish => spanish(key),
        }
    }
}

fn chinese(key: TextKey) -> &'static str {
    match key {
        TextKey::AppTitle => "Cast 模型合并器",
        TextKey::AppSubtitle => "按模型组管理和并发合并 Cast 部件；每组支持 2–15 个部件",
        TextKey::Language => "界面语言",
        TextKey::NewGroup => "新建模型组",
        TextKey::SaveSettings => "保存设置",
        TextKey::RestoreDefaults => "恢复默认",
        TextKey::Group => "模型组",
        TextKey::ModelParts => "模型部件",
        TextKey::ModelPartsHint => "点击“添加下一个”或空槽可多选 .cast 文件，也可拖入本组",
        TextKey::AddNext => "添加下一个",
        TextKey::Clear => "清空",
        TextKey::RootModel => "根模型",
        TextKey::Automatic => "自动识别",
        TextKey::Manual => "手动指定",
        TextKey::OutputFolder => "输出文件夹",
        TextKey::Browse => "浏览",
        TextKey::OutputFileName => "输出文件名（可选）",
        TextKey::GroupStatus => "本组状态",
        TextKey::RunLog => "运行日志",
        TextKey::Preview => "预览",
        TextKey::PreviewMerged => "预览合并模型",
        TextKey::StartGroup => "开始本组",
        TextKey::Cancel => "取消",
        TextKey::DeleteGroup => "删除组",
        TextKey::MergeAllReady => "合并全部就绪组",
        TextKey::CancelAll => "取消全部",
        TextKey::RememberOutput => "记住最近选择的输出目录",
        TextKey::AddPart => "添加部件",
        TextKey::Replace => "替换",
        TextKey::Remove => "移除",
        TextKey::SetAsRoot => "设为根",
        TextKey::RootBadge => "根",
        TextKey::FileMissing => "文件缺失",
        TextKey::SelectCast => "选择一个或多个 Cast 模型部件",
        TextKey::SelectOutput => "选择合并模型输出文件夹",
        TextKey::SettingsSaved => "设置已保存；不会保存模型路径",
        TextKey::SettingsSaveFailed => "无法保存设置",
        TextKey::DefaultsRestored => "已恢复默认设置",
        TextKey::Overwrite => "覆盖输出文件",
        TextKey::OverwriteQuestion => "输出文件已存在，是否覆盖？",
        TextKey::Yes => "是",
        TextKey::No => "否",
        TextKey::Close => "关闭",
        TextKey::PreviewLoading => "正在后台加载并准备模型…",
        TextKey::PreviewInstructions => "左键拖动旋转，滚轮缩放；也可使用按钮和键盘操作。",
        TextKey::RotateLeft => "向左旋转",
        TextKey::RotateRight => "向右旋转",
        TextKey::ZoomIn => "放大",
        TextKey::ZoomOut => "缩小",
        TextKey::ResetView => "重置视角",
        TextKey::PreviewSimplified => "为保持流畅已抽样显示；不会修改源文件和合并输出。",
        TextKey::PreviewFailed => "无法预览此 Cast 文件；请检查文件是否完整且包含模型网格。",
        TextKey::PreviewWorkerStopped => "预览任务意外停止；请关闭窗口后重试。",
        TextKey::Meshes => "网格",
        TextKey::Triangles => "三角形",
        TextKey::Actions => "操作",
        TextKey::MiSansMissing => "未找到 MiSans Medium 字体；请重新解压完整发布包。",
        TextKey::NeedTwoToFifteen => "添加 2 至 15 个 Cast 部件",
        TextKey::Ready => "已就绪",
        TextKey::Queued => "等待处理",
        TextKey::Running => "正在处理",
        TextKey::Succeeded => "已完成",
        TextKey::Failed => "失败",
        TextKey::Cancelled => "已取消",
        TextKey::OutputConflict => "另一个组正在使用相同输出路径",
        TextKey::MergeFailed => "合并失败",
        TextKey::Completed => "合并完成",
        TextKey::Concurrency => "最多 2 组并行",
        TextKey::AddPartInvalidPath => "文件路径无效",
        TextKey::AddPartMissing => "文件不存在",
        TextKey::AddPartNotCast => "仅支持 .cast 文件",
        TextKey::AddPartDuplicate => "该部件已添加",
        TextKey::AddPartFull => "本组已达到 15 个部件上限",
        TextKey::ValidationInvalidPartCount => "每组必须包含 2 至 15 个部件。",
        TextKey::ValidationInvalidPath => "部件路径无效：{0}",
        TextKey::ValidationMissingFile => "模型部件不存在：{0}",
        TextKey::ValidationUnsupportedExtension => "仅支持 .cast 部件：{0}",
        TextKey::ValidationDuplicateFile => "同一部件不能添加两次：{0}",
        TextKey::ValidationInvalidOutputDirectory => "请选择有效的输出文件夹。",
        TextKey::ValidationInvalidOutputFileName => "输出文件名无效，且必须使用 .cast 扩展名。",
        TextKey::ValidationOutputAlreadyExists => "输出文件已存在：{0}",
        TextKey::ValidationManualRootNotSelected => "手动根模型必须是本组已选择的部件。",
        TextKey::WarningNoAttachmentBone => "{0} 与 {1} 没有共同连接骨骼；已原位合并。",
        TextKey::WarningUnconnectedHierarchy => "{0} 无法连接到当前骨架层级；已原位合并。",
        TextKey::ProgressValidating => "正在验证合并请求",
        TextKey::ProgressLoading => "正在加载 {0}",
        TextKey::ProgressSelectingRoot => "正在选择根模型",
        TextKey::ProgressMerging => "正在合并 {0}",
        TextKey::ProgressSaving => "正在保存 {0}",
        TextKey::ProgressVerifying => "正在验证已保存的 Cast 模型",
        TextKey::ProgressCompleted => "已保存：{0}",
        TextKey::QueueWaiting => "正在等待可用处理槽位",
        TextKey::MergeCompletedLog => "合并完成：{0} 个骨骼，{1} 个网格",
        TextKey::CancelledLog => "用户取消了本组任务",
        TextKey::ModelPartReadError => "无法读取模型部件：{0}\n格式：{1}",
        TextKey::Attribution => {
            "基于 Scobalula / echo000 ModelMerger · MIT License · 中文界面使用 MiSans"
        }
    }
}

fn english(key: TextKey) -> &'static str {
    match key {
        TextKey::AppTitle => "Cast Model Merger",
        TextKey::AppSubtitle => {
            "Manage and merge Cast parts in parallel groups; 2–15 parts per group"
        }
        TextKey::Language => "Language",
        TextKey::NewGroup => "New model group",
        TextKey::SaveSettings => "Save settings",
        TextKey::RestoreDefaults => "Restore defaults",
        TextKey::Group => "Model group",
        TextKey::ModelParts => "Model parts",
        TextKey::ModelPartsHint => {
            "Use Add next or an empty slot to select multiple .cast files, or drop them here"
        }
        TextKey::AddNext => "Add next",
        TextKey::Clear => "Clear",
        TextKey::RootModel => "Root model",
        TextKey::Automatic => "Automatic",
        TextKey::Manual => "Manual",
        TextKey::OutputFolder => "Output folder",
        TextKey::Browse => "Browse",
        TextKey::OutputFileName => "Output file name (optional)",
        TextKey::GroupStatus => "Group status",
        TextKey::RunLog => "Run log",
        TextKey::Preview => "Preview",
        TextKey::PreviewMerged => "Preview merged model",
        TextKey::StartGroup => "Start group",
        TextKey::Cancel => "Cancel",
        TextKey::DeleteGroup => "Delete group",
        TextKey::MergeAllReady => "Merge all ready groups",
        TextKey::CancelAll => "Cancel all",
        TextKey::RememberOutput => "Remember the most recent output folder",
        TextKey::AddPart => "Add part",
        TextKey::Replace => "Replace",
        TextKey::Remove => "Remove",
        TextKey::SetAsRoot => "Set as root",
        TextKey::RootBadge => "Root",
        TextKey::FileMissing => "File missing",
        TextKey::SelectCast => "Select one or more Cast model parts",
        TextKey::SelectOutput => "Select merged-model output folder",
        TextKey::SettingsSaved => "Settings saved; model paths are never stored",
        TextKey::SettingsSaveFailed => "Could not save settings",
        TextKey::DefaultsRestored => "Default settings restored",
        TextKey::Overwrite => "Replace output file",
        TextKey::OverwriteQuestion => "The output file already exists. Replace it?",
        TextKey::Yes => "Yes",
        TextKey::No => "No",
        TextKey::Close => "Close",
        TextKey::PreviewLoading => "Loading and preparing the model in the background…",
        TextKey::PreviewInstructions => {
            "Drag with the left mouse button to rotate and use the wheel to zoom; buttons and keyboard also work."
        }
        TextKey::RotateLeft => "Rotate left",
        TextKey::RotateRight => "Rotate right",
        TextKey::ZoomIn => "Zoom in",
        TextKey::ZoomOut => "Zoom out",
        TextKey::ResetView => "Reset view",
        TextKey::PreviewSimplified => {
            "The view is sampled for responsiveness; source and merged output are unchanged."
        }
        TextKey::PreviewFailed => {
            "This Cast file cannot be previewed; check that it is complete and contains model meshes."
        }
        TextKey::PreviewWorkerStopped => {
            "The preview task stopped unexpectedly; close this window and try again."
        }
        TextKey::Meshes => "meshes",
        TextKey::Triangles => "triangles",
        TextKey::Actions => "Actions",
        TextKey::MiSansMissing => {
            "MiSans Medium is missing; extract the complete release package again."
        }
        TextKey::NeedTwoToFifteen => "Add 2 to 15 Cast parts",
        TextKey::Ready => "Ready",
        TextKey::Queued => "Queued",
        TextKey::Running => "Running",
        TextKey::Succeeded => "Completed",
        TextKey::Failed => "Failed",
        TextKey::Cancelled => "Cancelled",
        TextKey::OutputConflict => "Another group is using the same output path",
        TextKey::MergeFailed => "Merge failed",
        TextKey::Completed => "Merge completed",
        TextKey::Concurrency => "Up to 2 groups in parallel",
        TextKey::AddPartInvalidPath => "The file path is invalid",
        TextKey::AddPartMissing => "The file does not exist",
        TextKey::AddPartNotCast => "Only .cast files are supported",
        TextKey::AddPartDuplicate => "This part has already been added",
        TextKey::AddPartFull => "This group has reached the 15-part limit",
        TextKey::ValidationInvalidPartCount => "Each group must contain 2 to 15 parts.",
        TextKey::ValidationInvalidPath => "The part path is invalid: {0}",
        TextKey::ValidationMissingFile => "The model part does not exist: {0}",
        TextKey::ValidationUnsupportedExtension => "Only .cast parts are supported: {0}",
        TextKey::ValidationDuplicateFile => "The same part cannot be added twice: {0}",
        TextKey::ValidationInvalidOutputDirectory => "Choose a valid output folder.",
        TextKey::ValidationInvalidOutputFileName => {
            "The output file name is invalid and must use .cast."
        }
        TextKey::ValidationOutputAlreadyExists => "The output file already exists: {0}",
        TextKey::ValidationManualRootNotSelected => "The manual root must be a selected part.",
        TextKey::WarningNoAttachmentBone => {
            "{0} shares no attachment bone with {1}; it was merged in place."
        }
        TextKey::WarningUnconnectedHierarchy => {
            "{0} could not connect to the current hierarchy; it was merged in place."
        }
        TextKey::ProgressValidating => "Validating the merge request",
        TextKey::ProgressLoading => "Loading {0}",
        TextKey::ProgressSelectingRoot => "Selecting the root model",
        TextKey::ProgressMerging => "Merging {0}",
        TextKey::ProgressSaving => "Saving {0}",
        TextKey::ProgressVerifying => "Verifying the saved Cast model",
        TextKey::ProgressCompleted => "Saved: {0}",
        TextKey::QueueWaiting => "Waiting for an available processing slot",
        TextKey::MergeCompletedLog => "Merge completed: {0} bones, {1} meshes",
        TextKey::CancelledLog => "The user cancelled this group task",
        TextKey::ModelPartReadError => "Could not read model part: {0}\nFormat: {1}",
        TextKey::Attribution => "Based on Scobalula / echo000 ModelMerger · MIT License",
    }
}

fn french(key: TextKey) -> &'static str {
    match key {
        TextKey::AppTitle => "Fusionneur de modèles Cast",
        TextKey::AppSubtitle => {
            "Gérez et fusionnez des pièces Cast par groupes parallèles ; 2 à 15 pièces par groupe"
        }
        TextKey::Language => "Langue",
        TextKey::NewGroup => "Nouveau groupe",
        TextKey::SaveSettings => "Enregistrer",
        TextKey::RestoreDefaults => "Valeurs par défaut",
        TextKey::Group => "Groupe de modèles",
        TextKey::ModelParts => "Pièces du modèle",
        TextKey::ModelPartsHint => {
            "Ajoutez plusieurs fichiers .cast avec Ajouter ou un emplacement vide, ou déposez-les ici"
        }
        TextKey::AddNext => "Ajouter le suivant",
        TextKey::Clear => "Effacer",
        TextKey::RootModel => "Modèle racine",
        TextKey::Automatic => "Automatique",
        TextKey::Manual => "Manuel",
        TextKey::OutputFolder => "Dossier de sortie",
        TextKey::Browse => "Parcourir",
        TextKey::OutputFileName => "Nom du fichier (facultatif)",
        TextKey::GroupStatus => "État du groupe",
        TextKey::RunLog => "Journal",
        TextKey::Preview => "Aperçu",
        TextKey::PreviewMerged => "Aperçu du modèle fusionné",
        TextKey::StartGroup => "Démarrer le groupe",
        TextKey::Cancel => "Annuler",
        TextKey::DeleteGroup => "Supprimer le groupe",
        TextKey::MergeAllReady => "Fusionner tous les groupes prêts",
        TextKey::CancelAll => "Tout annuler",
        TextKey::RememberOutput => "Mémoriser le dernier dossier de sortie",
        TextKey::AddPart => "Ajouter une pièce",
        TextKey::Replace => "Remplacer",
        TextKey::Remove => "Retirer",
        TextKey::SetAsRoot => "Définir comme racine",
        TextKey::RootBadge => "Racine",
        TextKey::FileMissing => "Fichier manquant",
        TextKey::SelectCast => "Sélectionner une ou plusieurs pièces Cast",
        TextKey::SelectOutput => "Sélectionner le dossier de sortie",
        TextKey::SettingsSaved => {
            "Paramètres enregistrés ; les chemins des modèles ne le sont jamais"
        }
        TextKey::SettingsSaveFailed => "Impossible d’enregistrer les paramètres",
        TextKey::DefaultsRestored => "Paramètres par défaut restaurés",
        TextKey::Overwrite => "Remplacer le fichier de sortie",
        TextKey::OverwriteQuestion => "Le fichier de sortie existe déjà. Le remplacer ?",
        TextKey::Yes => "Oui",
        TextKey::No => "Non",
        TextKey::Close => "Fermer",
        TextKey::PreviewLoading => "Chargement et préparation du modèle en arrière-plan…",
        TextKey::PreviewInstructions => {
            "Faites glisser avec le bouton gauche pour pivoter et utilisez la molette pour zoomer."
        }
        TextKey::RotateLeft => "Pivoter à gauche",
        TextKey::RotateRight => "Pivoter à droite",
        TextKey::ZoomIn => "Zoom avant",
        TextKey::ZoomOut => "Zoom arrière",
        TextKey::ResetView => "Réinitialiser la vue",
        TextKey::PreviewSimplified => {
            "L’aperçu est échantillonné pour rester fluide ; les fichiers ne sont pas modifiés."
        }
        TextKey::PreviewFailed => {
            "Impossible d’afficher ce fichier Cast ; vérifiez qu’il est complet et contient des maillages."
        }
        TextKey::PreviewWorkerStopped => {
            "La tâche d’aperçu s’est arrêtée ; fermez cette fenêtre et réessayez."
        }
        TextKey::Meshes => "maillages",
        TextKey::Triangles => "triangles",
        TextKey::Actions => "Actions",
        TextKey::MiSansMissing => {
            "MiSans Medium est absent ; extrayez à nouveau le paquet complet."
        }
        TextKey::NeedTwoToFifteen => "Ajoutez 2 à 15 pièces Cast",
        TextKey::Ready => "Prêt",
        TextKey::Queued => "En attente",
        TextKey::Running => "En cours",
        TextKey::Succeeded => "Terminé",
        TextKey::Failed => "Échec",
        TextKey::Cancelled => "Annulé",
        TextKey::OutputConflict => "Un autre groupe utilise le même chemin de sortie",
        TextKey::MergeFailed => "Échec de la fusion",
        TextKey::Completed => "Fusion terminée",
        TextKey::Concurrency => "Jusqu’à 2 groupes en parallèle",
        TextKey::AddPartInvalidPath => "Le chemin du fichier n’est pas valide",
        TextKey::AddPartMissing => "Le fichier n’existe pas",
        TextKey::AddPartNotCast => "Seuls les fichiers .cast sont pris en charge",
        TextKey::AddPartDuplicate => "Cette pièce a déjà été ajoutée",
        TextKey::AddPartFull => "Ce groupe a atteint la limite de 15 pièces",
        TextKey::ValidationInvalidPartCount => "Chaque groupe doit contenir 2 à 15 pièces.",
        TextKey::ValidationInvalidPath => "Le chemin de la pièce n’est pas valide : {0}",
        TextKey::ValidationMissingFile => "La pièce du modèle n’existe pas : {0}",
        TextKey::ValidationUnsupportedExtension => "Seules les pièces .cast sont acceptées : {0}",
        TextKey::ValidationDuplicateFile => {
            "La même pièce ne peut pas être ajoutée deux fois : {0}"
        }
        TextKey::ValidationInvalidOutputDirectory => "Choisissez un dossier de sortie valide.",
        TextKey::ValidationInvalidOutputFileName => {
            "Le nom de sortie doit être valide et utiliser .cast."
        }
        TextKey::ValidationOutputAlreadyExists => "Le fichier de sortie existe déjà : {0}",
        TextKey::ValidationManualRootNotSelected => {
            "La racine manuelle doit être une pièce sélectionnée."
        }
        TextKey::WarningNoAttachmentBone => {
            "{0} ne partage aucun os de liaison avec {1} ; fusion sur place."
        }
        TextKey::WarningUnconnectedHierarchy => {
            "{0} n’a pas pu rejoindre la hiérarchie ; fusion sur place."
        }
        TextKey::ProgressValidating => "Validation de la demande de fusion",
        TextKey::ProgressLoading => "Chargement de {0}",
        TextKey::ProgressSelectingRoot => "Sélection du modèle racine",
        TextKey::ProgressMerging => "Fusion de {0}",
        TextKey::ProgressSaving => "Enregistrement de {0}",
        TextKey::ProgressVerifying => "Vérification du modèle Cast enregistré",
        TextKey::ProgressCompleted => "Enregistré : {0}",
        TextKey::QueueWaiting => "En attente d’un emplacement de traitement",
        TextKey::MergeCompletedLog => "Fusion terminée : {0} os, {1} maillages",
        TextKey::CancelledLog => "L’utilisateur a annulé la tâche de ce groupe",
        TextKey::ModelPartReadError => "Impossible de lire la pièce : {0}\nFormat : {1}",
        TextKey::Attribution => "Basé sur Scobalula / echo000 ModelMerger · Licence MIT",
    }
}

fn russian(key: TextKey) -> &'static str {
    match key {
        TextKey::AppTitle => "Объединение моделей Cast",
        TextKey::AppSubtitle => {
            "Управляйте группами деталей Cast и объединяйте их параллельно; 2–15 деталей в группе"
        }
        TextKey::Language => "Язык интерфейса",
        TextKey::NewGroup => "Новая группа",
        TextKey::SaveSettings => "Сохранить настройки",
        TextKey::RestoreDefaults => "По умолчанию",
        TextKey::Group => "Группа моделей",
        TextKey::ModelParts => "Детали модели",
        TextKey::ModelPartsHint => {
            "Выберите несколько файлов .cast через добавление или пустую ячейку либо перетащите их сюда"
        }
        TextKey::AddNext => "Добавить следующую",
        TextKey::Clear => "Очистить",
        TextKey::RootModel => "Корневая модель",
        TextKey::Automatic => "Автоматически",
        TextKey::Manual => "Вручную",
        TextKey::OutputFolder => "Папка вывода",
        TextKey::Browse => "Обзор",
        TextKey::OutputFileName => "Имя выходного файла (необязательно)",
        TextKey::GroupStatus => "Состояние группы",
        TextKey::RunLog => "Журнал",
        TextKey::Preview => "Предпросмотр",
        TextKey::PreviewMerged => "Предпросмотр объединённой модели",
        TextKey::StartGroup => "Запустить группу",
        TextKey::Cancel => "Отмена",
        TextKey::DeleteGroup => "Удалить группу",
        TextKey::MergeAllReady => "Объединить все готовые группы",
        TextKey::CancelAll => "Отменить всё",
        TextKey::RememberOutput => "Запомнить последнюю папку вывода",
        TextKey::AddPart => "Добавить деталь",
        TextKey::Replace => "Заменить",
        TextKey::Remove => "Удалить",
        TextKey::SetAsRoot => "Сделать корневой",
        TextKey::RootBadge => "Корень",
        TextKey::FileMissing => "Файл отсутствует",
        TextKey::SelectCast => "Выберите одну или несколько деталей Cast",
        TextKey::SelectOutput => "Выберите папку вывода",
        TextKey::SettingsSaved => "Настройки сохранены; пути моделей не сохраняются",
        TextKey::SettingsSaveFailed => "Не удалось сохранить настройки",
        TextKey::DefaultsRestored => "Настройки по умолчанию восстановлены",
        TextKey::Overwrite => "Заменить выходной файл",
        TextKey::OverwriteQuestion => "Выходной файл уже существует. Заменить его?",
        TextKey::Yes => "Да",
        TextKey::No => "Нет",
        TextKey::Close => "Закрыть",
        TextKey::PreviewLoading => "Модель загружается и подготавливается в фоне…",
        TextKey::PreviewInstructions => {
            "Перетаскивайте левой кнопкой для вращения и используйте колесо для масштаба."
        }
        TextKey::RotateLeft => "Повернуть влево",
        TextKey::RotateRight => "Повернуть вправо",
        TextKey::ZoomIn => "Приблизить",
        TextKey::ZoomOut => "Отдалить",
        TextKey::ResetView => "Сбросить вид",
        TextKey::PreviewSimplified => {
            "Для плавности предпросмотр упрощён; исходный и выходной файлы не меняются."
        }
        TextKey::PreviewFailed => {
            "Не удалось показать файл Cast; проверьте его целостность и наличие сеток модели."
        }
        TextKey::PreviewWorkerStopped => {
            "Задача предпросмотра остановилась; закройте окно и повторите попытку."
        }
        TextKey::Meshes => "сеток",
        TextKey::Triangles => "треугольников",
        TextKey::Actions => "Действия",
        TextKey::MiSansMissing => "Шрифт MiSans Medium не найден; распакуйте полный пакет ещё раз.",
        TextKey::NeedTwoToFifteen => "Добавьте от 2 до 15 деталей Cast",
        TextKey::Ready => "Готово",
        TextKey::Queued => "В очереди",
        TextKey::Running => "Выполняется",
        TextKey::Succeeded => "Завершено",
        TextKey::Failed => "Ошибка",
        TextKey::Cancelled => "Отменено",
        TextKey::OutputConflict => "Другая группа использует тот же путь вывода",
        TextKey::MergeFailed => "Ошибка объединения",
        TextKey::Completed => "Объединение завершено",
        TextKey::Concurrency => "До 2 групп параллельно",
        TextKey::AddPartInvalidPath => "Недопустимый путь к файлу",
        TextKey::AddPartMissing => "Файл не существует",
        TextKey::AddPartNotCast => "Поддерживаются только файлы .cast",
        TextKey::AddPartDuplicate => "Эта деталь уже добавлена",
        TextKey::AddPartFull => "Достигнут предел в 15 деталей",
        TextKey::ValidationInvalidPartCount => "В группе должно быть от 2 до 15 деталей.",
        TextKey::ValidationInvalidPath => "Недопустимый путь детали: {0}",
        TextKey::ValidationMissingFile => "Деталь модели не существует: {0}",
        TextKey::ValidationUnsupportedExtension => "Поддерживаются только детали .cast: {0}",
        TextKey::ValidationDuplicateFile => "Нельзя добавить одну деталь дважды: {0}",
        TextKey::ValidationInvalidOutputDirectory => "Выберите допустимую папку вывода.",
        TextKey::ValidationInvalidOutputFileName => {
            "Имя выхода должно быть допустимым и иметь .cast."
        }
        TextKey::ValidationOutputAlreadyExists => "Выходной файл уже существует: {0}",
        TextKey::ValidationManualRootNotSelected => "Корень должен быть выбранной деталью.",
        TextKey::WarningNoAttachmentBone => {
            "У {0} и {1} нет общей соединительной кости; объединено на месте."
        }
        TextKey::WarningUnconnectedHierarchy => {
            "{0} не удалось подключить к иерархии; объединено на месте."
        }
        TextKey::ProgressValidating => "Проверка запроса на объединение",
        TextKey::ProgressLoading => "Загрузка {0}",
        TextKey::ProgressSelectingRoot => "Выбор корневой модели",
        TextKey::ProgressMerging => "Объединение {0}",
        TextKey::ProgressSaving => "Сохранение {0}",
        TextKey::ProgressVerifying => "Проверка сохранённой модели Cast",
        TextKey::ProgressCompleted => "Сохранено: {0}",
        TextKey::QueueWaiting => "Ожидание свободного места обработки",
        TextKey::MergeCompletedLog => "Объединение завершено: костей {0}, сеток {1}",
        TextKey::CancelledLog => "Пользователь отменил задачу группы",
        TextKey::ModelPartReadError => "Не удалось прочитать деталь: {0}\nФормат: {1}",
        TextKey::Attribution => "На основе Scobalula / echo000 ModelMerger · Лицензия MIT",
    }
}

fn spanish(key: TextKey) -> &'static str {
    match key {
        TextKey::AppTitle => "Combinador de modelos Cast",
        TextKey::AppSubtitle => {
            "Gestiona y combina piezas Cast en grupos paralelos; de 2 a 15 piezas por grupo"
        }
        TextKey::Language => "Idioma",
        TextKey::NewGroup => "Nuevo grupo",
        TextKey::SaveSettings => "Guardar ajustes",
        TextKey::RestoreDefaults => "Restaurar valores",
        TextKey::Group => "Grupo de modelos",
        TextKey::ModelParts => "Piezas del modelo",
        TextKey::ModelPartsHint => {
            "Selecciona varios archivos .cast con Añadir o un espacio vacío, o arrástralos aquí"
        }
        TextKey::AddNext => "Añadir siguiente",
        TextKey::Clear => "Vaciar",
        TextKey::RootModel => "Modelo raíz",
        TextKey::Automatic => "Automático",
        TextKey::Manual => "Manual",
        TextKey::OutputFolder => "Carpeta de salida",
        TextKey::Browse => "Examinar",
        TextKey::OutputFileName => "Nombre del archivo (opcional)",
        TextKey::GroupStatus => "Estado del grupo",
        TextKey::RunLog => "Registro",
        TextKey::Preview => "Vista previa",
        TextKey::PreviewMerged => "Vista previa del modelo combinado",
        TextKey::StartGroup => "Iniciar grupo",
        TextKey::Cancel => "Cancelar",
        TextKey::DeleteGroup => "Eliminar grupo",
        TextKey::MergeAllReady => "Combinar todos los grupos listos",
        TextKey::CancelAll => "Cancelar todo",
        TextKey::RememberOutput => "Recordar la última carpeta de salida",
        TextKey::AddPart => "Añadir pieza",
        TextKey::Replace => "Reemplazar",
        TextKey::Remove => "Quitar",
        TextKey::SetAsRoot => "Fijar raíz",
        TextKey::RootBadge => "Raíz",
        TextKey::FileMissing => "Falta el archivo",
        TextKey::SelectCast => "Seleccionar una o varias piezas Cast",
        TextKey::SelectOutput => "Seleccionar la carpeta de salida",
        TextKey::SettingsSaved => "Ajustes guardados; las rutas de modelos nunca se almacenan",
        TextKey::SettingsSaveFailed => "No se pudieron guardar los ajustes",
        TextKey::DefaultsRestored => "Valores predeterminados restaurados",
        TextKey::Overwrite => "Reemplazar archivo de salida",
        TextKey::OverwriteQuestion => "El archivo de salida ya existe. ¿Quieres reemplazarlo?",
        TextKey::Yes => "Sí",
        TextKey::No => "No",
        TextKey::Close => "Cerrar",
        TextKey::PreviewLoading => "Cargando y preparando el modelo en segundo plano…",
        TextKey::PreviewInstructions => {
            "Arrastra con el botón izquierdo para girar y usa la rueda para acercar o alejar."
        }
        TextKey::RotateLeft => "Girar a la izquierda",
        TextKey::RotateRight => "Girar a la derecha",
        TextKey::ZoomIn => "Acercar",
        TextKey::ZoomOut => "Alejar",
        TextKey::ResetView => "Restablecer vista",
        TextKey::PreviewSimplified => {
            "La vista se ha muestreado para mantener la fluidez; los archivos no se modifican."
        }
        TextKey::PreviewFailed => {
            "No se puede previsualizar este archivo Cast; comprueba que esté completo y tenga mallas."
        }
        TextKey::PreviewWorkerStopped => {
            "La tarea de vista previa se detuvo; cierra esta ventana e inténtalo de nuevo."
        }
        TextKey::Meshes => "mallas",
        TextKey::Triangles => "triángulos",
        TextKey::Actions => "Acciones",
        TextKey::MiSansMissing => "Falta MiSans Medium; vuelve a extraer el paquete completo.",
        TextKey::NeedTwoToFifteen => "Añade de 2 a 15 piezas Cast",
        TextKey::Ready => "Listo",
        TextKey::Queued => "En espera",
        TextKey::Running => "Procesando",
        TextKey::Succeeded => "Completado",
        TextKey::Failed => "Error",
        TextKey::Cancelled => "Cancelado",
        TextKey::OutputConflict => "Otro grupo está usando la misma ruta de salida",
        TextKey::MergeFailed => "Error al combinar",
        TextKey::Completed => "Combinación completada",
        TextKey::Concurrency => "Hasta 2 grupos en paralelo",
        TextKey::AddPartInvalidPath => "La ruta del archivo no es válida",
        TextKey::AddPartMissing => "El archivo no existe",
        TextKey::AddPartNotCast => "Solo se admiten archivos .cast",
        TextKey::AddPartDuplicate => "Esta pieza ya se ha añadido",
        TextKey::AddPartFull => "Este grupo ha alcanzado el límite de 15 piezas",
        TextKey::ValidationInvalidPartCount => "Cada grupo debe tener entre 2 y 15 piezas.",
        TextKey::ValidationInvalidPath => "La ruta de la pieza no es válida: {0}",
        TextKey::ValidationMissingFile => "La pieza del modelo no existe: {0}",
        TextKey::ValidationUnsupportedExtension => "Solo se admiten piezas .cast: {0}",
        TextKey::ValidationDuplicateFile => "No se puede añadir la misma pieza dos veces: {0}",
        TextKey::ValidationInvalidOutputDirectory => "Elige una carpeta de salida válida.",
        TextKey::ValidationInvalidOutputFileName => {
            "El nombre de salida debe ser válido y usar .cast."
        }
        TextKey::ValidationOutputAlreadyExists => "El archivo de salida ya existe: {0}",
        TextKey::ValidationManualRootNotSelected => {
            "La raíz manual debe ser una pieza seleccionada."
        }
        TextKey::WarningNoAttachmentBone => {
            "{0} no comparte ningún hueso de unión con {1}; se combinó en su sitio."
        }
        TextKey::WarningUnconnectedHierarchy => {
            "{0} no pudo conectarse a la jerarquía; se combinó en su sitio."
        }
        TextKey::ProgressValidating => "Validando la solicitud de combinación",
        TextKey::ProgressLoading => "Cargando {0}",
        TextKey::ProgressSelectingRoot => "Seleccionando el modelo raíz",
        TextKey::ProgressMerging => "Combinando {0}",
        TextKey::ProgressSaving => "Guardando {0}",
        TextKey::ProgressVerifying => "Verificando el modelo Cast guardado",
        TextKey::ProgressCompleted => "Guardado: {0}",
        TextKey::QueueWaiting => "Esperando un puesto de procesamiento disponible",
        TextKey::MergeCompletedLog => "Combinación completada: {0} huesos, {1} mallas",
        TextKey::CancelledLog => "El usuario canceló la tarea de este grupo",
        TextKey::ModelPartReadError => "No se pudo leer la pieza: {0}\nFormato: {1}",
        TextKey::Attribution => "Basado en Scobalula / echo000 ModelMerger · Licencia MIT",
    }
}

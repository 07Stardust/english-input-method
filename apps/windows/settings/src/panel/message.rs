//! 设置窗口的消息类型：导航切换与各页的「改动」，根组件的 `update` 据此落盘。

/// 设置窗口的消息；「改动」消息带控件新值，`update` 据此落盘。
#[derive(Clone)]
pub(crate) enum Message {
    StudyEnabled(bool),
    StudyPrivacy(bool),
    StudyExam(bool),
    StudyUnknown(bool),
    StudyDaily(Option<f64>),
    StudyLevels(String),
    StudyTag(String),
    StudyBook(Option<usize>),
    StudyUseSelected,
    StudyUseAll,
    StudyToggleBook(String, bool),
    StudyName(String),
    StudyImport(bool),
    StudyColumn(usize, Option<usize>),
    StudyHeader(bool),
    StudyConfirmImport,
    StudyCancelImport,
    StudyPage(bool),
    StudyMapping(usize, String),
    StudyEquivalent(usize, String),
    StudyFavorite(usize, bool),
    StudyDelete(bool),
    StudyClear(bool),
    StudyExport,
    StudyStart(bool),
    StudyAnswer(String),
    StudyReveal,
    StudyGrade(qingjian_learning::study::Grade),
    StudyReset(usize),
    StudyMaster(usize),
    StudyOnlyFavorites(bool),
    StudyOnlyWrong(bool),
    StudyRecordShortcut(bool),
    StudyShortcutRecorded(bool, Option<String>),
    StudyClearShortcut(bool),
    /// 导航切换分节（`None` 是取消选中，忽略）。
    Navigate(Option<String>),

    // 通用页
    LearningLanguage(Option<usize>),
    PageSize(Option<f64>),
    Scheme(Option<usize>),
    ShuangpinRawPreedit(bool),
    Wubi(bool),
    Traditional(bool),
    EnglishCandidates(bool),
    ChineseFirst(bool),
    /// 中文模式下 Shift+字母：交给应用（缺省）还是进组句缓冲区。
    ShiftLetter(Option<usize>),
    FullWidthPunctuation(bool),
    EnglishFullWidthPunctuation(bool),
    /// 开=写入平台默认名单，关=清空。
    EnglishOffInApps(bool),
    /// 勾上 / 去掉一个中英切换键。
    SwitchKey(qingjian_platform::SwitchKey, bool),
    /// 内置英文模式总开关。
    EnglishMode(bool),

    // 候选窗口页
    Theme(Option<usize>),
    Layout(Option<usize>),
    Preedit(Option<usize>),
    Renderer(Option<usize>),
    /// 字体框里的文字变了：空或正好是某个字族名就落盘。
    FontQuery(String),
    /// 从提示里选了一个字族。
    Font(String),
    StatusBar(bool),

    // 云服务页
    LocalModel(bool),
    CloudEnabled(bool),
    CloudApiKey(String),
    CloudClearKey,
    CloudModel(String),
    CloudBaseUrl(String),
    CloudSlots(Option<f64>),
    CloudSentence(bool),
    TestConnection,
    CloudTestDone(u64, Result<String, String>),

    // 快捷键页
    PageKeys(Option<usize>),
    ModeExpression(Option<usize>),
    ModeQuestion(Option<usize>),
    QuestionMark(bool),
    Translation(Option<usize>),
    TranslationSecond(Option<usize>),
    DeleteCandidate(Option<usize>),
    /// 只换修饰键，字母键固定用当前的。
    TranslateSelection(Option<usize>),

    // 模糊音页
    /// 配置键 + 新值。
    Fuzzy(&'static str, bool),

    // 词库页
    ToggleDomain(String, bool),
    ToggleUserDict(String, bool),
    /// 挪进 dicts\removed，不真删。
    RemoveUserDict(String),
    ImportDictionary,

    // 辅码页
    /// 辅码总开关（`[aux_code] enabled`，缺省关）。
    AuxCodeEnabled(bool),
    /// 候选上是否显示码。
    AuxCodeShow(bool),
    /// 码删空后是否留在辅码态（`[general] aux_code_keep_empty`）。
    AuxCodeKeepEmpty(bool),
    /// 点「录制」：进入等一个键的状态。
    AuxRecordStart,
    /// 录制中放弃，保持原值。
    AuxRecordCancel,
    /// 录制框敲进来的文本，取第一个字符当新触发键。
    AuxRecorded(String),
    /// 码表开关（名字，开 / 关），关掉的进 `[aux_code] disabled`。
    ToggleAuxTable(String, bool),
    /// 挪进 codes\removed，不真删。
    RemoveAuxTable(String),
    /// 打开文件选择器导入一张码表（Rime `.dict.yaml`）。
    ImportCodeTable,

    // 高级页
    VerboseLog(bool),
    InputLog(bool),
    /// 学习输入习惯开关。
    Learning(bool),
    OpenConfigFile,
    OpenDataDir,
    OpenLogDir,
    /// 日志目录 + config.toml 打成 zip 放桌面。
    ExportLogs,
    ClearInputLog,

    // 关于页
    OpenWebsite,

    // 关于页：检查更新（`UpdateChecked` 的 `None` = 本地开发版没查）
    UpdateCheck(bool),
    UpdateChannel(Option<usize>),
    CheckUpdateNow,

    OpenDownload,
    OpenRepository,
}

impl Message {
    pub(super) fn is_study(&self) -> bool {
        matches!(
            self,
            Self::StudyEnabled(_)
                | Self::StudyPrivacy(_)
                | Self::StudyExam(_)
                | Self::StudyUnknown(_)
                | Self::StudyDaily(_)
                | Self::StudyLevels(_)
                | Self::StudyTag(_)
                | Self::StudyBook(_)
                | Self::StudyUseSelected
                | Self::StudyUseAll
                | Self::StudyToggleBook(..)
                | Self::StudyName(_)
                | Self::StudyImport(_)
                | Self::StudyColumn(..)
                | Self::StudyHeader(_)
                | Self::StudyConfirmImport
                | Self::StudyCancelImport
                | Self::StudyPage(_)
                | Self::StudyMapping(..)
                | Self::StudyEquivalent(..)
                | Self::StudyFavorite(..)
                | Self::StudyDelete(_)
                | Self::StudyClear(_)
                | Self::StudyExport
                | Self::StudyStart(_)
                | Self::StudyAnswer(_)
                | Self::StudyReveal
                | Self::StudyGrade(_)
                | Self::StudyReset(_)
                | Self::StudyMaster(_)
                | Self::StudyOnlyFavorites(_)
                | Self::StudyOnlyWrong(_)
                | Self::StudyRecordShortcut(_)
                | Self::StudyShortcutRecorded(..)
                | Self::StudyClearShortcut(_)
        )
    }
}

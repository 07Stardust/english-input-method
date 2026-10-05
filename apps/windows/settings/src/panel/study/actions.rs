//! 词书导入确认、编辑关联、练习与快捷键录制。

use std::time::{Duration, Instant};

use qingjian_core::Language;
use qingjian_learning::study::{Grade, ImportColumns, preview, read_table};
use qingjian_translate::Glossary;
use windows_reactor::ComponentContext;

use crate::panel::{Message, Settings};

pub(super) fn today() -> i64 {
    // 本地年月日转换为稳定日序号，不用 UTC 的午夜划分每日任务。
    let date = jiff::Zoned::now().date();
    i64::from(
        jiff::civil::Date::new(1970, 1, 1)
            .expect("date")
            .until(date)
            .expect("date span")
            .get_days(),
    )
}

impl Settings {
    pub(crate) fn study_action(&mut self, message: Message, context: &ComponentContext<Self>) {
        let result: Result<(), String> = (|| {
            match message {
                Message::StudyEnabled(value) => self.save("study", "enabled", value),
                Message::StudyPrivacy(value) => {
                    if value {
                        self.cloud_test_epoch
                            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                        self.cloud_status = crate::panel::cloud_status::CloudStatus::Idle;
                    }
                    self.save("study", "privacy", value);
                }
                Message::StudyExam(value) => self.save("study", "exam_mode", value),
                Message::StudyUnknown(value) => self.save("study", "include_unknown", value),
                Message::StudyDaily(Some(value)) => {
                    self.save("study", "daily_new", (value.round() as i64).clamp(0, 500))
                }
                Message::StudyLevels(value) => {
                    let levels: Vec<String> = value
                        .split([',', ';', ' '])
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                        .collect();
                    self.save_array("study", "levels", &levels);
                }
                Message::StudyTag(value) => self.save("study", "exam_tag", value),
                Message::StudyBook(Some(index)) => {
                    self.study.selected = self
                        .study
                        .store
                        .books
                        .get(index)
                        .map(|book| book.id.clone());
                    self.study.page = 0;
                    self.study.confirm_delete = None;
                }
                Message::StudyUseSelected => {
                    if let Some(id) = self.study.selected.clone() {
                        self.save_array("study", "selected_books", &[id]);
                    }
                }
                Message::StudyUseAll => {
                    self.save_array("study", "selected_books", &[] as &[String])
                }
                Message::StudyToggleBook(id, value) => {
                    self.study
                        .store
                        .set_enabled(&id, value)
                        .map_err(|error| error.to_string())?;
                    self.study_changed();
                }
                Message::StudyName(value) => self.study.name = value,
                Message::StudyImport(replace) => {
                    let file = rfd::FileDialog::new()
                        .add_filter("词书", &["csv", "tsv", "xlsx"])
                        .pick_file();
                    if let Some(file) = file {
                        let table = read_table(&file).map_err(|error| error.to_string())?;
                        self.study.columns = table
                            .first()
                            .map(|row| ImportColumns::from_header(row))
                            .unwrap_or_default();
                        self.study.table = table;
                        self.study.name = file
                            .file_stem()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "我的词书".into());
                        self.study.replacing = if replace {
                            Some(
                                self.study
                                    .selected
                                    .clone()
                                    .ok_or("请先选择需要更新的词书")?,
                            )
                        } else {
                            None
                        };
                        self.study.page = 0;
                        self.rebuild_preview();
                    }
                }
                Message::StudyColumn(field, column) => {
                    let column = column.and_then(|index| index.checked_sub(1));
                    match field {
                        0 => {
                            if let Some(value) = column {
                                self.study.columns.english = value;
                            }
                        }
                        1 => self.study.columns.meaning = column,
                        2 => self.study.columns.part_of_speech = column,
                        3 => self.study.columns.example = column,
                        4 => self.study.columns.level = column,
                        5 => self.study.columns.tags = column,
                        6 => self.study.columns.input_keys = column,
                        7 => self.study.columns.equivalents = column,
                        _ => {}
                    }
                    self.rebuild_preview();
                }
                Message::StudyHeader(value) => {
                    self.study.columns.header = value;
                    self.rebuild_preview();
                }
                Message::StudyConfirmImport => {
                    let id = self
                        .study
                        .store
                        .import(
                            &self.study.name,
                            self.study.preview.entries.clone(),
                            self.study.replacing.as_deref(),
                        )
                        .map_err(|error| error.to_string())?;
                    self.study.selected = Some(id);
                    self.study.table.clear();
                    self.study.preview = Default::default();
                    self.study.status = "词书已保存；输入提示将在下一次配置检查时更新".into();
                    self.study_changed();
                }
                Message::StudyCancelImport => {
                    self.study.table.clear();
                    self.study.preview = Default::default();
                }
                Message::StudyPage(forward) => {
                    if forward {
                        self.study.page += 1;
                    } else {
                        self.study.page = self.study.page.saturating_sub(1);
                    }
                }
                Message::StudyMapping(index, value) => {
                    let mappings = value
                        .split([';', '；', '|'])
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned)
                        .collect();
                    if !self.study.table.is_empty() {
                        if let Some(entry) = self.study.preview.entries.get_mut(index) {
                            entry.input_keys = mappings;
                        }
                    } else if let Some(id) = &self.study.selected {
                        let mut next = self.study.store.clone();
                        if let Some(entry) = next
                            .books
                            .iter_mut()
                            .find(|book| &book.id == id)
                            .and_then(|book| book.entries.get_mut(index))
                        {
                            entry.input_keys = mappings;
                        }
                        next.save().map_err(|error| error.to_string())?;
                        self.study.store = next;
                        self.study_changed();
                    }
                }
                Message::StudyEquivalent(index, value) => {
                    let mut next = self.study.store.clone();
                    if let Some(id) = &self.study.selected
                        && let Some(entry) = next
                            .books
                            .iter_mut()
                            .find(|book| &book.id == id)
                            .and_then(|book| book.entries.get_mut(index))
                    {
                        entry.equivalents = value
                            .split([';', '；', '|'])
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_owned)
                            .collect();
                    }
                    next.save().map_err(|error| error.to_string())?;
                    self.study.store = next;
                }
                Message::StudyFavorite(index, value) => {
                    if self.config.study.privacy || !self.config.study.enabled {
                        return Err("当前不记录学习标记".into());
                    }
                    if let Some(book) = self
                        .study
                        .store
                        .books
                        .iter()
                        .find(|book| Some(&book.id) == self.study.selected.as_ref())
                        && let Some(entry) = book.entries.get(index)
                    {
                        let (id, entry) = (book.id.clone(), entry.clone());
                        self.study
                            .store
                            .favorite(&id, &entry, value)
                            .map_err(|error| error.to_string())?;
                    }
                }
                Message::StudyDelete(confirm) => {
                    if !confirm || self.study.confirm_delete != Some(false) {
                        self.study.confirm_delete = Some(false);
                    } else if let Some(id) = self.study.selected.clone() {
                        self.study
                            .store
                            .delete_book(&id)
                            .map_err(|error| error.to_string())?;
                        self.study.selected = None;
                        self.study.confirm_delete = None;
                        self.study_changed();
                    }
                }
                Message::StudyClear(confirm) => {
                    if !confirm || self.study.confirm_delete != Some(true) {
                        self.study.confirm_delete = Some(true);
                    } else {
                        self.study
                            .store
                            .clear()
                            .map_err(|error| error.to_string())?;
                        self.study.selected = None;
                        self.study.queue.clear();
                        self.study.confirm_delete = None;
                        self.study_changed();
                    }
                }
                Message::StudyExport => {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("JSON 备份", &["json"])
                        .set_file_name("english-input-method-study.json")
                        .save_file()
                    {
                        self.study
                            .store
                            .export(&path)
                            .map_err(|error| error.to_string())?;
                        self.study.status = "已导出词书与复习记录；不包含输入原文或密钥".into();
                    }
                }
                Message::StudyStart(spelling) => {
                    self.study.spelling = spelling;
                    self.study.queue = self.study.store.queue(&self.config.study, today());
                    self.study.queue.retain(|(id, entry)| {
                        let record = self.study.store.record(id, entry);
                        (!self.study.only_favorites || record.favorite)
                            && (!self.study.only_wrong || record.wrong > 0)
                    });
                    self.study.answer.clear();
                    self.study.revealed = false;
                }
                Message::StudyAnswer(answer) => self.study.answer = answer,
                Message::StudyReveal => self.study.revealed = true,
                Message::StudyGrade(grade) => {
                    if self.config.study.privacy || !self.config.study.enabled {
                        return Err("学习关闭或隐私开启期间不记录练习".into());
                    }
                    if let Some((id, entry)) = self.study.queue.first().cloned() {
                        let grade = if self.study.spelling {
                            if entry.matches_answer(&self.study.answer) {
                                Grade::Remembered
                            } else {
                                Grade::Forgotten
                            }
                        } else {
                            grade
                        };
                        self.study
                            .store
                            .grade(&id, &entry, grade, today())
                            .map_err(|error| error.to_string())?;
                        self.study.status = if matches!(grade, Grade::Forgotten) {
                            format!(
                                "正确词条：{}（{}）；等价答案：{}；已加入明日复习",
                                entry.english,
                                entry.meaning,
                                entry.equivalents.join("、")
                            )
                        } else {
                            "练习已记录".into()
                        };
                        self.study.queue.remove(0);
                        self.study.answer.clear();
                        self.study.revealed = false;
                    }
                }
                Message::StudyReset(index) | Message::StudyMaster(index) => {
                    let master = matches!(message, Message::StudyMaster(_));
                    if self.config.study.privacy || !self.config.study.enabled {
                        return Err("当前不记录学习进度".into());
                    }
                    if let Some(book) = self
                        .study
                        .store
                        .books
                        .iter()
                        .find(|book| Some(&book.id) == self.study.selected.as_ref())
                        && let Some(entry) = book.entries.get(index)
                    {
                        let (id, entry) = (book.id.clone(), entry.clone());
                        if master {
                            self.study
                                .store
                                .grade(&id, &entry, Grade::Mastered, today())
                                .map_err(|error| error.to_string())?;
                        } else {
                            self.study
                                .store
                                .reset(&id, &entry)
                                .map_err(|error| error.to_string())?;
                        }
                    }
                }
                Message::StudyOnlyFavorites(value) => {
                    self.study.only_favorites = value;
                    self.study.page = 0;
                }
                Message::StudyOnlyWrong(value) => {
                    self.study.only_wrong = value;
                    self.study.page = 0;
                }
                Message::StudyRecordShortcut(privacy) => {
                    self.study.recording = Some(privacy);
                    self.study.status =
                        "请按 Ctrl/Alt 等修饰键与字母或数字；Esc 取消，15 秒超时".into();
                    context.spawn_background(move |cancel| {
                        let start = Instant::now();
                        while start.elapsed() < Duration::from_secs(15) && !cancel.is_cancelled() {
                            use windows::Win32::UI::WindowsAndMessaging::{
                                GetForegroundWindow, GetWindowThreadProcessId,
                            };
                            let mut foreground_process = 0;
                            unsafe {
                                GetWindowThreadProcessId(
                                    GetForegroundWindow(),
                                    Some(&mut foreground_process),
                                );
                            }
                            if foreground_process != std::process::id() {
                                std::thread::sleep(Duration::from_millis(20));
                                continue;
                            }
                            use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
                            let down = |key| unsafe { GetAsyncKeyState(key) } < 0;
                            if down(0x1b) {
                                break;
                            }
                            let ctrl = down(0x11);
                            let alt = down(0x12);
                            let shift = down(0x10);
                            if ctrl || alt {
                                for key in (0x30..=0x39).chain(0x41..=0x5a) {
                                    if down(key) {
                                        let mut parts = Vec::new();
                                        if ctrl {
                                            parts.push("ctrl".to_owned());
                                        }
                                        if alt {
                                            parts.push("alt".to_owned());
                                        }
                                        if shift {
                                            parts.push("shift".to_owned());
                                        }
                                        parts.push(
                                            char::from_u32(key as u32)
                                                .unwrap()
                                                .to_ascii_lowercase()
                                                .to_string(),
                                        );
                                        return Message::StudyShortcutRecorded(
                                            privacy,
                                            Some(parts.join("+")),
                                        );
                                    }
                                }
                            }
                            std::thread::sleep(Duration::from_millis(10));
                        }
                        Message::StudyShortcutRecorded(privacy, None)
                    });
                }
                Message::StudyShortcutRecorded(privacy, value) => {
                    self.study.recording = None;
                    if let Some(value) = value {
                        let combo = value
                            .parse::<qingjian_platform::KeyCombo>()
                            .map_err(|error| error.to_string())?;
                        let other = if privacy {
                            &self.config.study.toggle_learning
                        } else {
                            &self.config.study.toggle_privacy
                        };
                        if combo == self.config.shortcut.translate_selection
                            || other
                                .parse::<qingjian_platform::KeyCombo>()
                                .is_ok_and(|other| other == combo)
                        {
                            return Err("该快捷键已用于另一输入法功能，请重新录制".into());
                        }
                        if combo.modifiers.control
                            && !combo.modifiers.option
                            && !combo.modifiers.shift
                            && "acfnoprstvwxyz".contains(combo.key)
                        {
                            return Err("该组合通常用于编辑或应用操作，请使用额外修饰键".into());
                        }
                        self.save(
                            "study",
                            if privacy {
                                "toggle_privacy"
                            } else {
                                "toggle_learning"
                            },
                            value,
                        );
                    }
                }
                Message::StudyClearShortcut(privacy) => self.save(
                    "study",
                    if privacy {
                        "toggle_privacy"
                    } else {
                        "toggle_learning"
                    },
                    "",
                ),
                _ => {}
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.study.status = error;
        }
    }

    fn study_changed(&mut self) {
        self.save(
            "study",
            "revision",
            jiff::Timestamp::now().as_millisecond().to_string(),
        );
    }

    fn rebuild_preview(&mut self) {
        self.study.preview = preview(&self.study.table, &self.study.columns);
        if let Some(path) = crate::panel::controls::repo_resource("assets/levels/levels-en.tsv")
            && let Ok(levels) = qingjian_translate::LevelTable::from_path(path)
        {
            for entry in &mut self.study.preview.entries {
                if entry.level.is_empty() {
                    entry.level = levels.level(&entry.english).unwrap_or_default().to_owned();
                }
            }
        }
        // 从离线英译表建立反向建议；仅作为可编辑的关联，不把机器译词当考试标准答案。
        let mut wanted: std::collections::HashMap<String, Vec<usize>> =
            std::collections::HashMap::new();
        for (index, entry) in self.study.preview.entries.iter().enumerate() {
            if entry.input_keys.is_empty() {
                wanted.entry(entry.key()).or_default().push(index);
            }
        }
        if let Some(path) = crate::panel::controls::repo_resource("assets/glossary/glossary-en.tsv")
            && let Ok(glossary) = Glossary::from_path(Language::English, &path)
        {
            use qingjian_core::Translator;
            if let Ok(source) = std::fs::read_to_string(&path) {
                for line in source.lines().filter(|line| !line.starts_with('#')) {
                    let chinese = line.split('\t').next().unwrap_or_default();
                    if let Some(translation) = glossary.translate(chinese) {
                        for sense in translation.senses() {
                            if let Some(indices) = wanted.get(&sense.text.to_lowercase()) {
                                for index in indices {
                                    let keys = &mut self.study.preview.entries[*index].input_keys;
                                    if keys.len() < 8 && !keys.iter().any(|key| key == chinese) {
                                        keys.push(chinese.to_owned());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.study.status = format!(
            "预览：{} 条有效，{} 条重复，{} 行错误。关联为离线建议，请核对后确认。",
            self.study.preview.entries.len(),
            self.study.preview.duplicates,
            self.study.preview.errors.len()
        );
    }
}

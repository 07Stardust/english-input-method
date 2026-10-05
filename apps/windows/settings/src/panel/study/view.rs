//! 词书预览与离线练习；只渲染当前页，避免大词书拖慢设置窗口。

use crate::panel::controls::{field, note, page};
use crate::panel::{Message, Settings};
use qingjian_core::{Language, VocabularyTracker};
use qingjian_learning::study::Grade;
use windows_reactor::*;

fn button(label: &str, message: Message, context: &mut ViewContext<Settings>) -> View {
    Button::new()
        .on_click(context.message(message))
        .content(label)
}

pub(crate) fn view(settings: &Settings, context: &mut ViewContext<Settings>) -> View {
    let ui = &settings.study;
    let options = &settings.config.study;
    let mut rows: Vec<View> = vec![
        field(
            "学习开关",
            "暂停英语提示、曝光和学习标记；中文输入继续工作。",
            ToggleSwitch::new()
                .is_on(options.enabled)
                .on_toggled(context.callback(Message::StudyEnabled)),
        ),
        field(
            "隐私开关",
            "暂停输入日志、习惯学习、词汇记录和云请求；密码框保护始终生效。",
            ToggleSwitch::new()
                .is_on(options.privacy)
                .on_toggled(context.callback(Message::StudyPrivacy)),
        ),
        field(
            "考试模式",
            "只显示所选词书与筛选范围内的英语提示。日常模式允许通用释义兜底。",
            ToggleSwitch::new()
                .is_on(options.exam_mode)
                .on_toggled(context.callback(Message::StudyExam)),
        ),
        field(
            "每日新增",
            "默认 20 词；复习间隔为 1、3、7、14、30 天。",
            NumberBox::new()
                .minimum(0.0)
                .maximum(500.0)
                .value(options.daily_new as f64)
                .on_value_changed(context.callback(Message::StudyDaily)),
        ),
        field(
            "等级筛选",
            "多个等级用分号隔开，留空为全部；词书等级不等同于考试覆盖率。",
            TextBox::new()
                .text(options.levels.join(";"))
                .on_text_changed(context.callback(Message::StudyLevels)),
        ),
        field(
            "包含未知等级",
            "没有来源等级的词单独视为未知，不能据此判断为六级。",
            ToggleSwitch::new()
                .is_on(options.include_unknown)
                .on_toggled(context.callback(Message::StudyUnknown)),
        ),
        field(
            "考试标签",
            "例如 CET6；须与词书标签一致，留空为全部。",
            TextBox::new()
                .text(&options.exam_tag)
                .on_text_changed(context.callback(Message::StudyTag)),
        ),
        field(
            "词书",
            "可启用多本，或仅使用当前选中的一本。",
            ComboBox::new()
                .items_source(ui.store.books.iter().map(|book| book.name.clone()))
                .selected_index(
                    ui.store
                        .books
                        .iter()
                        .position(|book| Some(&book.id) == ui.selected.as_ref()),
                )
                .on_selection_changed(context.callback(Message::StudyBook)),
        ),
    ];
    for book in &ui.store.books {
        let id = book.id.clone();
        rows.push(
            CheckBox::new()
                .is_checked(book.enabled)
                .on_is_checked_changed(
                    context.callback(move |value| Message::StudyToggleBook(id.clone(), value)),
                )
                .content(format!("{} · {} 词", book.name, book.entries.len())),
        );
    }
    rows.push(
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children([
                button("只使用选中词书", Message::StudyUseSelected, context),
                button("使用全部启用词书", Message::StudyUseAll, context),
                button("导入词书", Message::StudyImport(false), context),
                button("更新选中词书", Message::StudyImport(true), context),
            ]),
    );
    rows.push(note(&ui.status));
    if !ui.table.is_empty() {
        rows.push(field(
            "词书名称",
            "确认后才保存；更新保留相同英文词条的收藏与进度。",
            TextBox::new()
                .text(&ui.name)
                .on_text_changed(context.callback(Message::StudyName)),
        ));
        rows.push(
            CheckBox::new()
                .is_checked(ui.columns.header)
                .on_is_checked_changed(context.callback(Message::StudyHeader))
                .content("首行为列名"),
        );
        let labels = [
            "英文（必需）",
            "中文释义",
            "词性",
            "例句",
            "等级",
            "考试标签",
            "关联中文",
            "等价答案",
        ];
        let current = [
            Some(ui.columns.english),
            ui.columns.meaning,
            ui.columns.part_of_speech,
            ui.columns.example,
            ui.columns.level,
            ui.columns.tags,
            ui.columns.input_keys,
            ui.columns.equivalents,
        ];
        let count = ui.table.iter().map(Vec::len).max().unwrap_or(0);
        for (index, label) in labels.iter().enumerate() {
            let columns = std::iter::once("不使用".to_owned()).chain((0..count).map(|column| {
                format!(
                    "第 {} 列：{}",
                    column + 1,
                    ui.table
                        .first()
                        .and_then(|row| row.get(column))
                        .map(String::as_str)
                        .unwrap_or("")
                )
            }));
            rows.push(field(
                label,
                "",
                ComboBox::new()
                    .items_source(columns)
                    .selected_index(current[index].map_or(0, |column| column + 1))
                    .on_selection_changed(
                        context.callback(move |column| Message::StudyColumn(index, column)),
                    ),
            ));
        }
        for error in ui.preview.errors.iter().take(10) {
            rows.push(note(error));
        }
        rows.push(
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .children([
                    button("确认导入有效条目", Message::StudyConfirmImport, context),
                    button("取消导入", Message::StudyCancelImport, context),
                ]),
        );
    }
    let book = ui
        .store
        .books
        .iter()
        .find(|book| Some(&book.id) == ui.selected.as_ref());
    let entries = if ui.table.is_empty() {
        book.map(|book| book.entries.as_slice()).unwrap_or_default()
    } else {
        &ui.preview.entries
    };
    if let Some(book) = book {
        let mastered = book
            .entries
            .iter()
            .filter(|entry| ui.store.record(&book.id, entry).mastered)
            .count();
        let introduced = book
            .entries
            .iter()
            .filter(|entry| ui.store.record(&book.id, entry).introduced_day.is_some())
            .count();
        rows.push(note(&format!(
            "已练习 {introduced}/{}，已掌握 {mastered}；练习进度与候选曝光分别统计。",
            book.entries.len()
        )));
    }
    rows.push(
        CheckBox::new()
            .is_checked(ui.only_favorites)
            .on_is_checked_changed(context.callback(Message::StudyOnlyFavorites))
            .content("仅收藏"),
    );
    rows.push(
        CheckBox::new()
            .is_checked(ui.only_wrong)
            .on_is_checked_changed(context.callback(Message::StudyOnlyWrong))
            .content("仅错词"),
    );
    for (index, entry) in entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            let record = book
                .map(|book| ui.store.record(&book.id, entry))
                .unwrap_or_default();
            (!ui.only_favorites || record.favorite) && (!ui.only_wrong || record.wrong > 0)
        })
        .skip(ui.page.saturating_mul(20))
        .take(20)
    {
        rows.push(
            TextBlock::new()
                .text(format!(
                    "{} [{}] {} · {}",
                    entry.english,
                    entry.part_of_speech,
                    entry.meaning,
                    if entry.level.is_empty() {
                        "未知等级"
                    } else {
                        &entry.level
                    }
                ))
                .into(),
        );
        rows.push(field(
            "输入关联",
            "多个中文词用分号分隔；未匹配词仍可练习。",
            TextBox::new()
                .text(entry.input_keys.join(";"))
                .on_text_changed(
                    context.callback(move |value| Message::StudyMapping(index, value)),
                ),
        ));
        rows.push(note(&format!(
            "候选曝光 {} 次；曝光不代表掌握",
            ui.vocabulary.exposures(Language::English, &entry.english)
        )));
        if ui.table.is_empty() {
            rows.push(field(
                "等价答案",
                "请核对相应释义后登记。",
                TextBox::new()
                    .text(entry.equivalents.join(";"))
                    .on_text_changed(
                        context.callback(move |value| Message::StudyEquivalent(index, value)),
                    ),
            ));
            let favorite = book.is_some_and(|book| ui.store.record(&book.id, entry).favorite);
            rows.push(
                CheckBox::new()
                    .is_checked(favorite)
                    .on_is_checked_changed(
                        context.callback(move |value| Message::StudyFavorite(index, value)),
                    )
                    .content("收藏"),
            );
            rows.push(
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .children([
                        button("标记已掌握", Message::StudyMaster(index), context),
                        button("重新学习", Message::StudyReset(index), context),
                    ]),
            );
        }
    }
    rows.push(
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children([
                button("上一页", Message::StudyPage(false), context),
                button("下一页", Message::StudyPage(true), context),
                button("释义词卡", Message::StudyStart(false), context),
                button("看中文写英文", Message::StudyStart(true), context),
            ]),
    );
    if let Some((_, entry)) = ui.queue.first() {
        rows.push(note(&format!("今日待练习 {} 词", ui.queue.len())));
        rows.push(
            TextBlock::new()
                .text(if ui.spelling {
                    &entry.meaning
                } else {
                    &entry.english
                })
                .font_size(24.0)
                .into(),
        );
        if ui.spelling {
            rows.push(
                TextBox::new()
                    .text(&ui.answer)
                    .placeholder_text("输入英文词条")
                    .on_text_changed(context.callback(Message::StudyAnswer))
                    .into(),
            );
            rows.push(button(
                "检查答案并继续",
                Message::StudyGrade(Grade::Remembered),
                context,
            ));
        } else {
            if ui.revealed {
                rows.push(note(&format!("{}\n{}", entry.meaning, entry.example)));
            }
            rows.push(
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .children([
                        button("翻面", Message::StudyReveal, context),
                        button("记住了", Message::StudyGrade(Grade::Remembered), context),
                        button("忘记了", Message::StudyGrade(Grade::Forgotten), context),
                    ]),
            );
        }
    }
    for privacy in [false, true] {
        let shortcut = if privacy {
            &options.toggle_privacy
        } else {
            &options.toggle_learning
        };
        rows.push(note(&format!(
            "{}快捷键：{}",
            if privacy { "隐私" } else { "学习" },
            if shortcut.is_empty() {
                "未设置"
            } else {
                shortcut
            }
        )));
        rows.push(
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .children([
                    button("录制快捷键", Message::StudyRecordShortcut(privacy), context),
                    button("清除快捷键", Message::StudyClearShortcut(privacy), context),
                ]),
        );
    }
    rows.push(
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children([
                button("导出词书与进度", Message::StudyExport, context),
                button(
                    if ui.confirm_delete == Some(false) {
                        "确认删除选中词书"
                    } else {
                        "删除选中词书"
                    },
                    Message::StudyDelete(ui.confirm_delete == Some(false)),
                    context,
                ),
                button(
                    if ui.confirm_delete == Some(true) {
                        "确认清空全部词书和进度"
                    } else {
                        "清空全部词书和进度"
                    },
                    Message::StudyClear(ui.confirm_delete == Some(true)),
                    context,
                ),
            ]),
    );
    page(
        "词书与学习",
        StackPanel::new().spacing(16.0).keyed_children(
            rows.into_iter()
                .enumerate()
                .map(|(index, view)| KeyedView::new(index.to_string(), view)),
        ),
    )
}

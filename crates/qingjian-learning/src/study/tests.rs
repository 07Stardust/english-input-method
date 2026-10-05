//! 词书导入、筛选、复习与持久化回归。

use std::path::PathBuf;

use qingjian_core::{Language, NoTranslator, Translator};

use super::{
    Entry, Grade, ImportColumns, StudyOptions, StudyStore, StudyTranslator, preview, read_table,
};

fn path(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "english-ime-tests-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory.join(name)
}

fn entry() -> Entry {
    Entry {
        english: "enhance".into(),
        meaning: "增强".into(),
        input_keys: vec!["增强".into()],
        level: "B2".into(),
        tags: vec!["CET6".into()],
        equivalents: vec!["strengthen".into()],
        ..Entry::default()
    }
}

#[test]
fn csv_preview_handles_quotes_duplicates_and_bad_rows() {
    let file = path("words.csv");
    std::fs::write(&file, "english,meaning,input_keys\nenhance,增强,增强\nENHANCE,增强,增强\n,无效,无效\nacknowledge,\"承认,认可\",承认\n").unwrap();
    let rows = read_table(&file).unwrap();
    let preview = preview(&rows, &ImportColumns::from_header(&rows[0]));
    assert_eq!(preview.entries.len(), 2);
    assert_eq!(preview.duplicates, 1);
    assert_eq!(preview.errors.len(), 1);
    assert_eq!(preview.entries[1].meaning, "承认,认可");
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

#[test]
fn tsv_gbk_and_utf16_import() {
    for (name, bytes) in [
        (
            "gbk.tsv",
            encoding_rs::GBK
                .encode("英文\t中文\nhello\t你好")
                .0
                .into_owned(),
        ),
        (
            "utf16.tsv",
            [
                vec![0xff, 0xfe],
                "英文\t中文\nhello\t你好"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .collect(),
            ]
            .concat(),
        ),
    ] {
        let file = path(name);
        std::fs::write(&file, bytes).unwrap();
        let rows = read_table(&file).unwrap();
        let preview = preview(&rows, &ImportColumns::from_header(&rows[0]));
        assert_eq!(preview.entries[0].meaning, "你好");
        std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
    }
}

#[test]
fn update_keeps_progress_and_failed_import_keeps_store() {
    let file = path("study.json");
    let mut store = StudyStore::open(&file).unwrap();
    let item = entry();
    let id = store.import("六级演示", vec![item.clone()], None).unwrap();
    store.favorite(&id, &item, true).unwrap();
    store.grade(&id, &item, Grade::Remembered, 100).unwrap();
    store
        .import("更新版", vec![item.clone()], Some(&id))
        .unwrap();
    assert!(store.record(&id, &item).favorite);
    assert_eq!(store.record(&id, &item).due_day, 101);
    assert!(store.import("无效", Vec::new(), Some(&id)).is_err());
    assert_eq!(StudyStore::open(&file).unwrap().books[0].name, "更新版");
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

#[test]
fn exam_filter_and_learning_toggle() {
    let store = StudyStore {
        books: vec![super::Book {
            id: "demo".into(),
            name: "demo".into(),
            enabled: true,
            entries: vec![entry()],
        }],
        ..StudyStore::default()
    };
    let mut options = StudyOptions {
        exam_mode: true,
        ..StudyOptions::default()
    };
    let translator = StudyTranslator::new(Box::new(NoTranslator), &store, options.clone());
    // NoTranslator 不声明英语；下面使用明确的英语离线提供方。
    assert!(translator.translate("不存在").is_none());
    struct English;
    impl Translator for English {
        fn language(&self) -> Language {
            Language::English
        }
        fn translate(&self, _: &str) -> Option<qingjian_core::Translation> {
            None
        }
    }
    let translator = StudyTranslator::new(Box::new(English), &store, options.clone());
    assert_eq!(
        translator.translate("增强").unwrap().senses()[0].text,
        "enhance"
    );
    assert!(translator.translate("你好").is_none());
    options.levels = vec!["A1".into()];
    assert!(
        StudyTranslator::new(Box::new(English), &store, options.clone())
            .translate("增强")
            .is_none()
    );
    options.levels.clear();
    options.enabled = false;
    assert!(
        StudyTranslator::new(Box::new(English), &store, options)
            .translate("增强")
            .is_none()
    );
}

#[test]
fn review_intervals_wrong_answer_and_daily_limit() {
    let mut record = super::ReviewRecord::default();
    for (day, due) in [(100, 101), (101, 104), (104, 111), (111, 125), (125, 155)] {
        record.grade(Grade::Remembered, day);
        assert_eq!(record.due_day, due);
    }
    record.grade(Grade::Forgotten, 155);
    assert_eq!(record.stage, 0);
    assert_eq!(record.due_day, 156);
    assert_eq!(record.wrong, 1);
    let mut same_day = super::ReviewRecord::default();
    same_day.grade(Grade::Remembered, 100);
    same_day.grade(Grade::Remembered, 100);
    assert_eq!(same_day.due_day, 101);
    assert_eq!(same_day.stage, 1);
    let item = entry();
    assert!(item.matches_answer(" Strengthen "));
    assert!(!item.matches_answer("enhanced"));
    let store = StudyStore {
        books: vec![super::Book {
            id: "demo".into(),
            name: "demo".into(),
            enabled: true,
            entries: (0..30)
                .map(|index| Entry {
                    english: format!("word{index}"),
                    ..Entry::default()
                })
                .collect(),
        }],
        ..StudyStore::default()
    };
    assert_eq!(store.queue(&StudyOptions::default(), 100).len(), 20);
}

#[test]
fn xlsx_import_reads_values_and_rejects_formulas_before_allocation() {
    use std::io::Write;
    fn workbook(path: &std::path::Path, content: &str) {
        let mut writer = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        let files = [
            (
                "_rels/.rels",
                "<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='rId1' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument' Target='xl/workbook.xml'/></Relationships>",
            ),
            (
                "[Content_Types].xml",
                "<Types xmlns='http://schemas.openxmlformats.org/package/2006/content-types'><Default Extension='xml' ContentType='application/xml'/><Override PartName='/xl/workbook.xml' ContentType='application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml'/><Override PartName='/xl/worksheets/sheet1.xml' ContentType='application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml'/></Types>",
            ),
            (
                "xl/workbook.xml",
                "<workbook xmlns='http://schemas.openxmlformats.org/spreadsheetml/2006/main' xmlns:r='http://schemas.openxmlformats.org/officeDocument/2006/relationships'><sheets><sheet name='Words' sheetId='1' r:id='rId1'/></sheets></workbook>",
            ),
            (
                "xl/_rels/workbook.xml.rels",
                "<Relationships xmlns='http://schemas.openxmlformats.org/package/2006/relationships'><Relationship Id='rId1' Type='http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet' Target='worksheets/sheet1.xml'/></Relationships>",
            ),
            ("xl/worksheets/sheet1.xml", content),
        ];
        for (name, xml) in files {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(xml.as_bytes()).unwrap();
        }
        writer.finish().unwrap();
    }
    let file = path("words.xlsx");
    workbook(
        &file,
        "<worksheet xmlns='http://schemas.openxmlformats.org/spreadsheetml/2006/main'><sheetData><row r='1'><c r='A1' t='inlineStr'><is><t>english</t></is></c><c r='B1' t='inlineStr'><is><t>meaning</t></is></c></row><row r='2'><c r='A2' t='inlineStr'><is><t>enhance</t></is></c><c r='B2' t='inlineStr'><is><t>增强</t></is></c></row></sheetData></worksheet>",
    );
    let rows = read_table(&file).unwrap();
    let result = preview(&rows, &ImportColumns::from_header(&rows[0]));
    assert_eq!(result.entries[0].english, "enhance");
    assert_eq!(result.entries[0].meaning, "增强");
    workbook(
        &file,
        "<worksheet><sheetData><row><c r='A1'><f>1+1</f><v>2</v></c></row></sheetData></worksheet>",
    );
    assert!(read_table(&file).is_err());
}

#[test]
fn manual_associations_work_without_bundled_glossary() {
    let file = path("manual-study.json");
    let mut store = StudyStore::open(&file).unwrap();
    store.import("自定义", vec![entry()], None).unwrap();
    let translator = StudyTranslator::new_with_language(
        Box::new(NoTranslator),
        &store,
        StudyOptions {
            exam_mode: true,
            ..StudyOptions::default()
        },
        Language::English,
    );
    assert_eq!(
        translator.translate("增强").unwrap().senses()[0].text,
        "enhance"
    );
    assert!(translator.translate("词书之外").is_none());
    std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
}

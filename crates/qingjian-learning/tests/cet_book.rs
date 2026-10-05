//! 官方词书经真实导入器校验，考试模式不得泄漏未选词书的译词。
use qingjian_core::{Language, NoTranslator, Translator};
use qingjian_learning::study::{
    ImportColumns, StudyOptions, StudyStore, StudyTranslator, preview, read_table,
};

#[test]
fn official_cet_books_import_and_filter() {
    for (filename, count, missing_meanings, missing_mappings, unknown_levels) in [
        ("cet6-neea-2016-star.csv", 1282, 37, 310, 685),
        ("cet6-neea-2016-full.csv", 5404, 83, 707, 1131),
    ] {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/study/cet")
            .join(filename);
        let rows = read_table(&file).unwrap();
        let imported = preview(&rows, &ImportColumns::from_header(&rows[0]));
        assert!(imported.errors.is_empty(), "{:?}", imported.errors);
        assert_eq!(imported.duplicates, 0);
        assert_eq!(imported.entries.len(), count);
        assert_eq!(
            imported
                .entries
                .iter()
                .filter(|e| e.meaning.is_empty())
                .count(),
            missing_meanings
        );
        assert_eq!(
            imported
                .entries
                .iter()
                .filter(|e| e.input_keys.is_empty())
                .count(),
            missing_mappings
        );
        assert_eq!(
            imported
                .entries
                .iter()
                .filter(|e| e.level.is_empty())
                .count(),
            unknown_levels
        );
        assert!(
            imported
                .entries
                .iter()
                .all(|e| e.tags.iter().any(|tag| tag == "CET6"))
        );
        let path = std::env::temp_dir().join(format!(
            "cet-import-{}-{}.json",
            std::process::id(),
            filename
        ));
        let mut store = StudyStore::open(&path).unwrap();
        let id = store.import(filename, imported.entries, None).unwrap();
        let options = StudyOptions {
            selected_books: vec![id],
            exam_mode: true,
            exam_tag: "CET6".into(),
            ..StudyOptions::default()
        };
        let translator = StudyTranslator::new_with_language(
            Box::new(NoTranslator),
            &store,
            options,
            Language::English,
        );
        assert!(
            translator
                .translate("缩写")
                .unwrap()
                .senses()
                .iter()
                .any(|s| s.text == "abbreviation")
        );
        assert!(translator.translate("未收录的测试输入").is_none());
        std::fs::remove_file(path).unwrap();
    }
}

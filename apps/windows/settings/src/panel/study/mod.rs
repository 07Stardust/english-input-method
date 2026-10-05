//! Windows 词书与练习页状态，磁盘写入只由设置程序执行。

mod actions;
pub(super) mod view;

use qingjian_learning::study::{Entry, ImportColumns, ImportPreview, StudyStore};

pub(super) struct StudyUi {
    pub vocabulary: qingjian_learning::VocabularyBook,
    pub store: StudyStore,

    pub selected: Option<String>,

    pub table: Vec<Vec<String>>,

    pub columns: ImportColumns,

    pub preview: ImportPreview,

    pub name: String,

    pub replacing: Option<String>,

    pub page: usize,

    pub status: String,

    pub queue: Vec<(String, Entry)>,

    pub answer: String,

    pub revealed: bool,

    pub spelling: bool,

    pub only_favorites: bool,

    pub only_wrong: bool,

    pub recording: Option<bool>,

    pub confirm_delete: Option<bool>,
}

impl StudyUi {
    pub fn new(directory: &std::path::Path) -> Self {
        let (store, status) = match StudyStore::open(&directory.join("study.json")) {
            Ok(store) => (store, String::new()),
            Err(error) => (
                StudyStore::default(),
                format!("词书读取失败，未覆盖文件：{error}"),
            ),
        };
        Self {
            vocabulary: qingjian_learning::VocabularyBook::open(directory.join("user-vocab.tsv")),
            store,
            selected: None,
            table: Vec::new(),
            columns: ImportColumns::default(),
            preview: ImportPreview::default(),
            name: String::new(),
            replacing: None,
            page: 0,
            status,
            queue: Vec::new(),
            answer: String::new(),
            revealed: false,
            spelling: false,
            only_favorites: false,
            only_wrong: false,
            recording: None,
            confirm_delete: None,
        }
    }
}

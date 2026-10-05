/// 零起始列索引；未提供的可选列为空。
#[derive(Debug, Clone, Default)]
pub struct ImportColumns {
    pub english: usize,

    pub meaning: Option<usize>,

    pub part_of_speech: Option<usize>,

    pub example: Option<usize>,

    pub level: Option<usize>,

    pub tags: Option<usize>,

    pub input_keys: Option<usize>,

    pub equivalents: Option<usize>,

    pub header: bool,
}

impl ImportColumns {
    pub fn from_header(row: &[String]) -> Self {
        let find = |aliases: &[&str]| {
            row.iter().position(|cell| {
                aliases
                    .iter()
                    .any(|alias| cell.trim().eq_ignore_ascii_case(alias))
            })
        };
        Self {
            english: find(&["english", "word", "英文", "单词"]).unwrap_or(0),
            meaning: find(&["meaning", "definition", "中文", "释义", "中文释义"]),
            part_of_speech: find(&["pos", "part_of_speech", "词性"]),
            example: find(&["example", "例句"]),
            level: find(&["level", "cefr", "等级"]),
            tags: find(&["tags", "exam", "标签", "考试"]),
            input_keys: find(&["input_keys", "输入词", "关联中文"]),
            equivalents: find(&["equivalents", "等价答案"]),
            header: true,
        }
    }
}

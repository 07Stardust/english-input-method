//! 学习场景与词书筛选选项；不依赖平台、文件解析或词书存储。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StudyOptions {
    pub enabled: bool,

    pub privacy: bool,

    pub exam_mode: bool,

    pub selected_books: Vec<String>,

    pub levels: Vec<String>,

    pub include_unknown: bool,

    pub exam_tag: String,

    pub daily_new: usize,

    pub toggle_learning: String,

    pub toggle_privacy: String,
}

impl Default for StudyOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            privacy: false,
            exam_mode: false,
            selected_books: Vec::new(),
            levels: Vec::new(),
            include_unknown: true,
            exam_tag: String::new(),
            daily_new: 20,
            toggle_learning: String::new(),
            toggle_privacy: String::new(),
        }
    }
}

impl StudyOptions {
    pub fn includes(&self, level: &str, tags: &[String]) -> bool {
        let level = level.trim();
        let level_allowed = if level.is_empty() {
            self.include_unknown
        } else {
            self.levels.is_empty()
                || self
                    .levels
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(level))
        };
        level_allowed
            && (self.exam_tag.trim().is_empty()
                || tags
                    .iter()
                    .any(|tag| tag.eq_ignore_ascii_case(&self.exam_tag)))
    }
}

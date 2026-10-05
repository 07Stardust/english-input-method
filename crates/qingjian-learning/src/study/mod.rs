//! 离线词书、导入预览、输入关联与复习。输入热路径只查询预构建索引。

mod book;
mod entry;
mod grade;
mod import;
mod review;
mod store;
mod translator;

pub use book::Book;
pub use entry::Entry;
pub use grade::Grade;
pub use import::{ImportColumns, ImportPreview, preview, read_table};
pub use qingjian_core::StudyOptions;
pub use review::ReviewRecord;
pub use store::StudyStore;
pub use translator::StudyTranslator;

#[cfg(test)]
mod tests;

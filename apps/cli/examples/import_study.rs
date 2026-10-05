//! 使用共享导入器把已核验词书写入独立用户目录，保留同名词书的练习记录。
use std::path::PathBuf;

use qingjian_learning::study::{ImportColumns, StudyStore, preview, read_table};
use qingjian_platform::Config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let directory = PathBuf::from(args.next().ok_or("usage: import_study DATA_DIR CSV NAME")?);
    let file = PathBuf::from(args.next().ok_or("missing CSV")?);
    let name = args
        .next()
        .ok_or("missing name")?
        .to_string_lossy()
        .into_owned();
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let rows = read_table(&file)?;
    let header = rows.first().ok_or("empty book")?;
    let result = preview(&rows, &ImportColumns::from_header(header));
    if !result.errors.is_empty() || result.duplicates != 0 || result.entries.is_empty() {
        return Err(format!(
            "import rejected: {} errors, {} duplicates",
            result.errors.len(),
            result.duplicates
        )
        .into());
    }
    let path = directory.join("study.json");
    let mut store = StudyStore::open(&path)?;
    let replacing = store
        .books
        .iter()
        .find(|book| book.name == name)
        .map(|book| book.id.clone());
    if path.exists() {
        let backup = path.with_file_name(format!(
            "study-before-import-{}.json",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::copy(&path, backup)?;
    }
    let count = result.entries.len();
    let id = store.import(&name, result.entries, replacing.as_deref())?;
    let config_path = directory.join("config.toml");
    Config::write_template_if_missing(&config_path)?;
    Config::set_array(&config_path, "study", "selected_books", &[id])?;
    Config::set_bool(&config_path, "study", "exam_mode", true)?;
    Config::set_value(&config_path, "study", "exam_tag", "CET6")?;
    let reopened = StudyStore::open(&path)?;
    let config = Config::load(&config_path)?;
    if reopened.entries(&config.study).count() != count {
        return Err("book filters exclude imported words; check existing level filters".into());
    }
    println!("Imported {count} entries; exam mode CET6; existing privacy settings retained.");
    Ok(())
}

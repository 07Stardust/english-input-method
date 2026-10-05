//! 词书与练习数据的原子存储。只有设置程序写入，输入进程只加载快照。

use std::collections::BTreeMap;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{Book, Entry, Grade, ReviewRecord, StudyOptions};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StudyStore {
    pub books: Vec<Book>,

    pub records: BTreeMap<String, ReviewRecord>,

    #[serde(skip)]
    pub(super) path: PathBuf,
}

impl StudyStore {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut store: Self = match std::fs::File::open(path) {
            Ok(file) => {
                let mut bytes = Vec::new();
                file.take(100 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
                if bytes.len() > 100 * 1024 * 1024 {
                    return Err(io::Error::other("study store too large"));
                }
                serde_json::from_slice(&bytes).map_err(io::Error::other)?
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error),
        };
        store.path = path.to_owned();
        Ok(store)
    }

    pub fn save(&self) -> io::Result<()> {
        if self.path.as_os_str().is_empty() {
            return Err(io::Error::other("study store has no path"));
        }
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        qingjian_core::storage::write_atomic_str(
            &self.path,
            &serde_json::to_string(self).map_err(io::Error::other)?,
        )
    }

    pub fn import(
        &mut self,
        name: &str,
        entries: Vec<Entry>,
        replacing: Option<&str>,
    ) -> io::Result<String> {
        if entries.is_empty() {
            return Err(io::Error::other("词书没有有效条目"));
        }
        let mut candidate = self.clone();
        let id = if let Some(id) = replacing {
            let book = candidate
                .books
                .iter_mut()
                .find(|book| book.id == id)
                .ok_or_else(|| io::Error::other("词书不存在"))?;
            book.name = name.to_owned();
            book.entries = entries;
            id.to_owned()
        } else {
            let id = format!(
                "book-{:x}-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(io::Error::other)?
                    .as_nanos(),
                std::process::id()
            );
            candidate.books.push(Book {
                id: id.clone(),
                name: name.to_owned(),
                enabled: true,
                entries,
            });
            id
        };
        candidate.save()?;
        *self = candidate;
        Ok(id)
    }

    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> io::Result<()> {
        let mut candidate = self.clone();
        let book = candidate
            .books
            .iter_mut()
            .find(|book| book.id == id)
            .ok_or_else(|| io::Error::other("词书不存在"))?;
        book.enabled = enabled;
        candidate.save()?;
        *self = candidate;
        Ok(())
    }

    pub fn delete_book(&mut self, id: &str) -> io::Result<()> {
        let mut candidate = self.clone();
        candidate.books.retain(|book| book.id != id);
        candidate
            .records
            .retain(|key, _| !key.starts_with(&format!("{id}\t")));
        candidate.save()?;
        *self = candidate;
        Ok(())
    }

    pub fn record_key(book: &str, entry: &Entry) -> String {
        format!("{book}\t{}", entry.key())
    }

    pub fn record(&self, book: &str, entry: &Entry) -> ReviewRecord {
        self.records
            .get(&Self::record_key(book, entry))
            .cloned()
            .unwrap_or_default()
    }

    pub fn grade(&mut self, book: &str, entry: &Entry, grade: Grade, day: i64) -> io::Result<()> {
        let mut candidate = self.clone();
        candidate
            .records
            .entry(Self::record_key(book, entry))
            .or_default()
            .grade(grade, day);
        candidate.save()?;
        *self = candidate;
        Ok(())
    }

    pub fn favorite(&mut self, book: &str, entry: &Entry, favorite: bool) -> io::Result<()> {
        let mut candidate = self.clone();
        candidate
            .records
            .entry(Self::record_key(book, entry))
            .or_default()
            .favorite = favorite;
        candidate.save()?;
        *self = candidate;
        Ok(())
    }

    pub fn reset(&mut self, book: &str, entry: &Entry) -> io::Result<()> {
        let mut candidate = self.clone();
        let record = candidate
            .records
            .entry(Self::record_key(book, entry))
            .or_default();
        let favorite = record.favorite;
        *record = ReviewRecord {
            favorite,
            ..ReviewRecord::default()
        };
        candidate.save()?;
        *self = candidate;
        Ok(())
    }

    pub fn entries<'a>(
        &'a self,
        options: &'a StudyOptions,
    ) -> impl Iterator<Item = (&'a Book, &'a Entry)> {
        self.books
            .iter()
            .filter(move |book| {
                book.enabled
                    && (options.selected_books.is_empty()
                        || options.selected_books.contains(&book.id))
            })
            .flat_map(move |book| {
                book.entries
                    .iter()
                    .filter(move |entry| options.includes(&entry.level, &entry.tags))
                    .map(move |entry| (book, entry))
            })
    }

    pub fn queue(&self, options: &StudyOptions, day: i64) -> Vec<(String, Entry)> {
        let entries: Vec<_> = self.entries(options).collect();
        let introduced = entries
            .iter()
            .filter(|(book, entry)| self.record(&book.id, entry).introduced_day == Some(day))
            .count();
        let mut new_remaining = options.daily_new.min(500).saturating_sub(introduced);
        let mut due = Vec::new();
        let mut new = Vec::new();
        for (book, entry) in entries {
            let record = self.record(&book.id, entry);
            if record.mastered {
                continue;
            }
            if record.introduced_day.is_some() && record.due_day <= day {
                due.push((book.id.clone(), entry.clone()));
            } else if record.introduced_day.is_none() && new_remaining > 0 {
                new.push((book.id.clone(), entry.clone()));
                new_remaining -= 1;
            }
        }
        due.extend(new);
        due
    }

    pub fn export(&self, destination: &Path) -> io::Result<()> {
        qingjian_core::storage::write_atomic_str(
            destination,
            &serde_json::to_string_pretty(self).map_err(io::Error::other)?,
        )
    }

    pub fn clear(&mut self) -> io::Result<()> {
        let candidate = Self {
            path: self.path.clone(),
            ..Self::default()
        };
        candidate.save()?;
        *self = candidate;
        Ok(())
    }
}

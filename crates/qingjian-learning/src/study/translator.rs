//! 词书译词覆盖与考试筛选：预建中文关联，候选生成时只做内存查询。

use std::collections::HashMap;

use qingjian_core::{Language, PartOfSpeech, Sense, Translation, Translator};

use super::{Entry, StudyOptions, StudyStore};

pub struct StudyTranslator {
    inner: Box<dyn Translator>,
    language: Language,

    options: StudyOptions,

    entries: HashMap<String, Entry>,

    index: HashMap<String, Vec<Sense>>,
}

impl StudyTranslator {
    pub fn new(inner: Box<dyn Translator>, store: &StudyStore, options: StudyOptions) -> Self {
        let language = inner.language();
        Self::new_with_language(inner, store, options, language)
    }

    /// 缺少通用释义表时仍按配置语言使用词书的人工关联。
    pub fn new_with_language(
        inner: Box<dyn Translator>,
        store: &StudyStore,
        options: StudyOptions,
        language: Language,
    ) -> Self {
        let mut entries = HashMap::new();
        let mut index: HashMap<String, Vec<Sense>> = HashMap::new();
        for (_, entry) in store.entries(&options) {
            entries.entry(entry.key()).or_insert_with(|| entry.clone());
            for key in &entry.input_keys {
                let sense = Self::sense(entry);
                let list = index.entry(key.trim().to_owned()).or_default();
                if !list
                    .iter()
                    .any(|item| item.text.eq_ignore_ascii_case(&sense.text))
                {
                    list.push(sense);
                }
            }
        }
        Self {
            inner,
            language,
            options,
            entries,
            index,
        }
    }

    fn sense(entry: &Entry) -> Sense {
        Sense {
            text: entry.english.clone(),
            part_of_speech: entry.part_of_speech.parse::<PartOfSpeech>().ok(),
            reading: None,
            fresh: false,
        }
    }
}

impl Translator for StudyTranslator {
    fn language(&self) -> Language {
        self.language
    }

    fn translate(&self, text: &str) -> Option<Translation> {
        if !self.options.enabled {
            return None;
        }
        if self.language() != Language::English {
            return self.inner.translate(text);
        }
        if let Some(senses) = self.index.get(text).filter(|senses| !senses.is_empty()) {
            return Some(Translation::new(Language::English, senses.clone()));
        }
        let bundled = self.inner.translate(text);
        let filtered = bundled
            .as_ref()
            .map(|translation| {
                translation
                    .senses()
                    .iter()
                    .filter_map(|sense| {
                        self.entries
                            .get(&sense.text.trim().to_lowercase())
                            .map(Self::sense)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !filtered.is_empty() {
            return Some(Translation::new(Language::English, filtered));
        }
        if self.options.exam_mode {
            None
        } else {
            bundled
        }
    }

    fn learn(&mut self, word: &str, translation: Translation) {
        if self.options.enabled && !self.options.privacy && !self.options.exam_mode {
            self.inner.learn(word, translation);
        }
    }

    fn flush(&mut self) {
        self.inner.flush();
    }
}

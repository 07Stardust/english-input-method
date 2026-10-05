//! 学习译者装配：一次读词书快照，候选路径不接触导入文件。

use std::path::Path;

use qingjian_core::{Language, NoTranslator, Translator};
use qingjian_learning::study::{StudyStore, StudyTranslator};
use qingjian_platform::Config;

pub(crate) fn apply(
    engine: &mut qingjian_core::Engine,
    config: &Config,
    root: &Path,
    user: Option<&Path>,
) {
    engine.set_study_enabled(config.study.enabled);
    let inner: Box<dyn Translator> = match super::learning_language(config)
        .and_then(|language| super::glossary_file(root, language).map(|path| (language, path)))
    {
        Some((language, path)) => match super::load_glossary(language, &path, user) {
            Ok(value) => Box::new(value),
            Err(_) => Box::new(NoTranslator),
        },
        None => Box::new(NoTranslator),
    };
    let store = user
        .map(|dir| StudyStore::open(&dir.join("study.json")))
        .transpose();
    match store {
        Ok(store) => {
            let store = store.unwrap_or_default();
            engine.set_translator(Box::new(StudyTranslator::new_with_language(
                inner,
                &store,
                config.study.clone(),
                super::learning_language(config).unwrap_or(Language::Chinese),
            )));
        }
        Err(error) => {
            tracing::warn!(%error, "词书加载失败");
            // 考试模式不能因坏文件悄悄变成通用词汇。
            if config.study.exam_mode {
                engine.set_translator(Box::new(NoTranslator));
            } else {
                engine.set_translator(inner);
            }
        }
    }
}

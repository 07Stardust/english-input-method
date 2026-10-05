use crate::study::Entry;
#[derive(Debug, Clone, Default)]
pub struct ImportPreview {
    pub entries: Vec<Entry>,

    pub duplicates: usize,

    pub errors: Vec<String>,
}

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub english: String,

    pub meaning: String,

    pub part_of_speech: String,

    pub example: String,

    pub level: String,

    pub tags: Vec<String>,

    pub input_keys: Vec<String>,

    pub equivalents: Vec<String>,
}

impl Entry {
    pub fn key(&self) -> String {
        self.english.trim().to_lowercase()
    }

    pub fn matches_answer(&self, answer: &str) -> bool {
        let answer = answer.trim().to_lowercase();
        answer == self.key()
            || self
                .equivalents
                .iter()
                .any(|value| value.trim().to_lowercase() == answer)
    }
}

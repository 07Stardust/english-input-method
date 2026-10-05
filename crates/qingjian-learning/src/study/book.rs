use super::Entry;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Book {
    pub id: String,

    pub name: String,

    pub enabled: bool,

    pub entries: Vec<Entry>,
}

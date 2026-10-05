use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct Hello {
    pub protocol: u32,
}

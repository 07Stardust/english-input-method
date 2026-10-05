use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct Welcome {
    pub protocol: u32,
}

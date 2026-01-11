use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TypeGenMode {
    Auto,
    Offline,
    Llm,
}

impl Default for TypeGenMode {
    fn default() -> Self {
        Self::Auto
    }
}

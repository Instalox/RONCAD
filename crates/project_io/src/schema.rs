//! On-disk schema for project files. Versioned so migration can slot in later.

use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub schema_version: u32,
    pub name: String,
}

impl Default for ProjectFile {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            name: "Untitled".to_string(),
        }
    }
}

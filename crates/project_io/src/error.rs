use thiserror::Error;

use crate::schema::CURRENT_SCHEMA_VERSION;

#[derive(Debug, Error)]
pub enum ProjectIoError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported project schema version {found}; expected {expected}")]
    UnsupportedSchemaVersion { found: u32, expected: u32 },
    #[error("project file contains a duplicate {kind} id {id}")]
    DuplicateId { kind: &'static str, id: u64 },
    #[error("project file references missing {kind} id {id}")]
    MissingReference { kind: &'static str, id: u64 },
    #[error("project contains a dangling {kind} reference")]
    DanglingReference { kind: &'static str },
}

impl ProjectIoError {
    pub(crate) fn unsupported_schema(found: u32) -> Self {
        Self::UnsupportedSchemaVersion {
            found,
            expected: CURRENT_SCHEMA_VERSION,
        }
    }
}

pub type Result<T> = std::result::Result<T, ProjectIoError>;

//! History row types (FR-HIS-1).

use serde::{Deserialize, Serialize};

/// Outcome of inserting dictated text into the target application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsertStatus {
    Success,
    Failed,
    Skipped,
}

impl InsertStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "success" => Some(Self::Success),
            "failed" => Some(Self::Failed),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

/// Fields supplied when persisting a new utterance (id and `created_at` are assigned by the store).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewHistoryEntry {
    pub duration_ms: u32,
    pub mode: String,
    pub src_lang: String,
    pub dst_lang: String,
    pub asr_model: String,
    pub mt_model: Option<String>,
    pub llm_model: Option<String>,
    pub app_id: Option<String>,
    pub raw_text: String,
    pub final_text: String,
    pub insert_status: InsertStatus,
    pub cleanup_flagged: bool,
    pub audio_path: Option<String>,
    pub pinned: bool,
}

/// A stored history row. Primary key is SQLite `INTEGER PRIMARY KEY` (`i64` rowid).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    /// Unix epoch milliseconds (UTC).
    pub created_at: i64,
    pub duration_ms: u32,
    pub mode: String,
    pub src_lang: String,
    pub dst_lang: String,
    pub asr_model: String,
    pub mt_model: Option<String>,
    pub llm_model: Option<String>,
    pub app_id: Option<String>,
    pub raw_text: String,
    pub final_text: String,
    pub insert_status: InsertStatus,
    pub cleanup_flagged: bool,
    pub audio_path: Option<String>,
    pub pinned: bool,
}

//! Shared value types for the Narad D-Bus API.

use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

/// D-Bus `StateChanged` state values (ARCHITECTURE §6).
pub struct SessionState;

impl SessionState {
    pub const IDLE: &str = "idle";
    pub const LISTENING: &str = "listening";
    pub const PROCESSING: &str = "processing";
    pub const ERROR: &str = "error";
    pub const COPIED: &str = "copied";
    /// Brief outcome after successful paste (FR-DOT-2); daemon returns to idle shortly after.
    pub const INSERTED: &str = "inserted";
    /// Brief outcome after cancel (FR-DOT-2); daemon returns to idle shortly after.
    pub const CANCELLED: &str = "cancelled";
}

/// One history row returned by [`DaemonProxy::history`].
///
/// Field layout matches `narad_core::store::HistoryEntry` (FR-HIS-1) for straightforward mapping in
/// `naradd`. `insert_status` uses the same string values as `InsertStatus::as_str` in core
/// (`success`, `failed`, `skipped`).
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    Type,
)]
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
    pub insert_status: String,
    pub cleanup_flagged: bool,
    pub audio_path: Option<String>,
    pub pinned: bool,
}

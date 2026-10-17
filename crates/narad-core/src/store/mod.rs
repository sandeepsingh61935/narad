//! SQLite + FTS5 history store (FR-HIS-1, FR-HIS-3, FR-HIS-6, FR-HIS-7, FR-HIS-8).
//!
//! Entry primary keys are `i64` values from SQLite `INTEGER PRIMARY KEY AUTOINCREMENT`
//! (the stable `rowid` for FTS5 external-content sync).

mod document;
mod entry;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Row};
use thiserror::Error;

pub use document::{
    Document, DocumentChunk, DocumentStatus, DocumentSummary, NewDocumentChunk, SpeakerBlock,
};
pub use entry::{HistoryEntry, InsertStatus, NewHistoryEntry};

fn ms_value(ms: Option<i64>) -> rusqlite::types::Value {
    match ms {
        Some(ms) => rusqlite::types::Value::Integer(ms),
        None => rusqlite::types::Value::Null,
    }
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("invalid insert_status in database")]
    InvalidInsertStatus,
    #[error("invalid document_status in database: {0}")]
    InvalidDocumentStatus(String),
    #[error("system time before Unix epoch")]
    InvalidTimestamp,
    #[error("no history row {0}")]
    Missing(i64),
    #[error("unknown export format: {0}")]
    UnknownFormat(String),
}

/// Dictation totals for one inclusive-exclusive millisecond range (FR-HIS-8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HistoryStats {
    pub words: u64,
    pub utterances: u64,
    pub speaking_wpm: u64,
    pub time_saved_ms: i64,
}

/// How many rows a retention pass changed (FR-HIS-7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PurgeStats {
    /// History rows deleted because they were older than the text TTL.
    pub text_deleted: u32,
    /// Rows whose audio file was unlinked and `audio_path` cleared.
    pub audio_cleared: u32,
}

/// Optional filters for [`HistoryStore::find`]. An empty string is omitted.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryFilter {
    pub mode: String,
    pub app_id: String,
    pub src_lang: String,
    /// Inclusive lower bound on `created_at`, Unix milliseconds. `None` omits it.
    pub from_ms: Option<i64>,
    /// Exclusive upper bound on `created_at`, Unix milliseconds. `None` omits it.
    pub to_ms: Option<i64>,
}

/// History database: `entries` table plus FTS5 virtual table `entries_fts`.
pub struct HistoryStore {
    pub(crate) conn: Connection,
}

impl HistoryStore {
    /// Open or create a history database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    /// Insert a new utterance; returns the assigned entry id.
    pub fn insert(&mut self, entry: &NewHistoryEntry) -> Result<i64, StoreError> {
        let created_at = now_millis()?;
        self.conn.execute(
            r"
            INSERT INTO entries (
                created_at, duration_ms, mode, src_lang, dst_lang,
                asr_model, mt_model, llm_model, app_id,
                raw_text, final_text, insert_status, cleanup_flagged,
                audio_path, pinned, kind
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13,
                ?14, ?15, ?16
            )
            ",
            params![
                created_at,
                entry.duration_ms,
                entry.mode,
                entry.src_lang,
                entry.dst_lang,
                entry.asr_model,
                entry.mt_model,
                entry.llm_model,
                entry.app_id,
                entry.raw_text,
                entry.final_text,
                entry.insert_status.as_str(),
                entry.cleanup_flagged as i32,
                entry.audio_path,
                entry.pinned as i32,
                entry.kind,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Fetch a single entry by primary key.
    pub fn get(&self, id: i64) -> Result<Option<HistoryEntry>, StoreError> {
        let entry = {
            let mut stmt = self.conn.prepare(
                r"
                SELECT
                    id, created_at, duration_ms, mode, src_lang, dst_lang,
                    asr_model, mt_model, llm_model, app_id,
                    raw_text, final_text, insert_status, cleanup_flagged,
                    audio_path, pinned, kind
                FROM entries
                WHERE id = ?1
                ",
            )?;
            let mut rows = stmt.query_map(params![id], row_to_entry)?;
            match rows.next() {
                Some(Ok(entry)) => Some(entry),
                Some(Err(e)) => return Err(StoreError::from(e)),
                None => None,
            }
        };
        match entry {
            Some(mut entry) => {
                entry.tags = tags_for(&self.conn, entry.id)?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    /// List entries newest-first.
    pub fn list(&self, offset: u32, limit: u32) -> Result<Vec<HistoryEntry>, StoreError> {
        let entries = {
            let mut stmt = self.conn.prepare(
                r"
                SELECT
                    id, created_at, duration_ms, mode, src_lang, dst_lang,
                    asr_model, mt_model, llm_model, app_id,
                    raw_text, final_text, insert_status, cleanup_flagged,
                    audio_path, pinned, kind
                FROM entries
                ORDER BY created_at DESC, id DESC
                LIMIT ?1 OFFSET ?2
                ",
            )?;
            let rows = stmt.query_map(params![limit, offset], row_to_entry)?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        with_tags(&self.conn, entries)
    }

    /// Full-text search over `raw_text` and `final_text` (FTS5).
    pub fn search(
        &self,
        query: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<HistoryEntry>, StoreError> {
        let entries = {
            let mut stmt = self.conn.prepare(
                r"
                SELECT
                    e.id, e.created_at, e.duration_ms, e.mode, e.src_lang, e.dst_lang,
                    e.asr_model, e.mt_model, e.llm_model, e.app_id,
                    e.raw_text, e.final_text, e.insert_status, e.cleanup_flagged,
                    e.audio_path, e.pinned, e.kind
                FROM entries e
                INNER JOIN entries_fts ON entries_fts.rowid = e.id
                WHERE entries_fts MATCH ?1
                ORDER BY e.created_at DESC, e.id DESC
                LIMIT ?2 OFFSET ?3
                ",
            )?;
            let rows = stmt.query_map(params![query, limit, offset], row_to_entry)?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        with_tags(&self.conn, entries)
    }

    /// Dictation rows with `created_at` in `[from_ms, to_ms)`.
    ///
    /// Words are whitespace tokens of `final_text`. `file` and `retranscribe` are excluded.
    /// Speaking WPM is `words * 60000 / duration_ms`, or 0 when the duration sum is 0.
    /// Time saved is `words * 1500 - duration_ms` when that is positive, otherwise 0.
    pub fn stats(&self, from_ms: i64, to_ms: i64) -> Result<HistoryStats, StoreError> {
        let mut stmt = self.conn.prepare(
            "SELECT final_text, duration_ms FROM entries
             WHERE kind = 'dictation'
               AND created_at >= ?1
               AND created_at < ?2",
        )?;
        let rows = stmt.query_map(params![from_ms, to_ms], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        let mut words = 0u64;
        let mut utterances = 0u64;
        let mut duration_ms = 0u64;
        for row in rows {
            let (text, duration) = row?;
            words += u64::try_from(text.split_whitespace().count()).unwrap_or(u64::MAX);
            utterances += 1;
            if duration > 0 {
                duration_ms += duration as u64;
            }
        }
        let speaking_wpm = if duration_ms == 0 {
            0
        } else {
            words.saturating_mul(60_000) / duration_ms
        };
        let typing_ms = words.saturating_mul(1_500);
        let time_saved_ms =
            i64::try_from(typing_ms.saturating_sub(duration_ms)).unwrap_or(i64::MAX);
        Ok(HistoryStats {
            words,
            utterances,
            speaking_wpm,
            time_saved_ms,
        })
    }

    /// List or full-text search, narrowed by `filter` (FR-HIS-3).
    ///
    /// An empty `query` lists rows. Filters combine with AND. An empty filter field is omitted.
    pub fn find(
        &self,
        query: &str,
        filter: &HistoryFilter,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<HistoryEntry>, StoreError> {
        let columns = "
            e.id, e.created_at, e.duration_ms, e.mode, e.src_lang, e.dst_lang,
            e.asr_model, e.mt_model, e.llm_model, e.app_id,
            e.raw_text, e.final_text, e.insert_status, e.cleanup_flagged,
            e.audio_path, e.pinned, e.kind
        ";
        let (sql, params_owned): (String, Vec<rusqlite::types::Value>) = if query.is_empty() {
            (
                format!(
                    "SELECT {columns}
                     FROM entries e
                     WHERE (?1 = '' OR e.mode = ?1)
                       AND (?2 = '' OR e.app_id = ?2)
                       AND (?3 = '' OR e.src_lang = ?3)
                       AND (?4 IS NULL OR e.created_at >= ?4)
                       AND (?5 IS NULL OR e.created_at < ?5)
                     ORDER BY e.created_at DESC, e.id DESC
                     LIMIT ?6 OFFSET ?7"
                ),
                vec![
                    filter.mode.clone().into(),
                    filter.app_id.clone().into(),
                    filter.src_lang.clone().into(),
                    ms_value(filter.from_ms),
                    ms_value(filter.to_ms),
                    limit.into(),
                    offset.into(),
                ],
            )
        } else {
            (
                format!(
                    "SELECT {columns}
                     FROM entries e
                     INNER JOIN entries_fts ON entries_fts.rowid = e.id
                     WHERE entries_fts MATCH ?1
                       AND (?2 = '' OR e.mode = ?2)
                       AND (?3 = '' OR e.app_id = ?3)
                       AND (?4 = '' OR e.src_lang = ?4)
                       AND (?5 IS NULL OR e.created_at >= ?5)
                       AND (?6 IS NULL OR e.created_at < ?6)
                     ORDER BY e.created_at DESC, e.id DESC
                     LIMIT ?7 OFFSET ?8"
                ),
                vec![
                    query.to_owned().into(),
                    filter.mode.clone().into(),
                    filter.app_id.clone().into(),
                    filter.src_lang.clone().into(),
                    ms_value(filter.from_ms),
                    ms_value(filter.to_ms),
                    limit.into(),
                    offset.into(),
                ],
            )
        };
        let entries = {
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(rusqlite::params_from_iter(params_owned), row_to_entry)?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        with_tags(&self.conn, entries)
    }

    /// Record the retained WAV path for an existing row.
    pub fn set_audio_path(&mut self, id: i64, path: &str) -> Result<(), StoreError> {
        let changed = self.conn.execute(
            "UPDATE entries SET audio_path = ?1 WHERE id = ?2",
            params![path, id],
        )?;
        if changed == 0 {
            return Err(StoreError::Missing(id));
        }
        Ok(())
    }

    /// Set `pinned` on one row. A missing id is an error and changes nothing.
    pub fn set_pinned(&mut self, id: i64, pinned: bool) -> Result<(), StoreError> {
        ensure_entry(&self.conn, id)?;
        self.conn.execute(
            "UPDATE entries SET pinned = ?1 WHERE id = ?2",
            params![pinned as i32, id],
        )?;
        Ok(())
    }

    /// Replace one entry's tags. Trims names, drops blanks and exact duplicates.
    ///
    /// Comparison is case-sensitive. Order is the first kept name. A missing id
    /// is an error and writes no tag rows.
    pub fn set_tags(&mut self, id: i64, names: &[&str]) -> Result<(), StoreError> {
        let kept = kept_tag_names(names);
        let tx = self.conn.transaction()?;
        ensure_entry(&tx, id)?;
        tx.execute("DELETE FROM entry_tags WHERE entry_id = ?1", params![id])?;
        for name in &kept {
            tx.execute(
                "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
                params![name],
            )?;
            let tag_id: i64 = tx.query_row(
                "SELECT id FROM tags WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )?;
            tx.execute(
                "INSERT INTO entry_tags (entry_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Delete entries by id. Returns the number of rows removed.
    pub fn delete(&mut self, ids: &[i64]) -> Result<u32, StoreError> {
        if ids.is_empty() {
            return Ok(0);
        }
        for id in ids {
            if let Some(path) = self.get(*id)?.and_then(|entry| entry.audio_path) {
                let _ = unlink_audio(&path);
            }
        }
        let mut deleted = 0u32;
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare("DELETE FROM entries WHERE id = ?1")?;
            for id in ids {
                deleted += stmt.execute(params![id])? as u32;
            }
        }
        tx.commit()?;
        Ok(deleted)
    }

    /// One document for `ids`, in that order. A missing id returns no document.
    ///
    /// The caller chooses the rows. This does not apply list filters.
    pub fn export(&self, ids: &[i64], format: &str) -> Result<String, StoreError> {
        let mut entries = Vec::with_capacity(ids.len());
        for id in ids {
            match self.get(*id)? {
                Some(entry) => entries.push(entry),
                None => return Err(StoreError::Missing(*id)),
            }
        }
        export_document(&entries, format)
    }

    /// Delete every history row and unlink stored audio. A missing file still counts.
    pub fn delete_all(&mut self) -> Result<u32, StoreError> {
        let paths = {
            let mut stmt = self
                .conn
                .prepare("SELECT audio_path FROM entries WHERE audio_path IS NOT NULL")?;
            let paths = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            paths
        };
        for path in paths {
            let _ = unlink_audio(&path);
        }
        let deleted = self.conn.execute("DELETE FROM entries", [])?;
        Ok(deleted as u32)
    }

    /// Apply text and audio retention (FR-HIS-7).
    ///
    /// `now_ms` is Unix epoch milliseconds. A `None` duration keeps that kind
    /// forever and touches nothing. Text TTL deletes the row. Audio TTL unlinks
    /// the file when `audio_path` is set and nulls `audio_path`; the text row stays.
    /// A text delete also unlinks audio so the file is not orphaned.
    pub fn purge_retention(
        &mut self,
        now_ms: i64,
        text_retention_days: Option<u32>,
        audio_retention_days: Option<u32>,
    ) -> Result<PurgeStats, StoreError> {
        let mut stats = PurgeStats::default();
        if let Some(days) = text_retention_days {
            stats.text_deleted = self.delete_older_than(cutoff_ms(now_ms, days))?;
        }
        if let Some(days) = audio_retention_days {
            stats.audio_cleared = self.clear_audio_older_than(cutoff_ms(now_ms, days))?;
        }
        Ok(stats)
    }

    /// Delete rows with `created_at` strictly before `cutoff_ms`.
    fn delete_older_than(&mut self, cutoff_ms: i64) -> Result<u32, StoreError> {
        let paths = audio_paths_before(&self.conn, cutoff_ms)?;
        for path in paths {
            let _ = unlink_audio(&path);
        }
        let deleted = self.conn.execute(
            "DELETE FROM entries WHERE created_at < ?1",
            params![cutoff_ms],
        )?;
        Ok(deleted as u32)
    }

    /// Unlink audio and null `audio_path` for rows strictly before `cutoff_ms`.
    fn clear_audio_older_than(&mut self, cutoff_ms: i64) -> Result<u32, StoreError> {
        let rows = {
            let mut stmt = self.conn.prepare(
                "SELECT id, audio_path FROM entries
                 WHERE created_at < ?1 AND audio_path IS NOT NULL",
            )?;
            let collected = stmt
                .query_map(params![cutoff_ms], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            collected
        };
        let mut cleared = 0u32;
        for (id, path) in rows {
            if !path.is_empty() && unlink_audio(&path).is_err() {
                continue;
            }
            cleared += self.conn.execute(
                "UPDATE entries SET audio_path = NULL WHERE id = ?1 AND audio_path IS NOT NULL",
                params![id],
            )? as u32;
        }
        Ok(cleared)
    }
}

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

fn cutoff_ms(now_ms: i64, days: u32) -> i64 {
    now_ms.saturating_sub((days as i64).saturating_mul(DAY_MS))
}

fn audio_paths_before(conn: &Connection, cutoff_ms: i64) -> Result<Vec<String>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT audio_path FROM entries
         WHERE created_at < ?1 AND audio_path IS NOT NULL",
    )?;
    let paths = stmt
        .query_map(params![cutoff_ms], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(paths)
}

/// Remove a retained audio file. Missing files count as already gone.
fn unlink_audio(path: &str) -> std::io::Result<()> {
    if path.is_empty() {
        return Ok(());
    }
    match std::fs::remove_file(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}

/// Build one export document. `format` is `json`, `markdown`, or `txt` after trim.
///
/// JSON is a compact array in entry order and includes pin, tags, and `audio_path`.
/// It does not read the audio file. Markdown and txt are `final_text` only, with a
/// blank line between entries and no trailing newline.
pub fn export_document(entries: &[HistoryEntry], format: &str) -> Result<String, StoreError> {
    match format.trim() {
        "json" => Ok(json_document(entries)),
        "markdown" | "txt" => Ok(entries
            .iter()
            .map(|entry| entry.final_text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")),
        other => Err(StoreError::UnknownFormat(other.to_string())),
    }
}

fn json_document(entries: &[HistoryEntry]) -> String {
    let mut out = String::from("[");
    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_entry(&mut out, entry);
    }
    out.push(']');
    out
}

fn push_json_entry(out: &mut String, entry: &HistoryEntry) {
    out.push('{');
    push_json_key(out, "id");
    push_json_i64(out, entry.id);
    push_json_key(out, "created_at");
    push_json_i64(out, entry.created_at);
    push_json_key(out, "duration_ms");
    push_json_u32(out, entry.duration_ms);
    push_json_key(out, "mode");
    push_json_string(out, &entry.mode);
    push_json_key(out, "src_lang");
    push_json_string(out, &entry.src_lang);
    push_json_key(out, "dst_lang");
    push_json_string(out, &entry.dst_lang);
    push_json_key(out, "asr_model");
    push_json_string(out, &entry.asr_model);
    push_json_key(out, "mt_model");
    push_json_opt_string(out, &entry.mt_model);
    push_json_key(out, "llm_model");
    push_json_opt_string(out, &entry.llm_model);
    push_json_key(out, "app_id");
    push_json_opt_string(out, &entry.app_id);
    push_json_key(out, "raw_text");
    push_json_string(out, &entry.raw_text);
    push_json_key(out, "final_text");
    push_json_string(out, &entry.final_text);
    push_json_key(out, "insert_status");
    push_json_string(out, entry.insert_status.as_str());
    push_json_key(out, "cleanup_flagged");
    push_json_bool(out, entry.cleanup_flagged);
    push_json_key(out, "audio_path");
    push_json_opt_string(out, &entry.audio_path);
    push_json_key(out, "pinned");
    push_json_bool(out, entry.pinned);
    push_json_key(out, "kind");
    push_json_string(out, &entry.kind);
    push_json_key(out, "tags");
    out.push('[');
    for (index, tag) in entry.tags.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_json_string(out, tag);
    }
    out.push(']');
    out.push('}');
}

fn push_json_key(out: &mut String, key: &str) {
    if !out.ends_with('{') {
        out.push(',');
    }
    push_json_string(out, key);
    out.push(':');
}

fn push_json_i64(out: &mut String, value: i64) {
    out.push_str(&value.to_string());
}

fn push_json_u32(out: &mut String, value: u32) {
    out.push_str(&value.to_string());
}

fn push_json_bool(out: &mut String, value: bool) {
    out.push_str(if value { "true" } else { "false" });
}

fn push_json_opt_string(out: &mut String, value: &Option<String>) {
    match value {
        Some(text) => push_json_string(out, text),
        None => out.push_str("null"),
    }
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

pub(crate) fn now_millis() -> Result<i64, StoreError> {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StoreError::InvalidTimestamp)?;
    Ok(dur.as_millis() as i64)
}

fn row_to_entry(row: &Row) -> Result<HistoryEntry, rusqlite::Error> {
    let status_str: String = row.get(12)?;
    let insert_status = InsertStatus::from_str(&status_str).ok_or_else(|| {
        rusqlite::Error::InvalidColumnType(12, "insert_status".into(), rusqlite::types::Type::Text)
    })?;
    Ok(HistoryEntry {
        id: row.get(0)?,
        created_at: row.get(1)?,
        duration_ms: row.get::<_, i64>(2)? as u32,
        mode: row.get(3)?,
        src_lang: row.get(4)?,
        dst_lang: row.get(5)?,
        asr_model: row.get(6)?,
        mt_model: row.get(7)?,
        llm_model: row.get(8)?,
        app_id: row.get(9)?,
        raw_text: row.get(10)?,
        final_text: row.get(11)?,
        insert_status,
        cleanup_flagged: row.get::<_, i32>(13)? != 0,
        audio_path: row.get(14)?,
        pinned: row.get::<_, i32>(15)? != 0,
        kind: row.get(16)?,
        tags: Vec::new(),
    })
}

fn ensure_entry(conn: &Connection, id: i64) -> Result<(), StoreError> {
    match conn.query_row("SELECT 1 FROM entries WHERE id = ?1", params![id], |_| {
        Ok(())
    }) {
        Ok(()) => Ok(()),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(StoreError::Missing(id)),
        Err(err) => Err(err.into()),
    }
}

/// Trim, drop blanks, drop a later exact duplicate. `A` and `a` both stay.
fn kept_tag_names(names: &[&str]) -> Vec<String> {
    let mut kept = Vec::new();
    for name in names {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            continue;
        }
        if kept.iter().any(|existing: &String| existing == trimmed) {
            continue;
        }
        kept.push(trimmed.to_string());
    }
    kept
}

fn tags_for(conn: &Connection, entry_id: i64) -> Result<Vec<String>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT t.name
         FROM entry_tags et
         JOIN tags t ON t.id = et.tag_id
         WHERE et.entry_id = ?1
         ORDER BY et.rowid",
    )?;
    let names = stmt
        .query_map(params![entry_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(names)
}

fn with_tags(
    conn: &Connection,
    mut entries: Vec<HistoryEntry>,
) -> Result<Vec<HistoryEntry>, StoreError> {
    for entry in &mut entries {
        entry.tags = tags_for(conn, entry.id)?;
    }
    Ok(entries)
}

fn migrate(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(
        r"
        CREATE TABLE IF NOT EXISTS entries (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            mode TEXT NOT NULL,
            src_lang TEXT NOT NULL,
            dst_lang TEXT NOT NULL,
            asr_model TEXT NOT NULL,
            mt_model TEXT,
            llm_model TEXT,
            app_id TEXT,
            raw_text TEXT NOT NULL,
            final_text TEXT NOT NULL,
            insert_status TEXT NOT NULL,
            cleanup_flagged INTEGER NOT NULL DEFAULT 0,
            audio_path TEXT,
            pinned INTEGER NOT NULL DEFAULT 0,
            kind TEXT NOT NULL DEFAULT 'dictation'
        );

        CREATE VIRTUAL TABLE IF NOT EXISTS entries_fts USING fts5(
            raw_text,
            final_text,
            content='entries',
            content_rowid='id'
        );

        CREATE TRIGGER IF NOT EXISTS entries_ai AFTER INSERT ON entries BEGIN
            INSERT INTO entries_fts(rowid, raw_text, final_text)
            VALUES (new.id, new.raw_text, new.final_text);
        END;

        CREATE TRIGGER IF NOT EXISTS entries_ad AFTER DELETE ON entries BEGIN
            INSERT INTO entries_fts(entries_fts, rowid, raw_text, final_text)
            VALUES ('delete', old.id, old.raw_text, old.final_text);
        END;

        CREATE TRIGGER IF NOT EXISTS entries_au AFTER UPDATE ON entries BEGIN
            INSERT INTO entries_fts(entries_fts, rowid, raw_text, final_text)
            VALUES ('delete', old.id, old.raw_text, old.final_text);
            INSERT INTO entries_fts(rowid, raw_text, final_text)
            VALUES (new.id, new.raw_text, new.final_text);
        END;

        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS entry_tags (
            entry_id INTEGER NOT NULL,
            tag_id INTEGER NOT NULL,
            PRIMARY KEY (entry_id, tag_id),
            FOREIGN KEY (entry_id) REFERENCES entries(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id)
        );

        CREATE TABLE IF NOT EXISTS documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'active'
        );

        CREATE TABLE IF NOT EXISTS document_chunks (
            doc_id INTEGER NOT NULL,
            chunk_index INTEGER NOT NULL,
            speaker_label TEXT,
            text TEXT NOT NULL,
            start_ms INTEGER NOT NULL,
            duration_ms INTEGER NOT NULL,
            PRIMARY KEY (doc_id, chunk_index),
            FOREIGN KEY (doc_id) REFERENCES documents(id) ON DELETE CASCADE
        );
        ",
    )?;
    ensure_kind_column(conn)?;
    Ok(())
}

/// Databases created before replay have no `kind`. Existing rows stay `dictation`.
fn ensure_kind_column(conn: &Connection) -> Result<(), StoreError> {
    let found = {
        let mut stmt = conn.prepare("PRAGMA table_info(entries)")?;
        let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
        names.collect::<Result<Vec<_>, _>>()?
    };
    if !found.iter().any(|name| name == "kind") {
        conn.execute(
            "ALTER TABLE entries ADD COLUMN kind TEXT NOT NULL DEFAULT 'dictation'",
            [],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    use tempfile::NamedTempFile;

    fn sample_new(raw: &str, final_text: &str) -> NewHistoryEntry {
        NewHistoryEntry {
            duration_ms: 1200,
            mode: "dictation".into(),
            src_lang: "en".into(),
            dst_lang: "en".into(),
            asr_model: "parakeet-test".into(),
            mt_model: None,
            llm_model: None,
            app_id: Some("org.gnome.Terminal".into()),
            raw_text: raw.into(),
            final_text: final_text.into(),
            insert_status: InsertStatus::Success,
            cleanup_flagged: false,
            audio_path: None,
            pinned: false,
            kind: "dictation".into(),
        }
    }

    #[test]
    fn open_insert_list_search_delete() {
        let file = NamedTempFile::new().expect("temp db");
        let path = file.path();

        let mut store = HistoryStore::open(path).expect("open");

        let id1 = store
            .insert(&sample_new("hello world", "Hello, world."))
            .expect("insert 1");
        thread::sleep(Duration::from_millis(2));
        let id2 = store
            .insert(&sample_new("narad voice stack", "Narad voice stack."))
            .expect("insert 2");

        let listed = store.list(0, 10).expect("list");
        assert_eq!(listed.len(), 2);
        assert!(listed[0].created_at >= listed[1].created_at);
        assert_eq!(listed[0].id, id2);
        assert_eq!(listed[1].id, id1);
        assert_eq!(listed[0].final_text, "Narad voice stack.");

        let hits = store.search("narad", 0, 10).expect("search");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, id2);
        assert!(hits[0].raw_text.contains("narad"));

        let removed = store.delete(&[id1]).expect("delete");
        assert_eq!(removed, 1);

        let listed_after = store.list(0, 10).expect("list after delete");
        assert_eq!(listed_after.len(), 1);
        assert_eq!(listed_after[0].id, id2);

        let hits_after = store.search("hello", 0, 10).expect("search after delete");
        assert_eq!(hits_after.len(), 0);
    }

    #[test]
    fn insert_status_roundtrip() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let mut entry = sample_new("x", "y");
        entry.insert_status = InsertStatus::Skipped;
        let id = store.insert(&entry).expect("insert");
        let rows = store.list(0, 1).expect("list");
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0].insert_status, InsertStatus::Skipped);
    }

    fn set_created_at(store: &HistoryStore, id: i64, created_at: i64) {
        store
            .conn
            .execute(
                "UPDATE entries SET created_at = ?1 WHERE id = ?2",
                params![created_at, id],
            )
            .expect("backdate");
    }

    #[test]
    fn text_ttl_deletes_rows_older_than_n_days() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let old_id = store
            .insert(&sample_new("old line", "old line"))
            .expect("old");
        let edge_id = store.insert(&sample_new("edge", "edge")).expect("edge");
        let fresh_id = store.insert(&sample_new("fresh", "fresh")).expect("fresh");

        let now = 1_700_000_000_000_i64;
        let days: u32 = 4;
        set_created_at(&store, old_id, cutoff_ms(now, days) - 1);
        set_created_at(&store, edge_id, cutoff_ms(now, days));
        set_created_at(&store, fresh_id, now);

        let stats = store.purge_retention(now, Some(days), None).expect("purge");
        assert_eq!(stats.text_deleted, 1);
        assert_eq!(stats.audio_cleared, 0);
        assert!(store.get(old_id).expect("get old").is_none());
        assert!(store.get(edge_id).expect("get edge").is_some());
        assert_eq!(
            store
                .get(fresh_id)
                .expect("get fresh")
                .expect("fresh")
                .final_text,
            "fresh"
        );
        let hits = store.search("old", 0, 10).expect("search");
        assert!(hits.is_empty());
    }

    #[test]
    fn blank_text_ttl_deletes_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav = dir.path().join("keep.wav");
        std::fs::write(&wav, b"not-audio").expect("wav");
        let mut store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let mut entry = sample_new("keep me", "keep me");
        entry.audio_path = Some(wav.display().to_string());
        let id = store.insert(&entry).expect("insert");
        let now = 1_700_000_000_000_i64;
        set_created_at(&store, id, now - 400 * DAY_MS);

        let stats = store.purge_retention(now, None, None).expect("purge");
        assert_eq!(stats, PurgeStats::default());
        let row = store.get(id).expect("get").expect("row");
        assert_eq!(row.final_text, "keep me");
        assert_eq!(row.audio_path.as_deref(), Some(wav.to_str().unwrap()));
        assert!(wav.is_file());
    }

    #[test]
    fn audio_ttl_nulls_path_and_keeps_the_row() {
        let dir = tempfile::tempdir().expect("tempdir");
        let old_wav = dir.path().join("old.wav");
        let fresh_wav = dir.path().join("fresh.wav");
        std::fs::write(&old_wav, b"old").expect("old wav");
        std::fs::write(&fresh_wav, b"fresh").expect("fresh wav");
        let mut store = HistoryStore::open(dir.path().join("t.db")).expect("open");

        let mut old = sample_new("spoken", "spoken");
        old.audio_path = Some(old_wav.display().to_string());
        let old_id = store.insert(&old).expect("old");
        let mut fresh = sample_new("later", "later");
        fresh.audio_path = Some(fresh_wav.display().to_string());
        let fresh_id = store.insert(&fresh).expect("fresh");

        let now = 1_700_000_000_000_i64;
        let days: u32 = 3;
        set_created_at(&store, old_id, cutoff_ms(now, days) - 1);
        set_created_at(&store, fresh_id, now);

        let stats = store.purge_retention(now, None, Some(days)).expect("purge");
        assert_eq!(stats.text_deleted, 0);
        assert_eq!(stats.audio_cleared, 1);
        assert!(!old_wav.exists());
        assert!(fresh_wav.is_file());

        let old_row = store.get(old_id).expect("get").expect("row stays");
        assert_eq!(old_row.final_text, "spoken");
        assert!(old_row.audio_path.is_none());
        assert!(store
            .get(fresh_id)
            .expect("get fresh")
            .expect("fresh")
            .audio_path
            .is_some());
        assert_eq!(store.list(0, 10).expect("list").len(), 2);
    }

    #[test]
    fn empty_query_with_a_mode_returns_only_that_mode() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let mut email = sample_new("hello mail", "hello mail");
        email.mode = "email".into();
        let mut plain = sample_new("hello note", "hello note");
        plain.mode = "plain".into();
        let email_id = store.insert(&email).expect("email");
        store.insert(&plain).expect("plain");

        let hits = store
            .find(
                "",
                &HistoryFilter {
                    mode: "email".into(),
                    ..HistoryFilter::default()
                },
                0,
                10,
            )
            .expect("find");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, email_id);
        assert_eq!(hits[0].final_text, "hello mail");
    }

    #[test]
    fn text_query_and_mode_both_apply() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let mut email = sample_new("alpha mail", "alpha mail");
        email.mode = "email".into();
        let mut plain = sample_new("alpha note", "alpha note");
        plain.mode = "plain".into();
        let email_id = store.insert(&email).expect("email");
        store.insert(&plain).expect("plain");

        let hits = store
            .find(
                "alpha",
                &HistoryFilter {
                    mode: "email".into(),
                    ..HistoryFilter::default()
                },
                0,
                10,
            )
            .expect("find");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, email_id);
    }

    #[test]
    fn app_filter_matches_the_stored_id_and_skips_a_missing_app() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let mut terminal = sample_new("one", "one");
        terminal.app_id = Some("org.gnome.Terminal".into());
        let mut missing = sample_new("two", "two");
        missing.app_id = None;
        let mut editor = sample_new("three", "three");
        editor.app_id = Some("org.gnome.gedit".into());
        let terminal_id = store.insert(&terminal).expect("terminal");
        store.insert(&missing).expect("missing");
        store.insert(&editor).expect("editor");

        let hits = store
            .find(
                "",
                &HistoryFilter {
                    app_id: "org.gnome.Terminal".into(),
                    ..HistoryFilter::default()
                },
                0,
                10,
            )
            .expect("find");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, terminal_id);
        assert_eq!(hits[0].final_text, "one");
    }

    #[test]
    fn language_filter_matches_source_language_only() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let mut english = sample_new("hello", "hello");
        english.src_lang = "en".into();
        english.dst_lang = "hi".into();
        let mut hindi = sample_new("namaste", "namaste");
        hindi.src_lang = "hi".into();
        hindi.dst_lang = "en".into();
        let english_id = store.insert(&english).expect("english");
        store.insert(&hindi).expect("hindi");

        let hits = store
            .find(
                "",
                &HistoryFilter {
                    src_lang: "en".into(),
                    ..HistoryFilter::default()
                },
                0,
                10,
            )
            .expect("find");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, english_id);
        assert_eq!(hits[0].final_text, "hello");
    }

    #[test]
    fn created_at_window_includes_the_start_and_excludes_the_end() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let before = store
            .insert(&sample_new("before", "before"))
            .expect("before");
        let start = store.insert(&sample_new("start", "start")).expect("start");
        let middle = store
            .insert(&sample_new("middle", "middle"))
            .expect("middle");
        let end = store.insert(&sample_new("end", "end")).expect("end");
        set_created_at(&store, before, 999);
        set_created_at(&store, start, 1_000);
        set_created_at(&store, middle, 1_500);
        set_created_at(&store, end, 2_000);

        let hits = store
            .find(
                "",
                &HistoryFilter {
                    from_ms: Some(1_000),
                    to_ms: Some(2_000),
                    ..HistoryFilter::default()
                },
                0,
                10,
            )
            .expect("find");
        let texts: Vec<_> = hits.iter().map(|row| row.final_text.as_str()).collect();
        assert_eq!(texts, ["middle", "start"]);
    }

    #[test]
    fn an_omitted_end_keeps_later_rows() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let early = store.insert(&sample_new("early", "early")).expect("early");
        let later = store.insert(&sample_new("later", "later")).expect("later");
        set_created_at(&store, early, 1_000);
        set_created_at(&store, later, 5_000);

        let hits = store
            .find(
                "",
                &HistoryFilter {
                    from_ms: Some(1_000),
                    ..HistoryFilter::default()
                },
                0,
                10,
            )
            .expect("find");
        let texts: Vec<_> = hits.iter().map(|row| row.final_text.as_str()).collect();
        assert_eq!(texts, ["later", "early"]);
    }

    #[test]
    fn mode_match_is_case_sensitive() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let mut email = sample_new("hello mail", "hello mail");
        email.mode = "email".into();
        store.insert(&email).expect("email");

        let hits = store
            .find(
                "",
                &HistoryFilter {
                    mode: "Email".into(),
                    ..HistoryFilter::default()
                },
                0,
                10,
            )
            .expect("find");
        assert!(hits.is_empty());
    }

    #[test]
    fn deleting_a_row_removes_its_wav() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav = dir.path().join("7.wav");
        std::fs::write(&wav, b"speech").expect("wav");
        let mut store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let mut entry = sample_new("hello", "hello");
        entry.audio_path = Some(wav.display().to_string());
        let id = store.insert(&entry).expect("insert");

        let removed = store.delete(&[id]).expect("delete");
        assert_eq!(removed, 1);
        assert!(store.get(id).expect("get").is_none());
        assert!(!wav.exists());
    }

    #[test]
    fn deleting_a_row_with_a_missing_wav_still_removes_the_row() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav = dir.path().join("missing.wav");
        let mut store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let mut entry = sample_new("hello", "hello");
        entry.audio_path = Some(wav.display().to_string());
        let id = store.insert(&entry).expect("insert");

        let removed = store.delete(&[id]).expect("delete");
        assert_eq!(removed, 1);
        assert!(store.get(id).expect("get").is_none());
    }

    #[test]
    fn deleting_a_row_removes_it_when_the_wav_cannot_be_unlinked() {
        let dir = tempfile::tempdir().expect("tempdir");
        let blocked = dir.path().join("not-a-file");
        std::fs::create_dir(&blocked).expect("dir");
        let mut store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let mut entry = sample_new("hello", "hello");
        entry.audio_path = Some(blocked.display().to_string());
        let id = store.insert(&entry).expect("insert");

        let removed = store.delete(&[id]).expect("delete");
        assert_eq!(removed, 1);
        assert!(store.get(id).expect("get").is_none());
        assert!(blocked.is_dir());
    }

    #[test]
    fn set_audio_path_is_what_get_returns() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let id = store.insert(&sample_new("hello", "hello")).expect("insert");
        let path = dir.path().join("1.wav");

        store
            .set_audio_path(id, path.to_str().unwrap())
            .expect("set");

        let row = store.get(id).expect("get").expect("row");
        assert_eq!(row.audio_path.as_deref(), Some(path.to_str().unwrap()));
    }

    #[test]
    fn set_audio_path_on_a_missing_row_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let err = store
            .set_audio_path(9, dir.path().join("9.wav").to_str().unwrap())
            .expect_err("missing");
        assert!(err.to_string().contains("9"));
    }

    #[test]
    fn insert_get_round_trips_retranscribe_kind() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let mut entry = sample_new("heard", "heard");
        entry.kind = "retranscribe".into();
        let id = store.insert(&entry).expect("insert");
        let row = store.get(id).expect("get").expect("row");
        assert_eq!(row.kind, "retranscribe");
        let listed = store.list(0, 10).expect("list");
        assert_eq!(listed[0].kind, "retranscribe");
    }

    #[test]
    fn existing_rows_stay_dictation() {
        let file = NamedTempFile::new().expect("temp db");
        let conn = Connection::open(file.path()).expect("open");
        conn.execute_batch(
            "
            CREATE TABLE entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                mode TEXT NOT NULL,
                src_lang TEXT NOT NULL,
                dst_lang TEXT NOT NULL,
                asr_model TEXT NOT NULL,
                mt_model TEXT,
                llm_model TEXT,
                app_id TEXT,
                raw_text TEXT NOT NULL,
                final_text TEXT NOT NULL,
                insert_status TEXT NOT NULL,
                cleanup_flagged INTEGER NOT NULL DEFAULT 0,
                audio_path TEXT,
                pinned INTEGER NOT NULL DEFAULT 0
            );
            INSERT INTO entries (
                created_at, duration_ms, mode, src_lang, dst_lang,
                asr_model, raw_text, final_text, insert_status
            ) VALUES (1, 10, 'plain', 'en', 'en', 'm', 'raw', 'final', 'success');
            ",
        )
        .expect("old schema");
        drop(conn);

        let store = HistoryStore::open(file.path()).expect("open");
        let row = store.get(1).expect("get").expect("row");
        assert_eq!(row.kind, "dictation");
        assert_eq!(row.final_text, "final");
    }

    #[test]
    fn set_pinned_then_get_returns_true_and_a_missing_id_is_an_error() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let id = store.insert(&sample_new("hello", "Hello")).expect("insert");
        let before = store.get(id).expect("get").expect("row");
        assert!(!before.pinned);
        assert_eq!(before.tags, [] as [&str; 0]);

        store.set_pinned(id, true).expect("pin");
        assert!(store.get(id).expect("get").expect("row").pinned);

        let err = store.set_pinned(41, true).expect_err("missing");
        assert!(matches!(err, StoreError::Missing(41)));
        assert!(store.get(41).expect("get").is_none());
    }

    #[test]
    fn set_tags_trims_drops_empty_and_keeps_case_sensitive_order() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let id = store
            .insert(&sample_new("hello world", "Hello, world."))
            .expect("insert");

        store.set_tags(id, &[" a ", "", "a", "A"]).expect("tags");

        assert_eq!(store.get(id).expect("get").expect("row").tags, ["a", "A"]);
        assert_eq!(store.list(0, 10).expect("list")[0].tags, ["a", "A"]);
        assert_eq!(
            store.search("hello", 0, 10).expect("search")[0].tags,
            ["a", "A"]
        );
        assert_eq!(
            store
                .find("", &HistoryFilter::default(), 0, 10)
                .expect("find")[0]
                .tags,
            ["a", "A"]
        );
    }

    #[test]
    fn set_tags_replaces_the_previous_names() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let id = store.insert(&sample_new("hello", "Hello")).expect("insert");
        store.set_tags(id, &["old"]).expect("first");
        store.set_tags(id, &["new"]).expect("second");
        let tags = store.get(id).expect("get").expect("row").tags;
        assert_eq!(tags, ["new"]);
    }

    #[test]
    fn set_tags_on_a_missing_row_writes_no_tag_rows() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let err = store.set_tags(4, &["kept"]).expect_err("missing");
        assert!(matches!(err, StoreError::Missing(4)));
        let tags: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))
            .expect("count tags");
        let links: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM entry_tags", [], |row| row.get(0))
            .expect("count links");
        assert_eq!(tags, 0);
        assert_eq!(links, 0);
    }

    #[test]
    fn deleting_an_entry_drops_its_tags_and_keeps_a_shared_name() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let first = store.insert(&sample_new("one", "One")).expect("first");
        thread::sleep(Duration::from_millis(2));
        let second = store.insert(&sample_new("two", "Two")).expect("second");
        store
            .set_tags(first, &["shared", "only-a"])
            .expect("tags a");
        store.set_tags(second, &["shared"]).expect("tags b");

        store.delete(&[first]).expect("delete");

        assert!(store.get(first).expect("get").is_none());
        assert_eq!(
            store.get(second).expect("get").expect("row").tags,
            ["shared"]
        );
    }

    fn quoted_entry() -> HistoryEntry {
        HistoryEntry {
            id: 7,
            created_at: 86_400_000,
            duration_ms: 1000,
            mode: "plain".into(),
            src_lang: "en".into(),
            dst_lang: "en".into(),
            asr_model: "parakeet-ctc-110m-int8".into(),
            mt_model: None,
            llm_model: None,
            app_id: Some("org.gnome.TextEditor".into()),
            raw_text: "raw".into(),
            final_text: "say \"hi\"".into(),
            insert_status: InsertStatus::Success,
            cleanup_flagged: false,
            audio_path: Some("/tmp/7.wav".into()),
            pinned: true,
            kind: "dictation".into(),
            tags: vec!["a".into(), "A".into()],
        }
    }

    fn finals(texts: &[&str]) -> Vec<HistoryEntry> {
        texts
            .iter()
            .enumerate()
            .map(|(index, text)| HistoryEntry {
                id: index as i64,
                created_at: 0,
                duration_ms: 0,
                mode: "plain".into(),
                src_lang: "en".into(),
                dst_lang: "en".into(),
                asr_model: "m".into(),
                mt_model: None,
                llm_model: None,
                app_id: None,
                raw_text: String::new(),
                final_text: (*text).into(),
                insert_status: InsertStatus::Success,
                cleanup_flagged: false,
                audio_path: None,
                pinned: false,
                kind: "dictation".into(),
                tags: Vec::new(),
            })
            .collect()
    }

    #[test]
    fn export_document_json_markdown_and_txt_match_the_literals() {
        let entry = quoted_entry();
        let json = export_document(std::slice::from_ref(&entry), "json").expect("json");
        assert_eq!(
            json,
            r#"[{"id":7,"created_at":86400000,"duration_ms":1000,"mode":"plain","src_lang":"en","dst_lang":"en","asr_model":"parakeet-ctc-110m-int8","mt_model":null,"llm_model":null,"app_id":"org.gnome.TextEditor","raw_text":"raw","final_text":"say \"hi\"","insert_status":"success","cleanup_flagged":false,"audio_path":"/tmp/7.wav","pinned":true,"kind":"dictation","tags":["a","A"]}]"#
        );
        let rows = finals(&["one", "two"]);
        assert_eq!(
            export_document(&rows, "markdown").expect("markdown"),
            "one\n\ntwo"
        );
        assert_eq!(export_document(&rows, "txt").expect("txt"), "one\n\ntwo");
    }

    #[test]
    fn delete_all_removes_both_rows_and_the_named_wav() {
        let dir = tempfile::tempdir().expect("tempdir");
        let wav = dir.path().join("7.wav");
        std::fs::write(&wav, b"speech").expect("wav");
        let missing = dir.path().join("missing.wav");
        let mut store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let mut first = sample_new("one", "one");
        first.audio_path = Some(wav.display().to_string());
        store.insert(&first).expect("one");
        let mut second = sample_new("two", "two");
        second.audio_path = Some(missing.display().to_string());
        store.insert(&second).expect("two");

        store.delete_all().expect("delete all");

        assert!(store.list(0, 10).expect("list").is_empty());
        assert!(!wav.exists());
    }

    #[test]
    fn export_by_ids_reports_a_missing_row_and_includes_final_text() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        let id = store
            .insert(&sample_new("raw", "kept final"))
            .expect("insert");
        let err = store.export(&[99], "json").expect_err("missing");
        assert!(err.to_string().contains("no history row 99"));
        let doc = store.export(&[id], "txt").expect("export");
        assert!(doc.contains("kept final"));
    }

    fn insert_at(
        store: &mut HistoryStore,
        final_text: &str,
        duration_ms: u32,
        kind: &str,
        status: InsertStatus,
        created_at: i64,
    ) {
        let mut entry = sample_new("raw raw raw raw raw", final_text);
        entry.duration_ms = duration_ms;
        entry.kind = kind.into();
        entry.insert_status = status;
        let id = store.insert(&entry).expect("insert");
        set_created_at(store, id, created_at);
    }

    #[test]
    fn stats_counts_dictation_words_inside_the_millisecond_range() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open");
        insert_at(
            &mut store,
            "one two",
            30_000,
            "dictation",
            InsertStatus::Success,
            0,
        );
        insert_at(
            &mut store,
            "three",
            30_000,
            "dictation",
            InsertStatus::Skipped,
            1,
        );
        insert_at(
            &mut store,
            "nope extra",
            30_000,
            "retranscribe",
            InsertStatus::Success,
            2,
        );
        insert_at(
            &mut store,
            "nope file",
            30_000,
            "file",
            InsertStatus::Success,
            3,
        );
        insert_at(
            &mut store,
            "outside words",
            30_000,
            "dictation",
            InsertStatus::Success,
            100_000,
        );

        let stats = store.stats(0, 100_000).expect("stats");
        assert_eq!(stats.words, 3);
        assert_eq!(stats.utterances, 2);
        assert_eq!(stats.speaking_wpm, 3);
        assert_eq!(stats.time_saved_ms, 0);

        insert_at(
            &mut store,
            "a b c d",
            0,
            "dictation",
            InsertStatus::Success,
            200_000,
        );
        let again = store.stats(200_000, 300_000).expect("zero duration");
        assert_eq!(again.words, 4);
        assert_eq!(again.utterances, 1);
        assert_eq!(again.speaking_wpm, 0);
        assert_eq!(again.time_saved_ms, 6_000);
    }
}

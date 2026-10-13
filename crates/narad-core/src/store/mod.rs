//! SQLite + FTS5 history store (FR-HIS-1, FR-HIS-3, FR-HIS-7).
//!
//! Entry primary keys are `i64` values from SQLite `INTEGER PRIMARY KEY AUTOINCREMENT`
//! (the stable `rowid` for FTS5 external-content sync).

mod entry;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Row};
use thiserror::Error;

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
    #[error("system time before Unix epoch")]
    InvalidTimestamp,
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
    conn: Connection,
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
                audio_path, pinned
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9,
                ?10, ?11, ?12, ?13,
                ?14, ?15
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
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Fetch a single entry by primary key.
    pub fn get(&self, id: i64) -> Result<Option<HistoryEntry>, StoreError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT
                id, created_at, duration_ms, mode, src_lang, dst_lang,
                asr_model, mt_model, llm_model, app_id,
                raw_text, final_text, insert_status, cleanup_flagged,
                audio_path, pinned
            FROM entries
            WHERE id = ?1
            ",
        )?;
        let mut rows = stmt.query_map(params![id], row_to_entry)?;
        match rows.next() {
            Some(Ok(entry)) => Ok(Some(entry)),
            Some(Err(e)) => Err(StoreError::from(e)),
            None => Ok(None),
        }
    }

    /// List entries newest-first.
    pub fn list(&self, offset: u32, limit: u32) -> Result<Vec<HistoryEntry>, StoreError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT
                id, created_at, duration_ms, mode, src_lang, dst_lang,
                asr_model, mt_model, llm_model, app_id,
                raw_text, final_text, insert_status, cleanup_flagged,
                audio_path, pinned
            FROM entries
            ORDER BY created_at DESC, id DESC
            LIMIT ?1 OFFSET ?2
            ",
        )?;
        let rows = stmt.query_map(params![limit, offset], row_to_entry)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    /// Full-text search over `raw_text` and `final_text` (FTS5).
    pub fn search(
        &self,
        query: &str,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<HistoryEntry>, StoreError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT
                e.id, e.created_at, e.duration_ms, e.mode, e.src_lang, e.dst_lang,
                e.asr_model, e.mt_model, e.llm_model, e.app_id,
                e.raw_text, e.final_text, e.insert_status, e.cleanup_flagged,
                e.audio_path, e.pinned
            FROM entries e
            INNER JOIN entries_fts ON entries_fts.rowid = e.id
            WHERE entries_fts MATCH ?1
            ORDER BY e.created_at DESC, e.id DESC
            LIMIT ?2 OFFSET ?3
            ",
        )?;
        let rows = stmt.query_map(params![query, limit, offset], row_to_entry)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
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
            e.audio_path, e.pinned
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
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(params_owned), row_to_entry)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(StoreError::from)
    }

    /// Delete entries by id. Returns the number of rows removed.
    pub fn delete(&mut self, ids: &[i64]) -> Result<u32, StoreError> {
        if ids.is_empty() {
            return Ok(0);
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

fn now_millis() -> Result<i64, StoreError> {
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
    })
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
            pinned INTEGER NOT NULL DEFAULT 0
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
        ",
    )?;
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
        let before = store.insert(&sample_new("before", "before")).expect("before");
        let start = store.insert(&sample_new("start", "start")).expect("start");
        let middle = store.insert(&sample_new("middle", "middle")).expect("middle");
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
}

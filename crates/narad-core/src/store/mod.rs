//! SQLite + FTS5 history store (FR-HIS-1, FR-HIS-3).
//!
//! Entry primary keys are `i64` values from SQLite `INTEGER PRIMARY KEY AUTOINCREMENT`
//! (the stable `rowid` for FTS5 external-content sync).

mod entry;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, Row};
use thiserror::Error;

pub use entry::{HistoryEntry, InsertStatus, NewHistoryEntry};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("invalid insert_status in database")]
    InvalidInsertStatus,
    #[error("system time before Unix epoch")]
    InvalidTimestamp,
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
        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
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
        rows.collect::<Result<Vec<_>, _>>().map_err(StoreError::from)
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
        rusqlite::Error::InvalidColumnType(
            12,
            "insert_status".into(),
            rusqlite::types::Type::Text,
        )
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
}

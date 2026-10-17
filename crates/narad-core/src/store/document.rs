//! Long-form document storage and chunked draft persistence (FR-LNG-1, FR-LNG-2).

use serde::{Deserialize, Serialize};

use super::{now_millis, HistoryStore, StoreError};

/// Status of a long-form or meeting document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatus {
    Active,
    Completed,
}

impl DocumentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(Self::Active),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }
}

impl std::str::FromStr for DocumentStatus {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_str(s).ok_or(())
    }
}

/// A stored document chunk representing an incremental segment of speech.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub chunk_index: u32,
    pub speaker_label: Option<String>,
    pub text: String,
    pub start_ms: u32,
    pub duration_ms: u32,
}

/// Fields supplied when appending a new chunk to an existing document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewDocumentChunk {
    pub speaker_label: Option<String>,
    pub text: String,
    pub start_ms: u32,
    pub duration_ms: u32,
}

impl NewDocumentChunk {
    pub fn new(text: impl Into<String>, start_ms: u32, duration_ms: u32) -> Self {
        Self {
            speaker_label: None,
            text: text.into(),
            start_ms,
            duration_ms,
        }
    }

    pub fn with_speaker(
        speaker_label: impl Into<String>,
        text: impl Into<String>,
        start_ms: u32,
        duration_ms: u32,
    ) -> Self {
        Self {
            speaker_label: Some(speaker_label.into()),
            text: text.into(),
            start_ms,
            duration_ms,
        }
    }
}

impl From<DocumentChunk> for NewDocumentChunk {
    fn from(chunk: DocumentChunk) -> Self {
        Self {
            speaker_label: chunk.speaker_label,
            text: chunk.text,
            start_ms: chunk.start_ms,
            duration_ms: chunk.duration_ms,
        }
    }
}

impl From<&DocumentChunk> for NewDocumentChunk {
    fn from(chunk: &DocumentChunk) -> Self {
        Self {
            speaker_label: chunk.speaker_label.clone(),
            text: chunk.text.clone(),
            start_ms: chunk.start_ms,
            duration_ms: chunk.duration_ms,
        }
    }
}

/// Grouped block of contiguous chunks from the same speaker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeakerBlock {
    pub speaker_label: Option<String>,
    pub text: String,
    pub start_ms: u32,
    pub duration_ms: u32,
}

/// A long-form or meeting document containing metadata and its transcript chunks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Document {
    pub id: i64,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub duration_ms: u32,
    pub status: DocumentStatus,
    pub chunks: Vec<DocumentChunk>,
}

impl Document {
    /// Combined whitespace-joined text of all chunks.
    pub fn full_text(&self) -> String {
        let mut out = String::new();
        for chunk in &self.chunks {
            let trimmed = chunk.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(trimmed);
        }
        out
    }

    /// Aggregate adjacent chunks with identical speaker labels into speaker blocks.
    pub fn speaker_blocks(&self) -> Vec<SpeakerBlock> {
        let mut blocks: Vec<SpeakerBlock> = Vec::new();
        for chunk in &self.chunks {
            if let Some(last) = blocks.last_mut() {
                if last.speaker_label == chunk.speaker_label {
                    let trimmed = chunk.text.trim();
                    if !trimmed.is_empty() {
                        if !last.text.is_empty() {
                            last.text.push(' ');
                        }
                        last.text.push_str(trimmed);
                    }
                    let chunk_end = chunk.start_ms.saturating_add(chunk.duration_ms);
                    last.duration_ms =
                        last.duration_ms.max(chunk_end.saturating_sub(last.start_ms));
                    continue;
                }
            }
            blocks.push(SpeakerBlock {
                speaker_label: chunk.speaker_label.clone(),
                text: chunk.text.trim().to_string(),
                start_ms: chunk.start_ms,
                duration_ms: chunk.duration_ms,
            });
        }
        blocks
    }
}

/// Summary of a stored document without loading all its chunk records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSummary {
    pub id: i64,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub duration_ms: u32,
    pub status: DocumentStatus,
    pub chunk_count: u32,
}

impl From<&Document> for DocumentSummary {
    fn from(doc: &Document) -> Self {
        Self {
            id: doc.id,
            title: doc.title.clone(),
            created_at: doc.created_at,
            updated_at: doc.updated_at,
            duration_ms: doc.duration_ms,
            status: doc.status,
            chunk_count: doc.chunks.len() as u32,
        }
    }
}

fn ensure_document(conn: &rusqlite::Connection, doc_id: i64) -> Result<(), StoreError> {
    match conn.query_row(
        "SELECT 1 FROM documents WHERE id = ?1",
        rusqlite::params![doc_id],
        |_| Ok(()),
    ) {
        Ok(()) => Ok(()),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(StoreError::Missing(doc_id)),
        Err(err) => Err(err.into()),
    }
}

impl HistoryStore {
    /// Create a new long-form document draft.
    pub fn create_document(&mut self, title: &str) -> Result<i64, StoreError> {
        let now = now_millis()?;
        let title_clean = if title.trim().is_empty() {
            "Untitled meeting"
        } else {
            title.trim()
        };

        self.conn.execute(
            r"
            INSERT INTO documents (
                title, created_at, updated_at, duration_ms, status
            ) VALUES (?1, ?2, ?3, 0, 'active')
            ",
            rusqlite::params![title_clean, now, now],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Incrementally append a transcribed chunk to an existing document.
    pub fn append_document_chunk(
        &mut self,
        doc_id: i64,
        chunk: &NewDocumentChunk,
    ) -> Result<(), StoreError> {
        ensure_document(&self.conn, doc_id)?;
        let now = now_millis()?;
        let chunk_end = chunk.start_ms.saturating_add(chunk.duration_ms);

        let tx = self.conn.transaction()?;

        let next_index: u32 = {
            let mut stmt = tx.prepare(
                "SELECT COALESCE(MAX(chunk_index) + 1, 0) FROM document_chunks WHERE doc_id = ?1",
            )?;
            stmt.query_row(rusqlite::params![doc_id], |row| row.get::<_, i64>(0))? as u32
        };

        tx.execute(
            r"
            INSERT INTO document_chunks (
                doc_id, chunk_index, speaker_label, text, start_ms, duration_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            rusqlite::params![
                doc_id,
                next_index as i64,
                chunk.speaker_label,
                chunk.text,
                chunk.start_ms as i64,
                chunk.duration_ms as i64,
            ],
        )?;

        tx.execute(
            r"
            UPDATE documents
            SET updated_at = ?1,
                duration_ms = MAX(duration_ms, ?2)
            WHERE id = ?3
            ",
            rusqlite::params![now, chunk_end as i64, doc_id],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Retrieve a full document by primary key, including all chunks ordered by index.
    pub fn get_document(&self, doc_id: i64) -> Result<Option<Document>, StoreError> {
        let doc_meta = {
            let mut stmt = self.conn.prepare(
                r"
                SELECT id, title, created_at, updated_at, duration_ms, status
                FROM documents
                WHERE id = ?1
                ",
            )?;
            let mut rows = stmt.query_map(rusqlite::params![doc_id], |row| {
                let id: i64 = row.get(0)?;
                let title: String = row.get(1)?;
                let created_at: i64 = row.get(2)?;
                let updated_at: i64 = row.get(3)?;
                let duration_ms: i64 = row.get(4)?;
                let status_str: String = row.get(5)?;
                Ok((
                    id,
                    title,
                    created_at,
                    updated_at,
                    duration_ms as u32,
                    status_str,
                ))
            })?;
            match rows.next() {
                Some(Ok(res)) => Some(res),
                Some(Err(e)) => return Err(StoreError::from(e)),
                None => None,
            }
        };

        let (id, title, created_at, updated_at, duration_ms, status_str) = match doc_meta {
            Some(meta) => meta,
            None => return Ok(None),
        };

        let status = DocumentStatus::from_str(&status_str)
            .ok_or_else(|| StoreError::InvalidDocumentStatus(status_str))?;

        let mut stmt = self.conn.prepare(
            r"
            SELECT chunk_index, speaker_label, text, start_ms, duration_ms
            FROM document_chunks
            WHERE doc_id = ?1
            ORDER BY chunk_index ASC
            ",
        )?;
        let chunk_rows = stmt.query_map(rusqlite::params![doc_id], |row| {
            Ok(DocumentChunk {
                chunk_index: row.get::<_, i64>(0)? as u32,
                speaker_label: row.get(1)?,
                text: row.get(2)?,
                start_ms: row.get::<_, i64>(3)? as u32,
                duration_ms: row.get::<_, i64>(4)? as u32,
            })
        })?;

        let mut chunks = Vec::new();
        for chunk in chunk_rows {
            chunks.push(chunk?);
        }

        Ok(Some(Document {
            id,
            title,
            created_at,
            updated_at,
            duration_ms,
            status,
            chunks,
        }))
    }

    /// List document summaries ordered newest first.
    pub fn list_documents(&self) -> Result<Vec<DocumentSummary>, StoreError> {
        let mut stmt = self.conn.prepare(
            r"
            SELECT
                d.id,
                d.title,
                d.created_at,
                d.updated_at,
                d.duration_ms,
                d.status,
                COUNT(c.chunk_index) AS chunk_count
            FROM documents d
            LEFT JOIN document_chunks c ON c.doc_id = d.id
            GROUP BY d.id
            ORDER BY d.created_at DESC, d.id DESC
            ",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let title: String = row.get(1)?;
            let created_at: i64 = row.get(2)?;
            let updated_at: i64 = row.get(3)?;
            let duration_ms: i64 = row.get(4)?;
            let status_str: String = row.get(5)?;
            let chunk_count: i64 = row.get(6)?;
            Ok((
                id,
                title,
                created_at,
                updated_at,
                duration_ms as u32,
                status_str,
                chunk_count as u32,
            ))
        })?;

        let mut summaries = Vec::new();
        for item in rows {
            let (id, title, created_at, updated_at, duration_ms, status_str, chunk_count) = item?;
            let status = DocumentStatus::from_str(&status_str)
                .ok_or_else(|| StoreError::InvalidDocumentStatus(status_str))?;
            summaries.push(DocumentSummary {
                id,
                title,
                created_at,
                updated_at,
                duration_ms,
                status,
                chunk_count,
            });
        }
        Ok(summaries)
    }

    /// Mark a document as completed.
    pub fn complete_document(&mut self, doc_id: i64) -> Result<(), StoreError> {
        ensure_document(&self.conn, doc_id)?;
        let now = now_millis()?;
        self.conn.execute(
            "UPDATE documents SET status = 'completed', updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, doc_id],
        )?;
        Ok(())
    }

    /// Delete a document and its cascaded chunks.
    pub fn delete_document(&mut self, doc_id: i64) -> Result<bool, StoreError> {
        let deleted = self.conn.execute(
            "DELETE FROM documents WHERE id = ?1",
            rusqlite::params![doc_id],
        )?;
        Ok(deleted > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn document_create_and_fetch_empty() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open store");

        let doc_id = store
            .create_document("Product Strategy Session")
            .expect("create document");
        assert!(doc_id > 0);

        let doc = store
            .get_document(doc_id)
            .expect("get document")
            .expect("document exists");
        assert_eq!(doc.id, doc_id);
        assert_eq!(doc.title, "Product Strategy Session");
        assert_eq!(doc.status, DocumentStatus::Active);
        assert_eq!(doc.duration_ms, 0);
        assert!(doc.created_at > 0);
        assert_eq!(doc.created_at, doc.updated_at);
        assert!(doc.chunks.is_empty());
        assert_eq!(doc.full_text(), "");
        assert!(doc.speaker_blocks().is_empty());
    }

    #[test]
    fn incremental_chunks_and_speaker_blocks() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open store");

        let doc_id = store
            .create_document("Quarterly Review")
            .expect("create document");

        // Chunk 0: Alice
        store
            .append_document_chunk(
                doc_id,
                &NewDocumentChunk::with_speaker(
                    "Alice",
                    "Welcome everyone to our Q3 review.",
                    0,
                    3000,
                ),
            )
            .expect("append chunk 0");

        // Chunk 1: Alice continues
        store
            .append_document_chunk(
                doc_id,
                &NewDocumentChunk::with_speaker(
                    "Alice",
                    "Let us start with the revenue numbers.",
                    3200,
                    2800,
                ),
            )
            .expect("append chunk 1");

        // Chunk 2: Bob interrupts
        store
            .append_document_chunk(
                doc_id,
                &NewDocumentChunk::with_speaker(
                    "Bob",
                    "I can share the slide deck for that.",
                    6500,
                    2500,
                ),
            )
            .expect("append chunk 2");

        // Chunk 3: Unknown speaker / no label
        store
            .append_document_chunk(
                doc_id,
                &NewDocumentChunk::new("Can everyone see the screen?", 9500, 1500),
            )
            .expect("append chunk 3");

        let doc = store
            .get_document(doc_id)
            .expect("get document")
            .expect("doc exists");

        assert_eq!(doc.chunks.len(), 4);
        assert_eq!(doc.chunks[0].chunk_index, 0);
        assert_eq!(doc.chunks[0].speaker_label.as_deref(), Some("Alice"));
        assert_eq!(doc.chunks[1].chunk_index, 1);
        assert_eq!(doc.chunks[1].speaker_label.as_deref(), Some("Alice"));
        assert_eq!(doc.chunks[2].chunk_index, 2);
        assert_eq!(doc.chunks[2].speaker_label.as_deref(), Some("Bob"));
        assert_eq!(doc.chunks[3].chunk_index, 3);
        assert_eq!(doc.chunks[3].speaker_label, None);

        // Document duration matches highest end timestamp: 9500 + 1500 = 11000 ms
        assert_eq!(doc.duration_ms, 11000);

        // Full text check
        assert_eq!(
            doc.full_text(),
            "Welcome everyone to our Q3 review. Let us start with the revenue numbers. I can share the slide deck for that. Can everyone see the screen?"
        );

        // Speaker blocks aggregation
        let blocks = doc.speaker_blocks();
        assert_eq!(blocks.len(), 3);

        // Block 0: Combined Alice
        assert_eq!(blocks[0].speaker_label.as_deref(), Some("Alice"));
        assert_eq!(
            blocks[0].text,
            "Welcome everyone to our Q3 review. Let us start with the revenue numbers."
        );
        assert_eq!(blocks[0].start_ms, 0);
        assert_eq!(blocks[0].duration_ms, 6000); // 3200 + 2800 = 6000

        // Block 1: Bob
        assert_eq!(blocks[1].speaker_label.as_deref(), Some("Bob"));
        assert_eq!(blocks[1].text, "I can share the slide deck for that.");
        assert_eq!(blocks[1].start_ms, 6500);
        assert_eq!(blocks[1].duration_ms, 2500);

        // Block 2: Unknown speaker
        assert_eq!(blocks[2].speaker_label, None);
        assert_eq!(blocks[2].text, "Can everyone see the screen?");
        assert_eq!(blocks[2].start_ms, 9500);
        assert_eq!(blocks[2].duration_ms, 1500);
    }

    #[test]
    fn complete_document_updates_status() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open store");

        let doc_id = store.create_document("Standup").expect("create document");
        let initial = store.get_document(doc_id).unwrap().unwrap();
        assert_eq!(initial.status, DocumentStatus::Active);

        store.complete_document(doc_id).expect("complete document");
        let completed = store.get_document(doc_id).unwrap().unwrap();
        assert_eq!(completed.status, DocumentStatus::Completed);
        assert!(completed.updated_at >= initial.updated_at);
    }

    #[test]
    fn list_documents_ordering_and_summary() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open store");

        let doc1 = store.create_document("First Meeting").unwrap();
        store
            .append_document_chunk(
                doc1,
                &NewDocumentChunk::new("Notes from first meeting.", 0, 1000),
            )
            .unwrap();
        store
            .append_document_chunk(
                doc1,
                &NewDocumentChunk::new("More notes from first meeting.", 1000, 1200),
            )
            .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(5));

        let doc2 = store.create_document("Second Meeting").unwrap();

        let summaries = store.list_documents().expect("list documents");
        assert_eq!(summaries.len(), 2);

        // Ordered newest first
        assert_eq!(summaries[0].id, doc2);
        assert_eq!(summaries[0].title, "Second Meeting");
        assert_eq!(summaries[0].chunk_count, 0);

        assert_eq!(summaries[1].id, doc1);
        assert_eq!(summaries[1].title, "First Meeting");
        assert_eq!(summaries[1].chunk_count, 2);
    }

    #[test]
    fn missing_document_error_handling() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open store");

        let missing_id = 999_999;
        assert_eq!(store.get_document(missing_id).unwrap(), None);

        let append_err = store.append_document_chunk(
            missing_id,
            &NewDocumentChunk::new("Orphan text", 0, 500),
        );
        assert!(matches!(append_err, Err(StoreError::Missing(id)) if id == missing_id));

        let complete_err = store.complete_document(missing_id);
        assert!(matches!(complete_err, Err(StoreError::Missing(id)) if id == missing_id));
    }

    #[test]
    fn crash_resilience_and_recovery() {
        let file = NamedTempFile::new().expect("temp db");
        let db_path = file.path().to_path_buf();

        let doc_id = {
            let mut store = HistoryStore::open(&db_path).expect("open first instance");
            let id = store
                .create_document("Emergency Incident Response")
                .expect("create doc");

            store
                .append_document_chunk(
                    id,
                    &NewDocumentChunk::with_speaker(
                        "OnCall",
                        "Incident detected on database replica.",
                        0,
                        4000,
                    ),
                )
                .expect("chunk 0 committed");

            store
                .append_document_chunk(
                    id,
                    &NewDocumentChunk::with_speaker(
                        "Lead",
                        "Flipping traffic to standby now.",
                        4200,
                        3500,
                    ),
                )
                .expect("chunk 1 committed");

            // Store instance is dropped here, simulating an abrupt crash or daemon restart
            id
        };

        // Reopen database from disk
        let mut recovered_store = HistoryStore::open(&db_path).expect("reopen store after crash");
        let recovered_doc = recovered_store
            .get_document(doc_id)
            .expect("query doc after crash")
            .expect("document exists");

        // Verify committed chunks survived intact
        assert_eq!(recovered_doc.chunks.len(), 2);
        assert_eq!(recovered_doc.chunks[0].chunk_index, 0);
        assert_eq!(recovered_doc.chunks[1].chunk_index, 1);
        assert_eq!(
            recovered_doc.full_text(),
            "Incident detected on database replica. Flipping traffic to standby now."
        );
        assert_eq!(recovered_doc.duration_ms, 7700);

        // Seamlessly append new chunks after recovery
        recovered_store
            .append_document_chunk(
                doc_id,
                &NewDocumentChunk::with_speaker(
                    "Lead",
                    "Failover successful. Traffic healthy.",
                    8000,
                    2000,
                ),
            )
            .expect("append chunk 2 after recovery");

        let updated_doc = recovered_store
            .get_document(doc_id)
            .expect("query doc")
            .expect("exists");
        assert_eq!(updated_doc.chunks.len(), 3);
        assert_eq!(updated_doc.chunks[2].chunk_index, 2);
        assert_eq!(updated_doc.duration_ms, 10000);
    }

    #[test]
    fn delete_document_cascades_chunk_deletion() {
        let file = NamedTempFile::new().expect("temp db");
        let mut store = HistoryStore::open(file.path()).expect("open store");

        let doc_id = store.create_document("To Delete").unwrap();
        store
            .append_document_chunk(doc_id, &NewDocumentChunk::new("Chunk 1", 0, 1000))
            .unwrap();
        store
            .append_document_chunk(doc_id, &NewDocumentChunk::new("Chunk 2", 1000, 1000))
            .unwrap();

        assert!(store.delete_document(doc_id).unwrap());
        assert_eq!(store.get_document(doc_id).unwrap(), None);

        // Verify chunks table directly
        let count: i64 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM document_chunks WHERE doc_id = ?1",
                rusqlite::params![doc_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn overlapping_chunks_maintain_monotonic_duration_in_speaker_blocks() {
        let doc = Document {
            id: 1,
            title: "Overlapping Test".to_string(),
            created_at: 1000,
            updated_at: 1000,
            duration_ms: 10000,
            status: DocumentStatus::Active,
            chunks: vec![
                DocumentChunk {
                    chunk_index: 0,
                    speaker_label: Some("Alice".to_string()),
                    text: "Part one.".to_string(),
                    start_ms: 0,
                    duration_ms: 5000,
                },
                DocumentChunk {
                    chunk_index: 1,
                    speaker_label: Some("Alice".to_string()),
                    text: "Part two overlapping.".to_string(),
                    start_ms: 2000,
                    duration_ms: 1000,
                },
            ],
        };

        let blocks = doc.speaker_blocks();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start_ms, 0);
        assert_eq!(blocks[0].duration_ms, 5000);
    }
}

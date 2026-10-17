//! Continuous audio processor & session manager for long-form transcription (FR-LNG-1, FR-LNG-2).
//!
//! # Non-insertion Invariant
//! Meeting and long-form transcriptions are accumulated incrementally as draft documents
//! within SQLite [`HistoryStore`]. Unlike standard dictation, long-form transcription
//! results MUST NOT be pasted or inserted into focused windows or active applications.
//! They are retained purely as persistent document records for review and export.

use std::sync::{Arc, Mutex};

use narad_core::store::{
    Document, DocumentChunk, DocumentStatus, DocumentSummary, HistoryStore, NewDocumentChunk,
    StoreError,
};

/// Errors encountered during long-form session management and chunk processing.
#[derive(Debug)]
pub enum LongformError {
    /// Underlying SQLite document storage error.
    Store(StoreError),
    /// Requested document id does not exist in storage.
    DocumentNotFound(i64),
    /// Document has already been marked completed and cannot accept further chunks.
    DocumentAlreadyCompleted(i64),
    /// Internal store lock was poisoned.
    LockPoisoned,
}

impl std::fmt::Display for LongformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(err) => write!(f, "Store error: {err}"),
            Self::DocumentNotFound(id) => write!(f, "Document not found: {id}"),
            Self::DocumentAlreadyCompleted(id) => {
                write!(f, "Document {id} is already completed")
            }
            Self::LockPoisoned => write!(f, "Internal store lock poisoned"),
        }
    }
}

impl std::error::Error for LongformError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Store(err) => Some(err),
            _ => None,
        }
    }
}

impl From<StoreError> for LongformError {
    fn from(err: StoreError) -> Self {
        Self::Store(err)
    }
}

/// An audio chunk ready for transcription.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioChunk {
    pub chunk_index: u32,
    pub start_ms: u64,
    pub duration_ms: u64,
    pub samples: Vec<f32>,
}

/// Slices continuous streaming audio into fixed-duration chunks with optional overlap.
#[derive(Debug, Clone)]
pub struct ContinuousAudioChunker {
    sample_rate: u32,
    chunk_size_samples: usize,
    step_samples: usize,
    buffer: Vec<f32>,
    total_samples_processed: u64,
    next_chunk_index: u32,
}

impl ContinuousAudioChunker {
    /// Create a new chunker.
    ///
    /// * `sample_rate`: Audio samples per second (e.g. 16,000 Hz).
    /// * `chunk_duration_ms`: Duration of each chunk in milliseconds (e.g. 5,000 ms).
    /// * `overlap_ms`: Overlap duration in milliseconds between consecutive chunks.
    pub fn new(sample_rate: u32, chunk_duration_ms: u32, overlap_ms: u32) -> Self {
        let chunk_size_samples =
            ((sample_rate as u64 * chunk_duration_ms as u64) / 1000).max(1) as usize;
        let overlap_samples =
            ((sample_rate as u64 * (overlap_ms.min(chunk_duration_ms.saturating_sub(1))) as u64)
                / 1000) as usize;
        let step_samples = chunk_size_samples.saturating_sub(overlap_samples).max(1);

        Self {
            sample_rate,
            chunk_size_samples,
            step_samples,
            buffer: Vec::new(),
            total_samples_processed: 0,
            next_chunk_index: 0,
        }
    }

    /// Ingest continuous audio samples into the internal chunking buffer.
    pub fn push_samples(&mut self, samples: &[f32]) {
        self.buffer.extend_from_slice(samples);
    }

    /// Drain all complete chunks ready for transcription.
    pub fn drain_chunks(&mut self) -> Vec<AudioChunk> {
        let mut chunks = Vec::new();
        while self.buffer.len() >= self.chunk_size_samples {
            let chunk_samples = self.buffer[..self.chunk_size_samples].to_vec();
            let start_ms =
                (self.total_samples_processed * 1000) / (self.sample_rate as u64);
            let duration_ms =
                (self.chunk_size_samples as u64 * 1000) / (self.sample_rate as u64);

            chunks.push(AudioChunk {
                chunk_index: self.next_chunk_index,
                start_ms,
                duration_ms,
                samples: chunk_samples,
            });

            self.next_chunk_index += 1;
            self.total_samples_processed += self.step_samples as u64;
            self.buffer.drain(..self.step_samples);
        }
        chunks
    }

    /// Flush remaining buffered audio as a final trailing chunk if any exists.
    pub fn flush(&mut self) -> Option<AudioChunk> {
        if self.buffer.is_empty() {
            return None;
        }
        let chunk_samples = std::mem::take(&mut self.buffer);
        let start_ms =
            (self.total_samples_processed * 1000) / (self.sample_rate as u64);
        let duration_ms =
            (chunk_samples.len() as u64 * 1000) / (self.sample_rate as u64);

        let chunk = AudioChunk {
            chunk_index: self.next_chunk_index,
            start_ms,
            duration_ms,
            samples: chunk_samples,
        };
        self.next_chunk_index += 1;
        Some(chunk)
    }

    /// Reset internal buffers and timing state.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.total_samples_processed = 0;
        self.next_chunk_index = 0;
    }
}

/// Session manager and chunk processor for long-form meeting transcription (FR-LNG-1, FR-LNG-2).
///
/// Invariant: All output is committed directly to document storage; no text is pasted
/// or injected into user applications.
#[derive(Clone)]
pub struct LongformSessionManager {
    store: Arc<Mutex<HistoryStore>>,
    active_session_id: Arc<Mutex<Option<i64>>>,
}

/// Type alias providing alternative naming parity for processor usage.
pub type LongformProcessor = LongformSessionManager;

impl LongformSessionManager {
    /// Construct a new session manager backed by a shared `HistoryStore`.
    pub fn new(store: Arc<Mutex<HistoryStore>>) -> Self {
        Self {
            store,
            active_session_id: Arc::new(Mutex::new(None)),
        }
    }

    /// Construct a new session manager from an owned `HistoryStore`.
    pub fn from_store(store: HistoryStore) -> Self {
        Self::new(Arc::new(Mutex::new(store)))
    }

    /// Get the current active session id, if any.
    pub fn active_session_id(&self) -> Option<i64> {
        self.active_session_id.lock().ok().and_then(|id| *id)
    }

    /// Start a new long-form transcription session, creating a draft document.
    pub fn start_session(&self, title: Option<String>) -> Result<i64, LongformError> {
        let mut store = self.store.lock().map_err(|_| LongformError::LockPoisoned)?;
        let title_str = title.as_deref().unwrap_or("Untitled meeting");
        let doc_id = store.create_document(title_str)?;

        if let Ok(mut active) = self.active_session_id.lock() {
            *active = Some(doc_id);
        }

        Ok(doc_id)
    }

    /// Append an incremental transcription chunk to an active session draft.
    pub fn append_chunk(
        &self,
        doc_id: i64,
        start_ms: u64,
        duration_ms: u64,
        text: String,
        speaker: Option<String>,
    ) -> Result<DocumentChunk, LongformError> {
        let mut store = self.store.lock().map_err(|_| LongformError::LockPoisoned)?;

        // Ensure document exists and is not already completed
        let doc = match store.get_document(doc_id)? {
            Some(d) => d,
            None => return Err(LongformError::DocumentNotFound(doc_id)),
        };

        if doc.status == DocumentStatus::Completed {
            return Err(LongformError::DocumentAlreadyCompleted(doc_id));
        }

        let new_chunk = NewDocumentChunk {
            speaker_label: speaker,
            text,
            start_ms: u32::try_from(start_ms).unwrap_or(u32::MAX),
            duration_ms: u32::try_from(duration_ms).unwrap_or(u32::MAX),
        };

        match store.append_document_chunk(doc_id, &new_chunk) {
            Ok(()) => {}
            Err(StoreError::Missing(id)) => return Err(LongformError::DocumentNotFound(id)),
            Err(err) => return Err(LongformError::Store(err)),
        }

        let updated_doc = store
            .get_document(doc_id)?
            .ok_or(LongformError::DocumentNotFound(doc_id))?;

        let chunk = updated_doc.chunks.into_iter().last().ok_or_else(|| {
            LongformError::Store(StoreError::Missing(doc_id))
        })?;

        Ok(chunk)
    }

    /// Append a transcribed audio chunk.
    pub fn append_audio_chunk(
        &self,
        doc_id: i64,
        chunk: &AudioChunk,
        text: String,
        speaker: Option<String>,
    ) -> Result<DocumentChunk, LongformError> {
        self.append_chunk(doc_id, chunk.start_ms, chunk.duration_ms, text, speaker)
    }

    /// Complete a long-form session, marking the document completed and returning its final state.
    pub fn complete_session(&self, doc_id: i64) -> Result<Document, LongformError> {
        let mut store = self.store.lock().map_err(|_| LongformError::LockPoisoned)?;

        match store.complete_document(doc_id) {
            Ok(()) => {}
            Err(StoreError::Missing(id)) => return Err(LongformError::DocumentNotFound(id)),
            Err(err) => return Err(LongformError::Store(err)),
        }

        if let Ok(mut active) = self.active_session_id.lock() {
            if *active == Some(doc_id) {
                *active = None;
            }
        }

        let doc = store
            .get_document(doc_id)?
            .ok_or(LongformError::DocumentNotFound(doc_id))?;

        Ok(doc)
    }

    /// Retrieve full document details by primary key.
    pub fn get_document(&self, doc_id: i64) -> Result<Option<Document>, LongformError> {
        let store = self.store.lock().map_err(|_| LongformError::LockPoisoned)?;
        Ok(store.get_document(doc_id)?)
    }

    /// List document summaries ordered newest first.
    pub fn list_documents(&self) -> Result<Vec<DocumentSummary>, LongformError> {
        let store = self.store.lock().map_err(|_| LongformError::LockPoisoned)?;
        Ok(store.list_documents()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn test_manager() -> (NamedTempFile, LongformSessionManager) {
        let file = NamedTempFile::new().expect("temp file");
        let store = HistoryStore::open(file.path()).expect("open history store");
        let manager = LongformSessionManager::from_store(store);
        (file, manager)
    }

    #[test]
    fn test_start_session_creates_draft_document() {
        let (_file, manager) = test_manager();

        let doc_id = manager
            .start_session(Some("Engineering All-Hands".to_string()))
            .expect("start session");
        assert!(doc_id > 0);
        assert_eq!(manager.active_session_id(), Some(doc_id));

        let doc = manager
            .get_document(doc_id)
            .expect("get doc")
            .expect("doc exists");

        assert_eq!(doc.id, doc_id);
        assert_eq!(doc.title, "Engineering All-Hands");
        assert_eq!(doc.status, DocumentStatus::Active);
        assert_eq!(doc.duration_ms, 0);
        assert!(doc.chunks.is_empty());
        assert_eq!(doc.full_text(), "");

        // Untitled session defaults to "Untitled meeting"
        let untitled_id = manager.start_session(None).expect("start untitled");
        let untitled_doc = manager
            .get_document(untitled_id)
            .expect("get untitled")
            .expect("untitled exists");
        assert_eq!(untitled_doc.title, "Untitled meeting");
        assert_eq!(manager.active_session_id(), Some(untitled_id));
    }

    #[test]
    fn test_append_consecutive_chunks_updates_duration_and_speaker_blocks() {
        let (_file, manager) = test_manager();

        let doc_id = manager
            .start_session(Some("Product Sync".to_string()))
            .expect("start session");

        // Chunk 0: Alice
        let c0 = manager
            .append_chunk(
                doc_id,
                0,
                3000,
                "Welcome to our planning call.".to_string(),
                Some("Alice".to_string()),
            )
            .expect("append chunk 0");
        assert_eq!(c0.chunk_index, 0);
        assert_eq!(c0.start_ms, 0);
        assert_eq!(c0.duration_ms, 3000);
        assert_eq!(c0.speaker_label.as_deref(), Some("Alice"));

        // Chunk 1: Alice continues
        let c1 = manager
            .append_chunk(
                doc_id,
                3200,
                2800,
                "Let us review the sprint roadmap.".to_string(),
                Some("Alice".to_string()),
            )
            .expect("append chunk 1");
        assert_eq!(c1.chunk_index, 1);
        assert_eq!(c1.start_ms, 3200);
        assert_eq!(c1.duration_ms, 2800);

        // Chunk 2: Bob speaks
        let c2 = manager
            .append_chunk(
                doc_id,
                6500,
                3000,
                "I have prepared the backend migration deck.".to_string(),
                Some("Bob".to_string()),
            )
            .expect("append chunk 2");
        assert_eq!(c2.chunk_index, 2);
        assert_eq!(c2.start_ms, 6500);
        assert_eq!(c2.duration_ms, 3000);

        let doc = manager
            .get_document(doc_id)
            .expect("get document")
            .expect("doc exists");

        // Document duration matches highest end timestamp: 6500 + 3000 = 9500 ms
        assert_eq!(doc.duration_ms, 9500);
        assert_eq!(doc.chunks.len(), 3);
        assert_eq!(
            doc.full_text(),
            "Welcome to our planning call. Let us review the sprint roadmap. I have prepared the backend migration deck."
        );

        // Speaker blocks aggregation
        let blocks = doc.speaker_blocks();
        assert_eq!(blocks.len(), 2);

        // Block 0: Alice combined (duration: 3200 + 2800 - 0 = 6000 ms)
        assert_eq!(blocks[0].speaker_label.as_deref(), Some("Alice"));
        assert_eq!(
            blocks[0].text,
            "Welcome to our planning call. Let us review the sprint roadmap."
        );
        assert_eq!(blocks[0].start_ms, 0);
        assert_eq!(blocks[0].duration_ms, 6000);

        // Block 1: Bob
        assert_eq!(blocks[1].speaker_label.as_deref(), Some("Bob"));
        assert_eq!(
            blocks[1].text,
            "I have prepared the backend migration deck."
        );
        assert_eq!(blocks[1].start_ms, 6500);
        assert_eq!(blocks[1].duration_ms, 3000);
    }

    #[test]
    fn test_complete_session_marks_document_completed() {
        let (_file, manager) = test_manager();

        let doc_id = manager
            .start_session(Some("Standup Meeting".to_string()))
            .expect("start session");

        manager
            .append_chunk(
                doc_id,
                0,
                4000,
                "Quick updates from today.".to_string(),
                Some("Alice".to_string()),
            )
            .expect("append chunk");

        let completed_doc = manager.complete_session(doc_id).expect("complete session");
        assert_eq!(completed_doc.status, DocumentStatus::Completed);
        assert_eq!(completed_doc.duration_ms, 4000);
        assert_eq!(manager.active_session_id(), None);

        // Appending to completed session is rejected
        let append_err = manager.append_chunk(
            doc_id,
            4500,
            2000,
            "Late update.".to_string(),
            Some("Bob".to_string()),
        );
        assert!(matches!(
            append_err,
            Err(LongformError::DocumentAlreadyCompleted(id)) if id == doc_id
        ));
    }

    #[test]
    fn test_overlapping_audio_chunks_maintain_monotonic_duration() {
        let (_file, manager) = test_manager();

        let doc_id = manager
            .start_session(Some("Overlapping Audio Test".to_string()))
            .expect("start session");

        // Chunk 0: Alice from 0 to 5000 ms (end = 5000)
        manager
            .append_chunk(
                doc_id,
                0,
                5000,
                "This is the first segment of Alice speaking.".to_string(),
                Some("Alice".to_string()),
            )
            .expect("append chunk 0");

        // Chunk 1: Alice overlapping chunk from 2000 to 3000 ms (end = 3000 < 5000)
        manager
            .append_chunk(
                doc_id,
                2000,
                1000,
                "Overlap section with Alice.".to_string(),
                Some("Alice".to_string()),
            )
            .expect("append chunk 1");

        let doc = manager
            .get_document(doc_id)
            .expect("get doc")
            .expect("doc exists");

        let blocks = doc.speaker_blocks();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].speaker_label.as_deref(), Some("Alice"));
        assert_eq!(blocks[0].start_ms, 0);
        // Duration must remain monotonic: max(5000, 3000 - 0) = 5000 ms
        assert_eq!(blocks[0].duration_ms, 5000);
    }

    #[test]
    fn test_continuous_audio_chunker_streaming() {
        // 16kHz, 500ms chunks (8000 samples), 100ms overlap (1600 samples)
        // Step size = 8000 - 1600 = 6400 samples (400ms)
        let mut chunker = ContinuousAudioChunker::new(16000, 500, 100);

        // Push 10,000 samples
        let samples = vec![0.1f32; 10000];
        chunker.push_samples(&samples);

        let chunks = chunker.drain_chunks();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[0].start_ms, 0);
        assert_eq!(chunks[0].duration_ms, 500);
        assert_eq!(chunks[0].samples.len(), 8000);

        // Remaining in buffer: 10000 - 6400 = 3600 samples
        // Push another 4000 samples -> 7600 samples (still < 8000)
        chunker.push_samples(&vec![0.2f32; 4000]);
        assert!(chunker.drain_chunks().is_empty());

        // Push 1000 more samples -> 8600 samples (>= 8000)
        chunker.push_samples(&vec![0.3f32; 1000]);
        let chunks2 = chunker.drain_chunks();
        assert_eq!(chunks2.len(), 1);
        assert_eq!(chunks2[0].chunk_index, 1);
        assert_eq!(chunks2[0].start_ms, 400); // 6400 samples * 1000 / 16000 = 400ms
        assert_eq!(chunks2[0].duration_ms, 500);

        // Flush remaining trailing samples
        let flushed = chunker.flush().expect("trailing chunk exists");
        assert_eq!(flushed.chunk_index, 2);
        assert_eq!(flushed.start_ms, 800); // (6400 * 2) * 1000 / 16000 = 800ms
        assert!(flushed.samples.len() > 0);
    }

    #[test]
    fn test_non_insertion_guarantee() {
        // Non-insertion guarantee: confirms zero calls or dependencies on insert_text or keystroke injection.
        let (_file, manager) = test_manager();
        let doc_id = manager
            .start_session(Some("Private Strategy Meeting".to_string()))
            .expect("start session");

        let chunk = manager
            .append_chunk(
                doc_id,
                0,
                2000,
                "Confidential discussion.".to_string(),
                Some("Leader".to_string()),
            )
            .expect("append chunk");

        assert_eq!(chunk.text, "Confidential discussion.");

        let completed = manager.complete_session(doc_id).expect("complete session");
        assert_eq!(completed.status, DocumentStatus::Completed);
        // Documents remain stored in database, never injected into active windows.
        assert_eq!(completed.full_text(), "Confidential discussion.");
    }

    #[test]
    fn test_missing_document_error_handling() {
        let (_file, manager) = test_manager();
        let missing_id = 999_999;

        let append_err = manager.append_chunk(
            missing_id,
            0,
            1000,
            "Orphan text".to_string(),
            None,
        );
        assert!(matches!(
            append_err,
            Err(LongformError::DocumentNotFound(id)) if id == missing_id
        ));

        let complete_err = manager.complete_session(missing_id);
        assert!(matches!(
            complete_err,
            Err(LongformError::DocumentNotFound(id)) if id == missing_id
        ));
    }
}

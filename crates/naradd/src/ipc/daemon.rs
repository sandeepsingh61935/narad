//! D-Bus server for `dev.narad.Daemon`.

use narad_core::store::HistoryEntry as CoreHistoryEntry;
use narad_dbus::{HistoryEntry, HistoryStats};
use zbus::{interface, object_server::SignalEmitter};

use crate::history_days::{history_filter, stats_bounds};
use crate::pipeline::PipelineHandle;

pub struct DaemonIpc {
    pipeline: PipelineHandle,
}

impl DaemonIpc {
    pub fn new(pipeline: PipelineHandle) -> Self {
        Self { pipeline }
    }

    /// Emit `StateChanged` from outside interface methods (pipeline task).
    pub async fn emit_state(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()> {
        Self::state_changed(emitter, state).await
    }

    /// Emit `Level` while listening.
    pub async fn emit_level(emitter: &SignalEmitter<'_>, rms: f64) -> zbus::Result<()> {
        Self::level(emitter, rms).await
    }

    /// Emit `EntryAdded` after a successful history insert.
    pub async fn emit_entry_added(emitter: &SignalEmitter<'_>, id: i64) -> zbus::Result<()> {
        Self::entry_added(emitter, id).await
    }

    /// Emit `PartialTranscript` during streaming recognition.
    pub async fn emit_partial_transcript(emitter: &SignalEmitter<'_>, text: &str) -> zbus::Result<()> {
        Self::partial_transcript(emitter, text).await
    }
}

#[interface(name = "dev.narad.Daemon")]
impl DaemonIpc {
    async fn start(
        &self,
        mode: &str,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        if self.pipeline.transcribing() {
            return Err(zbus::fdo::Error::Failed("transcription in progress".into()));
        }
        let mode = if mode.is_empty() {
            None
        } else {
            Some(mode.to_string())
        };
        self.pipeline.request_start(mode);
        Ok(())
    }

    async fn stop(
        &self,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.pipeline.request_stop();
        Ok(())
    }

    async fn stop_and_insert(
        &self,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.pipeline.request_stop_and_insert();
        Ok(())
    }

    async fn cancel(
        &self,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.pipeline.request_cancel();
        Ok(())
    }

    async fn discard(
        &self,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.pipeline.request_discard();
        Ok(())
    }

    async fn toggle(
        &self,
        mode: &str,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        if !self.pipeline.is_listening() && self.pipeline.transcribing() {
            return Err(zbus::fdo::Error::Failed("transcription in progress".into()));
        }
        let mode = if mode.is_empty() {
            None
        } else {
            Some(mode.to_string())
        };
        self.pipeline.request_toggle(mode);
        Ok(())
    }

    async fn selection_command(&self) -> zbus::fdo::Result<()> {
        self.pipeline.request_selection_command();
        Ok(())
    }

    async fn history(
        &self,
        query: &str,
        offset: u32,
        limit: u32,
        from_day: &str,
        to_day: &str,
        mode: &str,
        app_id: &str,
        src_lang: &str,
    ) -> zbus::fdo::Result<Vec<HistoryEntry>> {
        let prepared = history_filter(from_day, to_day, mode, app_id, src_lang)
            .map_err(zbus::fdo::Error::Failed)?;
        if prepared.match_none {
            return Ok(Vec::new());
        }
        self.pipeline
            .with_store(|store| {
                let rows = store
                    .find(query, &prepared.filter, offset, limit)
                    .map_err(|e| e.to_string())?;
                Ok(rows.iter().map(core_history_to_dbus).collect())
            })
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn reinsert_entry(&self, id: i64) -> zbus::fdo::Result<()> {
        self.pipeline
            .reinsert(id)
            .await
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn paste_last(&self) -> zbus::fdo::Result<()> {
        self.pipeline
            .paste_last()
            .await
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn undo_last(&self) -> zbus::fdo::Result<()> {
        self.pipeline
            .undo_last()
            .await
            .map(|_| ())
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Server-side takes owned `Vec` so zbus can deserialize the array body.
    async fn delete_entries(&self, ids: Vec<i64>) -> zbus::fdo::Result<()> {
        self.pipeline
            .with_store(|store| {
                store.delete(&ids).map_err(|e| e.to_string())?;
                Ok(())
            })
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn read_audio(&self, id: i64) -> zbus::fdo::Result<Vec<u8>> {
        self.pipeline
            .read_audio(id)
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn transcribe(
        &self,
        path: &str,
        mode: &str,
        dst_lang: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<i64> {
        let path = path.to_string();
        let mode = mode.to_string();
        let dst_lang = dst_lang.to_string();
        let pipeline = self.pipeline.clone();
        let new_id =
            tokio::task::spawn_blocking(move || pipeline.transcribe(&path, &mode, &dst_lang))
                .await
                .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?
                .map_err(zbus::fdo::Error::Failed)?;
        Self::entry_added(&emitter, new_id)
            .await
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
        Ok(new_id)
    }

    async fn retranscribe(
        &self,
        id: i64,
        mode: &str,
        model_id: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<i64> {
        let mode = mode.to_string();
        let model_id = model_id.to_string();
        let pipeline = self.pipeline.clone();
        let new_id =
            tokio::task::spawn_blocking(move || pipeline.retranscribe(id, &mode, &model_id))
                .await
                .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?
                .map_err(zbus::fdo::Error::Failed)?;
        Self::entry_added(&emitter, new_id)
            .await
            .map_err(|err| zbus::fdo::Error::Failed(err.to_string()))?;
        Ok(new_id)
    }

    async fn set_pinned(&self, id: i64, pinned: bool) -> zbus::fdo::Result<()> {
        self.pipeline
            .with_store(|store| {
                store
                    .set_pinned(id, pinned)
                    .map_err(|err| err.to_string())?;
                Ok(())
            })
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn set_tags(&self, id: i64, names: Vec<String>) -> zbus::fdo::Result<()> {
        self.pipeline
            .with_store(|store| {
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                store.set_tags(id, &refs).map_err(|err| err.to_string())?;
                Ok(())
            })
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Ids are the caller's selection. This does not re-run history filters.
    async fn export(&self, ids: Vec<i64>, format: &str) -> zbus::fdo::Result<String> {
        self.pipeline
            .with_store(|store| store.export(&ids, format).map_err(|err| err.to_string()))
            .map_err(zbus::fdo::Error::Failed)
    }

    /// Every history row, including ones outside the dashboard's current filter.
    async fn delete_all(&self) -> zbus::fdo::Result<()> {
        self.pipeline
            .with_store(|store| {
                store.delete_all().map_err(|err| err.to_string())?;
                Ok(())
            })
            .map_err(zbus::fdo::Error::Failed)
    }

    /// `day` or `week`. The store receives local millisecond bounds only.
    async fn stats(&self, range: &str) -> zbus::fdo::Result<HistoryStats> {
        let bounds = stats_bounds(range).map_err(zbus::fdo::Error::Failed)?;
        self.pipeline
            .with_store(|store| {
                let totals = store
                    .stats(bounds.from_ms, bounds.to_ms)
                    .map_err(|err| err.to_string())?;
                Ok(HistoryStats {
                    words: totals.words,
                    utterances: totals.utterances,
                    speaking_wpm: totals.speaking_wpm,
                    time_saved_ms: totals.time_saved_ms,
                    from_day: bounds.from_day,
                    to_day: bounds.to_day,
                })
            })
            .map_err(zbus::fdo::Error::Failed)
    }

    async fn report_focus(&self, app_id: &str) -> zbus::fdo::Result<()> {
        self.pipeline.report_focus(app_id).map_err(lock_err)
    }

    #[zbus(signal)]
    async fn state_changed(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn level(emitter: &SignalEmitter<'_>, rms: f64) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn entry_added(emitter: &SignalEmitter<'_>, id: i64) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn partial_transcript(emitter: &SignalEmitter<'_>, text: &str) -> zbus::Result<()>;

    #[zbus(property)]
    async fn mode(&self) -> zbus::fdo::Result<String> {
        let (_, mode, _, _) = self.pipeline.snapshot_props().map_err(lock_err)?;
        Ok(mode)
    }

    #[zbus(property)]
    async fn set_mode(&self, value: &str) -> zbus::fdo::Result<()> {
        self.pipeline
            .set_mode(value)
            .map_err(zbus::fdo::Error::Failed)
    }

    #[zbus(property)]
    async fn paused(&self) -> zbus::fdo::Result<bool> {
        let (_, _, paused, _) = self.pipeline.snapshot_props().map_err(lock_err)?;
        Ok(paused)
    }

    #[zbus(property)]
    async fn set_paused(&self, value: bool) -> zbus::fdo::Result<()> {
        self.pipeline.set_paused(value).map_err(lock_err)
    }

    #[zbus(property)]
    async fn outcome_detail(&self) -> zbus::fdo::Result<String> {
        self.pipeline.outcome_detail().map_err(lock_err)
    }

    #[zbus(property)]
    async fn hud_visible(&self) -> zbus::fdo::Result<bool> {
        let (_, _, _, hud) = self.pipeline.snapshot_props().map_err(lock_err)?;
        Ok(hud)
    }

    #[zbus(property)]
    async fn set_hud_visible(&self, value: bool) -> zbus::fdo::Result<()> {
        self.pipeline.set_hud_visible(value).map_err(lock_err)
    }
}

pub fn core_history_to_dbus(entry: &CoreHistoryEntry) -> HistoryEntry {
    HistoryEntry {
        id: entry.id,
        created_at: entry.created_at,
        duration_ms: entry.duration_ms,
        mode: entry.mode.clone(),
        src_lang: entry.src_lang.clone(),
        dst_lang: entry.dst_lang.clone(),
        asr_model: entry.asr_model.clone(),
        mt_model: entry.mt_model.clone(),
        llm_model: entry.llm_model.clone(),
        app_id: entry.app_id.clone(),
        raw_text: entry.raw_text.clone(),
        final_text: entry.final_text.clone(),
        insert_status: entry.insert_status.as_str().to_string(),
        cleanup_flagged: entry.cleanup_flagged,
        audio_path: entry.audio_path.clone(),
        pinned: entry.pinned,
        kind: entry.kind.clone(),
        tags: entry.tags.clone(),
    }
}

fn lock_err(e: String) -> zbus::fdo::Error {
    zbus::fdo::Error::Failed(format!("internal: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use narad_core::store::{HistoryStore, InsertStatus, NewHistoryEntry};
    use tempfile::TempDir;

    #[test]
    fn core_history_maps_insert_status_as_str() {
        let dir = TempDir::new().expect("tempdir");
        let mut store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let id = store
            .insert(&NewHistoryEntry {
                duration_ms: 100,
                mode: "plain".into(),
                src_lang: "en".into(),
                dst_lang: "en".into(),
                asr_model: "test".into(),
                mt_model: None,
                llm_model: None,
                app_id: None,
                raw_text: "hi".into(),
                final_text: "Hi".into(),
                insert_status: InsertStatus::Skipped,
                cleanup_flagged: false,
                audio_path: None,
                pinned: true,
                kind: "dictation".into(),
            })
            .expect("insert");
        let row = store.get(id).expect("get").expect("row");
        let dbus = core_history_to_dbus(&row);
        assert_eq!(dbus.insert_status, "skipped");
        assert_eq!(dbus.raw_text, "hi");
        assert!(dbus.pinned);
        assert_eq!(dbus.kind, "dictation");
        assert_eq!(dbus.tags, [] as [&str; 0]);
    }

    #[tokio::test]
    async fn set_pinned_and_set_tags_round_trip() {
        let dir = TempDir::new().expect("tempdir");
        let store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let pipe = crate::pipeline::PipelineHandle::new(
            crate::config::NaradConfig::default(),
            store,
            None,
            dir.path().join("audio"),
        );
        let id = pipe
            .with_store(|store| {
                store
                    .insert(&NewHistoryEntry {
                        duration_ms: 100,
                        mode: "plain".into(),
                        src_lang: "en".into(),
                        dst_lang: "en".into(),
                        asr_model: "test".into(),
                        mt_model: None,
                        llm_model: None,
                        app_id: None,
                        raw_text: "hello".into(),
                        final_text: "Hello".into(),
                        insert_status: InsertStatus::Success,
                        cleanup_flagged: false,
                        audio_path: None,
                        pinned: false,
                        kind: "dictation".into(),
                    })
                    .map_err(|err| err.to_string())
            })
            .expect("insert");
        let ipc = DaemonIpc::new(pipe.clone());
        ipc.set_pinned(id, true).await.expect("pin");
        ipc.set_tags(id, vec![" a ".into(), "".into(), "a".into(), "A".into()])
            .await
            .expect("tags");
        let missing = ipc.set_pinned(41, true).await.expect_err("missing");
        assert!(missing.to_string().contains("41"));

        pipe.with_store(|store| {
            let row = store.get(id).map_err(|err| err.to_string())?.expect("row");
            let dbus = core_history_to_dbus(&row);
            assert!(dbus.pinned);
            assert_eq!(dbus.tags, ["a", "A"]);
            Ok(())
        })
        .expect("mapped");
    }

    #[tokio::test]
    async fn export_returns_final_text_and_delete_all_empties_the_store() {
        let dir = TempDir::new().expect("tempdir");
        let wav = dir.path().join("7.wav");
        std::fs::write(&wav, b"speech").expect("wav");
        let store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let pipe = crate::pipeline::PipelineHandle::new(
            crate::config::NaradConfig::default(),
            store,
            None,
            dir.path().join("audio"),
        );
        let id = pipe
            .with_store(|store| {
                store
                    .insert(&NewHistoryEntry {
                        duration_ms: 1000,
                        mode: "plain".into(),
                        src_lang: "en".into(),
                        dst_lang: "en".into(),
                        asr_model: "parakeet-ctc-110m-int8".into(),
                        mt_model: None,
                        llm_model: None,
                        app_id: Some("org.gnome.TextEditor".into()),
                        raw_text: "raw".into(),
                        final_text: "kept final".into(),
                        insert_status: InsertStatus::Success,
                        cleanup_flagged: false,
                        audio_path: Some(wav.display().to_string()),
                        pinned: false,
                        kind: "dictation".into(),
                    })
                    .map_err(|err| err.to_string())
            })
            .expect("insert");
        let ipc = DaemonIpc::new(pipe.clone());
        let doc = ipc.export(vec![id], "txt").await.expect("export");
        assert!(doc.contains("kept final"));
        let missing = ipc.export(vec![99], "json").await.expect_err("missing");
        assert!(missing.to_string().contains("no history row 99"));
        ipc.delete_all().await.expect("delete all");
        assert!(!wav.exists());
        pipe.with_store(|store| {
            let rows = store.list(0, 10).map_err(|err| err.to_string())?;
            assert!(rows.is_empty());
            Ok(())
        })
        .expect("empty");
    }

    #[tokio::test]
    async fn selection_command_calls_pipeline() {
        let dir = TempDir::new().expect("tempdir");
        let store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let pipe = crate::pipeline::PipelineHandle::new(
            crate::config::NaradConfig::default(),
            store,
            None,
            dir.path().join("audio"),
        );
        let ipc = DaemonIpc::new(pipe);
        ipc.selection_command().await.expect("selection_command");
    }
}

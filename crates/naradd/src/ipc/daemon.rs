//! D-Bus server for `dev.narad.Daemon` (Phase 1).

use narad_core::store::HistoryEntry as CoreHistoryEntry;
use narad_dbus::HistoryEntry;
use zbus::{interface, object_server::SignalEmitter};

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
}

#[interface(name = "dev.narad.Daemon")]
impl DaemonIpc {
    async fn start(
        &self,
        mode: &str,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
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

    async fn cancel(
        &self,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.pipeline.request_cancel();
        Ok(())
    }

    async fn toggle(
        &self,
        mode: &str,
        #[zbus(signal_emitter)] _emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let mode = if mode.is_empty() {
            None
        } else {
            Some(mode.to_string())
        };
        self.pipeline.request_toggle(mode);
        Ok(())
    }

    async fn history(
        &self,
        query: &str,
        offset: u32,
        limit: u32,
    ) -> zbus::fdo::Result<Vec<HistoryEntry>> {
        self.pipeline
            .with_store(|store| {
                let rows = if query.is_empty() {
                    store.list(offset, limit)
                } else {
                    store.search(query, offset, limit)
                }
                .map_err(|e| e.to_string())?;
                Ok(rows.iter().map(core_history_to_dbus).collect())
            })
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    async fn reinsert_entry(&self, id: i64) -> zbus::fdo::Result<()> {
        self.pipeline
            .reinsert(id)
            .await
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

    #[zbus(signal)]
    async fn state_changed(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn level(emitter: &SignalEmitter<'_>, rms: f64) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn entry_added(emitter: &SignalEmitter<'_>, id: i64) -> zbus::Result<()>;

    #[zbus(property)]
    async fn mode(&self) -> zbus::fdo::Result<String> {
        let (_, mode, _, _) = self.pipeline.snapshot_props().map_err(lock_err)?;
        Ok(mode)
    }

    #[zbus(property)]
    async fn paused(&self) -> zbus::fdo::Result<bool> {
        let (_, _, paused, _) = self.pipeline.snapshot_props().map_err(lock_err)?;
        Ok(paused)
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
            })
            .expect("insert");
        let row = store.get(id).expect("get").expect("row");
        let dbus = core_history_to_dbus(&row);
        assert_eq!(dbus.insert_status, "skipped");
        assert_eq!(dbus.raw_text, "hi");
        assert!(dbus.pinned);
    }
}

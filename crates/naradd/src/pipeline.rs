//! Dictation session: idle → listening → processing → inserted|copied|cancelled|error → idle.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use narad_core::store::{HistoryStore, InsertStatus, NewHistoryEntry};
use narad_dbus::{OBJECT_PATH, SessionState};
use tracing::{info, warn};
use zbus::Connection;

use crate::asr::{self, AsrEngine};
use crate::audio::{self, CaptureHandle};
use crate::config::NaradConfig;
use crate::insert::{self, InsertOutcome};
use crate::ipc::DaemonIpc;

struct SessionRuntime {
    dbus_state: String,
    mode: String,
    paused: bool,
    hud_visible: bool,
    capture: Option<CaptureHandle>,
    listen_started: Option<Instant>,
    /// Bumped on each successful `begin_listen` so max-duration timers ignore stale sessions.
    listen_generation: u64,
    /// Cancel flag observed by the processing task.
    cancel: Arc<AtomicBool>,
    /// Latest app id from `ReportFocus`. Empty or never reported is `None`.
    focused_app: Option<String>,
    /// Focused app when this recording started. Written to history `app_id` (FR-CTX-1).
    app_id_at_start: Option<String>,
    /// Focused app when text was last pasted. In memory for later undo / terminal chord.
    app_id_at_paste: Option<String>,
}

struct PipelineInner {
    config: NaradConfig,
    state: Mutex<SessionRuntime>,
    store: Mutex<HistoryStore>,
    asr: Mutex<Option<AsrEngine>>,
    conn: OnceLock<Connection>,
    /// Prevent overlapping stop/process tasks.
    processing: AtomicBool,
}

/// Shared handle used by D-Bus IPC and the hotkeys task.
#[derive(Clone)]
pub struct PipelineHandle {
    inner: Arc<PipelineInner>,
}

impl PipelineHandle {
    pub fn new(config: NaradConfig, store: HistoryStore, asr: Option<AsrEngine>) -> Self {
        Self {
            inner: Arc::new(PipelineInner {
                config: config.clone(),
                state: Mutex::new(SessionRuntime {
                    dbus_state: SessionState::IDLE.to_string(),
                    mode: config.general.active_mode.clone(),
                    paused: false,
                    hud_visible: true,
                    capture: None,
                    listen_started: None,
                    listen_generation: 0,
                    cancel: Arc::new(AtomicBool::new(false)),
                    focused_app: None,
                    app_id_at_start: None,
                    app_id_at_paste: None,
                }),
                store: Mutex::new(store),
                asr: Mutex::new(asr),
                conn: OnceLock::new(),
                processing: AtomicBool::new(false),
            }),
        }
    }

    /// Sleep then return to idle (error toast / copied toast).
    fn schedule_idle_after(&self, delay: Duration) {
        let handle = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            handle.set_state(SessionState::IDLE).await;
        });
    }

    pub fn set_connection(&self, conn: Connection) {
        let _ = self.inner.conn.set(conn);
    }

    pub fn config(&self) -> NaradConfig {
        self.inner.config.clone()
    }

    pub fn with_store<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut HistoryStore) -> Result<R, String>,
    {
        let mut store = self
            .inner
            .store
            .lock()
            .map_err(|e| format!("store lock: {e}"))?;
        f(&mut store)
    }

    pub fn snapshot_props(&self) -> Result<(String, String, bool, bool), String> {
        let st = self
            .inner
            .state
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        Ok((
            st.dbus_state.clone(),
            st.mode.clone(),
            st.paused,
            st.hud_visible,
        ))
    }

    pub fn set_hud_visible(&self, visible: bool) -> Result<(), String> {
        let mut st = self
            .inner
            .state
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        st.hud_visible = visible;
        Ok(())
    }

    /// Cache the focused app id. Empty string clears it (`None`).
    pub fn report_focus(&self, app_id: &str) -> Result<(), String> {
        let mut st = self
            .inner
            .state
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        st.focused_app = normalize_app_id(app_id);
        Ok(())
    }

    pub fn is_listening(&self) -> bool {
        self.inner
            .state
            .lock()
            .map(|s| s.dbus_state == SessionState::LISTENING)
            .unwrap_or(false)
    }

    pub fn request_start(&self, mode: Option<String>) {
        let handle = self.clone();
        tokio::spawn(async move {
            if let Err(err) = handle.begin_listen(mode).await {
                warn!(error = %err, "begin_listen failed");
                handle.set_state(SessionState::ERROR).await;
                handle.schedule_idle_after(Duration::from_millis(800));
            }
        });
    }

    pub fn request_stop(&self) {
        let handle = self.clone();
        tokio::spawn(async move {
            if let Err(err) = handle.begin_stop().await {
                warn!(error = %err, "begin_stop failed");
                handle.set_state(SessionState::ERROR).await;
                handle.schedule_idle_after(Duration::from_millis(800));
            }
        });
    }

    pub fn request_cancel(&self) {
        let handle = self.clone();
        tokio::spawn(async move {
            handle.cancel_listen().await;
        });
    }

    pub fn request_toggle(&self, mode: Option<String>) {
        if self.is_listening() {
            self.request_stop();
        } else {
            self.request_start(mode);
        }
    }

    async fn begin_listen(&self, mode: Option<String>) -> Result<(), String> {
        {
            let mut st = self
                .inner
                .state
                .lock()
                .map_err(|e| format!("state lock: {e}"))?;
            if st.dbus_state == SessionState::LISTENING
                || st.dbus_state == SessionState::PROCESSING
            {
                return Ok(());
            }
            if let Some(m) = mode {
                if !m.is_empty() {
                    st.mode = m;
                }
            }
            st.cancel.store(false, Ordering::SeqCst);
            snapshot_focus_at_start(&mut st);
        }

        audio::play_cue("start", self.inner.config.audio.sounds_enabled);

        let capture = CaptureHandle::start().map_err(|e| e.to_string())?;
        let rms_bits = capture.rms_bits.clone();

        let listen_gen = {
            let mut st = self
                .inner
                .state
                .lock()
                .map_err(|e| format!("state lock: {e}"))?;
            st.capture = Some(capture);
            st.listen_started = Some(Instant::now());
            st.listen_generation = st.listen_generation.wrapping_add(1);
            st.dbus_state = SessionState::LISTENING.to_string();
            st.listen_generation
        };

        self.emit_state(SessionState::LISTENING).await;
        self.spawn_level_pump(rms_bits);
        self.spawn_max_duration_timer(listen_gen);
        info!("listening");
        Ok(())
    }

    /// Hard max recording length (FR-AUD-3). `auto_stop_silence_secs` remains Phase 2 unused.
    fn spawn_max_duration_timer(&self, listen_gen: u64) {
        let max_secs = self.inner.config.audio.max_duration_secs;
        if max_secs == 0 {
            return;
        }
        let handle = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(u64::from(max_secs))).await;
            let still_this_listen = handle
                .inner
                .state
                .lock()
                .map(|s| {
                    s.dbus_state == SessionState::LISTENING && s.listen_generation == listen_gen
                })
                .unwrap_or(false);
            if still_this_listen {
                info!(max_secs, "max_duration_secs reached; auto-stop");
                handle.request_stop();
            }
        });
    }

    fn spawn_level_pump(&self, rms_bits: Arc<std::sync::atomic::AtomicU32>) {
        let handle = self.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(33));
            loop {
                tick.tick().await;
                if !handle.is_listening() {
                    break;
                }
                let rms = f32::from_bits(rms_bits.load(Ordering::Relaxed)) as f64;
                handle.emit_level(rms).await;
            }
        });
    }

    async fn cancel_listen(&self) {
        audio::play_cue("cancel", self.inner.config.audio.sounds_enabled);
        let capture = {
            let mut st = match self.inner.state.lock() {
                Ok(s) => s,
                Err(_) => return,
            };
            st.cancel.store(true, Ordering::SeqCst);
            st.listen_started = None;
            st.capture.take()
        };
        drop(capture);
        self.inner.processing.store(false, Ordering::SeqCst);
        self.set_state(SessionState::CANCELLED).await;
        info!("cancelled");
        self.schedule_idle_after(Duration::from_millis(1200));
    }

    async fn begin_stop(&self) -> Result<(), String> {
        if !self
            .inner
            .processing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return Ok(());
        }

        let (capture, started, mode, cancel_flag) = {
            let mut st = self
                .inner
                .state
                .lock()
                .map_err(|e| format!("state lock: {e}"))?;
            if st.dbus_state != SessionState::LISTENING {
                self.inner.processing.store(false, Ordering::SeqCst);
                return Ok(());
            }
            let capture = st.capture.take();
            let started = st.listen_started.take();
            st.dbus_state = SessionState::PROCESSING.to_string();
            (
                capture,
                started,
                st.mode.clone(),
                st.cancel.clone(),
            )
        };

        audio::play_cue("stop", self.inner.config.audio.sounds_enabled);
        self.emit_state(SessionState::PROCESSING).await;

        let (raw_samples, device_rate) = match capture {
            Some(c) => c.stop().map_err(|e| e.to_string())?,
            None => {
                self.inner.processing.store(false, Ordering::SeqCst);
                self.set_state(SessionState::IDLE).await;
                return Ok(());
            }
        };

        if cancel_flag.load(Ordering::SeqCst) {
            self.inner.processing.store(false, Ordering::SeqCst);
            self.set_state(SessionState::CANCELLED).await;
            self.schedule_idle_after(Duration::from_millis(1200));
            return Ok(());
        }

        let duration_ms = started
            .map(|t| t.elapsed().as_millis() as u32)
            .unwrap_or(0)
            .min(u32::MAX);

        let samples_16k = audio::resample_to_16k(&raw_samples, device_rate);
        let trimmed = match audio::trim_silence(&samples_16k, audio::TARGET_SAMPLE_RATE) {
            Some(t) => t,
            None => {
                info!("VAD: no speech; returning to idle");
                audio::play_cue("error", self.inner.config.audio.sounds_enabled);
                self.inner.processing.store(false, Ordering::SeqCst);
                self.set_state(SessionState::IDLE).await;
                return Ok(());
            }
        };

        let model_id = self.inner.config.models.asr_english.clone();
        // Run decode on a blocking pool; lock only inside the worker (Standards #5).
        let asr_slot = Arc::clone(&self.inner);
        let transcribed = tokio::task::spawn_blocking(move || {
            let asr_guard = asr_slot
                .asr
                .lock()
                .map_err(|e| format!("asr lock: {e}"))?;
            match asr_guard.as_ref() {
                Some(engine) => engine
                    .transcribe(&trimmed, audio::TARGET_SAMPLE_RATE)
                    .map_err(|e| e.to_string())
                    .map(Some),
                None => Ok(None),
            }
        })
        .await
        .map_err(|e| format!("asr join: {e}"))?;

        let text = match transcribed {
            Ok(Some(t)) => t,
            Ok(None) => {
                warn!("ASR engine not loaded");
                self.inner.processing.store(false, Ordering::SeqCst);
                self.set_state(SessionState::ERROR).await;
                self.schedule_idle_after(Duration::from_millis(800));
                return Ok(());
            }
            Err(e) => {
                self.inner.processing.store(false, Ordering::SeqCst);
                return Err(e);
            }
        };

        if cancel_flag.load(Ordering::SeqCst) {
            self.inner.processing.store(false, Ordering::SeqCst);
            self.set_state(SessionState::CANCELLED).await;
            self.schedule_idle_after(Duration::from_millis(1200));
            return Ok(());
        }

        if asr::is_hallucination(&text) {
            info!(raw = %text, "hallucination filter dropped output");
            self.inner.processing.store(false, Ordering::SeqCst);
            self.set_state(SessionState::IDLE).await;
            return Ok(());
        }

        let final_text = text.clone();
        let outcome = insert::insert_text(&final_text, false).await;
        let insert_status = match outcome {
            InsertOutcome::Inserted => InsertStatus::Success,
            InsertOutcome::Copied => InsertStatus::Skipped,
        };

        let entry_id =
            self.commit_dictation(duration_ms, mode, model_id, text, final_text, insert_status)?;
        self.emit_entry_added(entry_id).await;

        self.inner.processing.store(false, Ordering::SeqCst);
        match outcome {
            InsertOutcome::Inserted => {
                info!(entry_id, "inserted");
                self.set_state(SessionState::INSERTED).await;
                self.schedule_idle_after(Duration::from_millis(1200));
            }
            InsertOutcome::Copied => {
                info!(entry_id, "copied (no focus / paste failed)");
                self.set_state(SessionState::COPIED).await;
                self.schedule_idle_after(Duration::from_secs(2));
            }
        }
        Ok(())
    }

    pub async fn reinsert(&self, id: i64) -> Result<(), String> {
        let entry = {
            let store = self
                .inner
                .store
                .lock()
                .map_err(|e| format!("store lock: {e}"))?;
            store
                .get(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("entry {id} not found"))?
        };
        let outcome = insert::insert_text(&entry.final_text, false).await;
        if matches!(outcome, InsertOutcome::Inserted) {
            if let Ok(mut st) = self.inner.state.lock() {
                note_paste_focus(&mut st);
            }
        }
        match outcome {
            InsertOutcome::Inserted => Ok(()),
            InsertOutcome::Copied => {
                self.set_state(SessionState::COPIED).await;
                self.schedule_idle_after(Duration::from_secs(2));
                Ok(())
            }
        }
    }

    async fn set_state(&self, state: &str) {
        if let Ok(mut st) = self.inner.state.lock() {
            st.dbus_state = state.to_string();
        }
        self.emit_state(state).await;
    }

    async fn emit_state(&self, state: &str) {
        let Some(conn) = self.inner.conn.get() else {
            return;
        };
        match conn
            .object_server()
            .interface::<_, DaemonIpc>(OBJECT_PATH)
            .await
        {
            Ok(iface) => {
                if let Err(err) = DaemonIpc::emit_state(iface.signal_emitter(), state).await {
                    warn!(error = %err, "emit StateChanged");
                }
            }
            Err(err) => warn!(error = %err, "StateChanged interface"),
        }
    }

    async fn emit_level(&self, rms: f64) {
        let Some(conn) = self.inner.conn.get() else {
            return;
        };
        if let Ok(iface) = conn
            .object_server()
            .interface::<_, DaemonIpc>(OBJECT_PATH)
            .await
        {
            let _ = DaemonIpc::emit_level(iface.signal_emitter(), rms).await;
        }
    }

    async fn emit_entry_added(&self, id: i64) {
        let Some(conn) = self.inner.conn.get() else {
            return;
        };
        if let Ok(iface) = conn
            .object_server()
            .interface::<_, DaemonIpc>(OBJECT_PATH)
            .await
        {
            if let Err(err) = DaemonIpc::emit_entry_added(iface.signal_emitter(), id).await {
                warn!(error = %err, "emit EntryAdded");
            }
        }
    }

    /// Persist one live utterance. History `app_id` is the focus frozen at record start.
    /// A successful paste also remembers the app focused now.
    fn commit_dictation(
        &self,
        duration_ms: u32,
        mode: String,
        asr_model: String,
        raw_text: String,
        final_text: String,
        insert_status: InsertStatus,
    ) -> Result<i64, String> {
        let app_id = {
            let mut st = self
                .inner
                .state
                .lock()
                .map_err(|e| format!("state lock: {e}"))?;
            if insert_status == InsertStatus::Success {
                note_paste_focus(&mut st);
            }
            st.app_id_at_start.clone()
        };
        let mut store = self
            .inner
            .store
            .lock()
            .map_err(|e| format!("store lock: {e}"))?;
        store
            .insert(&NewHistoryEntry {
                duration_ms,
                mode,
                src_lang: "en".into(),
                dst_lang: "en".into(),
                asr_model,
                mt_model: None,
                llm_model: None,
                app_id,
                raw_text,
                final_text,
                insert_status,
                cleanup_flagged: false,
                audio_path: None,
                pinned: false,
            })
            .map_err(|e| e.to_string())
    }
}

fn normalize_app_id(app_id: &str) -> Option<String> {
    if app_id.is_empty() {
        None
    } else {
        Some(app_id.to_string())
    }
}

fn snapshot_focus_at_start(st: &mut SessionRuntime) {
    st.app_id_at_start = st.focused_app.clone();
}

fn note_paste_focus(st: &mut SessionRuntime) {
    st.app_id_at_paste = st.focused_app.clone();
}

#[cfg(test)]
mod tests {
    use super::*;
    use narad_core::store::HistoryStore;
    use tempfile::TempDir;

    fn test_pipe() -> (TempDir, PipelineHandle) {
        let dir = TempDir::new().expect("tempdir");
        let store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let pipe = PipelineHandle::new(NaradConfig::default(), store, None);
        (dir, pipe)
    }

    fn mark_started(pipe: &PipelineHandle) {
        let mut st = pipe.inner.state.lock().expect("state");
        snapshot_focus_at_start(&mut st);
    }

    fn insert_hello(pipe: &PipelineHandle) -> i64 {
        pipe.commit_dictation(
            10,
            "plain".into(),
            "test".into(),
            "hello".into(),
            "hello".into(),
            InsertStatus::Success,
        )
        .expect("insert")
    }

    fn stored_app(pipe: &PipelineHandle, id: i64) -> Option<String> {
        pipe.with_store(|store| {
            Ok(store
                .get(id)
                .map_err(|e| e.to_string())?
                .expect("row")
                .app_id)
        })
        .expect("store")
    }

    #[test]
    fn reported_focus_is_what_the_next_insert_stores() {
        let (_dir, pipe) = test_pipe();

        mark_started(&pipe);
        let id = insert_hello(&pipe);
        assert!(id > 0);
        assert_eq!(stored_app(&pipe, id), None);
        assert_eq!(pipe.inner.state.lock().unwrap().app_id_at_paste, None);

        pipe.report_focus("").expect("clear");
        mark_started(&pipe);
        let id = insert_hello(&pipe);
        assert!(id > 0);
        assert_eq!(stored_app(&pipe, id), None);

        pipe.report_focus("org.gnome.TextEditor").expect("report");
        mark_started(&pipe);
        pipe.report_focus("org.gnome.Terminal")
            .expect("later focus");
        let id = insert_hello(&pipe);
        assert!(id > 0);
        assert_eq!(
            stored_app(&pipe, id).as_deref(),
            Some("org.gnome.TextEditor")
        );
        assert_eq!(
            pipe.inner.state.lock().unwrap().app_id_at_paste.as_deref(),
            Some("org.gnome.Terminal")
        );
    }
}

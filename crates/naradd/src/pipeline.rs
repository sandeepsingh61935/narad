//! Dictation session: idle → listening → processing → inserted|copied|cancelled|error → idle.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use narad_core::store::{HistoryStore, InsertStatus, NewHistoryEntry};
use narad_dbus::{SessionState, OBJECT_PATH};
use tracing::{info, warn};
use zbus::Connection;

use crate::asr::{self, AsrEngine};
use crate::audio::{self, CaptureHandle};
use crate::denoise;
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
    /// Bumped on each successful `begin_listen` so listen timers ignore stale sessions.
    listen_generation: u64,
    /// True while a dictate-key press is still down (hold). Silence stop stays off.
    awaiting_hold_release: bool,
    /// Bumped when a press is classified as a tap, so a late `begin_listen` does not re-arm hold.
    hold_epoch: u64,
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

/// How often uptime retention runs after the startup pass (FR-HIS-7).
const RETENTION_PURGE_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

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
                    awaiting_hold_release: false,
                    hold_epoch: 0,
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

    /// FR-HIS-7: purge once now, then every 24 hours of uptime.
    ///
    /// `retain_audio` is not a timer. Audio TTL runs only when `audio_retention_days` is set.
    pub fn start_retention_purge(&self) {
        self.purge_history_retention();
        let handle = self.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(RETENTION_PURGE_EVERY).await;
                handle.purge_history_retention();
            }
        });
    }

    fn purge_history_retention(&self) {
        let text_days = self.inner.config.history.text_retention_days;
        let audio_days = self.inner.config.history.audio_retention_days;
        if text_days.is_none() && audio_days.is_none() {
            return;
        }
        let now_ms = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(elapsed) => elapsed.as_millis() as i64,
            Err(err) => {
                warn!(error = %err, "history retention purge skipped; clock before epoch");
                return;
            }
        };
        match self.with_store(|store| {
            store
                .purge_retention(now_ms, text_days, audio_days)
                .map_err(|err| err.to_string())
        }) {
            Ok(stats) => {
                if stats.text_deleted > 0 || stats.audio_cleared > 0 {
                    info!(
                        text_deleted = stats.text_deleted,
                        audio_cleared = stats.audio_cleared,
                        "history retention purge"
                    );
                }
            }
            Err(err) => warn!(error = %err, "history retention purge failed"),
        }
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

    pub fn set_paused(&self, paused: bool) -> Result<(), String> {
        let mut st = self
            .inner
            .state
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        st.paused = paused;
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
        self.spawn_listen(mode, ListenTrigger::Open);
    }

    /// Paste-last shortcut. One press pastes; release is not a second action.
    pub fn request_paste_last(&self) {
        let handle = self.clone();
        tokio::spawn(async move {
            if let Err(err) = handle.paste_last().await {
                warn!(error = %err, "paste last failed");
                handle.set_state(SessionState::ERROR).await;
                handle.schedule_idle_after(Duration::from_millis(800));
            }
        });
    }

    /// Hotkey press. Silence stop stays off until the release is a tap (or this was a HUD/open start).
    pub fn request_hotkey_press(&self) {
        let epoch = self.inner.state.lock().map(|st| st.hold_epoch).unwrap_or(0);
        self.spawn_listen(None, ListenTrigger::Hotkey { epoch });
    }

    /// Dictate key released before the hold threshold. Listening continues; silence stop may run.
    pub fn note_tap_release(&self) {
        if let Ok(mut st) = self.inner.state.lock() {
            st.awaiting_hold_release = false;
            st.hold_epoch = st.hold_epoch.wrapping_add(1);
        }
    }

    fn spawn_listen(&self, mode: Option<String>, trigger: ListenTrigger) {
        let handle = self.clone();
        tokio::spawn(async move {
            if let Err(err) = handle.begin_listen(mode, trigger).await {
                handle.clear_hold_gate();
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

    async fn begin_listen(
        &self,
        mode: Option<String>,
        trigger: ListenTrigger,
    ) -> Result<(), String> {
        {
            let mut st = self
                .inner
                .state
                .lock()
                .map_err(|e| format!("state lock: {e}"))?;
            if st.dbus_state == SessionState::LISTENING || st.dbus_state == SessionState::PROCESSING
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

        let capture = CaptureHandle::start(&self.inner.config.audio.input_target)
            .map_err(|e| e.to_string())?;
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
            st.awaiting_hold_release = hold_flag_for_listen(&trigger, st.hold_epoch);
            st.dbus_state = SessionState::LISTENING.to_string();
            st.listen_generation
        };

        self.emit_state(SessionState::LISTENING).await;
        self.spawn_level_pump(rms_bits);
        self.spawn_max_duration_timer(listen_gen);
        self.spawn_silence_timer(listen_gen);
        info!("listening");
        Ok(())
    }

    /// Hard max recording length (FR-AUD-3). `0` disables this cap only.
    fn spawn_max_duration_timer(&self, listen_gen: u64) {
        let Some(limit) = configured_timeout(self.inner.config.audio.max_duration_secs) else {
            return;
        };
        let handle = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(limit).await;
            let still_this_listen = handle
                .inner
                .state
                .lock()
                .map(|s| {
                    s.dbus_state == SessionState::LISTENING && s.listen_generation == listen_gen
                })
                .unwrap_or(false);
            if still_this_listen {
                info!(
                    max_secs = limit.as_secs(),
                    "max_duration_secs reached; auto-stop"
                );
                handle.request_stop();
            }
        });
    }

    /// Tap-mode and HUD silence auto-stop (FR-AUD-3). Hold does not stop.
    /// `0` disables this timer only.
    fn spawn_silence_timer(&self, listen_gen: u64) {
        let Some(limit) = configured_timeout(self.inner.config.audio.auto_stop_silence_secs) else {
            return;
        };
        let handle = self.clone();
        tokio::spawn(async move {
            let mut watch = SilenceWatch::new(Some(limit), Instant::now());
            let mut tick = tokio::time::interval(Duration::from_millis(50));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                let Some(poll) = handle.silence_poll(listen_gen, watch.scanned) else {
                    break;
                };
                watch.scanned = poll.end;
                if watch.observe(
                    Instant::now(),
                    &poll.fresh,
                    audio::TARGET_SAMPLE_RATE,
                    poll.awaiting_hold,
                ) {
                    info!(
                        silence_secs = limit.as_secs(),
                        "auto_stop_silence_secs reached; auto-stop"
                    );
                    handle.request_stop();
                    break;
                }
            }
        });
    }

    fn silence_poll(&self, listen_gen: u64, scanned: usize) -> Option<SilencePoll> {
        let st = self.inner.state.lock().ok()?;
        if st.dbus_state != SessionState::LISTENING || st.listen_generation != listen_gen {
            return None;
        }
        let (fresh, end) = match st.capture.as_ref() {
            Some(capture) => capture.copy_from(scanned),
            None => (Vec::new(), scanned),
        };
        Some(SilencePoll {
            fresh,
            end,
            awaiting_hold: st.awaiting_hold_release,
        })
    }

    fn clear_hold_gate(&self) {
        if let Ok(mut st) = self.inner.state.lock() {
            st.awaiting_hold_release = false;
        }
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
            st.awaiting_hold_release = false;
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
            st.awaiting_hold_release = false;
            st.dbus_state = SessionState::PROCESSING.to_string();
            (capture, started, st.mode.clone(), st.cancel.clone())
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
        // Off: same 16 kHz buffer, no denoise pass (FR-AUD-5).
        let samples_16k = denoise::apply(samples_16k, self.inner.config.audio.noise_suppression);
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
            let asr_guard = asr_slot.asr.lock().map_err(|e| format!("asr lock: {e}"))?;
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
        let history_id =
            self.record_dictation(duration_ms, mode, model_id, text, final_text, outcome)?;
        if let Some(id) = history_id {
            self.emit_entry_added(id).await;
        }

        self.inner.processing.store(false, Ordering::SeqCst);
        match outcome {
            InsertOutcome::Inserted => {
                info!(?history_id, "inserted");
                self.set_state(SessionState::INSERTED).await;
                self.schedule_idle_after(Duration::from_millis(1200));
            }
            InsertOutcome::Copied => {
                info!(?history_id, "copied (no focus / paste failed)");
                self.set_state(SessionState::COPIED).await;
                self.schedule_idle_after(Duration::from_secs(2));
            }
        }
        Ok(())
    }

    /// Paste the newest history row's `final_text` (FR-INS-4).
    ///
    /// Not [`Self::reinsert`]: no entry id. A copied (`skipped`) row still counts.
    /// No final text signals the HUD error outcome and pastes nothing.
    pub async fn paste_last(&self) -> Result<(), String> {
        let Some(text) = self.newest_final_text()? else {
            self.set_state(SessionState::ERROR).await;
            self.schedule_idle_after(Duration::from_millis(800));
            return Ok(());
        };
        let outcome = insert::insert_text(&text, false).await;
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

    /// Newest row by `created_at`, then id. Empty `final_text` is nothing to paste.
    fn newest_final_text(&self) -> Result<Option<String>, String> {
        self.with_store(|store| {
            let rows = store.list(0, 1).map_err(|e| e.to_string())?;
            Ok(rows.into_iter().next().and_then(|row| {
                if row.final_text.is_empty() {
                    None
                } else {
                    Some(row.final_text)
                }
            }))
        })
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

    /// Paste already happened (`outcome`). Write a history row unless paused (FR-HIS-7).
    ///
    /// `Ok(None)` means no row, so the caller must not emit `EntryAdded`.
    /// Explicit transcribe / re-insert must call [`Self::commit_dictation`], not this.
    fn record_dictation(
        &self,
        duration_ms: u32,
        mode: String,
        asr_model: String,
        raw_text: String,
        final_text: String,
        outcome: InsertOutcome,
    ) -> Result<Option<i64>, String> {
        let insert_status = match outcome {
            InsertOutcome::Inserted => InsertStatus::Success,
            InsertOutcome::Copied => InsertStatus::Skipped,
        };
        let paused = {
            let mut st = self
                .inner
                .state
                .lock()
                .map_err(|e| format!("state lock: {e}"))?;
            if st.paused && insert_status == InsertStatus::Success {
                note_paste_focus(&mut st);
            }
            st.paused
        };
        if paused {
            return Ok(None);
        }
        self.commit_dictation(
            duration_ms,
            mode,
            asr_model,
            raw_text,
            final_text,
            insert_status,
        )
        .map(Some)
    }

    /// Persist one utterance. Not paused-gated.
    /// History `app_id` is the focus frozen at record start.
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

/// Why listening started. Hold suppresses silence stop until the key release is a tap.
#[derive(Clone, Copy)]
enum ListenTrigger {
    /// HUD click or D-Bus Start/Toggle: not waiting for key release.
    Open,
    /// Dictate key went down. A tap release bumps `hold_epoch` so this does not stick.
    Hotkey { epoch: u64 },
}

fn hold_flag_for_listen(trigger: &ListenTrigger, hold_epoch: u64) -> bool {
    match *trigger {
        ListenTrigger::Open => false,
        ListenTrigger::Hotkey { epoch } => epoch == hold_epoch,
    }
}

/// `0` disables that one timer. It does not restore the FR-AUD-3 default.
fn configured_timeout(secs: u32) -> Option<Duration> {
    if secs == 0 {
        None
    } else {
        Some(Duration::from_secs(u64::from(secs)))
    }
}

struct SilencePoll {
    fresh: Vec<f32>,
    end: usize,
    awaiting_hold: bool,
}

/// Time since the last energy-VAD speech frame. Fires only when not awaiting hold release.
struct SilenceWatch {
    limit: Option<Duration>,
    last_speech: Instant,
    pending: Vec<f32>,
    scanned: usize,
}

impl SilenceWatch {
    fn new(limit: Option<Duration>, now: Instant) -> Self {
        Self {
            limit,
            last_speech: now,
            pending: Vec::new(),
            scanned: 0,
        }
    }

    /// `fresh` is audio appended since the previous poll. Speech on the shared energy gate resets the clock.
    fn observe(&mut self, now: Instant, fresh: &[f32], rate: i32, awaiting_hold: bool) -> bool {
        self.pending.extend_from_slice(fresh);
        let frame = audio::speech_frame_len(rate);
        if frame > 0 && self.pending.len() >= frame {
            let usable = self.pending.len() - (self.pending.len() % frame);
            if audio::contains_speech(&self.pending[..usable], rate) {
                self.last_speech = now;
            }
            self.pending.drain(..usable);
        }
        match self.limit {
            Some(limit) if !awaiting_hold && elapsed_since(now, self.last_speech) >= limit => true,
            _ => false,
        }
    }
}

fn elapsed_since(now: Instant, then: Instant) -> Duration {
    now.checked_duration_since(then).unwrap_or(Duration::ZERO)
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

    /// Stand-in for `insert::insert_text`: non-empty text pastes successfully.
    fn paste_success(text: &str) -> InsertOutcome {
        assert!(!text.is_empty());
        InsertOutcome::Inserted
    }

    fn history_len(pipe: &PipelineHandle) -> usize {
        pipe.with_store(|store| {
            store
                .list(0, 50)
                .map(|rows| rows.len())
                .map_err(|e| e.to_string())
        })
        .expect("history len")
    }

    fn dictate(pipe: &PipelineHandle, text: &str) -> Result<Option<i64>, String> {
        let outcome = paste_success(text);
        pipe.record_dictation(
            12,
            "plain".into(),
            "test".into(),
            text.into(),
            text.into(),
            outcome,
        )
    }

    #[tokio::test]
    async fn paste_last_picks_newest_final() {
        use std::sync::{Arc, Mutex};

        let (_dir, pipe) = test_pipe();
        let log = Arc::new(Mutex::new(Vec::new()));
        insert::set_test_insert_log(Some(Arc::clone(&log)));
        struct ClearLog;
        impl Drop for ClearLog {
            fn drop(&mut self) {
                insert::set_test_insert_log(None);
            }
        }
        let _clear = ClearLog;

        pipe.paste_last().await.expect("empty history");
        assert!(log.lock().expect("log").is_empty());
        assert_eq!(pipe.snapshot_props().expect("props").0, SessionState::ERROR);

        let older = pipe
            .commit_dictation(
                10,
                "plain".into(),
                "test".into(),
                "older raw".into(),
                "older final".into(),
                InsertStatus::Success,
            )
            .expect("older");
        let newest = pipe
            .commit_dictation(
                11,
                "plain".into(),
                "test".into(),
                "newest raw".into(),
                "newest final".into(),
                InsertStatus::Skipped,
            )
            .expect("newest copied");
        assert!(newest > older);

        pipe.paste_last().await.expect("paste newest");
        assert_eq!(log.lock().expect("log").as_slice(), ["newest final"]);
        let status = pipe
            .with_store(|store| {
                Ok(store
                    .get(newest)
                    .map_err(|e| e.to_string())?
                    .expect("row")
                    .insert_status)
            })
            .expect("status");
        assert_eq!(status, InsertStatus::Skipped);
        assert_eq!(history_len(&pipe), 2);

        pipe.commit_dictation(
            12,
            "plain".into(),
            "test".into(),
            "blank raw".into(),
            String::new(),
            InsertStatus::Success,
        )
        .expect("empty final");
        pipe.paste_last().await.expect("no final text");
        assert_eq!(log.lock().expect("log").as_slice(), ["newest final"]);
        assert_eq!(pipe.snapshot_props().expect("props").0, SessionState::ERROR);
    }

    #[test]
    fn paused_dictation_pastes_without_a_row_and_unpaused_inserts() {
        let (_dir, pipe) = test_pipe();
        pipe.set_paused(true).expect("pause");
        assert!(pipe.snapshot_props().expect("props").2);

        let paused_id = dictate(&pipe, "hello").expect("paused dictation succeeds");
        assert!(paused_id.is_none());
        assert_eq!(history_len(&pipe), 0);

        let copied = pipe
            .record_dictation(
                12,
                "plain".into(),
                "test".into(),
                "hello".into(),
                "hello".into(),
                InsertOutcome::Copied,
            )
            .expect("paused copy succeeds");
        assert!(copied.is_none());
        assert_eq!(history_len(&pipe), 0);

        // Ungated writer still inserts while paused (future transcribe / re-insert).
        let explicit = insert_hello(&pipe);
        assert!(explicit > 0);
        assert_eq!(history_len(&pipe), 1);

        pipe.set_paused(false).expect("unpause");
        assert!(!pipe.snapshot_props().expect("props").2);
        let id = dictate(&pipe, "hello")
            .expect("unpaused dictation")
            .expect("history id");
        assert!(id > 0);
        assert_eq!(history_len(&pipe), 2);
        let final_text = pipe
            .with_store(|store| {
                Ok(store
                    .get(id)
                    .map_err(|e| e.to_string())?
                    .expect("row")
                    .final_text)
            })
            .expect("row");
        assert_eq!(final_text, "hello");
    }

    #[test]
    fn zero_disables_one_timer_without_restoring_defaults() {
        assert_eq!(configured_timeout(0), None);
        assert_eq!(configured_timeout(2), Some(Duration::from_secs(2)));
        assert_eq!(configured_timeout(300), Some(Duration::from_secs(300)));
        assert_ne!(configured_timeout(0), Some(Duration::from_secs(2)));
        assert_ne!(configured_timeout(0), Some(Duration::from_secs(300)));
    }

    #[test]
    fn hud_listen_is_not_a_hold_and_a_late_tap_clears_it() {
        assert!(!hold_flag_for_listen(&ListenTrigger::Open, 0));
        assert!(hold_flag_for_listen(&ListenTrigger::Hotkey { epoch: 3 }, 3));
        // Tap release bumps `hold_epoch` before begin_listen commits the flag.
        assert!(!hold_flag_for_listen(
            &ListenTrigger::Hotkey { epoch: 3 },
            4
        ));
    }

    fn quiet_frame() -> Vec<f32> {
        vec![0.0; audio::speech_frame_len(audio::TARGET_SAMPLE_RATE)]
    }

    fn speech_frame() -> Vec<f32> {
        vec![0.2; audio::speech_frame_len(audio::TARGET_SAMPLE_RATE)]
    }

    #[test]
    fn tap_and_hud_stop_after_silence_but_hold_does_not() {
        let t0 = Instant::now();
        let limit = configured_timeout(2);
        let mut watch = SilenceWatch::new(limit, t0);
        let rate = audio::TARGET_SAMPLE_RATE;
        let quiet = quiet_frame();

        assert!(!watch.observe(t0, &quiet, rate, false));
        assert!(!watch.observe(t0 + Duration::from_millis(1_999), &quiet, rate, false));
        assert!(watch.observe(t0 + Duration::from_secs(2), &quiet, rate, false));

        let mut held = SilenceWatch::new(limit, t0);
        assert!(!held.observe(t0 + Duration::from_secs(10), &quiet, rate, true));
        // Key released as a tap: silence already exceeded, so stop on the next poll.
        assert!(held.observe(t0 + Duration::from_secs(10), &quiet, rate, false));
    }

    #[test]
    fn speech_resets_silence_stop_and_zero_never_stops() {
        let t0 = Instant::now();
        let mut watch = SilenceWatch::new(configured_timeout(2), t0);
        let rate = audio::TARGET_SAMPLE_RATE;

        assert!(!watch.observe(t0 + Duration::from_secs(1), &speech_frame(), rate, false));
        assert!(!watch.observe(t0 + Duration::from_secs(2), &quiet_frame(), rate, false));
        assert!(watch.observe(t0 + Duration::from_secs(3), &quiet_frame(), rate, false));

        let mut off = SilenceWatch::new(configured_timeout(0), t0);
        assert!(!off.observe(t0 + Duration::from_secs(30), &quiet_frame(), rate, false));
    }
}

//! Dictation session: idle → listening → processing → inserted|copied|cancelled|error|undone → idle.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use narad_core::store::{HistoryStore, InsertStatus, NewHistoryEntry};
use narad_dbus::{SessionState, OBJECT_PATH};
use tracing::{info, warn};
use zbus::Connection;

use crate::asr::{self, AsrEngine};
use crate::audio::{self, CaptureHandle};
use crate::config::NaradConfig;
use crate::denoise;
use crate::insert::{self, InsertOutcome};
use crate::ipc::DaemonIpc;

/// HUD outcome signaled on `StateChanged`.
///
/// `Undone` includes the deleted text (FR-INS-5). Inserted, copied, cancelled,
/// and error stay the existing brief outcomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HudOutcome {
    Inserted,
    Copied,
    Cancelled,
    Error,
    Undone(String),
}

impl HudOutcome {
    fn state_name(&self) -> &'static str {
        match self {
            HudOutcome::Inserted => SessionState::INSERTED,
            HudOutcome::Copied => SessionState::COPIED,
            HudOutcome::Cancelled => SessionState::CANCELLED,
            HudOutcome::Error => SessionState::ERROR,
            HudOutcome::Undone(_) => SessionState::UNDONE,
        }
    }

    pub fn deleted_text(&self) -> Option<&str> {
        match self {
            HudOutcome::Undone(text) => Some(text.as_str()),
            _ => None,
        }
    }
}

enum UndoTarget {
    Delete(String),
    Refuse(&'static str),
}

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
    /// Mode captured when this recording started. Menu changes apply next time.
    utterance_mode: String,
    /// Latest app id from `ReportFocus`. Empty or never reported is `None`.
    focused_app: Option<String>,
    /// Focused app when this recording started. Written to history `app_id` (FR-CTX-1).
    app_id_at_start: Option<String>,
    /// Focused app when text was last pasted. In memory for later undo / terminal chord.
    app_id_at_paste: Option<String>,
    /// Final text of the last successful insertion. A copied row does not set this.
    last_inserted: Option<String>,
    /// Latest brief HUD outcome, including deleted text after undo.
    hud_outcome: Option<HudOutcome>,
    /// Deleted text, or the reason an undo was refused. Empty otherwise.
    outcome_detail: String,
}

struct PipelineInner {
    config: NaradConfig,
    state: Mutex<SessionRuntime>,
    store: Mutex<HistoryStore>,
    asr: Mutex<Option<AsrEngine>>,
    conn: OnceLock<Connection>,
    /// Prevent overlapping stop/process tasks.
    processing: AtomicBool,
    /// One file transcription at a time. Dictate start refuses while this is set.
    transcribing: AtomicBool,
    /// `$XDG_DATA_HOME/narad/audio`. Created on the first retained write.
    audio_dir: PathBuf,
}

/// How often uptime retention runs after the startup pass (FR-HIS-7).
const RETENTION_PURGE_EVERY: Duration = Duration::from_secs(24 * 60 * 60);

/// Shared handle used by D-Bus IPC and the hotkeys task.
#[derive(Clone)]
pub struct PipelineHandle {
    inner: Arc<PipelineInner>,
}

impl PipelineHandle {
    pub fn new(
        config: NaradConfig,
        store: HistoryStore,
        asr: Option<AsrEngine>,
        audio_dir: PathBuf,
    ) -> Self {
        Self {
            inner: Arc::new(PipelineInner {
                config: config.clone(),
                state: Mutex::new(SessionRuntime {
                    dbus_state: SessionState::IDLE.to_string(),
                    mode: config.general.active_mode.clone(),
                    utterance_mode: config.general.active_mode.clone(),
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
                    last_inserted: None,
                    hud_outcome: None,
                    outcome_detail: String::new(),
                }),
                store: Mutex::new(store),
                asr: Mutex::new(asr),
                conn: OnceLock::new(),
                processing: AtomicBool::new(false),
                transcribing: AtomicBool::new(false),
                audio_dir,
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

    /// Session mode for the next utterance. An in-progress `utterance_mode` stays
    /// as copied at listen start. Does not rewrite `config.toml`.
    pub fn set_mode(&self, mode: &str) -> Result<(), String> {
        let mode = mode.trim();
        if !matches!(mode, "plain" | "email" | "bullet" | "code") {
            return Err(format!("unknown mode: {mode}"));
        }
        let mut st = self
            .inner
            .state
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        st.mode = mode.to_string();
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

    /// True while a file transcription has not finished. Dictate start must refuse.
    pub fn transcribing(&self) -> bool {
        self.inner.transcribing.load(Ordering::SeqCst)
    }

    pub fn request_start(&self, mode: Option<String>) {
        self.spawn_listen(mode, ListenTrigger::Open);
    }

    /// Undo-last shortcut. One press deletes; release is not a second action.
    pub fn request_undo_last(&self) {
        let handle = self.clone();
        tokio::spawn(async move {
            if let Err(err) = handle.undo_last().await {
                warn!(error = %err, "undo last failed");
                handle.set_outcome_detail("Couldn't undo");
                handle.signal_hud(HudOutcome::Error).await;
                handle.schedule_idle_after(Duration::from_millis(800));
            }
        });
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
        if self.inner.transcribing.load(Ordering::SeqCst) {
            return Err("transcription in progress".into());
        }
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
            st.utterance_mode = st.mode.clone();
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
            (
                capture,
                started,
                st.utterance_mode.clone(),
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
        let retained_samples = self
            .inner
            .config
            .history
            .retain_audio
            .then(|| trimmed.clone());
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

        let Some(final_text) = self.accepted_transcript(&text) else {
            info!(raw = %text, "hallucination filter dropped output");
            self.inner.processing.store(false, Ordering::SeqCst);
            self.set_state(SessionState::IDLE).await;
            return Ok(());
        };
        let terminal = self.focused_is_terminal();
        let outcome = insert::insert_text(&final_text, terminal).await;
        if matches!(outcome, InsertOutcome::Copied) {
            self.set_outcome_detail(insert::paste_hint(terminal));
        }
        let history_id = self.record_dictation(
            duration_ms,
            mode,
            model_id,
            text,
            final_text,
            outcome,
            retained_samples.as_deref().unwrap_or(&[]),
        )?;
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

    /// Hallucination filter, then commands, replacements, and vocabulary (FR-TXT-1..4).
    ///
    /// Email, bullet, and code share this path. Code does not invent punctuation.
    /// `raw` is kept separately for history. No translation and no cleanup.
    /// DRIFT: `docs/ARCHITECTURE.md` shows the hallucination filter after commands.
    fn accepted_transcript(&self, raw: &str) -> Option<String> {
        if asr::is_hallucination(raw) {
            return None;
        }
        let replacements = phrase_table(&self.inner.config.replacements);
        let vocabulary = phrase_table(&self.inner.config.vocabulary);
        Some(crate::text::apply_commands_replacements_then_vocabulary(
            raw,
            &replacements,
            &vocabulary,
        ))
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
        let terminal = self.focused_is_terminal();
        let outcome = insert::insert_text(&text, terminal).await;
        if matches!(outcome, InsertOutcome::Copied) {
            self.set_outcome_detail(insert::paste_hint(terminal));
        }
        if matches!(outcome, InsertOutcome::Inserted) {
            if let Ok(mut st) = self.inner.state.lock() {
                note_paste_focus(&mut st, &text);
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

    /// Delete the last inserted final text when focus is still that app (FR-INS-5).
    ///
    /// A copied history row is not an insertion. Missing paste-time app id, missing
    /// current focus, or a different app sends nothing and signals the HUD error outcome.
    /// Success signals `undone` and includes the deleted text.
    pub async fn undo_last(&self) -> Result<HudOutcome, String> {
        let text = match self.undo_target()? {
            UndoTarget::Delete(text) => text,
            UndoTarget::Refuse(reason) => {
                self.set_outcome_detail(reason);
                self.signal_hud(HudOutcome::Error).await;
                self.schedule_idle_after(Duration::from_millis(800));
                return Ok(HudOutcome::Error);
            }
        };
        let count = text.chars().count();
        if let Err(err) = insert::send_backspaces(count).await {
            warn!(error = %err, "undo backspaces failed");
            self.signal_hud(HudOutcome::Error).await;
            self.schedule_idle_after(Duration::from_millis(800));
            return Ok(HudOutcome::Error);
        }
        {
            let mut st = self
                .inner
                .state
                .lock()
                .map_err(|e| format!("state lock: {e}"))?;
            st.last_inserted = None;
        }
        let outcome = HudOutcome::Undone(text);
        self.signal_hud(outcome.clone()).await;
        self.schedule_idle_after(Duration::from_millis(1200));
        Ok(outcome)
    }

    /// Text to delete, or why undo must not touch the focused field.
    fn undo_target(&self) -> Result<UndoTarget, String> {
        let st = self
            .inner
            .state
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        let Some(text) = st.last_inserted.clone() else {
            return Ok(UndoTarget::Refuse("Nothing to undo"));
        };
        let Some(pasted_in) = st.app_id_at_paste.clone() else {
            return Ok(UndoTarget::Refuse("Focus unknown"));
        };
        let Some(focused) = st.focused_app.clone() else {
            return Ok(UndoTarget::Refuse("Focus unknown"));
        };
        if focused != pasted_in {
            return Ok(UndoTarget::Refuse("App changed"));
        }
        Ok(UndoTarget::Delete(text))
    }

    fn set_outcome_detail(&self, detail: &str) {
        if let Ok(mut st) = self.inner.state.lock() {
            st.outcome_detail = detail.to_string();
        }
    }

    /// Deleted text after a successful undo, or the refuse reason. Empty otherwise.
    pub fn outcome_detail(&self) -> Result<String, String> {
        let st = self
            .inner
            .state
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        Ok(st.outcome_detail.clone())
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
        let terminal = self.focused_is_terminal();
        let outcome = insert::insert_text(&entry.final_text, terminal).await;
        if matches!(outcome, InsertOutcome::Copied) {
            self.set_outcome_detail(insert::paste_hint(terminal));
        }
        if matches!(outcome, InsertOutcome::Inserted) {
            if let Ok(mut st) = self.inner.state.lock() {
                note_paste_focus(&mut st, &entry.final_text);
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

    async fn signal_hud(&self, outcome: HudOutcome) {
        let name = outcome.state_name();
        if let Ok(mut st) = self.inner.state.lock() {
            st.dbus_state = name.to_string();
            st.outcome_detail = match &outcome {
                HudOutcome::Undone(text) => text.clone(),
                HudOutcome::Error => st.outcome_detail.clone(),
                _ => String::new(),
            };
            st.hud_outcome = Some(outcome);
        }
        self.emit_state(name).await;
    }

    async fn set_state(&self, state: &str) {
        if let Ok(mut st) = self.inner.state.lock() {
            st.dbus_state = state.to_string();
            if state == SessionState::IDLE {
                st.outcome_detail.clear();
                st.hud_outcome = None;
            }
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
    /// File transcription uses [`Self::store_file_row`], not this.
    fn record_dictation(
        &self,
        duration_ms: u32,
        mode: String,
        asr_model: String,
        raw_text: String,
        final_text: String,
        outcome: InsertOutcome,
        samples: &[f32],
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
                note_paste_focus(&mut st, &final_text);
            }
            st.paused
        };
        if paused {
            return Ok(None);
        }
        let id = self.commit_dictation(
            duration_ms,
            mode,
            asr_model,
            raw_text,
            final_text,
            insert_status,
        )?;
        let mut store = self
            .inner
            .store
            .lock()
            .map_err(|e| format!("store lock: {e}"))?;
        crate::retained::attach_retained_audio(
            &mut store,
            self.inner.config.history.retain_audio,
            &self.inner.audio_dir,
            id,
            samples,
        )?;
        Ok(Some(id))
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
                note_paste_focus(&mut st, &final_text);
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
                kind: "dictation".into(),
            })
            .map_err(|e| e.to_string())
    }

    /// Transcribe one file into a `kind=file` row. Does not paste or copy.
    ///
    /// Empty `mode` is the session mode at the start of this call. A non-empty
    /// trimmed `dst_lang` is refused. `EntryAdded` is emitted by the D-Bus method.
    pub fn transcribe(&self, path: &str, mode: &str, dst_lang: &str) -> Result<i64, String> {
        if self.inner.transcribing.load(Ordering::SeqCst) {
            return Err("transcription in progress".into());
        }
        if !dst_lang.trim().is_empty() {
            return Err("translation is a Phase 3 feature".into());
        }
        let session = self.snapshot_props()?.1;
        let mode = resolved_mode(mode, &session)?;
        let path = std::path::Path::new(path);
        if !path.is_file() {
            return Err("no file".into());
        }
        let _busy = self.begin_file_transcription()?;
        let model_id = self.loaded_model_id()?;
        let samples = crate::retained::load_pcm_f32(path)?;
        let raw = self.transcribe_loaded(&model_id, &samples)?;
        let Some(final_text) = self.accepted_transcript(&raw) else {
            return Err("hallucination filter dropped output".into());
        };
        self.store_file_row(&mode, &model_id, &raw, &final_text, &samples)
    }

    fn begin_file_transcription(&self) -> Result<FileTranscription, String> {
        if self
            .inner
            .transcribing
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("transcription in progress".into());
        }
        Ok(FileTranscription {
            inner: Arc::clone(&self.inner),
        })
    }

    fn loaded_model_id(&self) -> Result<String, String> {
        let guard = self
            .inner
            .asr
            .lock()
            .map_err(|err| format!("asr lock: {err}"))?;
        match guard.as_ref() {
            Some(engine) => Ok(engine.model_id.clone()),
            None => Err("model is not loaded".into()),
        }
    }

    /// One `kind=file` row. Writes even when history is paused. Does not paste.
    ///
    /// `duration_ms` is `samples.len() / 16`. `app_id` stays empty.
    fn store_file_row(
        &self,
        mode: &str,
        asr_model: &str,
        raw_text: &str,
        final_text: &str,
        samples: &[f32],
    ) -> Result<i64, String> {
        let duration_ms = u32::try_from(samples.len() / 16).unwrap_or(u32::MAX);
        let retain = self.inner.config.history.retain_audio;
        let audio_dir = self.inner.audio_dir.clone();
        self.with_store(|store| {
            let id = store
                .insert(&NewHistoryEntry {
                    duration_ms,
                    mode: mode.to_string(),
                    src_lang: "en".into(),
                    dst_lang: "en".into(),
                    asr_model: asr_model.to_string(),
                    mt_model: None,
                    llm_model: None,
                    app_id: None,
                    raw_text: raw_text.to_string(),
                    final_text: final_text.to_string(),
                    insert_status: InsertStatus::Failed,
                    cleanup_flagged: false,
                    audio_path: None,
                    pinned: false,
                    kind: "file".into(),
                })
                .map_err(|err| err.to_string())?;
            crate::retained::attach_retained_audio(store, retain, &audio_dir, id, samples)?;
            Ok(id)
        })
    }

    /// WAV bytes for a row. Missing row, empty path, missing file, or empty
    /// bytes is an error. This does not play audio.
    pub fn read_audio(&self, id: i64) -> Result<Vec<u8>, String> {
        let path = self.with_store(|store| {
            let row = store
                .get(id)
                .map_err(|err| err.to_string())?
                .ok_or_else(|| format!("no history row {id}"))?;
            match row.audio_path {
                Some(path) if !path.is_empty() => Ok(path),
                _ => Err("no audio".into()),
            }
        })?;
        match std::fs::read(&path) {
            Ok(bytes) if !bytes.is_empty() => Ok(bytes),
            _ => Err("no audio".into()),
        }
    }

    /// New `retranscribe` row from retained audio. Does not paste, does not
    /// write a WAV, and is allowed while history is paused.
    ///
    /// `EntryAdded` is emitted by the D-Bus method after this returns an id.
    pub fn retranscribe(&self, id: i64, mode: &str, model_id: &str) -> Result<i64, String> {
        let source = self.with_store(|store| {
            store
                .get(id)
                .map_err(|err| err.to_string())?
                .ok_or_else(|| format!("no history row {id}"))
        })?;
        let bytes = match source.audio_path.as_deref().filter(|path| !path.is_empty()) {
            Some(path) => match std::fs::read(path) {
                Ok(bytes) if !bytes.is_empty() => bytes,
                _ => return Err("no audio".into()),
            },
            None => return Err("no audio".into()),
        };
        if repeats_source_mode_and_model(&source.mode, &source.asr_model, mode, model_id) {
            return Err("same mode and model".into());
        }
        self.require_loaded_model(model_id)?;
        let pcm = crate::retained::decode_pcm16_wav(&bytes)
            .map_err(|_| "wav is not pcm16 16 khz mono".to_string())?;
        let samples = crate::retained::pcm16_to_f32(&pcm);
        let raw = self.transcribe_loaded(model_id, &samples)?;
        let Some(final_text) = self.accepted_transcript(&raw) else {
            return Err("hallucination filter dropped output".into());
        };
        let duration_ms = u32::try_from(pcm.len() / 16).unwrap_or(u32::MAX);
        self.with_store(|store| {
            store
                .insert(&NewHistoryEntry {
                    duration_ms,
                    mode: mode.to_string(),
                    src_lang: source.src_lang.clone(),
                    dst_lang: source.dst_lang.clone(),
                    asr_model: model_id.to_string(),
                    mt_model: None,
                    llm_model: None,
                    app_id: source.app_id.clone(),
                    raw_text: raw,
                    final_text,
                    insert_status: InsertStatus::Failed,
                    cleanup_flagged: false,
                    audio_path: None,
                    pinned: false,
                    kind: "retranscribe".into(),
                })
                .map_err(|err| err.to_string())
        })
    }

    fn require_loaded_model(&self, model_id: &str) -> Result<(), String> {
        if model_id.is_empty() {
            return Err("model is not loaded".into());
        }
        let guard = self
            .inner
            .asr
            .lock()
            .map_err(|err| format!("asr lock: {err}"))?;
        match guard.as_ref() {
            Some(engine) if engine.model_id == model_id => Ok(()),
            _ => Err("model is not loaded".into()),
        }
    }

    fn transcribe_loaded(&self, model_id: &str, samples: &[f32]) -> Result<String, String> {
        self.require_loaded_model(model_id)?;
        let guard = self
            .inner
            .asr
            .lock()
            .map_err(|err| format!("asr lock: {err}"))?;
        let engine = guard
            .as_ref()
            .ok_or_else(|| "model is not loaded".to_string())?;
        if engine.model_id != model_id {
            return Err("model is not loaded".into());
        }
        engine
            .transcribe(samples, audio::TARGET_SAMPLE_RATE)
            .map_err(|err| err.to_string())
    }
}

/// Empty `requested` is the session mode captured at the start of the call.
/// The stored mode is this result, not a later read of the session mode.
fn resolved_mode(requested: &str, session: &str) -> Result<String, String> {
    let requested = requested.trim();
    let mode = if requested.is_empty() {
        session.trim()
    } else {
        requested
    };
    if !matches!(mode, "plain" | "email" | "bullet" | "code") {
        return Err(format!("unknown mode: {mode}"));
    }
    Ok(mode.to_string())
}

/// Same mode and same model cannot be retranscribed. Missing audio is not decided here.
fn repeats_source_mode_and_model(
    source_mode: &str,
    source_model: &str,
    mode: &str,
    model_id: &str,
) -> bool {
    source_mode == mode && source_model == model_id
}

fn phrase_table(pairs: &[crate::config::Replacement]) -> Vec<crate::text::Replacement> {
    pairs
        .iter()
        .map(|pair| crate::text::Replacement {
            spoken: pair.spoken.clone(),
            written: pair.written.clone(),
        })
        .collect()
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

impl PipelineHandle {
    fn focused_is_terminal(&self) -> bool {
        self.inner
            .state
            .lock()
            .map(|st| insert::uses_terminal_paste(st.focused_app.as_deref()))
            .unwrap_or(false)
    }
}

fn note_paste_focus(st: &mut SessionRuntime, text: &str) {
    st.app_id_at_paste = st.focused_app.clone();
    st.last_inserted = Some(text.to_string());
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

/// Clears the file-transcription flag when the call returns, including on error.
struct FileTranscription {
    inner: Arc<PipelineInner>,
}

impl Drop for FileTranscription {
    fn drop(&mut self) {
        self.inner.transcribing.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use narad_core::store::HistoryStore;
    use tempfile::TempDir;

    #[test]
    fn transcript_filters_hallucinations_then_formats() {
        let mut cfg = NaradConfig::default();
        cfg.replacements.push(crate::config::Replacement {
            spoken: "hi".into(),
            written: "hello".into(),
        });
        cfg.vocabulary.push(crate::config::Replacement {
            spoken: "nar ad".into(),
            written: "Narad".into(),
        });
        let dir = TempDir::new().expect("tempdir");
        let store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let pipe = PipelineHandle::new(cfg, store, None, dir.path().join("audio"));

        assert!(pipe.accepted_transcript("thanks for watching").is_none());
        assert_eq!(
            pipe.accepted_transcript("hi period").as_deref(),
            Some("hello.")
        );
        assert_eq!(pipe.accepted_transcript("nar ad").as_deref(), Some("Narad"));
        assert_eq!(
            pipe.accepted_transcript("camel case user name").as_deref(),
            Some("userName")
        );
        assert_eq!(
            pipe.accepted_transcript("hello world").as_deref(),
            Some("hello world")
        );
    }

    fn test_pipe() -> (TempDir, PipelineHandle) {
        let dir = TempDir::new().expect("tempdir");
        let store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let pipe = PipelineHandle::new(
            NaradConfig::default(),
            store,
            None,
            dir.path().join("audio"),
        );
        (dir, pipe)
    }

    #[test]
    fn file_row_records_kind_duration_and_optional_wav() {
        let samples = [0.5f32; 16];
        let (dir, pipe) = test_pipe();
        let id = pipe
            .store_file_row("bullet", "", "hello file", "hello file", &samples)
            .expect("store");
        let row = pipe
            .with_store(|store| store.get(id).map_err(|err| err.to_string()))
            .expect("get")
            .expect("row");
        assert_eq!(row.kind, "file");
        assert_eq!(row.insert_status.as_str(), "failed");
        assert_eq!(row.duration_ms, 1);
        assert_eq!(row.mode, "bullet");
        assert_eq!(row.final_text, "hello file");
        assert!(row.audio_path.as_deref().unwrap_or("").is_empty());
        assert!(!dir.path().join("audio").join(format!("{id}.wav")).exists());

        let mut cfg = NaradConfig::default();
        cfg.history.retain_audio = true;
        let kept = TempDir::new().expect("tempdir");
        let store = HistoryStore::open(kept.path().join("t.db")).expect("open");
        let pipe = PipelineHandle::new(cfg, store, None, kept.path().join("audio"));
        let id = pipe
            .store_file_row("bullet", "", "hello file", "hello file", &samples)
            .expect("store");
        let row = pipe
            .with_store(|store| store.get(id).map_err(|err| err.to_string()))
            .expect("get")
            .expect("row");
        let bytes = std::fs::read(row.audio_path.expect("audio")).expect("wav");
        let pcm = crate::retained::decode_pcm16_wav(&bytes).expect("decode");
        assert_eq!(pcm[0], 16384);
    }

    #[tokio::test]
    async fn file_transcribe_refuses_translation_and_overlap() {
        let (_dir, pipe) = test_pipe();
        let err = pipe
            .transcribe("/no/such/clip.mp3", "plain", "hi")
            .expect_err("hi");
        assert!(err.contains("translation is a Phase 3 feature"), "{err}");
        assert_eq!(row_count(&pipe), 0);

        pipe.inner.transcribing.store(true, Ordering::SeqCst);
        let err = pipe
            .transcribe("/no/such/clip.mp3", "", "")
            .expect_err("busy");
        assert!(err.contains("transcription in progress"), "{err}");
        assert_eq!(row_count(&pipe), 0);

        let err = pipe
            .begin_listen(None, ListenTrigger::Open)
            .await
            .expect_err("listen");
        assert!(err.contains("transcription in progress"), "{err}");
        let st = pipe.inner.state.lock().expect("state");
        assert!(st.capture.is_none());
        assert_ne!(st.dbus_state, SessionState::LISTENING);
    }

    #[test]
    fn resolved_mode_trims_or_uses_the_session_mode() {
        assert_eq!(resolved_mode("  code ", "plain").expect("code"), "code");
        assert_eq!(resolved_mode("", "bullet").expect("bullet"), "bullet");
        let err = resolved_mode("nope", "plain").expect_err("nope");
        assert!(err.contains("unknown mode: nope"), "{err}");
    }

    #[test]
    fn set_mode_accepts_code_and_rejects_unknown() {
        let (_dir, pipe) = test_pipe();
        pipe.set_mode("code").expect("code");
        assert_eq!(pipe.snapshot_props().expect("props").1, "code");
        pipe.set_mode("  email  ").expect("trimmed");
        assert_eq!(pipe.snapshot_props().expect("props").1, "email");
        let err = pipe.set_mode("nope").expect_err("nope");
        assert!(err.contains("unknown mode: nope"), "{err}");
        assert_eq!(pipe.snapshot_props().expect("props").1, "email");
        let padded = pipe.set_mode("  nope  ").expect_err("padded");
        assert!(padded.contains("unknown mode: nope"), "{padded}");
        assert_eq!(pipe.snapshot_props().expect("props").1, "email");
        assert_eq!(pipe.config().general.active_mode, "plain");
    }

    #[test]
    fn set_mode_while_listening_keeps_the_utterance_mode() {
        let (_dir, pipe) = test_pipe();
        {
            let mut st = pipe.inner.state.lock().expect("state");
            st.dbus_state = SessionState::LISTENING.to_string();
            st.mode = "plain".into();
            st.utterance_mode = "plain".into();
        }
        pipe.set_mode("email").expect("email");
        let st = pipe.inner.state.lock().expect("state");
        assert_eq!(st.utterance_mode, "plain");
        assert_eq!(st.mode, "email");
    }

    #[test]
    fn retained_dictation_stores_the_recognizer_samples() {
        let dir = TempDir::new().expect("tempdir");
        let mut cfg = NaradConfig::default();
        cfg.history.retain_audio = true;
        let store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let audio = dir.path().join("audio");
        let pipe = PipelineHandle::new(cfg, store, None, audio.clone());
        let id = pipe
            .record_dictation(
                12,
                "plain".into(),
                "test".into(),
                "hello".into(),
                "hello".into(),
                InsertOutcome::Inserted,
                &[0.5],
            )
            .expect("record")
            .expect("id");
        let stored = pipe
            .with_store(|store| store.get(id).map_err(|err| err.to_string()))
            .expect("store")
            .expect("row");
        let wav = audio.join(format!("{id}.wav"));
        assert_eq!(stored.audio_path.as_deref(), Some(wav.to_str().unwrap()));
        let bytes = std::fs::read(wav).expect("wav");
        assert_eq!(&bytes[44..], &[0x00, 0x40]);
    }

    #[test]
    fn same_mode_and_model_refuses_and_a_different_mode_allows() {
        assert!(repeats_source_mode_and_model(
            "plain",
            "parakeet-ctc-110m-int8",
            "plain",
            "parakeet-ctc-110m-int8"
        ));
        assert!(!repeats_source_mode_and_model(
            "plain",
            "parakeet-ctc-110m-int8",
            "email",
            "parakeet-ctc-110m-int8"
        ));
    }

    #[test]
    fn retranscribe_without_audio_errors_and_keeps_the_row_count_while_paused() {
        let (_dir, pipe) = test_pipe();
        pipe.set_paused(true).expect("pause");
        let id = insert_hello(&pipe);
        assert_eq!(
            pipe.retranscribe(id, "email", "other-model")
                .expect_err("no audio"),
            "no audio"
        );
        assert_eq!(row_count(&pipe), 1);
    }

    #[test]
    fn retranscribe_same_mode_and_model_adds_no_row() {
        let (dir, pipe) = test_pipe();
        let id = insert_hello(&pipe);
        let audio = dir.path().join("audio");
        let wav = crate::retained::write_retained_wav(&audio, id, &[0.5]).expect("wav");
        let before = std::fs::read(&wav).expect("read");
        pipe.with_store(|store| {
            store
                .set_audio_path(id, wav.to_str().unwrap())
                .map_err(|err| err.to_string())
        })
        .expect("path");
        assert_eq!(
            pipe.retranscribe(id, "plain", "test").expect_err("same"),
            "same mode and model"
        );
        assert_eq!(row_count(&pipe), 1);
        let stored = pipe
            .with_store(|store| store.get(id).map_err(|err| err.to_string()))
            .expect("store")
            .expect("row");
        assert_eq!(stored.kind, "dictation");
        assert_eq!(stored.audio_path.as_deref(), Some(wav.to_str().unwrap()));
        assert_eq!(std::fs::read(&wav).expect("still"), before);
    }

    fn row_count(pipe: &PipelineHandle) -> usize {
        pipe.with_store(|store| Ok(store.list(0, 50).map_err(|err| err.to_string())?.len()))
            .expect("count")
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
            &[],
        )
    }

    #[tokio::test]
    async fn paste_last_picks_newest_final() {
        use std::sync::{Arc, Mutex};

        let (_dir, pipe) = test_pipe();
        let log = Arc::new(Mutex::new(Vec::new()));

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

        insert::with_test_inserts(Arc::clone(&log), pipe.paste_last())
            .await
            .expect("paste newest");
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

    fn count_backspaces(keys: &[i32]) -> usize {
        assert!(keys.iter().all(|key| *key == insert::KEY_BACKSPACE));
        keys.len()
    }

    async fn undo_with_keys(pipe: &PipelineHandle) -> (HudOutcome, Vec<i32>) {
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let outcome = insert::with_test_backspaces(std::sync::Arc::clone(&log), pipe.undo_last())
            .await
            .expect("undo");
        let keys = log.lock().expect("keys").clone();
        (outcome, keys)
    }

    fn commit_text(pipe: &PipelineHandle, text: &str, status: InsertStatus) -> i64 {
        pipe.commit_dictation(
            10,
            "plain".into(),
            "test".into(),
            text.into(),
            text.into(),
            status,
        )
        .expect("commit")
    }

    #[tokio::test]
    async fn undo_last_matching_focus_deletes_each_scalar() {
        let (_dir, pipe) = test_pipe();
        let text = "café";
        pipe.report_focus("org.gnome.TextEditor").expect("focus");
        commit_text(&pipe, text, InsertStatus::Success);

        let (outcome, keys) = undo_with_keys(&pipe).await;
        assert_eq!(outcome, HudOutcome::Undone(text.into()));
        assert_eq!(outcome.deleted_text(), Some(text));
        assert_eq!(pipe.outcome_detail().expect("detail"), text);
        assert_eq!(count_backspaces(&keys), text.chars().count());
        assert_ne!(text.chars().count(), text.len());
        assert_eq!(
            pipe.snapshot_props().expect("props").0,
            SessionState::UNDONE
        );
    }

    #[tokio::test]
    async fn undo_last_refuses_when_focus_app_differs() {
        let (_dir, pipe) = test_pipe();
        pipe.report_focus("org.gnome.TextEditor").expect("focus");
        commit_text(&pipe, "hello", InsertStatus::Success);
        pipe.report_focus("org.gnome.Terminal").expect("other app");

        let outcome = pipe.undo_last().await.expect("refuse");
        assert_eq!(outcome, HudOutcome::Error);
        assert!(outcome.deleted_text().is_none());
        assert_eq!(pipe.outcome_detail().expect("detail"), "App changed");
        assert_eq!(pipe.snapshot_props().expect("props").0, SessionState::ERROR);

        pipe.report_focus("org.gnome.TextEditor").expect("back");
        let (outcome, keys) = undo_with_keys(&pipe).await;
        assert_eq!(outcome.deleted_text(), Some("hello"));
        assert_eq!(count_backspaces(&keys), "hello".chars().count());
    }

    #[tokio::test]
    async fn undo_last_refuses_when_paste_app_id_is_missing() {
        let (_dir, pipe) = test_pipe();
        commit_text(&pipe, "hello", InsertStatus::Success);
        assert!(pipe.inner.state.lock().unwrap().app_id_at_paste.is_none());
        pipe.report_focus("org.gnome.TextEditor")
            .expect("later focus");

        let outcome = pipe.undo_last().await.expect("refuse");
        assert_eq!(outcome, HudOutcome::Error);
        assert_eq!(pipe.outcome_detail().expect("detail"), "Focus unknown");
        assert_eq!(pipe.snapshot_props().expect("props").0, SessionState::ERROR);
    }

    #[tokio::test]
    async fn undo_last_refuses_when_current_focus_is_missing() {
        let (_dir, pipe) = test_pipe();
        pipe.report_focus("org.gnome.TextEditor").expect("focus");
        commit_text(&pipe, "hello", InsertStatus::Success);
        pipe.report_focus("").expect("clear");

        let outcome = pipe.undo_last().await.expect("refuse");
        assert_eq!(outcome, HudOutcome::Error);
        assert_eq!(pipe.outcome_detail().expect("detail"), "Focus unknown");
        assert_eq!(pipe.snapshot_props().expect("props").0, SessionState::ERROR);
    }

    #[tokio::test]
    async fn undo_last_skips_copied_rows() {
        let (_dir, pipe) = test_pipe();
        pipe.report_focus("org.gnome.TextEditor").expect("focus");
        commit_text(&pipe, "only copied", InsertStatus::Skipped);
        let outcome = pipe.undo_last().await.expect("no insertion");
        assert_eq!(outcome, HudOutcome::Error);
        assert_eq!(pipe.outcome_detail().expect("detail"), "Nothing to undo");

        let text = "café";
        commit_text(&pipe, text, InsertStatus::Success);
        commit_text(&pipe, "copied-row", InsertStatus::Skipped);
        let (outcome, keys) = undo_with_keys(&pipe).await;
        assert_eq!(outcome, HudOutcome::Undone(text.into()));
        assert_eq!(count_backspaces(&keys), text.chars().count());
        assert_ne!(count_backspaces(&keys), "copied-row".chars().count());
    }

    #[tokio::test]
    async fn undo_last_undoes_a_successful_paste_last() {
        use std::sync::{Arc, Mutex};

        let (_dir, pipe) = test_pipe();
        let pasted = Arc::new(Mutex::new(Vec::new()));

        pipe.report_focus("org.gnome.TextEditor").expect("focus");
        commit_text(&pipe, "old", InsertStatus::Success);
        commit_text(&pipe, "néw", InsertStatus::Skipped);
        insert::with_test_inserts(Arc::clone(&pasted), pipe.paste_last())
            .await
            .expect("paste");
        assert_eq!(pasted.lock().expect("pasted").as_slice(), ["néw"]);

        let (outcome, keys) = undo_with_keys(&pipe).await;
        assert_eq!(outcome, HudOutcome::Undone("néw".into()));
        assert_eq!(count_backspaces(&keys), "néw".chars().count());
        assert_ne!("néw".chars().count(), "néw".len());
        assert_eq!(pasted.lock().expect("pasted").as_slice(), ["néw"]);
        assert_eq!(
            pipe.snapshot_props().expect("props").0,
            SessionState::UNDONE
        );
    }

    #[tokio::test]
    async fn copied_paste_hint_follows_the_focused_app() {
        use std::sync::{Arc, Mutex};

        let (_dir, pipe) = test_pipe();
        commit_text(&pipe, "hello", InsertStatus::Skipped);
        let log = Arc::new(Mutex::new(Vec::new()));
        let chords = Arc::new(Mutex::new(Vec::new()));

        pipe.report_focus("org.gnome.Terminal").expect("terminal");
        insert::with_test_copied_inserts(Arc::clone(&log), Arc::clone(&chords), pipe.paste_last())
            .await
            .expect("terminal copy");
        assert_eq!(log.lock().expect("log").as_slice(), ["hello"]);
        assert_eq!(chords.lock().expect("chords").as_slice(), [true]);
        assert_eq!(pipe.outcome_detail().expect("detail"), "Ctrl+Shift+V");
        assert_eq!(
            pipe.snapshot_props().expect("props").0,
            SessionState::COPIED
        );

        pipe.report_focus("kgx").expect("console");
        insert::with_test_copied_inserts(Arc::clone(&log), Arc::clone(&chords), pipe.paste_last())
            .await
            .expect("console copy");

        pipe.report_focus("org.gnome.gedit").expect("editor");
        insert::with_test_copied_inserts(Arc::clone(&log), Arc::clone(&chords), pipe.paste_last())
            .await
            .expect("editor copy");
        assert_eq!(
            chords.lock().expect("chords").as_slice(),
            [true, true, false]
        );
        assert_eq!(pipe.outcome_detail().expect("detail"), "Ctrl+V");
        assert_eq!(
            pipe.snapshot_props().expect("props").0,
            SessionState::COPIED
        );
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
                &[],
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

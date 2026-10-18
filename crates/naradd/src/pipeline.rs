//! Dictation session: idle → listening → processing → inserted|copied|cancelled|error|undone → idle.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use narad_core::store::{HistoryStore, InsertStatus, NewHistoryEntry};
use narad_dbus::{SessionState, OBJECT_PATH};
use tracing::{debug, info, trace, warn};
use zbus::Connection;

use crate::asr::{self, AsrEngine};
use crate::audio::{self, CaptureHandle};
use crate::config::NaradConfig;
use crate::denoise;
use crate::insert::{self, InsertOutcome};
use crate::ipc::DaemonIpc;
use crate::wakeword::{WakeWordDetection, WakeWordSpotter};

/// HUD outcome signaled on `StateChanged`.
///
/// `Undone` includes the deleted text (FR-INS-5). Inserted, copied, cancelled,
/// and error stay the existing brief outcomes.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
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
    /// User manually called set_mode during the current focus context (FR-CTX-2).
    manual_mode_override: bool,
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
    /// Active selection text captured when selection command started (FR-INS-6).
    selection_context: Option<String>,
}

struct PipelineInner {
    config: NaradConfig,
    state: Mutex<SessionRuntime>,
    store: Mutex<HistoryStore>,
    asr: Mutex<Option<AsrEngine>>,
    online_asr: Mutex<Option<crate::asr::OnlineAsrEngine>>,
    wake_spotter: Mutex<Option<WakeWordSpotter>>,
    last_activity: Mutex<Instant>,
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
        let wake_spotter = if config.activation.wake_word_enabled {
            Some(WakeWordSpotter::new(
                &config.activation.wake_word_phrase,
                crate::wakeword::DEFAULT_CONFIDENCE_THRESHOLD,
            ))
        } else {
            None
        };
        let online_asr = crate::asr::try_load_online(crate::asr::DEFAULT_ONLINE_ASR_ID);
        Self {
            inner: Arc::new(PipelineInner {
                config: config.clone(),
                state: Mutex::new(SessionRuntime {
                    dbus_state: SessionState::IDLE.to_string(),
                    mode: config.general.active_mode.clone(),
                    manual_mode_override: false,
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
                    selection_context: None,
                }),
                store: Mutex::new(store),
                asr: Mutex::new(asr),
                online_asr: Mutex::new(online_asr),
                wake_spotter: Mutex::new(wake_spotter),
                last_activity: Mutex::new(Instant::now()),
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
        let active_mode = if st.dbus_state == SessionState::LISTENING || st.dbus_state == SessionState::PROCESSING {
            st.utterance_mode.clone()
        } else {
            st.mode.clone()
        };
        Ok((
            st.dbus_state.clone(),
            active_mode,
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
        if !matches!(mode, "plain" | "clean" | "email" | "bullet" | "code" | "selection") {
            return Err(format!("unknown mode: {mode}"));
        }
        let mut st = self
            .inner
            .state
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        st.mode = mode.to_string();
        st.manual_mode_override = true;
        Ok(())
    }

    /// Cache the focused app id and switch mode to matching profile (FR-CTX-2).
    /// Empty string clears it (`None`).
    pub fn report_focus(&self, app_id: &str) -> Result<(), String> {
        let mut st = self
            .inner
            .state
            .lock()
            .map_err(|e| format!("state lock: {e}"))?;
        let norm = normalize_app_id(app_id);
        if norm != st.focused_app {
            st.manual_mode_override = false;
            if let Some(app) = norm.as_deref() {
                if let Some(prof) = self.inner.config.find_profile(app) {
                    st.mode = prof.mode.clone();
                } else {
                    st.mode = self.inner.config.general.active_mode.clone();
                }
            } else {
                st.mode = self.inner.config.general.active_mode.clone();
            }
            st.focused_app = norm;
        }
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

    /// Feed audio stream in idle for wake word detection (FR-ACT-6).
    ///
    /// If wake word detection is disabled or the session is not idle, returns `None`.
    /// When the configured wake phrase is spotted in idle audio:
    /// - Logs `wake word detected, starting dictation hands-free`
    /// - Automatically starts dictation session in current mode
    /// - Returns `Some(WakeWordDetection)`
    pub fn feed_idle_pcm(
        &self,
        samples: &[f32],
        sample_rate: u32,
    ) -> Option<WakeWordDetection> {
        if !self.inner.config.activation.wake_word_enabled {
            return None;
        }

        let mode = {
            let st = self.inner.state.lock().ok()?;
            if st.dbus_state != SessionState::IDLE {
                return None;
            }
            st.mode.clone()
        };

        let mut spotter_guard = self.inner.wake_spotter.lock().ok()?;
        let spotter = spotter_guard.as_mut()?;
        let detection = spotter.process_pcm(samples, sample_rate)?;

        trace!("wake word detected, starting dictation hands-free");
        info!(
            phrase = %detection.phrase,
            confidence = format!("{:.3}", detection.confidence),
            "wake word detected, starting dictation hands-free"
        );

        self.request_start(Some(mode));

        Some(detection)
    }

    /// Process audio stream in idle for wake word spotting (alias for `feed_idle_pcm`).
    #[allow(dead_code)]
    pub fn process_idle_audio(
        &self,
        samples: &[f32],
        sample_rate: u32,
    ) -> Option<WakeWordDetection> {
        self.feed_idle_pcm(samples, sample_rate)
    }

    /// Check if wake word engine is enabled in this pipeline.
    #[allow(dead_code)]
    pub fn is_wake_word_enabled(&self) -> bool {
        self.inner.config.activation.wake_word_enabled
    }

    /// Spawn continuous microphone capture tap in idle when wake word is enabled (FR-AUD-1, FR-ACT-6).
    #[allow(dead_code)]
    pub fn spawn_idle_audio_tap(&self) -> Option<tokio::task::JoinHandle<()>> {
        if !self.inner.config.activation.wake_word_enabled {
            return None;
        }
        let handle = self.clone();
        Some(tokio::spawn(async move {
            handle.run_idle_audio_tap().await;
        }))
    }

    #[allow(dead_code)]
    async fn run_idle_audio_tap(&self) {
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        let mut active_capture: Option<CaptureHandle> = None;
        let mut scanned = 0;

        loop {
            tick.tick().await;

            let is_idle = self
                .inner
                .state
                .lock()
                .map(|st| st.dbus_state == SessionState::IDLE)
                .unwrap_or(false);

            if !is_idle {
                if active_capture.is_some() {
                    active_capture.take();
                    scanned = 0;
                }
                continue;
            }

            // In idle: ensure capture is running
            if active_capture.is_none() {
                match CaptureHandle::start(&self.inner.config.audio.input_target) {
                    Ok(cap) => {
                        active_capture = Some(cap);
                        scanned = 0;
                    }
                    Err(err) => {
                        warn!(error = %err, "idle audio tap failed to start capture");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                }
            }

            if let Some(ref capture) = active_capture {
                let (new_samples, new_scanned) = capture.copy_from(scanned);
                scanned = new_scanned;
                if !new_samples.is_empty()
                    && self
                        .feed_idle_pcm(&new_samples, audio::TARGET_SAMPLE_RATE as u32)
                        .is_some()
                {
                    // Wake word detected; release idle capture handle so begin_listen can capture.
                    active_capture.take();
                    scanned = 0;
                }
            }
        }
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

    /// Explicit Stop & Insert for streaming dictation.
    pub fn request_stop_and_insert(&self) {
        self.request_stop();
    }

    pub fn request_cancel(&self) {
        let handle = self.clone();
        tokio::spawn(async move {
            handle.cancel_listen().await;
        });
    }

    /// Discard current streaming dictation without inserting or saving to history.
    pub fn request_discard(&self) {
        self.request_cancel();
    }

    pub fn request_toggle(&self, mode: Option<String>) {
        if self.is_listening() {
            self.request_stop();
        } else {
            self.request_start(mode);
        }
    }

    /// Selection command mode (FR-INS-6). Captures primary selection and listens for instruction.
    pub fn request_selection_command(&self) {
        let selection = insert::read_primary_selection();
        match selection {
            None => {
                self.set_outcome_detail("No text selected");
                let handle = self.clone();
                tokio::spawn(async move {
                    handle.signal_hud(HudOutcome::Error).await;
                    handle.schedule_idle_after(Duration::from_millis(800));
                });
            }
            Some(text) => {
                if let Ok(mut st) = self.inner.state.lock() {
                    st.selection_context = Some(text);
                }
                self.spawn_listen(Some("selection".to_string()), ListenTrigger::Open);
            }
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
                    if m == "selection" {
                        st.utterance_mode = m;
                    } else {
                        st.mode = m;
                        st.utterance_mode = st.mode.clone();
                    }
                }
            } else {
                st.utterance_mode = st.mode.clone();
            }
            st.cancel.store(false, Ordering::SeqCst);
            snapshot_focus_at_start(&mut st);
        }

        if let Ok(mut g) = self.inner.wake_spotter.lock() {
            if let Some(spotter) = g.as_mut() {
                spotter.reset();
            }
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
        self.spawn_streaming_pump(listen_gen);
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
    /// In continuous mode, applies unattended_silence_secs as a safety failsafe.
    /// `0` disables this timer only.
    fn spawn_silence_timer(&self, listen_gen: u64) {
        let continuous = self.inner.config.audio.continuous_mode;
        let secs = if continuous {
            self.inner.config.audio.unattended_silence_secs
        } else {
            self.inner.config.audio.auto_stop_silence_secs
        };
        let Some(limit) = configured_timeout(secs) else {
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
                        continuous,
                        "silence threshold reached; auto-stop"
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

    /// Pumps audio frames incrementally into OnlineAsrStream and emits PartialTranscript.
    fn spawn_streaming_pump(&self, listen_gen: u64) {
        let handle = self.clone();
        tokio::spawn(async move {
            let mut scanned = 0;
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(50));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let stream_slot = {
                let asr_guard = handle.inner.online_asr.lock().ok();
                asr_guard
                    .as_ref()
                    .and_then(|g| g.as_ref().and_then(|engine| engine.create_stream().ok()))
            };
            let Some(mut stream) = stream_slot else {
                return;
            };
            loop {
                tick.tick().await;
                if !handle.is_listening() {
                    break;
                }
                let Some(poll) = handle.silence_poll(listen_gen, scanned) else {
                    break;
                };
                scanned = poll.end;
                if !poll.fresh.is_empty() {
                    stream.accept_waveform(audio::TARGET_SAMPLE_RATE as u32, &poll.fresh);
                    let partial = {
                        let asr_guard = handle.inner.online_asr.lock().ok();
                        asr_guard.as_ref().and_then(|g| {
                            g.as_ref().and_then(|engine| stream.decode(engine.recognizer()))
                        })
                    };
                    if let Some(partial) = partial {
                        if !partial.is_empty() {
                            handle.emit_partial_transcript(&partial).await;
                        }
                    }
                }
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
            st.selection_context = None;
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

        let (capture, started, mode, cancel_flag, selection_context) = {
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
            let selection_context = st.selection_context.take();
            st.awaiting_hold_release = false;
            st.dbus_state = SessionState::PROCESSING.to_string();
            (
                capture,
                started,
                st.utterance_mode.clone(),
                st.cancel.clone(),
                selection_context,
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
        if samples_16k.len() < audio::TARGET_SAMPLE_RATE as usize / 10 {
            info!("recording too short; returning to idle");
            self.inner.processing.store(false, Ordering::SeqCst);
            self.set_state(SessionState::IDLE).await;
            return Ok(());
        }

        let model_id = self.inner.config.models.asr_english.clone();
        let retained_samples = self
            .inner
            .config
            .history
            .retain_audio
            .then(|| samples_16k.clone());
        // Run decode on a blocking pool; lock only inside the worker (Standards #5).
        let asr_slot = Arc::clone(&self.inner);
        let target_model_id = model_id.clone();
        let decode_samples = samples_16k.clone();
        let transcribed = tokio::task::spawn_blocking(move || {
            let mut asr_guard = asr_slot.asr.lock().map_err(|e| format!("asr lock: {e}"))?;
            let needs_reload = match asr_guard.as_ref() {
                Some(engine) => engine.model_id != target_model_id,
                None => true,
            };
            if needs_reload {
                match AsrEngine::load(&target_model_id) {
                    Ok(engine) => {
                        info!(model_id = %target_model_id, "dynamically loaded configured ASR model");
                        *asr_guard = Some(engine);
                    }
                    Err(e) => {
                        warn!(error = %e, model_id = %target_model_id, "failed to load configured ASR model; clearing resident model");
                        *asr_guard = None;
                    }
                }
            }
            if let Ok(mut last) = asr_slot.last_activity.lock() {
                *last = Instant::now();
            }
            match asr_guard.as_ref() {
                Some(engine) => engine
                    .transcribe(&decode_samples, audio::TARGET_SAMPLE_RATE)
                    .map_err(|e| e.to_string())
                    .map(|t| Some((t, target_model_id))),
                None => {
                    // Fall back to online_asr if offline asr is not available
                    if let Ok(guard) = asr_slot.online_asr.lock() {
                        if let Some(ref online) = *guard {
                            match online.transcribe(&decode_samples, audio::TARGET_SAMPLE_RATE) {
                                Ok(text) => return Ok(Some((text, online.model_id.clone()))),
                                Err(e) => warn!(error = %e, "fallback online asr transcribe failed"),
                            }
                        }
                    }
                    Ok(None)
                }
            }
        })
        .await
        .map_err(|e| format!("asr join: {e}"))?;

        let (text, effective_model_id) = match transcribed {
            Ok(Some((t, m))) => (t, m),
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

        if text.trim().is_empty() {
            info!("ASR decoded empty transcript; returning to idle");
            self.inner.processing.store(false, Ordering::SeqCst);
            self.set_state(SessionState::IDLE).await;
            return Ok(());
        }

        if cancel_flag.load(Ordering::SeqCst) {
            self.inner.processing.store(false, Ordering::SeqCst);
            self.set_state(SessionState::CANCELLED).await;
            self.schedule_idle_after(Duration::from_millis(1200));
            return Ok(());
        }

        let had_selection = selection_context.is_some();
        let (effective_mode, raw_text_for_history) = match crate::cleanup::extract_spoken_mode_prefix(&text) {
            Some((spoken_mode, _)) => (spoken_mode.to_string(), text.clone()),
            None => (mode.clone(), text.clone()),
        };

        let Some(cleanup_res) = self.accepted_transcript_with_selection(&text, &mode, selection_context) else {
            info!(raw = %text, "hallucination filter dropped output");
            self.inner.processing.store(false, Ordering::SeqCst);
            self.set_state(SessionState::IDLE).await;
            return Ok(());
        };
        let terminal = self.focused_is_terminal();
        let outcome = insert::insert_text(&cleanup_res.text, terminal).await;
        if matches!(outcome, InsertOutcome::Copied) {
            self.set_outcome_detail(insert::paste_hint(terminal));
        }
        let kind = if had_selection {
            "selection_command"
        } else {
            "dictation"
        };
        let history_id = self.record_entry(
            kind,
            duration_ms,
            effective_mode,
            effective_model_id,
            raw_text_for_history,
            cleanup_res.text,
            cleanup_res.flagged,
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

    /// Hallucination filter, commands, replacements, vocabulary, and mode cleanup (FR-TXT-1..4, 6..8, FR-ACT-5, FR-INS-6).
    fn accepted_transcript(&self, raw: &str, mode: &str) -> Option<crate::cleanup::CleanupResult> {
        let selection_ctx = self
            .inner
            .state
            .lock()
            .ok()
            .and_then(|mut st| st.selection_context.take());
        self.accepted_transcript_with_selection(raw, mode, selection_ctx)
    }

    fn accepted_transcript_with_selection(
        &self,
        raw: &str,
        mode: &str,
        selection_ctx: Option<String>,
    ) -> Option<crate::cleanup::CleanupResult> {
        if asr::is_hallucination(raw) {
            return None;
        }

        let (effective_mode, effective_raw) = match crate::cleanup::extract_spoken_mode_prefix(raw) {
            Some((spoken_mode, stripped)) => {
                info!(spoken_mode, original_mode = mode, "spoken mode prefix matched");
                (spoken_mode, stripped)
            }
            None => (mode, raw),
        };

        if let Some(selected) = selection_ctx {
            let final_text = crate::cleanup::apply_selection_command(&selected, effective_raw);
            return Some(crate::cleanup::CleanupResult {
                text: final_text,
                flagged: false,
                applied: true,
            });
        }

        let replacements = phrase_table(&self.inner.config.replacements);
        let vocabulary = phrase_table(&self.inner.config.vocabulary);
        let pre_cleanup = crate::text::apply_commands_replacements_then_vocabulary(
            effective_raw,
            &replacements,
            &vocabulary,
        );
        Some(crate::cleanup::apply_cleanup(&pre_cleanup, effective_mode))
    }

    /// Finishes transcript processing without audio recording (used in tests and commands).
    pub async fn finish_transcript(&self, raw: &str) -> Result<Option<i64>, String> {
        let (mode, selection_context) = {
            let mut st = self
                .inner
                .state
                .lock()
                .map_err(|e| format!("state lock: {e}"))?;
            let sel = st.selection_context.take();
            (st.mode.clone(), sel)
        };
        let had_selection = selection_context.is_some();
        let (effective_mode, raw_text_for_history) = match crate::cleanup::extract_spoken_mode_prefix(raw) {
            Some((spoken_mode, _)) => (spoken_mode.to_string(), raw.to_string()),
            None => (mode.clone(), raw.to_string()),
        };

        let Some(cleanup_res) = self.accepted_transcript_with_selection(raw, &mode, selection_context) else {
            self.set_state(SessionState::IDLE).await;
            return Ok(None);
        };
        let terminal = self.focused_is_terminal();
        let outcome = insert::insert_text(&cleanup_res.text, terminal).await;
        if matches!(outcome, InsertOutcome::Copied) {
            self.set_outcome_detail(insert::paste_hint(terminal));
        }
        let kind = if had_selection {
            "selection_command"
        } else {
            "dictation"
        };
        let history_id = self.record_entry(
            kind,
            100,
            effective_mode,
            "test".into(),
            raw_text_for_history,
            cleanup_res.text,
            cleanup_res.flagged,
            outcome,
            &[],
        )?;
        if let Some(id) = history_id {
            self.emit_entry_added(id).await;
        }
        match outcome {
            InsertOutcome::Inserted => {
                self.set_state(SessionState::INSERTED).await;
                self.schedule_idle_after(Duration::from_millis(1200));
            }
            InsertOutcome::Copied => {
                self.set_state(SessionState::COPIED).await;
                self.schedule_idle_after(Duration::from_secs(2));
            }
        }
        Ok(history_id)
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
        if state == SessionState::IDLE {
            if let Ok(mut g) = self.inner.wake_spotter.lock() {
                if let Some(spotter) = g.as_mut() {
                    spotter.reset();
                }
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

    /// Emits `PartialTranscript` signal to active D-Bus subscribers.
    pub async fn emit_partial_transcript(&self, text: &str) {
        let Some(conn) = self.inner.conn.get() else {
            return;
        };
        if let Ok(iface) = conn
            .object_server()
            .interface::<_, DaemonIpc>(OBJECT_PATH)
            .await
        {
            if let Err(err) = DaemonIpc::emit_partial_transcript(iface.signal_emitter(), text).await {
                warn!(error = %err, "emit PartialTranscript");
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
        cleanup_flagged: bool,
        outcome: InsertOutcome,
        samples: &[f32],
    ) -> Result<Option<i64>, String> {
        self.record_entry(
            "dictation",
            duration_ms,
            mode,
            asr_model,
            raw_text,
            final_text,
            cleanup_flagged,
            outcome,
            samples,
        )
    }

    fn record_entry(
        &self,
        kind: &str,
        duration_ms: u32,
        mode: String,
        asr_model: String,
        raw_text: String,
        final_text: String,
        cleanup_flagged: bool,
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
        let id = self.commit_entry(
            kind,
            duration_ms,
            mode,
            asr_model,
            raw_text,
            final_text,
            cleanup_flagged,
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

    fn commit_entry(
        &self,
        kind: &str,
        duration_ms: u32,
        mode: String,
        asr_model: String,
        raw_text: String,
        final_text: String,
        cleanup_flagged: bool,
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
                cleanup_flagged,
                audio_path: None,
                pinned: false,
                kind: kind.to_string(),
            })
            .map_err(|e| e.to_string())
    }

    fn commit_dictation(
        &self,
        duration_ms: u32,
        mode: String,
        asr_model: String,
        raw_text: String,
        final_text: String,
        cleanup_flagged: bool,
        insert_status: InsertStatus,
    ) -> Result<i64, String> {
        self.commit_entry(
            "dictation",
            duration_ms,
            mode,
            asr_model,
            raw_text,
            final_text,
            cleanup_flagged,
            insert_status,
        )
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
        let Some(cleanup_res) = self.accepted_transcript(&raw, &mode) else {
            return Err("hallucination filter dropped output".into());
        };
        self.store_file_row(&mode, &model_id, &raw, &cleanup_res.text, cleanup_res.flagged, &samples)
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

    pub fn is_asr_loaded(&self) -> bool {
        self.inner
            .asr
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    pub fn loaded_model_id(&self) -> Result<String, String> {
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

    /// Check and unload idle ASR model (FR-MDL-5).
    pub fn check_idle_unload(&self) {
        let Some(mins) = self.inner.config.models.idle_unload_minutes else {
            return;
        };
        if mins == 0 {
            return;
        }
        let threshold = Duration::from_secs(mins as u64 * 60);
        let elapsed = self
            .inner
            .last_activity
            .lock()
            .map(|t| t.elapsed())
            .unwrap_or(Duration::ZERO);
        if elapsed >= threshold {
            if let Ok(mut asr) = self.inner.asr.lock() {
                if asr.is_some() {
                    info!(mins, "unloading idle ASR model (FR-MDL-5)");
                    *asr = None;
                }
            }
        }
    }

    #[cfg(test)]
    pub fn set_last_activity(&self, when: Instant) {
        if let Ok(mut last) = self.inner.last_activity.lock() {
            *last = when;
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
        cleanup_flagged: bool,
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
                    cleanup_flagged,
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
        let Some(cleanup_res) = self.accepted_transcript(&raw, mode) else {
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
                    final_text: cleanup_res.text,
                    insert_status: InsertStatus::Failed,
                    cleanup_flagged: cleanup_res.flagged,
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
    if !matches!(mode, "plain" | "clean" | "email" | "bullet" | "code") {
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
            .map(|st| {
                if let Some(app) = st.focused_app.as_deref() {
                    if let Some(profile) = self.inner.config.find_profile(app) {
                        if let Some(paste_with_shift) = profile.paste_with_shift {
                            return paste_with_shift;
                        }
                    }
                }
                insert::uses_terminal_paste(st.focused_app.as_deref())
            })
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

        assert!(pipe.accepted_transcript("thanks for watching", "plain").is_none());
        assert_eq!(
            pipe.accepted_transcript("hi period", "plain").map(|r| r.text).as_deref(),
            Some("hello.")
        );
        assert_eq!(
            pipe.accepted_transcript("nar ad", "plain").map(|r| r.text).as_deref(),
            Some("Narad")
        );
        assert_eq!(
            pipe.accepted_transcript("camel case user name", "plain").map(|r| r.text).as_deref(),
            Some("userName")
        );
        assert_eq!(
            pipe.accepted_transcript("hello world", "plain").map(|r| r.text).as_deref(),
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
            .store_file_row("bullet", "", "hello file", "hello file", false, &samples)
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
            .store_file_row("bullet", "", "hello file", "hello file", false, &samples)
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
                false,
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
            false,
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
            false,
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
                false,
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
                false,
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
            false,
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
            false,
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
                false,
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

    #[test]
    fn continuous_mode_unattended_silence_timeout_and_pause_behavior() {
        let t0 = Instant::now();
        // In continuous mode, limit is 30s unattended failsafe
        let mut continuous_watch = SilenceWatch::new(configured_timeout(30), t0);
        let rate = audio::TARGET_SAMPLE_RATE;
        let quiet = quiet_frame();

        // 2s pause does NOT trigger cutoff in continuous mode
        assert!(!continuous_watch.observe(t0 + Duration::from_secs(2), &quiet, rate, false));
        // 10s thinking pause does NOT trigger cutoff
        assert!(!continuous_watch.observe(t0 + Duration::from_secs(10), &quiet, rate, false));
        // 29s pause does NOT trigger cutoff
        assert!(!continuous_watch.observe(t0 + Duration::from_secs(29), &quiet, rate, false));
        // 30s unattended silence triggers safety cutoff
        assert!(continuous_watch.observe(t0 + Duration::from_secs(30), &quiet, rate, false));
    }

    #[test]
    fn idle_unloading_configuration_and_behavior() {
        let (dir, pipe) = test_pipe();
        // Default: idle_unload_minutes is None, should not unload
        assert_eq!(pipe.config().models.idle_unload_minutes, None);
        pipe.check_idle_unload();

        // Configure idle_unload_minutes = Some(5)
        let mut cfg = NaradConfig::default();
        cfg.models.idle_unload_minutes = Some(5);
        let store = HistoryStore::open(dir.path().join("t2.db")).expect("open");
        let pipe = PipelineHandle::new(cfg.clone(), store, None, dir.path().join("audio2"));

        // If not loaded, check_idle_unload does not fail
        pipe.check_idle_unload();
        assert!(!pipe.is_asr_loaded());

        // When activity is fresh, check_idle_unload does not unload
        pipe.set_last_activity(Instant::now());
        pipe.check_idle_unload();

        // When activity elapsed >= 5 mins, check_idle_unload triggers unload
        let past = Instant::now()
            .checked_sub(Duration::from_secs(301))
            .unwrap_or(Instant::now());
        pipe.set_last_activity(past);
        pipe.check_idle_unload();
        assert!(!pipe.is_asr_loaded());

        // When engine is loaded, verify unload transition
        if let Some(engine) = asr::try_load(asr::DEFAULT_ASR_ID) {
            let store = HistoryStore::open(dir.path().join("t3.db")).expect("open");
            let loaded_pipe = PipelineHandle::new(cfg, store, Some(engine), dir.path().join("audio3"));
            assert!(loaded_pipe.is_asr_loaded());

            // Fresh activity: still loaded
            loaded_pipe.set_last_activity(Instant::now());
            loaded_pipe.check_idle_unload();
            assert!(loaded_pipe.is_asr_loaded());

            // Expired activity (5 mins elapsed): unloaded
            let past = Instant::now()
                .checked_sub(Duration::from_secs(301))
                .unwrap_or(Instant::now());
            loaded_pipe.set_last_activity(past);
            loaded_pipe.check_idle_unload();
            assert!(!loaded_pipe.is_asr_loaded());
        }
    }

    #[test]
    fn plain_mode_leaves_text_unchanged_and_cleanup_flagged_false() {
        let (_dir, pipe) = test_pipe();
        let raw = "um I think basically we should talk";
        let res = pipe.accepted_transcript(raw, "plain").expect("transcript");
        assert_eq!(res.text, raw);
        assert!(!res.applied);
        assert!(!res.flagged);

        let id = pipe
            .record_dictation(
                10,
                "plain".into(),
                "test".into(),
                raw.into(),
                res.text,
                res.flagged,
                InsertOutcome::Inserted,
                &[],
            )
            .expect("record")
            .expect("id");

        let entry = pipe
            .with_store(|s| s.get(id).map_err(|e| e.to_string()))
            .expect("store")
            .expect("entry");
        assert_eq!(entry.final_text, raw);
        assert_eq!(entry.mode, "plain");
        assert!(!entry.cleanup_flagged);
    }

    #[test]
    fn clean_mode_applies_heuristics_and_not_flagged() {
        let (_dir, pipe) = test_pipe();
        let raw = "um I think basically we should talk";
        let res = pipe.accepted_transcript(raw, "clean").expect("transcript");
        assert_eq!(res.text, "I think we should talk.");
        assert!(res.applied);
        assert!(!res.flagged);

        let id = pipe
            .record_dictation(
                10,
                "clean".into(),
                "test".into(),
                raw.into(),
                res.text,
                res.flagged,
                InsertOutcome::Inserted,
                &[],
            )
            .expect("record")
            .expect("id");

        let entry = pipe
            .with_store(|s| s.get(id).map_err(|e| e.to_string()))
            .expect("store")
            .expect("entry");
        assert_eq!(entry.final_text, "I think we should talk.");
        assert_eq!(entry.mode, "clean");
        assert!(!entry.cleanup_flagged);
    }

    #[test]
    fn ungrounded_cleanup_falls_back_to_pre_cleanup_and_records_cleanup_flagged_true() {
        let (_dir, pipe) = test_pipe();
        let raw = "The quick brown fox jumps over the lazy dog in the sunny park today.";
        let hallucinated =
            "The quick brown cat leaps above the sleepy wolf in the dark forest tonight.";

        // Grounding guard validates candidate against pre-cleanup text
        let guard = crate::cleanup::GroundingGuard::default();
        let cleanup_res = guard.validate(raw, hallucinated, "clean");
        assert_eq!(cleanup_res.text, raw); // Falls back to pre-cleanup text
        assert!(cleanup_res.flagged);
        assert!(!cleanup_res.applied);

        // Record in dictation history
        let id = pipe
            .record_dictation(
                15,
                "clean".into(),
                "test".into(),
                raw.into(),
                cleanup_res.text.clone(),
                cleanup_res.flagged,
                InsertOutcome::Inserted,
                &[],
            )
            .expect("record")
            .expect("id");

        let entry = pipe
            .with_store(|s| s.get(id).map_err(|e| e.to_string()))
            .expect("store")
            .expect("entry");
        assert_eq!(entry.final_text, raw); // pre-cleanup text restored
        assert!(entry.cleanup_flagged);

        // Also test file transcription store_file_row with ungrounded flagged in email mode
        let email_raw = "Short update.";
        let email_bloated = "Short update. ".repeat(30);
        let email_res = guard.validate(email_raw, &email_bloated, "email");
        assert_eq!(email_res.text, email_raw);
        assert!(email_res.flagged);

        let file_id = pipe
            .store_file_row(
                "email",
                "test",
                email_raw,
                &email_res.text,
                email_res.flagged,
                &[],
            )
            .expect("store file row");
        let file_entry = pipe
            .with_store(|s| s.get(file_id).map_err(|e| e.to_string()))
            .expect("store")
            .expect("entry");
        assert_eq!(file_entry.final_text, email_raw);
        assert!(file_entry.cleanup_flagged);
    }

    fn test_pipe_with_config(cfg: NaradConfig) -> (TempDir, PipelineHandle) {
        let dir = TempDir::new().expect("tempdir");
        let store = HistoryStore::open(dir.path().join("t.db")).expect("open");
        let pipe = PipelineHandle::new(
            cfg,
            store,
            None,
            dir.path().join("audio"),
        );
        (dir, pipe)
    }

    #[test]
    fn focus_mode_switching_and_manual_override() {
        let mut cfg = NaradConfig::default();
        cfg.general.active_mode = "plain".to_string();
        cfg.profiles.push(crate::config::AppProfile {
            app_id: "code".into(),
            mode: "code".into(),
            paste_with_shift: None,
        });
        cfg.profiles.push(crate::config::AppProfile {
            app_id: "slack".into(),
            mode: "bullet".into(),
            paste_with_shift: None,
        });

        let (_dir, pipe) = test_pipe_with_config(cfg);
        // Initially default mode
        assert_eq!(pipe.snapshot_props().expect("props").1, "plain");

        // Focus app with profile: automatically switches mode
        pipe.report_focus("com.visualstudio.code").expect("focus");
        assert_eq!(pipe.snapshot_props().expect("props").1, "code");

        // Manual set_mode overrides while focused on this app
        pipe.set_mode("email").expect("set_mode");
        assert_eq!(pipe.snapshot_props().expect("props").1, "email");

        // Reporting same focus keeps the manual override
        pipe.report_focus("com.visualstudio.code").expect("focus again");
        assert_eq!(pipe.snapshot_props().expect("props").1, "email");

        // Switching focus to another app clears manual override and applies that app's profile mode
        pipe.report_focus("com.slack.Slack").expect("focus slack");
        assert_eq!(pipe.snapshot_props().expect("props").1, "bullet");

        // Switching focus to app without profile reverts to general.active_mode
        pipe.report_focus("org.mozilla.firefox").expect("focus firefox");
        assert_eq!(pipe.snapshot_props().expect("props").1, "plain");

        // Empty focus reverts to general.active_mode
        pipe.report_focus("com.visualstudio.code").expect("focus code");
        assert_eq!(pipe.snapshot_props().expect("props").1, "code");
        pipe.report_focus("").expect("focus empty");
        assert_eq!(pipe.snapshot_props().expect("props").1, "plain");
    }

    #[test]
    fn focused_is_terminal_profile_paste_with_shift_override() {
        let mut cfg = NaradConfig::default();
        // App that is NOT in TERMINAL_APPS, but has paste_with_shift = Some(true)
        cfg.profiles.push(crate::config::AppProfile {
            app_id: "com.editor.CustomEditor".into(),
            mode: "plain".into(),
            paste_with_shift: Some(true),
        });
        // App that IS in TERMINAL_APPS (e.g. "org.gnome.Terminal"), but has paste_with_shift = Some(false)
        cfg.profiles.push(crate::config::AppProfile {
            app_id: "org.gnome.Terminal".into(),
            mode: "plain".into(),
            paste_with_shift: Some(false),
        });
        // App with no shift override (None)
        cfg.profiles.push(crate::config::AppProfile {
            app_id: "slack".into(),
            mode: "plain".into(),
            paste_with_shift: None,
        });

        let (_dir, pipe) = test_pipe_with_config(cfg);

        // Custom editor -> true even though not in TERMINAL_APPS
        pipe.report_focus("com.editor.CustomEditor").expect("focus");
        assert!(pipe.focused_is_terminal());

        // org.gnome.Terminal -> false even though in TERMINAL_APPS
        pipe.report_focus("org.gnome.Terminal").expect("focus");
        assert!(!pipe.focused_is_terminal());

        // Slack with None -> fallback to uses_terminal_paste -> false
        pipe.report_focus("com.slack.Slack").expect("focus");
        assert!(!pipe.focused_is_terminal());

        // Ptyxis with no profile -> fallback to uses_terminal_paste -> true
        pipe.report_focus("org.gnome.Ptyxis").expect("focus");
        assert!(pipe.focused_is_terminal());

        // No focus -> false
        pipe.report_focus("").expect("focus empty");
        assert!(!pipe.focused_is_terminal());
    }

    #[tokio::test]
    async fn request_selection_command_refuses_when_no_text_selected() {
        let (_dir, pipe) = test_pipe();
        let mock = Arc::new(Mutex::new(None));
        insert::with_test_primary_selection(mock, async {
            pipe.request_selection_command();
            tokio::time::sleep(Duration::from_millis(50)).await;
            assert_eq!(
                pipe.snapshot_props().expect("props").0,
                SessionState::ERROR
            );
            assert_eq!(pipe.outcome_detail().expect("detail"), "No text selected");
        })
        .await;
    }

    #[tokio::test]
    async fn request_selection_command_captures_selection_and_applies_spoken_instruction_on_finish() {
        let (_dir, pipe) = test_pipe();
        let pasted = Arc::new(Mutex::new(Vec::new()));
        let mock = Arc::new(Mutex::new(Some("don't do it, won't work".to_string())));

        insert::with_test_inserts(Arc::clone(&pasted), async {
            insert::with_test_primary_selection(mock, async {
                pipe.request_selection_command();
                tokio::time::sleep(Duration::from_millis(50)).await;
                assert_eq!(pipe.snapshot_props().expect("props").1, "selection");

                let id = pipe
                    .finish_transcript("make this formal")
                    .await
                    .expect("finish")
                    .expect("id");
                assert_eq!(
                    pasted.lock().expect("pasted").as_slice(),
                    ["Do not do it, will not work."]
                );
                let entry = pipe
                    .with_store(|s| s.get(id).map_err(|e| e.to_string()))
                    .expect("store")
                    .expect("entry");
                assert_eq!(entry.kind, "selection_command");
                assert_eq!(entry.raw_text, "make this formal");
                assert_eq!(entry.final_text, "Do not do it, will not work.");
            })
            .await;
        })
        .await;
    }

    #[tokio::test]
    async fn spoken_prefix_overrides_active_mode_and_applies_email_formatting() {
        let (_dir, pipe) = test_pipe();
        assert_eq!(pipe.snapshot_props().expect("props").1, "plain");

        let res = pipe
            .accepted_transcript("email: meeting at two", "plain")
            .expect("transcript");
        assert_eq!(
            res.text,
            "Hi,\n\nMeeting at two.\n\nBest,\nSender"
        );

        let pasted = Arc::new(Mutex::new(Vec::new()));
        insert::with_test_inserts(Arc::clone(&pasted), async {
            let id = pipe
                .finish_transcript("email: meeting at two")
                .await
                .expect("finish")
                .expect("id");
            let entry = pipe
                .with_store(|s| s.get(id).map_err(|e| e.to_string()))
                .expect("store")
                .expect("entry");
            assert_eq!(entry.mode, "email");
            assert_eq!(
                entry.final_text,
                "Hi,\n\nMeeting at two.\n\nBest,\nSender"
            );
        })
        .await;
    }

    #[test]
    fn set_mode_accepts_clean_and_selection() {
        let (_dir, pipe) = test_pipe();
        pipe.set_mode("clean").expect("clean");
        assert_eq!(pipe.snapshot_props().expect("props").1, "clean");
        pipe.set_mode("selection").expect("selection");
        assert_eq!(pipe.snapshot_props().expect("props").1, "selection");
    }

    #[tokio::test]
    async fn selection_command_preserves_persistent_session_mode_and_subsequent_dictation() {
        let (_dir, pipe) = test_pipe();
        assert_eq!(pipe.snapshot_props().expect("props").1, "plain");

        let mock_sel = Arc::new(Mutex::new(Some("original text".to_string())));
        insert::with_test_primary_selection(mock_sel, async {
            pipe.request_selection_command();
        })
        .await;

        let pasted = Arc::new(Mutex::new(Vec::new()));
        insert::with_test_inserts(Arc::clone(&pasted), async {
            let id = pipe
                .finish_transcript("uppercase")
                .await
                .expect("finish")
                .expect("id");
            let entry = pipe
                .with_store(|s| s.get(id).map_err(|e| e.to_string()))
                .expect("store")
                .expect("entry");
            assert_eq!(entry.kind, "selection_command");
            assert_eq!(entry.final_text, "ORIGINAL TEXT");

            // Session mode must NOT be latched to selection
            assert_eq!(pipe.snapshot_props().expect("props").1, "plain");

            // Subsequent dictation must be classified as normal dictation in plain mode
            let next_id = pipe
                .finish_transcript("regular dictation")
                .await
                .expect("finish next")
                .expect("id next");
            let next_entry = pipe
                .with_store(|s| s.get(next_id).map_err(|e| e.to_string()))
                .expect("store")
                .expect("next entry");
            assert_eq!(next_entry.kind, "dictation");
            assert_eq!(next_entry.mode, "plain");
            assert_eq!(next_entry.final_text, "regular dictation");
        })
        .await;
    }

    #[test]
    fn wake_word_disabled_by_default_rejects_audio_and_stays_idle() {
        let (_dir, pipe) = test_pipe();
        assert!(!pipe.is_wake_word_enabled());
        let samples = crate::wakeword::generate_wake_phrase_samples("hey narad", 16_000);
        assert!(pipe.feed_idle_pcm(&samples, 16_000).is_none());
        assert!(pipe.process_idle_audio(&samples, 16_000).is_none());
        assert!(!pipe.is_listening());
        assert_eq!(pipe.snapshot_props().expect("props").0, SessionState::IDLE);
    }

    #[tokio::test]
    async fn wake_word_enabled_detects_phrase_and_transitions_to_listening() {
        let mut cfg = NaradConfig::default();
        cfg.activation.wake_word_enabled = true;
        cfg.activation.wake_word_phrase = "hey narad".to_string();

        let (_dir, pipe) = test_pipe_with_config(cfg);
        assert!(pipe.is_wake_word_enabled());
        assert!(!pipe.is_listening());

        // 1. Feeding pure silence should not trigger
        let silence = vec![0.0f32; 16_000];
        assert!(pipe.feed_idle_pcm(&silence, 16_000).is_none());
        assert!(!pipe.is_listening());

        // 2. Feeding synthesized wake phrase triggers detection and automatic hands-free start
        let samples = crate::wakeword::generate_wake_phrase_samples("hey narad", 16_000);
        let detection = pipe.feed_idle_pcm(&samples, 16_000);
        assert!(detection.is_some(), "wake phrase must trigger detection in idle");
        let det = detection.unwrap();
        assert_eq!(det.phrase, "hey narad");
        assert!(det.confidence >= 0.70);

        // Allow spawned start task to complete
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(pipe.is_listening(), "pipeline must transition to listening state");
        assert_eq!(pipe.snapshot_props().expect("props").0, SessionState::LISTENING);

        // 3. While listening, additional audio feed is ignored
        assert!(
            pipe.feed_idle_pcm(&samples, 16_000).is_none(),
            "feed in non-idle state must be ignored"
        );
    }

    #[tokio::test]
    async fn wake_word_custom_phrase_and_mode_preservation() {
        let mut cfg = NaradConfig::default();
        cfg.activation.wake_word_enabled = true;
        cfg.activation.wake_word_phrase = "start recording".to_string();

        let (_dir, pipe) = test_pipe_with_config(cfg);
        pipe.set_mode("email").expect("set_mode");

        // "hey narad" does not trigger custom phrase
        let default_samples = crate::wakeword::generate_wake_phrase_samples("hey narad", 16_000);
        assert!(pipe.feed_idle_pcm(&default_samples, 16_000).is_none());
        assert!(!pipe.is_listening());

        // Custom phrase triggers
        let custom_samples = crate::wakeword::generate_wake_phrase_samples("start recording", 16_000);
        let detection = pipe.feed_idle_pcm(&custom_samples, 16_000);
        assert!(detection.is_some(), "custom wake phrase must trigger detection");
        assert_eq!(detection.unwrap().phrase, "start recording");

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(pipe.is_listening());
        assert_eq!(pipe.snapshot_props().expect("props").1, "email");
    }
}


//! Mic capture via PipeWire `pw-record` + energy VAD trim (FR-AUD-1/2/4/6).
//!
//! Uses `pw-record` so the daemon does not need ALSA headers at build time
//! (ARCHITECTURE allows PipeWire). Mic process lives only while listening.

use std::io::Read;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use tracing::{debug, info, warn};

pub const TARGET_SAMPLE_RATE: i32 = 16_000;

/// Live mic capture. Mic is open only while this handle exists (FR-AUD-1).
pub struct CaptureHandle {
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    /// Reinterpret bits of f32 RMS for Level signal.
    pub rms_bits: Arc<AtomicU32>,
    sample_rate: i32,
    child: Child,
    reader: Option<JoinHandle<()>>,
}

impl CaptureHandle {
    /// Spawn `pw-record` for mono f32 @ 16 kHz raw PCM on stdout.
    ///
    /// An empty `input_target` omits `--target` so PipeWire uses the default source (FR-AUD-4).
    pub fn start(input_target: &str) -> Result<Self> {
        let sample_rate = TARGET_SAMPLE_RATE;
        let args = pw_record_args(sample_rate, input_target);
        let mut child = Command::new("pw-record")
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("spawn pw-record (is pipewire-utils installed?)")?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("pw-record stdout missing"))?;

        let stop = Arc::new(AtomicBool::new(false));
        let samples = Arc::new(Mutex::new(Vec::new()));
        let rms_bits = Arc::new(AtomicU32::new(0f32.to_bits()));

        let stop_c = stop.clone();
        let samples_c = samples.clone();
        let rms_c = rms_bits.clone();
        let reader = thread::spawn(move || read_pcm_loop(stdout, stop_c, samples_c, rms_c));

        info!(sample_rate, "mic open (pw-record)");
        Ok(Self {
            stop,
            samples,
            rms_bits,
            sample_rate,
            child,
            reader: Some(reader),
        })
    }

    #[allow(dead_code)]
    pub fn sample_rate(&self) -> i32 {
        self.sample_rate
    }

    #[allow(dead_code)]
    pub fn current_rms(&self) -> f64 {
        f32::from_bits(self.rms_bits.load(Ordering::Relaxed)) as f64
    }

    /// Samples appended since `from`, plus the new buffer length.
    pub fn copy_from(&self, from: usize) -> (Vec<f32>, usize) {
        let guard = match self.samples.lock() {
            Ok(guard) => guard,
            Err(_) => return (Vec::new(), from),
        };
        let end = guard.len();
        let start = from.min(end);
        (guard[start..end].to_vec(), end)
    }

    /// Stop capture and return mono samples at the capture rate.
    pub fn stop(mut self) -> Result<(Vec<f32>, i32)> {
        // Brief drain pause so PipeWire flushes the final audio chunk through stdout.
        // Fast-path when empty (hotkey bounce) or running integration test suites.
        let is_empty = self.samples.lock().map(|s| s.is_empty()).unwrap_or(true);
        let fast_drain = is_empty
            || std::env::var_os("CARGO_TARGET_TMPDIR").is_some()
            || std::env::var_os("NARAD_TEST_FAST_DRAIN").is_some();
        if !fast_drain {
            thread::sleep(Duration::from_millis(250));
        }
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
        let samples = self
            .samples
            .lock()
            .map_err(|_| anyhow!("samples lock poisoned"))?
            .clone();
        debug!(n = samples.len(), rate = self.sample_rate, "mic closed");
        Ok((samples, self.sample_rate))
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
    }
}

/// Argv for `pw-record` mono f32 raw PCM. Blank `input_target` omits `--target` (FR-AUD-4).
pub fn pw_record_args(sample_rate: i32, input_target: &str) -> Vec<String> {
    let mut args = vec![
        "--rate".to_string(),
        sample_rate.to_string(),
        "--channels".to_string(),
        "1".to_string(),
        "--format".to_string(),
        "f32".to_string(),
        "--raw".to_string(),
        "-".to_string(),
    ];
    let target = input_target.trim();
    if !target.is_empty() {
        // Options before the `-` file operand.
        let file = args.pop();
        args.push("--target".to_string());
        args.push(target.to_string());
        if let Some(file) = file {
            args.push(file);
        }
    }
    args
}

fn read_pcm_loop(
    mut stdout: ChildStdout,
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    rms_bits: Arc<AtomicU32>,
) {
    let mut buf = [0u8; 4096];
    loop {
        match stdout.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let usable = n - (n % 4);
                if usable == 0 {
                    continue;
                }
                let mut chunk = Vec::with_capacity(usable / 4);
                for w in buf[..usable].chunks_exact(4) {
                    chunk.push(f32::from_le_bytes([w[0], w[1], w[2], w[3]]));
                }
                let rms = rms_of(&chunk);
                rms_bits.store(rms.to_bits(), Ordering::Relaxed);
                if let Ok(mut guard) = samples.lock() {
                    guard.extend_from_slice(&chunk);
                }
            }
            Err(err) => {
                if !stop.load(Ordering::Relaxed) {
                    warn!(error = %err, "pw-record read");
                }
                break;
            }
        }
    }
}

fn rms_of(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

/// Linear resample to 16 kHz mono.
pub fn resample_to_16k(samples: &[f32], from_rate: i32) -> Vec<f32> {
    if from_rate == TARGET_SAMPLE_RATE || samples.is_empty() {
        return samples.to_vec();
    }
    if from_rate <= 0 {
        return samples.to_vec();
    }
    let ratio = TARGET_SAMPLE_RATE as f64 / from_rate as f64;
    let out_len = ((samples.len() as f64) * ratio).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let i0 = src.floor() as usize;
        let i1 = (i0 + 1).min(samples.len().saturating_sub(1));
        let frac = (src - i0 as f64) as f32;
        let s = samples[i0] * (1.0 - frac) + samples[i1] * frac;
        out.push(s);
    }
    out
}

/// 10 ms energy frame, at least 80 samples (FR-AUD-2 gate).
pub fn speech_frame_len(sample_rate: i32) -> usize {
    (sample_rate as usize / 100).max(80)
}

fn energy_threshold(samples: &[f32], frame: usize) -> f32 {
    let mut peak = 0.0f32;
    for chunk in samples.chunks(frame) {
        let e = rms_of(chunk);
        if e > peak {
            peak = e;
        }
    }
    (peak * 0.02).max(0.002)
}

/// True when any full frame passes the same energy gate [`trim_silence`] uses.
///
/// This is the live speech detector (a short burst resets auto-stop). It does not
/// apply the trimmer's minimum-length / whole-clip RMS checks.
pub fn contains_speech(samples: &[f32], sample_rate: i32) -> bool {
    if samples.is_empty() {
        return false;
    }
    let frame = speech_frame_len(sample_rate);
    if samples.len() < frame {
        return false;
    }
    let threshold = energy_threshold(samples, frame);
    samples
        .chunks(frame)
        .any(|chunk| chunk.len() == frame && rms_of(chunk) >= threshold)
}

/// Energy-gate VAD: trim leading/trailing low-energy frames (FR-AUD-2).
/// Returns `None` when the clip looks like silence/noise only.
pub fn trim_silence(samples: &[f32], sample_rate: i32) -> Option<Vec<f32>> {
    if samples.is_empty() {
        return None;
    }
    let frame = speech_frame_len(sample_rate);
    let threshold = energy_threshold(samples, frame);

    let mut start = 0usize;
    while start + frame <= samples.len() {
        if rms_of(&samples[start..start + frame]) >= threshold {
            break;
        }
        start += frame;
    }
    if start >= samples.len() {
        return None;
    }

    let mut end = samples.len();
    while end > start + frame {
        let lo = end - frame;
        if rms_of(&samples[lo..end]) >= threshold {
            break;
        }
        end = lo;
    }

    // 350ms pad to preserve soft onsets (h, w, s, th) and trailing vowels/nasals
    let pad = (sample_rate as usize) * 35 / 100;
    let start = start.saturating_sub(pad);
    let end = (end + pad).min(samples.len());
    let trimmed = samples[start..end].to_vec();

    if trimmed.len() < sample_rate as usize / 10 {
        return None;
    }
    if rms_of(&trimmed) < 0.002 {
        return None;
    }
    Some(trimmed)
}

/// Optional start/stop/cancel/error cue (FR-AUD-6). Fire-and-forget via `pw-play`/`paplay`.
pub fn play_cue(kind: &str, enabled: bool) {
    if !enabled {
        return;
    }
    let kind = kind.to_string();
    thread::spawn(move || {
        if let Err(err) = play_cue_blocking(&kind) {
            warn_cue_once(&kind, &err);
        }
    });
}

fn play_cue_blocking(kind: &str) -> Result<()> {
    let path = resolve_cue_path(kind).ok_or_else(|| anyhow!("no cue asset for {kind}"))?;
    let player = which_player().ok_or_else(|| anyhow!("neither pw-play nor paplay on PATH"))?;
    let status = Command::new(&player)
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("spawn {}", player.display()))?;
    if !status.success() {
        return Err(anyhow!("{} exited {}", player.display(), status));
    }
    debug!(cue = kind, path = %path.display(), player = %player.display(), "played audio cue");
    Ok(())
}

fn warn_cue_once(kind: &str, err: &anyhow::Error) {
    use std::sync::Once;
    static WARNED: Once = Once::new();
    WARNED.call_once(|| {
        warn!(cue = kind, error = %err, "audio cue unavailable; continuing without sound");
    });
}

fn which_player() -> Option<std::path::PathBuf> {
    for name in ["pw-play", "paplay"] {
        if let Some(p) = which_bin(name) {
            return Some(p);
        }
    }
    None
}

fn which_bin(name: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for dir in std::env::split_paths(&paths) {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
        None
    })
}

/// Prefer embedded WAV extracted under XDG; fall back to freedesktop theme sounds.
fn resolve_cue_path(kind: &str) -> Option<std::path::PathBuf> {
    if let Some(p) = ensure_embedded_cue(kind) {
        return Some(p);
    }
    for candidate in cue_search_paths(kind) {
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn ensure_embedded_cue(kind: &str) -> Option<std::path::PathBuf> {
    let (name, bytes): (&str, &[u8]) = match kind {
        "start" => ("cue-start.wav", include_bytes!("../assets/cue-start.wav")),
        "stop" => ("cue-stop.wav", include_bytes!("../assets/cue-stop.wav")),
        "cancel" => ("cue-cancel.wav", include_bytes!("../assets/cue-cancel.wav")),
        "error" => ("cue-error.wav", include_bytes!("../assets/cue-error.wav")),
        _ => return None,
    };
    let dir = crate::paths::resolve_data_dir()?.join("sounds");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        debug!(error = %err, "create sounds dir");
        return None;
    }
    let path = dir.join(name);
    if !path.is_file() {
        if let Err(err) = std::fs::write(&path, bytes) {
            debug!(error = %err, path = %path.display(), "write embedded cue");
            return None;
        }
    }
    Some(path)
}

fn cue_search_paths(kind: &str) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    let file = match kind {
        "start" => "cue-start.wav",
        "stop" => "cue-stop.wav",
        "cancel" => "cue-cancel.wav",
        "error" => "cue-error.wav",
        _ => return paths,
    };
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        paths.push(std::path::PathBuf::from(manifest).join("assets").join(file));
    }
    paths.push(std::path::PathBuf::from("crates/naradd/assets").join(file));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("assets").join(file));
            paths.push(dir.join("../share/narad/sounds").join(file));
        }
    }
    let freedesktop = match kind {
        "start" => "message.oga",
        "stop" => "complete.oga",
        "cancel" => "dialog-warning.oga",
        "error" => "dialog-error.oga",
        _ => return paths,
    };
    paths.push(std::path::PathBuf::from("/usr/share/sounds/freedesktop/stereo").join(freedesktop));
    paths
}

// ---------------------------------------------------------------------------
// Real-time DSP Prosody Extraction: YIN F0 Pitch Tracker & Energy Decay (Task 3)
// ---------------------------------------------------------------------------

/// Frame size for prosodic analysis: 512 samples (32 ms @ 16 kHz).
pub const PROSODY_FRAME_SIZE: usize = 512;
/// Minimum lag for human vocal range (500 Hz @ 16 kHz).
pub const PROSODY_MIN_LAG: usize = 32;
/// Maximum lag for human vocal range (50 Hz @ 16 kHz).
pub const PROSODY_MAX_LAG: usize = 320;
/// Absolute dip threshold theta for YIN CMNDF.
pub const YIN_THRESHOLD: f32 = 0.15;
/// Minimum log energy in dB for a frame to be considered voiced speech.
pub const PROSODY_MIN_VOICED_ENERGY_DB: f32 = -45.0;

/// High-level prosodic event emitted by pitch/energy classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProsodicEvent {
    /// Terminal declarative drop: Delta F0 <= -35 Hz and dE/dt < -0.10 dB/ms.
    FallingIntonation,
    /// Terminal interrogative rise: Delta F0 >= +30 Hz.
    RisingIntonation,
    /// Clausal continuation: |Delta F0| < 18 Hz.
    LevelContinuation,
    /// Pause/silence with accumulated duration.
    Silence(Duration),
}

impl ProsodicEvent {
    /// Returns true if this event indicates a terminal clause or sentence boundary (falling or rising intonation).
    #[allow(dead_code)]
    pub fn is_terminal_boundary(&self) -> bool {
        matches!(self, ProsodicEvent::FallingIntonation | ProsodicEvent::RisingIntonation)
    }
}

/// Instantaneous prosodic frame features.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProsodyFrame {
    pub pitch_hz: f32,
    pub energy_db: f32,
    pub energy_slope: f32,
    pub is_voiced: bool,
}

/// Real-time YIN fundamental frequency ($F_0$) pitch tracker and energy decay estimator.
///
/// Designed with zero heap allocations in the hot processing loop to achieve
/// < 0.2 ms CPU compute cost per 32 ms audio frame.
#[derive(Debug)]
pub struct ProsodyTracker {
    sample_rate: u32,
    ring_buffer: [f32; 4096],
    ring_read_pos: usize,
    ring_write_pos: usize,
    ring_len: usize,
    energy_history: [(f32, f32); 5],
    energy_history_len: usize,
    pitch_history: [(f32, f32); 32],
    pitch_history_len: usize,
    current_time_ms: f32,
    silence_duration: Duration,
    last_pitch: f32,
    last_energy_db: f32,
    last_energy_slope: f32,
    last_pitch_delta: f32,
    is_voiced: bool,
}

impl Default for ProsodyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ProsodyTracker {
    /// Create a new prosody tracker at the default 16 kHz sample rate.
    pub fn new() -> Self {
        Self::with_sample_rate(TARGET_SAMPLE_RATE as u32)
    }

    /// Create a prosody tracker with a custom sample rate.
    pub fn with_sample_rate(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            ring_buffer: [0.0; 4096],
            ring_read_pos: 0,
            ring_write_pos: 0,
            ring_len: 0,
            energy_history: [(0.0, 0.0); 5],
            energy_history_len: 0,
            pitch_history: [(0.0, 0.0); 32],
            pitch_history_len: 0,
            current_time_ms: 0.0,
            silence_duration: Duration::ZERO,
            last_pitch: 0.0,
            last_energy_db: -120.0,
            last_energy_slope: 0.0,
            last_pitch_delta: 0.0,
            is_voiced: false,
        }
    }

    /// Reset internal state, histories, and ring buffer.
    pub fn reset(&mut self) {
        self.ring_read_pos = 0;
        self.ring_write_pos = 0;
        self.ring_len = 0;
        self.energy_history_len = 0;
        self.pitch_history_len = 0;
        self.current_time_ms = 0.0;
        self.silence_duration = Duration::ZERO;
        self.last_pitch = 0.0;
        self.last_energy_db = -120.0;
        self.last_energy_slope = 0.0;
        self.last_pitch_delta = 0.0;
        self.is_voiced = false;
    }

    /// Push samples into the internal FIFO ring buffer without processing.
    pub fn push_samples(&mut self, samples: &[f32]) {
        let cap = self.ring_buffer.len();
        for &s in samples {
            self.ring_buffer[self.ring_write_pos] = s;
            self.ring_write_pos = (self.ring_write_pos + 1) % cap;
            if self.ring_len < cap {
                self.ring_len += 1;
            } else {
                self.ring_read_pos = (self.ring_read_pos + 1) % cap;
            }
        }
    }

    /// Number of unconsumed samples currently held in the ring buffer.
    pub fn ring_len(&self) -> usize {
        self.ring_len
    }

    /// Current sample rate in Hz.
    #[allow(dead_code)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Current fundamental pitch F0 in Hz (0.0 if unvoiced).
    #[allow(dead_code)]
    pub fn current_pitch(&self) -> f32 {
        self.last_pitch
    }

    /// Current frame log energy in dB.
    #[allow(dead_code)]
    pub fn current_energy_db(&self) -> f32 {
        self.last_energy_db
    }

    /// Current linear regression energy decay slope (dE/dt) in dB/ms.
    #[allow(dead_code)]
    pub fn current_energy_slope(&self) -> f32 {
        self.last_energy_slope
    }

    /// Pitch change over the trailing ~150 ms in Hz.
    #[allow(dead_code)]
    pub fn pitch_delta_150ms(&self) -> f32 {
        self.last_pitch_delta
    }

    /// Accumulated silence duration during non-speech/pause.
    pub fn silence_duration(&self) -> Duration {
        self.silence_duration
    }

    /// Returns true if the most recent frame was classified as voiced speech.
    #[allow(dead_code)]
    pub fn is_voiced(&self) -> bool {
        self.is_voiced
    }

    /// Snapshot of the latest prosodic frame features.
    #[allow(dead_code)]
    pub fn current_frame(&self) -> ProsodyFrame {
        ProsodyFrame {
            pitch_hz: self.last_pitch,
            energy_db: self.last_energy_db,
            energy_slope: self.last_energy_slope,
            is_voiced: self.is_voiced,
        }
    }

    /// Compute frame log energy: E = 10 * log10( (1/W) * sum(x_i^2) + 1e-12 ).
    pub fn compute_frame_energy(&self, window: &[f32]) -> f32 {
        let n = window.len().min(PROSODY_FRAME_SIZE);
        if n == 0 {
            return -120.0;
        }
        let sum_sq: f32 = window[..n].iter().map(|&x| x * x).sum();
        let mean_sq = sum_sq / (n as f32);
        10.0 * (mean_sq + 1e-12).log10()
    }

    /// Compute fundamental pitch F0 on a window using the YIN algorithm with parabolic interpolation.
    pub fn compute_pitch(&self, window: &[f32]) -> Option<f32> {
        let (pitch, _) = self.compute_pitch_and_cmndf_dip(window);
        pitch
    }

    /// Compute pitch and whether a dip below YIN threshold was found.
    pub fn compute_pitch_and_cmndf_dip(&self, window: &[f32]) -> (Option<f32>, bool) {
        if window.len() < PROSODY_FRAME_SIZE {
            return (None, false);
        }

        let limit = if window.len() >= PROSODY_FRAME_SIZE + PROSODY_MAX_LAG {
            PROSODY_FRAME_SIZE
        } else {
            PROSODY_FRAME_SIZE - PROSODY_MAX_LAG // 192
        };

        // Difference function d_t(tau) for tau in [1, PROSODY_MAX_LAG]
        let mut d = [0.0f32; PROSODY_MAX_LAG + 1];
        for tau in 1..=PROSODY_MAX_LAG {
            let s1 = &window[..limit];
            let s2 = &window[tau..tau + limit];
            let mut sum0 = 0.0f32;
            let mut sum1 = 0.0f32;
            let mut sum2 = 0.0f32;
            let mut sum3 = 0.0f32;
            let c1 = s1.chunks_exact(4);
            let c2 = s2.chunks_exact(4);
            for (w1, w2) in c1.zip(c2) {
                let d0 = w1[0] - w2[0];
                let d1 = w1[1] - w2[1];
                let d2 = w1[2] - w2[2];
                let d3 = w1[3] - w2[3];
                sum0 += d0 * d0;
                sum1 += d1 * d1;
                sum2 += d2 * d2;
                sum3 += d3 * d3;
            }
            d[tau] = (sum0 + sum1) + (sum2 + sum3);
        }

        // Cumulative Mean Normalized Difference Function (CMNDF) d'_t(tau)
        let mut cmnd = [1.0f32; PROSODY_MAX_LAG + 1];
        let mut running_sum = 0.0f32;
        cmnd[0] = 1.0;
        for tau in 1..=PROSODY_MAX_LAG {
            running_sum += d[tau];
            if running_sum > 1e-12 {
                cmnd[tau] = (d[tau] * (tau as f32)) / running_sum;
            } else {
                cmnd[tau] = 1.0;
            }
        }

        // Absolute thresholding theta = 0.15
        let mut tau_est = None;
        for tau in PROSODY_MIN_LAG..=PROSODY_MAX_LAG {
            if cmnd[tau] < YIN_THRESHOLD {
                let mut best_tau = tau;
                while best_tau + 1 <= PROSODY_MAX_LAG && cmnd[best_tau + 1] < cmnd[best_tau] {
                    best_tau += 1;
                }
                tau_est = Some(best_tau);
                break;
            }
        }

        let has_dip = tau_est.is_some();
        if let Some(best_tau) = tau_est {
            let x0 = best_tau.saturating_sub(1);
            let x2 = (best_tau + 1).min(PROSODY_MAX_LAG);
            let s0 = cmnd[x0];
            let s1 = cmnd[best_tau];
            let s2 = cmnd[x2];
            let denom = 2.0 * (2.0 * s1 - s0 - s2);
            let delta = if denom.abs() > 1e-12 {
                (s2 - s0) / denom
            } else {
                0.0
            };
            let refined_tau = best_tau as f32 + delta;
            if refined_tau > 0.0 {
                let f0 = (self.sample_rate as f32) / refined_tau;
                return (Some(f0), true);
            }
        }

        (None, has_dip)
    }

    fn push_energy(&mut self, t_ms: f32, energy_db: f32) {
        if self.energy_history_len < 5 {
            self.energy_history[self.energy_history_len] = (t_ms, energy_db);
            self.energy_history_len += 1;
        } else {
            for i in 0..4 {
                self.energy_history[i] = self.energy_history[i + 1];
            }
            self.energy_history[4] = (t_ms, energy_db);
        }
    }

    fn calculate_energy_slope(&self) -> f32 {
        if self.energy_history_len < 2 {
            return 0.0;
        }
        let n = self.energy_history_len as f32;
        let mut sum_t = 0.0f32;
        let mut sum_e = 0.0f32;
        for i in 0..self.energy_history_len {
            sum_t += self.energy_history[i].0;
            sum_e += self.energy_history[i].1;
        }
        let mean_t = sum_t / n;
        let mean_e = sum_e / n;

        let mut num = 0.0f32;
        let mut den = 0.0f32;
        for i in 0..self.energy_history_len {
            let dt = self.energy_history[i].0 - mean_t;
            let de = self.energy_history[i].1 - mean_e;
            num += dt * de;
            den += dt * dt;
        }
        if den > 1e-12 {
            num / den
        } else {
            0.0
        }
    }

    fn push_pitch(&mut self, t_ms: f32, pitch: f32) {
        if self.pitch_history_len < 32 {
            self.pitch_history[self.pitch_history_len] = (t_ms, pitch);
            self.pitch_history_len += 1;
        } else {
            for i in 0..31 {
                self.pitch_history[i] = self.pitch_history[i + 1];
            }
            self.pitch_history[31] = (t_ms, pitch);
        }
    }

    fn calculate_pitch_delta_150ms(&self, current_t_ms: f32, current_f0: f32) -> Option<f32> {
        if self.pitch_history_len < 2 {
            return None;
        }
        let target_t = current_t_ms - 150.0;
        let mut best_diff = f32::MAX;
        let mut best_f0 = self.pitch_history[0].1;
        let mut best_t = self.pitch_history[0].0;

        for i in 0..self.pitch_history_len {
            let (t, f0) = self.pitch_history[i];
            let diff = (t - target_t).abs();
            if diff < best_diff {
                best_diff = diff;
                best_f0 = f0;
                best_t = t;
            }
        }

        // When fewer than 100ms of voiced frames exist, do not compute delta from a premature frame
        if (current_t_ms - best_t) < 100.0 {
            return None;
        }
        if (current_t_ms - best_t) > 350.0 {
            return None;
        }

        Some(current_f0 - best_f0)
    }

    /// Process a single 32 ms frame (512 samples) directly without buffer duplication.
    ///
    /// Executes with zero heap allocations on the hot path.
    pub fn process_frame(&mut self, frame: &[f32]) -> Option<ProsodicEvent> {
        let frame_duration_ms = (PROSODY_FRAME_SIZE as f32 / self.sample_rate as f32) * 1000.0;
        let t_ms = self.current_time_ms + frame_duration_ms;
        self.current_time_ms = t_ms;

        let energy_db = self.compute_frame_energy(frame);
        self.last_energy_db = energy_db;
        self.push_energy(t_ms, energy_db);
        let energy_slope = self.calculate_energy_slope();
        self.last_energy_slope = energy_slope;

        let (pitch_opt, _has_dip) = self.compute_pitch_and_cmndf_dip(frame);
        let is_voiced = pitch_opt.is_some() && energy_db > PROSODY_MIN_VOICED_ENERGY_DB;
        self.is_voiced = is_voiced;

        if is_voiced {
            let f0 = pitch_opt.unwrap();
            self.last_pitch = f0;
            self.push_pitch(t_ms, f0);
            self.silence_duration = Duration::ZERO;

            if let Some(delta_f0) = self.calculate_pitch_delta_150ms(t_ms, f0) {
                self.last_pitch_delta = delta_f0;
                if delta_f0 <= -35.0 && energy_slope < -0.10 {
                    Some(ProsodicEvent::FallingIntonation)
                } else if delta_f0 >= 30.0 {
                    Some(ProsodicEvent::RisingIntonation)
                } else if delta_f0.abs() < 18.0 {
                    Some(ProsodicEvent::LevelContinuation)
                } else {
                    None
                }
            } else {
                self.last_pitch_delta = 0.0;
                None
            }
        } else {
            self.last_pitch = 0.0;
            let frame_duration_us = ((PROSODY_FRAME_SIZE as u64) * 1_000_000) / (self.sample_rate as u64);
            self.silence_duration += Duration::from_micros(frame_duration_us);
            Some(ProsodicEvent::Silence(self.silence_duration))
        }
    }

    /// Process a streaming audio chunk of arbitrary size, consuming frames from the ring buffer FIFO.
    pub fn process_chunk(&mut self, samples: &[f32]) -> Vec<ProsodicEvent> {
        self.push_samples(samples);
        let mut events = Vec::new();
        let mut frame_buf = [0.0f32; PROSODY_FRAME_SIZE];
        let cap = self.ring_buffer.len();

        while self.ring_len >= PROSODY_FRAME_SIZE {
            for i in 0..PROSODY_FRAME_SIZE {
                frame_buf[i] = self.ring_buffer[(self.ring_read_pos + i) % cap];
            }
            if let Some(ev) = self.process_frame(&frame_buf) {
                events.push(ev);
            }
            self.ring_read_pos = (self.ring_read_pos + PROSODY_FRAME_SIZE) % cap;
            self.ring_len -= PROSODY_FRAME_SIZE;
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;
    use std::time::Instant;

    fn generate_sine(freq: f32, n_samples: usize, sample_rate: f32) -> Vec<f32> {
        (0..n_samples)
            .map(|i| (2.0 * PI * freq * (i as f32) / sample_rate).sin())
            .collect()
    }

    #[test]
    fn test_process_chunk_fifo_buffer() {
        let mut tracker = ProsodyTracker::new();

        // 300 samples: less than 512, no frame should be processed yet
        let chunk1 = vec![0.1f32; 300];
        let events1 = tracker.process_chunk(&chunk1);
        assert_eq!(tracker.ring_len(), 300);
        assert!(events1.is_empty(), "Should not emit with < 512 samples");

        // 400 samples: 300 + 400 = 700 >= 512, 1 frame processed, 188 samples left
        let chunk2 = vec![0.1f32; 400];
        let events2 = tracker.process_chunk(&chunk2);
        assert_eq!(tracker.ring_len(), 700 - 512);
        assert_eq!(events2.len(), 1, "Should have processed 1 frame");

        // 900 samples: 188 + 900 = 1088 >= 1024, 2 frames processed, 64 samples left
        let chunk3 = vec![0.1f32; 900];
        let events3 = tracker.process_chunk(&chunk3);
        assert_eq!(tracker.ring_len(), 1088 - 1024);
        assert_eq!(events3.len(), 2, "Should have processed 2 frames");
    }

    #[test]
    fn test_yin_pitch_detection_synthetic_sines() {
        let tracker = ProsodyTracker::new();
        for &freq in &[100.0f32, 220.0f32, 440.0f32] {
            let sine = generate_sine(freq, PROSODY_FRAME_SIZE, TARGET_SAMPLE_RATE as f32);
            let detected = tracker.compute_pitch(&sine).expect("pitch should be detected");
            let error = (detected - freq).abs();
            assert!(
                error < 2.0,
                "Pitch detection error for {freq} Hz must be < 2 Hz, got {error:.4} Hz ({detected:.2} Hz detected)"
            );
        }
    }

    #[test]
    fn test_falling_pitch_classification_sweep() {
        let mut tracker = ProsodyTracker::new();
        let sample_rate = TARGET_SAMPLE_RATE as f32;
        let duration_sec = 0.250f32;
        let n_samples = (sample_rate * duration_sec) as usize;
        let mut sweep_falling = Vec::with_capacity(n_samples);
        let mut phase = 0.0f32;
        for i in 0..n_samples {
            let t = i as f32 / sample_rate;
            let frac = t / duration_sec;
            let freq = 200.0 - 70.0 * frac; // 200 Hz -> 130 Hz
            let amp = 0.8 * (-3.0 * frac).exp(); // decay energy
            phase += 2.0 * PI * freq / sample_rate;
            sweep_falling.push(amp * phase.sin());
        }

        let mut falling_detected = false;
        for chunk in sweep_falling.chunks(PROSODY_FRAME_SIZE) {
            if chunk.len() == PROSODY_FRAME_SIZE {
                if let Some(event) = tracker.process_frame(chunk) {
                    if event == ProsodicEvent::FallingIntonation {
                        falling_detected = true;
                    }
                }
            }
        }
        assert!(falling_detected, "Falling intonation (200 Hz -> 130 Hz) should be detected");
    }

    #[test]
    fn test_rising_pitch_classification_sweep() {
        let mut tracker = ProsodyTracker::new();
        let sample_rate = TARGET_SAMPLE_RATE as f32;
        let duration_sec = 0.250f32;
        let n_samples = (sample_rate * duration_sec) as usize;
        let mut sweep_rising = Vec::with_capacity(n_samples);
        let mut phase = 0.0f32;
        for i in 0..n_samples {
            let t = i as f32 / sample_rate;
            let frac = t / duration_sec;
            let freq = 180.0 + 70.0 * frac; // 180 Hz -> 250 Hz
            phase += 2.0 * PI * freq / sample_rate;
            sweep_rising.push(0.5 * phase.sin());
        }

        let mut rising_detected = false;
        for chunk in sweep_rising.chunks(PROSODY_FRAME_SIZE) {
            if chunk.len() == PROSODY_FRAME_SIZE {
                if let Some(event) = tracker.process_frame(chunk) {
                    if event == ProsodicEvent::RisingIntonation {
                        rising_detected = true;
                    }
                }
            }
        }
        assert!(rising_detected, "Rising intonation (180 Hz -> 250 Hz) should be detected");
    }

    #[test]
    fn test_level_continuation() {
        let mut tracker = ProsodyTracker::new();
        let sine = generate_sine(220.0, PROSODY_FRAME_SIZE * 5, TARGET_SAMPLE_RATE as f32);
        let mut level_seen = false;
        for chunk in sine.chunks(PROSODY_FRAME_SIZE) {
            if chunk.len() == PROSODY_FRAME_SIZE {
                if let Some(event) = tracker.process_frame(chunk) {
                    if event == ProsodicEvent::LevelContinuation {
                        level_seen = true;
                    }
                }
            }
        }
        assert!(level_seen, "Level continuation should be detected for steady pitch");
    }

    #[test]
    fn test_prosody_benchmark_per_frame() {
        let mut tracker = ProsodyTracker::new();
        let frame = generate_sine(220.0, PROSODY_FRAME_SIZE, TARGET_SAMPLE_RATE as f32);
        // Warm up
        for _ in 0..10 {
            let _ = tracker.process_frame(&frame);
        }
        let iters = 100;
        let start = Instant::now();
        for _ in 0..iters {
            let _ = tracker.process_frame(&frame);
        }
        let elapsed = start.elapsed();
        let per_frame = elapsed / iters;
        eprintln!("Benchmark: {per_frame:?} per 32ms frame");

        #[cfg(not(debug_assertions))]
        {
            assert!(
                per_frame < Duration::from_micros(200),
                "Release compute cost must be < 0.2ms, got {per_frame:?}"
            );
        }
        #[cfg(debug_assertions)]
        {
            eprintln!("Note: Running in unoptimized debug mode ({per_frame:?}/frame); release mode achieves < 0.2ms");
            assert!(
                per_frame < Duration::from_millis(100),
                "Debug compute sanity check failed: {per_frame:?}"
            );
        }
    }

    #[test]
    fn test_unvoiced_silence_duration() {
        let mut tracker = ProsodyTracker::new();
        let silence_frame = vec![0.0f32; PROSODY_FRAME_SIZE];

        let event1 = tracker.process_frame(&silence_frame);
        assert_eq!(event1, Some(ProsodicEvent::Silence(Duration::from_millis(32))));
        assert_eq!(tracker.silence_duration(), Duration::from_millis(32));

        let event2 = tracker.process_frame(&silence_frame);
        assert_eq!(event2, Some(ProsodicEvent::Silence(Duration::from_millis(64))));
        assert_eq!(tracker.silence_duration(), Duration::from_millis(64));
    }

    #[test]
    fn contains_speech_matches_energy_gate() {
        let silence = vec![0.0f32; TARGET_SAMPLE_RATE as usize];
        assert!(!contains_speech(&silence, TARGET_SAMPLE_RATE));

        let quiet = vec![0.001f32; speech_frame_len(TARGET_SAMPLE_RATE)];
        assert!(!contains_speech(&quiet, TARGET_SAMPLE_RATE));

        let mut one_frame = vec![0.0f32; speech_frame_len(TARGET_SAMPLE_RATE) * 3];
        let frame = speech_frame_len(TARGET_SAMPLE_RATE);
        for s in &mut one_frame[frame..frame * 2] {
            *s = 0.2;
        }
        assert!(contains_speech(&one_frame, TARGET_SAMPLE_RATE));
        // Shorter than the trimmer's minimum keep length, but still speech for auto-stop.
        assert!(trim_silence(&one_frame, TARGET_SAMPLE_RATE).is_none());
    }

    #[test]
    fn trim_rejects_silence() {
        let silence = vec![0.0f32; TARGET_SAMPLE_RATE as usize];
        assert!(trim_silence(&silence, TARGET_SAMPLE_RATE).is_none());
    }

    #[test]
    fn trim_keeps_speech_like_burst() {
        let mut samples = vec![0.0f32; TARGET_SAMPLE_RATE as usize];
        for s in &mut samples[8000..16000] {
            *s = 0.2;
        }
        let trimmed = trim_silence(&samples, TARGET_SAMPLE_RATE).expect("speech");
        assert!(trimmed.len() < samples.len());
        assert!(!trimmed.is_empty());
    }

    #[test]
    fn resample_identity_at_16k() {
        let s = vec![0.1, -0.2, 0.3];
        assert_eq!(resample_to_16k(&s, 16_000), s);
    }

    #[test]
    fn resample_halves_when_doubling_rate() {
        let s: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let out = resample_to_16k(&s, 32_000);
        assert!((out.len() as i32 - 50).abs() <= 1);
    }

    #[test]
    fn empty_target_omits_pw_record_target_flag() {
        for target in ["", "   "] {
            let args = pw_record_args(TARGET_SAMPLE_RATE, target);
            assert!(
                !args.iter().any(|arg| arg == "--target"),
                "target {target:?} argv {args:?}"
            );
            assert_eq!(args.last().map(String::as_str), Some("-"));
        }
    }

    #[test]
    fn set_target_passes_pw_record_target() {
        let id = "alsa_input.pci-0000_00_1f.3.analog-stereo";
        let args = pw_record_args(TARGET_SAMPLE_RATE, id);
        let pos = args.iter().position(|arg| arg == "--target").expect("flag");
        assert_eq!(args[pos + 1], id);
        assert_eq!(args.iter().filter(|arg| *arg == "--target").count(), 1);
        assert!(
            pos + 1 < args.len() - 1,
            "--target stays before the file operand"
        );
    }

    #[test]
    fn pw_record_is_on_path_or_skip() {
        // Document runtime dependency; do not fail CI without PipeWire.
        if which_pw_record().is_none() {
            eprintln!("pw-record not on PATH — mic capture will fail at runtime");
        }
    }

    fn which_pw_record() -> Option<std::path::PathBuf> {
        std::env::var_os("PATH").and_then(|paths| {
            for dir in std::env::split_paths(&paths) {
                let p = dir.join("pw-record");
                if p.is_file() {
                    return Some(p);
                }
            }
            None
        })
    }
}

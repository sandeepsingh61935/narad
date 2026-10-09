//! Mic capture via PipeWire `pw-record` + energy VAD trim (FR-AUD-1/2/6).
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
    pub fn start() -> Result<Self> {
        let sample_rate = TARGET_SAMPLE_RATE;
        let mut child = Command::new("pw-record")
            .args([
                "--rate",
                &sample_rate.to_string(),
                "--channels",
                "1",
                "--format",
                "f32",
                "--raw",
                "-",
            ])
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

    /// Stop capture and return mono samples at the capture rate.
    pub fn stop(mut self) -> Result<(Vec<f32>, i32)> {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(h) = self.reader.take() {
            let _ = h.join();
        }
        // Brief settle so the reader exits.
        thread::sleep(Duration::from_millis(20));
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

fn read_pcm_loop(
    mut stdout: ChildStdout,
    stop: Arc<AtomicBool>,
    samples: Arc<Mutex<Vec<f32>>>,
    rms_bits: Arc<AtomicU32>,
) {
    let mut buf = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
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

/// Energy-gate VAD: trim leading/trailing low-energy frames (FR-AUD-2).
/// Returns `None` when the clip looks like silence/noise only.
pub fn trim_silence(samples: &[f32], sample_rate: i32) -> Option<Vec<f32>> {
    if samples.is_empty() {
        return None;
    }
    let frame = (sample_rate as usize / 100).max(80); // 10 ms
    let threshold = {
        let mut peak = 0.0f32;
        for chunk in samples.chunks(frame) {
            let e = rms_of(chunk);
            if e > peak {
                peak = e;
            }
        }
        (peak * 0.08).max(0.01)
    };

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

    let pad = (sample_rate as usize) / 10;
    let start = start.saturating_sub(pad);
    let end = (end + pad).min(samples.len());
    let trimmed = samples[start..end].to_vec();

    if trimmed.len() < sample_rate as usize / 5 {
        return None;
    }
    if rms_of(&trimmed) < 0.008 {
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

#[cfg(test)]
mod tests {
    use super::*;

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

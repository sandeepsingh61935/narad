//! Offline ASR via sherpa-onnx (FR-ASR-1/2/5, FR-MDL-6).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use sherpa_onnx::{
    OfflineNemoEncDecCtcModelConfig, OfflineRecognizer, OfflineRecognizerConfig,
};
use tracing::{debug, info, warn};

/// Default pack id (REQUIREMENTS §8 decision #1).
pub const DEFAULT_ASR_ID: &str = "parakeet-ctc-110m-int8";

/// No pack in this phase has a hotword path. Parakeet CTC stays post-correction only (FR-TXT-2).
pub fn supports_vocabulary_bias(_model_id: &str) -> bool {
    false
}

/// Warm-loaded English CTC recogniser.
pub struct AsrEngine {
    recognizer: OfflineRecognizer,
    pub model_id: String,
    #[allow(dead_code)]
    pub model_dir: PathBuf,
}

impl AsrEngine {
    /// Load `model_id` from the resolved pack directory and keep it resident (FR-ASR-2).
    pub fn load(model_id: &str) -> Result<Self> {
        let model_dir = resolve_model_dir(model_id)?;
        info!(
            model_id,
            dir = %model_dir.display(),
            "loading ASR model"
        );

        let onnx = find_file(&model_dir, &["model.int8.onnx", "model.onnx"])?;
        let tokens = find_file(&model_dir, &["tokens.txt"])?;

        let mut config = OfflineRecognizerConfig::default();
        config.model_config.num_threads = default_threads();
        config.model_config.provider = Some("cpu".into());
        config.model_config.tokens = Some(tokens.display().to_string());
        config.model_config.nemo_ctc = OfflineNemoEncDecCtcModelConfig {
            model: Some(onnx.display().to_string()),
        };

        let recognizer = OfflineRecognizer::create(&config)
            .ok_or_else(|| anyhow!("OfflineRecognizer::create failed for {model_id}"))?;

        info!(model_id, "ASR model warm");
        Ok(Self {
            recognizer,
            model_id: model_id.to_string(),
            model_dir,
        })
    }

    /// Decode mono PCM; returns trimmed text (may be empty).
    pub fn transcribe(&self, samples: &[f32], sample_rate: i32) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(sample_rate, samples);
        self.recognizer.decode(&stream);
        let text = stream
            .get_result()
            .map(|r| r.text)
            .unwrap_or_default()
            .trim()
            .to_string();
        debug!(len = text.len(), "ASR raw text");
        Ok(text)
    }
}

/// Drop known phantom phrases and empty/noise-only outputs (FR-ASR-5).
pub fn is_hallucination(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return true;
    }
    let lower = t.to_ascii_lowercase();
    const PHANTOMS: &[&str] = &[
        "thanks for watching",
        "thank you for watching",
        "thanks for watching!",
        "subscribe",
        "please subscribe",
        "thank you",
        "thanks",
        ".",
        "...",
    ];
    for p in PHANTOMS {
        if lower == *p || lower.trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace()) == *p
        {
            return true;
        }
    }
    // Very short non-lexical noise tokens
    if t.chars().count() <= 1 {
        return true;
    }
    false
}

/// `~/.local/share/narad/models/asr/<id>/`, with spike fallback for local dev.
pub fn resolve_model_dir(model_id: &str) -> Result<PathBuf> {
    for candidate in candidate_dirs(model_id) {
        if candidate.is_dir()
            && (candidate.join("model.int8.onnx").is_file() || candidate.join("model.onnx").is_file())
            && candidate.join("tokens.txt").is_file()
        {
            return Ok(candidate);
        }
    }
    let xdg = xdg_asr_dir(model_id);
    bail!(
        "ASR model {:?} not found under {}\n  symlink from spike:\n  mkdir -p ~/.local/share/narad/models/asr && \\\n  ln -sfn <repo>/spikes/asr-bench/models/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8 \\\n    ~/.local/share/narad/models/asr/parakeet-ctc-110m-int8",
        model_id,
        xdg.display()
    )
}

fn candidate_dirs(model_id: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.push(xdg_asr_dir(model_id));

    // Dev fallback: spike extract dir when running from the workspace.
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(manifest)
            .join("../..")
            .join("spikes/asr-bench/models");
        if model_id == DEFAULT_ASR_ID || model_id == "parakeet-ctc-110m-int8" {
            dirs.push(
                root.join("sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8"),
            );
        }
        dirs.push(root.join(model_id));
    }

    // Relative to cwd (cargo run from repo root).
    dirs.push(PathBuf::from(format!(
        "spikes/asr-bench/models/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8"
    )));
    dirs.push(PathBuf::from(format!(
        "spikes/asr-bench/models/{model_id}"
    )));
    dirs
}

fn xdg_asr_dir(model_id: &str) -> PathBuf {
    crate::paths::resolve_data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("models")
        .join("asr")
        .join(model_id)
}

fn find_file(dir: &Path, names: &[&str]) -> Result<PathBuf> {
    for name in names {
        let p = dir.join(name);
        if p.is_file() {
            return Ok(p);
        }
    }
    Err(anyhow!(
        "none of {:?} found in {}",
        names,
        dir.display()
    ))
}

fn default_threads() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4)
        .min(8)
}

/// Best-effort load; logs and returns None if the pack is missing.
pub fn try_load(model_id: &str) -> Option<AsrEngine> {
    match AsrEngine::load(model_id) {
        Ok(engine) => Some(engine),
        Err(err) => {
            warn!(error = %err, model_id, "ASR model not loaded; dictation will fail until installed");
            None
        }
    }
}

/// First-run install into XDG when the pack is missing (FR-MDL-6).
///
/// Symlinks (or copies) from the asr-bench spike extract when present; otherwise
/// logs a clear install hint and leaves the XDG path absent.
pub fn ensure_default_pack_installed(model_id: &str) {
    let xdg = xdg_asr_dir(model_id);
    if pack_looks_complete(&xdg) {
        info!(dir = %xdg.display(), model_id, "ASR pack present under XDG");
        return;
    }

    if let Some(parent) = xdg.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            warn!(
                error = %err,
                dir = %parent.display(),
                "could not create XDG ASR models directory"
            );
            return;
        }
    }

    let Some(source) = find_spike_pack(model_id) else {
        warn!(
            model_id,
            xdg = %xdg.display(),
            "ASR pack missing under XDG and no spike source found; install with:\n  \
             mkdir -p ~/.local/share/narad/models/asr && \\\n  \
             ln -sfn <repo>/spikes/asr-bench/models/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8 \\\n  \
             ~/.local/share/narad/models/asr/{model_id}\n  \
             (or run spikes/asr-bench/scripts/download-models.sh)"
        );
        return;
    };

    match install_pack_link(&source, &xdg) {
        Ok(()) => info!(
            model_id,
            from = %source.display(),
            to = %xdg.display(),
            "installed default ASR pack into XDG (FR-MDL-6)"
        ),
        Err(err) => warn!(
            error = %err,
            from = %source.display(),
            to = %xdg.display(),
            "failed to install ASR pack into XDG"
        ),
    }
}

fn pack_looks_complete(dir: &Path) -> bool {
    dir.is_dir()
        && (dir.join("model.int8.onnx").is_file() || dir.join("model.onnx").is_file())
        && dir.join("tokens.txt").is_file()
}

fn find_spike_pack(model_id: &str) -> Option<PathBuf> {
    for candidate in spike_source_dirs(model_id) {
        if pack_looks_complete(&candidate) {
            return candidate.canonicalize().ok().or(Some(candidate));
        }
    }
    None
}

fn spike_source_dirs(model_id: &str) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let spike_name = "sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8";
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(manifest).join("../..").join("spikes/asr-bench/models");
        if model_id == DEFAULT_ASR_ID || model_id == "parakeet-ctc-110m-int8" {
            dirs.push(root.join(spike_name));
        }
        dirs.push(root.join(model_id));
    }
    dirs.push(PathBuf::from(format!("spikes/asr-bench/models/{spike_name}")));
    dirs.push(PathBuf::from(format!("spikes/asr-bench/models/{model_id}")));
    dirs
}

fn install_pack_link(source: &Path, dest: &Path) -> Result<()> {
    if dest.exists() || dest.symlink_metadata().is_ok() {
        // Broken or incomplete prior install — replace.
        if dest.is_dir() && !dest.is_symlink() {
            bail!(
                "incomplete pack at {} (not a symlink); remove it and retry",
                dest.display()
            );
        }
        std::fs::remove_file(dest).or_else(|_| std::fs::remove_dir_all(dest))?;
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, dest).with_context(|| {
            format!("symlink {} → {}", source.display(), dest.display())
        })?;
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        // Best-effort directory copy when symlinks are unavailable.
        copy_dir_all(source, dest)?;
        Ok(())
    }
}

#[cfg(not(unix))]
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parakeet_ctc_has_no_vocabulary_bias() {
        assert!(!supports_vocabulary_bias(DEFAULT_ASR_ID));
        assert!(!supports_vocabulary_bias("parakeet-ctc-110m-int8"));
    }

    #[test]
    fn hallucination_filter_catches_phantoms() {
        assert!(is_hallucination(""));
        assert!(is_hallucination("   "));
        assert!(is_hallucination("Thanks for watching"));
        assert!(is_hallucination("thank you"));
        assert!(is_hallucination("."));
        assert!(!is_hallucination("Hello world"));
        assert!(!is_hallucination("Narad is a voice stack"));
    }

    #[test]
    fn resolve_finds_spike_or_xdg() {
        // Symlink or spike extract should satisfy at least one candidate in this workspace.
        let result = resolve_model_dir(DEFAULT_ASR_ID);
        if let Err(e) = &result {
            // Allow failure only when neither path exists (CI without models).
            let xdg = xdg_asr_dir(DEFAULT_ASR_ID);
            let spike = PathBuf::from(
                "spikes/asr-bench/models/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8",
            );
            if !xdg.is_dir() && !spike.is_dir() {
                return;
            }
            panic!("expected model dir, got {e}");
        }
        let dir = result.unwrap();
        assert!(dir.join("tokens.txt").is_file());
    }
}

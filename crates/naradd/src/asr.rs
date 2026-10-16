//! Offline ASR via sherpa-onnx (FR-ASR-1/2/5, FR-MDL-6).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use sherpa_onnx::{
    OfflineMoonshineModelConfig, OfflineNemoEncDecCtcModelConfig, OfflineRecognizer,
    OfflineRecognizerConfig, OfflineTransducerModelConfig, OfflineWhisperModelConfig,
};
use tracing::{debug, info, warn};

/// Default pack id (REQUIREMENTS §8 decision #1).
pub const DEFAULT_ASR_ID: &str = "parakeet-ctc-110m-int8";

/// Supported Phase 3 ASR model architectures (FR-ASR-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsrArchitecture {
    Transducer,
    Moonshine,
    Whisper,
    Ctc,
}

pub fn detect_architecture(dir: &Path) -> AsrArchitecture {
    if dir.join("joiner.int8.onnx").is_file() || dir.join("joiner.onnx").is_file() {
        AsrArchitecture::Transducer
    } else if dir.join("preprocess.onnx").is_file()
        || dir.join("preprocessor.onnx").is_file()
        || dir.join("encode.int8.onnx").is_file()
        || dir.join("encode.onnx").is_file()
    {
        AsrArchitecture::Moonshine
    } else if has_whisper_files(dir) {
        AsrArchitecture::Whisper
    } else {
        AsrArchitecture::Ctc
    }
}

fn has_whisper_files(dir: &Path) -> bool {
    if dir.join("small.en-decoder").is_file()
        || dir.join("small.en-decoder.int8.onnx").is_file()
        || dir.join("small.en-decoder.onnx").is_file()
    {
        return true;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let s = name.to_string_lossy();
            if s.contains("small.en-decoder") || s.contains("whisper") {
                return true;
            }
        }
    }
    false
}

/// No pack in this phase has a hotword path. Parakeet CTC stays post-correction only (FR-TXT-2).
pub fn supports_vocabulary_bias(_model_id: &str) -> bool {
    false
}

/// Warm-loaded English ASR recogniser.
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

        let arch = detect_architecture(&model_dir);
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.num_threads = default_threads();
        config.model_config.provider = Some("cpu".into());

        match arch {
            AsrArchitecture::Transducer => {
                let encoder = find_file(&model_dir, &["encoder.int8.onnx", "encoder.onnx"])?;
                let decoder = find_file(&model_dir, &["decoder.int8.onnx", "decoder.onnx"])?;
                let joiner = find_file(&model_dir, &["joiner.int8.onnx", "joiner.onnx"])?;
                let tokens = find_file(&model_dir, &["tokens.txt"])?;
                config.model_config.tokens = Some(tokens.display().to_string());
                config.model_config.transducer = OfflineTransducerModelConfig {
                    encoder: Some(encoder.display().to_string()),
                    decoder: Some(decoder.display().to_string()),
                    joiner: Some(joiner.display().to_string()),
                };
            }
            AsrArchitecture::Moonshine => {
                let preprocessor =
                    find_file(&model_dir, &["preprocess.onnx", "preprocessor.onnx"])?;
                let encoder = find_file(
                    &model_dir,
                    &[
                        "encode.int8.onnx",
                        "encode.onnx",
                        "encoder.int8.onnx",
                        "encoder.onnx",
                    ],
                )?;
                let uncached_decoder = find_file(
                    &model_dir,
                    &[
                        "uncached_decode.int8.onnx",
                        "uncached_decode.onnx",
                        "uncached_decoder.int8.onnx",
                        "uncached_decoder.onnx",
                    ],
                )?;
                let cached_decoder = find_file(
                    &model_dir,
                    &[
                        "cached_decode.int8.onnx",
                        "cached_decode.onnx",
                        "cached_decoder.int8.onnx",
                        "cached_decoder.onnx",
                    ],
                )?;
                let tokens = find_file(&model_dir, &["tokens.txt"])?;
                config.model_config.tokens = Some(tokens.display().to_string());
                config.model_config.moonshine = OfflineMoonshineModelConfig {
                    preprocessor: Some(preprocessor.display().to_string()),
                    encoder: Some(encoder.display().to_string()),
                    uncached_decoder: Some(uncached_decoder.display().to_string()),
                    cached_decoder: Some(cached_decoder.display().to_string()),
                    merged_decoder: None,
                };
            }
            AsrArchitecture::Whisper => {
                let encoder = find_file_with_filter(
                    &model_dir,
                    &[
                        "small.en-encoder.int8.onnx",
                        "small.en-encoder.onnx",
                        "encoder.int8.onnx",
                        "encoder.onnx",
                    ],
                    |s| s.contains("encoder") && s.ends_with(".onnx"),
                )?;
                let decoder = find_file_with_filter(
                    &model_dir,
                    &[
                        "small.en-decoder.int8.onnx",
                        "small.en-decoder.onnx",
                        "decoder.int8.onnx",
                        "decoder.onnx",
                    ],
                    |s| s.contains("decoder") && s.ends_with(".onnx"),
                )?;
                let tokens = find_file_with_filter(
                    &model_dir,
                    &["small.en-tokens.txt", "tokens.txt"],
                    |s| s.ends_with("tokens.txt"),
                )?;
                config.model_config.tokens = Some(tokens.display().to_string());
                config.model_config.whisper = OfflineWhisperModelConfig {
                    encoder: Some(encoder.display().to_string()),
                    decoder: Some(decoder.display().to_string()),
                    language: Some("en".into()),
                    task: Some("transcribe".into()),
                    tail_paddings: 0,
                    enable_token_timestamps: false,
                    enable_segment_timestamps: false,
                };
            }
            AsrArchitecture::Ctc => {
                let onnx = find_file(&model_dir, &["model.int8.onnx", "model.onnx"])?;
                let tokens = find_file(&model_dir, &["tokens.txt"])?;
                config.model_config.tokens = Some(tokens.display().to_string());
                config.model_config.nemo_ctc = OfflineNemoEncDecCtcModelConfig {
                    model: Some(onnx.display().to_string()),
                };
            }
        }

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
        if pack_looks_complete(&candidate) {
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

    let known_spike_names = match model_id {
        DEFAULT_ASR_ID => {
            vec!["sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8"]
        }
        "parakeet-tdt-0.6b-int8" | "parakeet-tdt-0.6b-v2-int8" => {
            vec!["sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8"]
        }
        "moonshine-base-en-int8" => vec!["sherpa-onnx-moonshine-base-en-int8"],
        "whisper-small-en" | "whisper-small.en" => vec!["sherpa-onnx-whisper-small.en"],
        _ => vec![],
    };

    // Dev fallback: spike extract dir when running from the workspace.
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(manifest)
            .join("../..")
            .join("spikes/asr-bench/models");
        for name in &known_spike_names {
            dirs.push(root.join(name));
        }
        dirs.push(root.join(format!("sherpa-onnx-{model_id}")));
        dirs.push(root.join(model_id));
    }

    // Relative to cwd (cargo run from repo root).
    for name in &known_spike_names {
        dirs.push(PathBuf::from(format!("spikes/asr-bench/models/{name}")));
    }
    dirs.push(PathBuf::from(format!(
        "spikes/asr-bench/models/sherpa-onnx-{model_id}"
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

fn find_file_with_filter<F>(dir: &Path, exact: &[&str], filter: F) -> Result<PathBuf>
where
    F: Fn(&str) -> bool,
{
    for name in exact {
        let p = dir.join(name);
        if p.is_file() {
            return Ok(p);
        }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut matches = Vec::new();
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
                    let name = entry.file_name();
                    let s = name.to_string_lossy();
                    if filter(&s) {
                        matches.push(entry.path());
                    }
                }
            }
        }
        matches.sort();
        if let Some(first) = matches.into_iter().next() {
            return Ok(first);
        }
    }
    Err(anyhow!("no matching file found in {}", dir.display()))
}

pub fn pack_looks_complete(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    match detect_architecture(dir) {
        AsrArchitecture::Transducer => {
            (dir.join("joiner.int8.onnx").is_file() || dir.join("joiner.onnx").is_file())
                && (dir.join("encoder.int8.onnx").is_file() || dir.join("encoder.onnx").is_file())
                && (dir.join("decoder.int8.onnx").is_file() || dir.join("decoder.onnx").is_file())
                && dir.join("tokens.txt").is_file()
        }
        AsrArchitecture::Moonshine => {
            (dir.join("preprocess.onnx").is_file() || dir.join("preprocessor.onnx").is_file())
                && (dir.join("encode.int8.onnx").is_file()
                    || dir.join("encode.onnx").is_file()
                    || dir.join("encoder.int8.onnx").is_file()
                    || dir.join("encoder.onnx").is_file())
                && (dir.join("uncached_decode.int8.onnx").is_file()
                    || dir.join("uncached_decode.onnx").is_file()
                    || dir.join("uncached_decoder.int8.onnx").is_file()
                    || dir.join("uncached_decoder.onnx").is_file())
                && (dir.join("cached_decode.int8.onnx").is_file()
                    || dir.join("cached_decode.onnx").is_file()
                    || dir.join("cached_decoder.int8.onnx").is_file()
                    || dir.join("cached_decoder.onnx").is_file())
                && dir.join("tokens.txt").is_file()
        }
        AsrArchitecture::Whisper => {
            find_file_with_filter(
                dir,
                &[
                    "small.en-encoder.int8.onnx",
                    "small.en-encoder.onnx",
                    "encoder.int8.onnx",
                    "encoder.onnx",
                ],
                |s| s.contains("encoder") && s.ends_with(".onnx"),
            )
            .is_ok()
                && find_file_with_filter(
                    dir,
                    &[
                        "small.en-decoder.int8.onnx",
                        "small.en-decoder.onnx",
                        "decoder.int8.onnx",
                        "decoder.onnx",
                    ],
                    |s| s.contains("decoder") && s.ends_with(".onnx"),
                )
                .is_ok()
                && find_file_with_filter(
                    dir,
                    &["small.en-tokens.txt", "tokens.txt"],
                    |s| s.ends_with("tokens.txt"),
                )
                .is_ok()
        }
        AsrArchitecture::Ctc => {
            (dir.join("model.int8.onnx").is_file() || dir.join("model.onnx").is_file())
                && dir.join("tokens.txt").is_file()
        }
    }
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
    let known_spike_names = match model_id {
        DEFAULT_ASR_ID => {
            vec!["sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8"]
        }
        "parakeet-tdt-0.6b-int8" | "parakeet-tdt-0.6b-v2-int8" => {
            vec!["sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8"]
        }
        "moonshine-base-en-int8" => vec!["sherpa-onnx-moonshine-base-en-int8"],
        "whisper-small-en" | "whisper-small.en" => vec!["sherpa-onnx-whisper-small.en"],
        _ => vec![],
    };

    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let root = PathBuf::from(manifest).join("../..").join("spikes/asr-bench/models");
        for name in &known_spike_names {
            dirs.push(root.join(name));
        }
        dirs.push(root.join(format!("sherpa-onnx-{model_id}")));
        dirs.push(root.join(model_id));
    }
    for name in &known_spike_names {
        dirs.push(PathBuf::from(format!("spikes/asr-bench/models/{name}")));
    }
    dirs.push(PathBuf::from(format!("spikes/asr-bench/models/sherpa-onnx-{model_id}")));
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

    #[test]
    fn detect_architecture_and_pack_completeness() {
        use tempfile::TempDir;
        let dir = TempDir::new().expect("tempdir");

        // Transducer
        let td_dir = dir.path().join("td");
        std::fs::create_dir_all(&td_dir).unwrap();
        std::fs::write(td_dir.join("joiner.int8.onnx"), b"").unwrap();
        std::fs::write(td_dir.join("encoder.int8.onnx"), b"").unwrap();
        std::fs::write(td_dir.join("decoder.int8.onnx"), b"").unwrap();
        std::fs::write(td_dir.join("tokens.txt"), b"").unwrap();
        assert_eq!(detect_architecture(&td_dir), AsrArchitecture::Transducer);
        assert!(pack_looks_complete(&td_dir));

        // Moonshine
        let ms_dir = dir.path().join("ms");
        std::fs::create_dir_all(&ms_dir).unwrap();
        std::fs::write(ms_dir.join("preprocess.onnx"), b"").unwrap();
        std::fs::write(ms_dir.join("encode.int8.onnx"), b"").unwrap();
        std::fs::write(ms_dir.join("uncached_decode.int8.onnx"), b"").unwrap();
        std::fs::write(ms_dir.join("cached_decode.int8.onnx"), b"").unwrap();
        std::fs::write(ms_dir.join("tokens.txt"), b"").unwrap();
        assert_eq!(detect_architecture(&ms_dir), AsrArchitecture::Moonshine);
        assert!(pack_looks_complete(&ms_dir));

        // Moonshine with preprocessor.onnx
        let ms2_dir = dir.path().join("ms2");
        std::fs::create_dir_all(&ms2_dir).unwrap();
        std::fs::write(ms2_dir.join("preprocessor.onnx"), b"").unwrap();
        std::fs::write(ms2_dir.join("encode.int8.onnx"), b"").unwrap();
        std::fs::write(ms2_dir.join("uncached_decode.int8.onnx"), b"").unwrap();
        std::fs::write(ms2_dir.join("cached_decode.int8.onnx"), b"").unwrap();
        std::fs::write(ms2_dir.join("tokens.txt"), b"").unwrap();
        assert_eq!(detect_architecture(&ms2_dir), AsrArchitecture::Moonshine);
        assert!(pack_looks_complete(&ms2_dir));

        // Whisper
        let wh_dir = dir.path().join("wh");
        std::fs::create_dir_all(&wh_dir).unwrap();
        std::fs::write(wh_dir.join("small.en-decoder.int8.onnx"), b"").unwrap();
        std::fs::write(wh_dir.join("small.en-encoder.int8.onnx"), b"").unwrap();
        std::fs::write(wh_dir.join("small.en-tokens.txt"), b"").unwrap();
        assert_eq!(detect_architecture(&wh_dir), AsrArchitecture::Whisper);
        assert!(pack_looks_complete(&wh_dir));

        // CTC
        let ctc_dir = dir.path().join("ctc");
        std::fs::create_dir_all(&ctc_dir).unwrap();
        std::fs::write(ctc_dir.join("model.int8.onnx"), b"").unwrap();
        std::fs::write(ctc_dir.join("tokens.txt"), b"").unwrap();
        assert_eq!(detect_architecture(&ctc_dir), AsrArchitecture::Ctc);
        assert!(pack_looks_complete(&ctc_dir));
    }
}

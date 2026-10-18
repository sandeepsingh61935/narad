//! Offline and online streaming ASR via sherpa-onnx (FR-ASR-1/2/5, FR-MDL-6, Task 1).

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use sherpa_onnx::{
    OfflineMoonshineModelConfig, OfflineNemoEncDecCtcModelConfig, OfflineRecognizer,
    OfflineRecognizerConfig, OfflineTransducerModelConfig, OfflineWhisperModelConfig,
    OnlineRecognizer, OnlineRecognizerConfig, OnlineStream, OnlineTransducerModelConfig,
    OnlineZipformer2CtcModelConfig,
};
use tracing::{debug, info, warn};

/// Default pack id (REQUIREMENTS §8 decision #1).
pub const DEFAULT_ASR_ID: &str = "parakeet-ctc-110m-int8";

/// Default streaming online ASR pack id (Task 1 / Phase 5).
pub const DEFAULT_ONLINE_ASR_ID: &str = "sherpa-onnx-streaming-zipformer-en-2023-06-26-int8";

/// Supported Phase 3 ASR model architectures (FR-ASR-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsrArchitecture {
    Transducer,
    Moonshine,
    Whisper,
    Ctc,
}

pub fn detect_architecture(dir: &Path) -> AsrArchitecture {
    if dir.join("joiner.int8.onnx").is_file()
        || dir.join("joiner.onnx").is_file()
        || has_joiner_files(dir)
    {
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

fn has_joiner_files(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let s = name.to_string_lossy();
            if s.contains("joiner") && s.ends_with(".onnx") {
                return true;
            }
        }
    }
    false
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
        if model_id.contains("streaming") || model_dir.to_string_lossy().contains("streaming") {
            bail!("model {model_id} is an online streaming model, not an offline model; load via OnlineAsrEngine");
        }
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
                let encoder = find_file_with_filter(
                    &model_dir,
                    &["encoder.int8.onnx", "encoder.onnx"],
                    |s| s.contains("encoder") && s.ends_with(".onnx"),
                )?;
                let decoder = find_file_with_filter(
                    &model_dir,
                    &["decoder.int8.onnx", "decoder.onnx"],
                    |s| s.contains("decoder") && s.ends_with(".onnx"),
                )?;
                let joiner = find_file_with_filter(
                    &model_dir,
                    &["joiner.int8.onnx", "joiner.onnx"],
                    |s| s.contains("joiner") && s.ends_with(".onnx"),
                )?;
                let tokens = find_file_with_filter(
                    &model_dir,
                    &["tokens.txt"],
                    |s| s.ends_with("tokens.txt"),
                )?;
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

/// Real-time punctuation mapping for streaming dictation.
/// Capitalize first letter of sentences and lowercase others for all-caps transcripts.
pub fn to_sentence_case(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;
    for c in text.chars() {
        if c.is_alphabetic() {
            if capitalize_next {
                result.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                result.extend(c.to_lowercase());
            }
        } else {
            result.push(c);
            if c == '.' || c == '?' || c == '!' {
                capitalize_next = true;
            }
        }
    }
    result
}

/// Real-time punctuation mapping for streaming dictation.
///
/// Replaces standalone spoken tokens:
/// - "comma" -> ","
/// - "period" -> "."
/// - "question mark" -> "?"
/// - "exclamation mark" -> "!"
/// And normalizes all-caps LibriSpeech model output into natural sentence casing.
pub fn format_streaming_punctuation(text: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return String::new();
    }
    let is_all_caps = text.chars().any(|c| c.is_alphabetic())
        && text.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase());

    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut i = 0;

    while i < tokens.len() {
        let token = tokens[i];
        let lower = token.to_ascii_lowercase();

        // Check 2-word tokens first: "question mark", "exclamation mark"
        if let Some(next_tok) = tokens.get(i + 1) {
            let next_lower = next_tok.to_ascii_lowercase();
            if lower == "question" && next_lower == "mark" {
                result.push('?');
                i += 2;
                continue;
            } else if lower == "exclamation" && next_lower == "mark" {
                result.push('!');
                i += 2;
                continue;
            }
        }

        // Check 1-word tokens: "comma", "period"
        if lower == "comma" {
            result.push(',');
            i += 1;
            continue;
        } else if lower == "period" {
            result.push('.');
            i += 1;
            continue;
        }

        // Regular word token
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(token);
        i += 1;
    }

    if is_all_caps {
        to_sentence_case(&result)
    } else {
        result
    }
}

/// Map a standalone punctuation token or phrase to its corresponding symbol.
pub fn map_punctuation_token(token: &str) -> Option<&'static str> {
    let normalized = token.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "comma" => Some(","),
        "period" => Some("."),
        "question mark" => Some("?"),
        "exclamation mark" => Some("!"),
        _ => None,
    }
}

/// Streaming online ASR stream wrapper.
pub struct OnlineAsrStream {
    stream: OnlineStream,
}

impl OnlineAsrStream {
    pub fn new(stream: OnlineStream) -> Self {
        Self { stream }
    }

    /// Ingest a mono PCM audio chunk.
    pub fn accept_waveform(&mut self, sample_rate: u32, samples: &[f32]) {
        self.stream.accept_waveform(sample_rate as i32, samples);
    }

    /// Decode available audio frames and return the latest punctuation-mapped hypothesis,
    /// or None if no non-empty hypothesis is available yet.
    pub fn decode(&mut self, recognizer: &OnlineRecognizer) -> Option<String> {
        while recognizer.is_ready(&self.stream) {
            recognizer.decode(&self.stream);
        }
        let res = recognizer.get_result(&self.stream)?;
        let trimmed = res.text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(format_streaming_punctuation(trimmed))
        }
    }

    /// Finish input and return final punctuation-mapped hypothesis.
    pub fn finish(&mut self, recognizer: &OnlineRecognizer) -> String {
        self.stream.input_finished();
        while recognizer.is_ready(&self.stream) {
            recognizer.decode(&self.stream);
        }
        recognizer
            .get_result(&self.stream)
            .map(|r| format_streaming_punctuation(r.text.trim()))
            .unwrap_or_default()
    }

    /// Reset stream state after clause or utterance boundary.
    #[allow(dead_code)]
    pub fn reset(&mut self, recognizer: &OnlineRecognizer) {
        recognizer.reset(&self.stream);
    }

    /// Check if recognizer detected an endpoint.
    #[allow(dead_code)]
    pub fn is_endpoint(&self, recognizer: &OnlineRecognizer) -> bool {
        recognizer.is_endpoint(&self.stream)
    }

    /// Access the underlying sherpa-onnx OnlineStream.
    #[allow(dead_code)]
    pub fn inner(&self) -> &OnlineStream {
        &self.stream
    }

    /// Mutable access to the underlying sherpa-onnx OnlineStream.
    #[allow(dead_code)]
    pub fn inner_mut(&mut self) -> &mut OnlineStream {
        &mut self.stream
    }
}

/// Warm-loaded streaming online ASR engine.
pub struct OnlineAsrEngine {
    pub recognizer: OnlineRecognizer,
    #[allow(dead_code)]
    pub model_id: String,
    #[allow(dead_code)]
    pub model_dir: PathBuf,
}

fn validate_model_file(p: &Path) -> Result<()> {
    let meta = std::fs::metadata(p).with_context(|| format!("reading metadata for {}", p.display()))?;
    if meta.len() == 0 {
        bail!("model file is empty: {}", p.display());
    }
    if p.extension().map(|e| e == "onnx").unwrap_or(false) && meta.len() < 64 {
        bail!("model file is too small to be a valid ONNX model: {}", p.display());
    }
    Ok(())
}

impl OnlineAsrEngine {
    /// Load `model_id` from the resolved pack directory.
    pub fn load(model_id: &str) -> Result<Self> {
        let model_dir = resolve_model_dir(model_id)?;
        info!(
            model_id,
            dir = %model_dir.display(),
            "loading streaming online ASR model"
        );

        let arch = detect_architecture(&model_dir);
        let mut config = OnlineRecognizerConfig::default();
        config.model_config.num_threads = default_threads();
        config.model_config.provider = Some("cpu".into());
        config.decoding_method = Some("greedy_search".into());
        config.enable_endpoint = false;

        match arch {
            AsrArchitecture::Transducer => {
                let encoder = find_file_with_filter(
                    &model_dir,
                    &["encoder.int8.onnx", "encoder.onnx"],
                    |s| s.contains("encoder") && s.ends_with(".onnx"),
                )?;
                let decoder = find_file_with_filter(
                    &model_dir,
                    &["decoder.int8.onnx", "decoder.onnx"],
                    |s| s.contains("decoder") && s.ends_with(".onnx"),
                )?;
                let joiner = find_file_with_filter(
                    &model_dir,
                    &["joiner.int8.onnx", "joiner.onnx"],
                    |s| s.contains("joiner") && s.ends_with(".onnx"),
                )?;
                let tokens = find_file_with_filter(
                    &model_dir,
                    &["tokens.txt"],
                    |s| s.ends_with("tokens.txt"),
                )?;

                validate_model_file(&encoder)?;
                validate_model_file(&decoder)?;
                validate_model_file(&joiner)?;
                validate_model_file(&tokens)?;

                config.model_config.tokens = Some(tokens.display().to_string());
                config.model_config.transducer = OnlineTransducerModelConfig {
                    encoder: Some(encoder.display().to_string()),
                    decoder: Some(decoder.display().to_string()),
                    joiner: Some(joiner.display().to_string()),
                };
            }
            AsrArchitecture::Ctc => {
                if model_id.contains("parakeet") {
                    bail!("model {model_id} is an offline CTC model, not an online streaming model");
                }
                let onnx = find_file(&model_dir, &["model.int8.onnx", "model.onnx"])?;
                let tokens = find_file(&model_dir, &["tokens.txt"])?;
                validate_model_file(&onnx)?;
                validate_model_file(&tokens)?;

                config.model_config.tokens = Some(tokens.display().to_string());
                config.model_config.zipformer2_ctc = OnlineZipformer2CtcModelConfig {
                    model: Some(onnx.display().to_string()),
                };
            }
            _ => {
                bail!(
                    "unsupported online streaming ASR architecture {:?} for {model_id}",
                    arch
                );
            }
        }

        let recognizer = OnlineRecognizer::create(&config)
            .ok_or_else(|| anyhow!("OnlineRecognizer::create failed for {model_id}"))?;

        info!(model_id, "streaming online ASR model warm");
        Ok(Self {
            recognizer,
            model_id: model_id.to_string(),
            model_dir,
        })
    }

    /// Create a new online recognition stream.
    pub fn create_stream(&self) -> Result<OnlineAsrStream> {
        let stream = self.recognizer.create_stream();
        Ok(OnlineAsrStream::new(stream))
    }

    /// Transcribe a complete waveform buffer using this online recognizer.
    pub fn transcribe(&self, samples: &[f32], sample_rate: i32) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }
        let mut stream = self.create_stream()?;
        stream.accept_waveform(sample_rate as u32, samples);
        let text = stream.finish(&self.recognizer);
        Ok(text)
    }

    /// Access the underlying OnlineRecognizer.
    pub fn recognizer(&self) -> &OnlineRecognizer {
        &self.recognizer
    }

    /// Best-effort load; logs and returns None if the pack is missing.
    pub fn try_load(model_id: &str) -> Option<Self> {
        match Self::load(model_id) {
            Ok(engine) => Some(engine),
            Err(err) => {
                warn!(error = %err, model_id, "Online ASR model not loaded; streaming will fail until installed");
                None
            }
        }
    }

    /// Convenience wrapper to decode a stream with this engine.
    #[allow(dead_code)]
    pub fn decode_stream(&self, stream: &mut OnlineAsrStream) -> Option<String> {
        stream.decode(&self.recognizer)
    }

    /// Convenience wrapper to finish a stream with this engine.
    #[allow(dead_code)]
    pub fn finish_stream(&self, stream: &mut OnlineAsrStream) -> String {
        stream.finish(&self.recognizer)
    }
}

/// Best-effort online load function.
#[allow(dead_code)]
pub fn try_load_online(model_id: &str) -> Option<OnlineAsrEngine> {
    OnlineAsrEngine::try_load(model_id)
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
    let p = PathBuf::from(model_id);
    if p.is_dir() {
        dirs.push(p);
    }
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
        DEFAULT_ONLINE_ASR_ID
        | "streaming-zipformer-en-2023-06-26-int8"
        | "streaming-zipformer-en-int8"
        | "streaming-zipformer" => {
            vec![
                "sherpa-onnx-streaming-zipformer-en-2023-06-26-int8",
                "sherpa-onnx-streaming-zipformer-en-2023-06-26",
            ]
        }
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
            (dir.join("joiner.int8.onnx").is_file()
                || dir.join("joiner.onnx").is_file()
                || has_joiner_files(dir))
                && (dir.join("encoder.int8.onnx").is_file()
                    || dir.join("encoder.onnx").is_file()
                    || find_file_with_filter(dir, &[], |s| s.contains("encoder") && s.ends_with(".onnx")).is_ok())
                && (dir.join("decoder.int8.onnx").is_file()
                    || dir.join("decoder.onnx").is_file()
                    || find_file_with_filter(dir, &[], |s| s.contains("decoder") && s.ends_with(".onnx")).is_ok())
                && (dir.join("tokens.txt").is_file()
                    || find_file_with_filter(dir, &[], |s| s.ends_with("tokens.txt")).is_ok())
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
        DEFAULT_ONLINE_ASR_ID
        | "streaming-zipformer-en-2023-06-26-int8"
        | "streaming-zipformer-en-int8"
        | "streaming-zipformer" => {
            vec![
                "sherpa-onnx-streaming-zipformer-en-2023-06-26-int8",
                "sherpa-onnx-streaming-zipformer-en-2023-06-26",
            ]
        }
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

    #[test]
    fn online_streaming_zipformer_architecture_and_completeness() {
        use tempfile::TempDir;
        let dir = TempDir::new().expect("tempdir");

        // Standard Zipformer transducer files
        let zip_dir = dir.path().join("zipformer-std");
        std::fs::create_dir_all(&zip_dir).unwrap();
        std::fs::write(zip_dir.join("encoder.int8.onnx"), b"").unwrap();
        std::fs::write(zip_dir.join("decoder.int8.onnx"), b"").unwrap();
        std::fs::write(zip_dir.join("joiner.int8.onnx"), b"").unwrap();
        std::fs::write(zip_dir.join("tokens.txt"), b"").unwrap();
        assert_eq!(detect_architecture(&zip_dir), AsrArchitecture::Transducer);
        assert!(pack_looks_complete(&zip_dir));

        // Epoch/avg naming as commonly downloaded from sherpa-onnx releases
        let zip_avg_dir = dir.path().join("zipformer-avg");
        std::fs::create_dir_all(&zip_avg_dir).unwrap();
        std::fs::write(zip_avg_dir.join("encoder-epoch-99-avg-1.int8.onnx"), b"").unwrap();
        std::fs::write(zip_avg_dir.join("decoder-epoch-99-avg-1.onnx"), b"").unwrap();
        std::fs::write(zip_avg_dir.join("joiner-epoch-99-avg-1.int8.onnx"), b"").unwrap();
        std::fs::write(zip_avg_dir.join("tokens.txt"), b"").unwrap();
        assert_eq!(detect_architecture(&zip_avg_dir), AsrArchitecture::Transducer);
        assert!(pack_looks_complete(&zip_avg_dir));

        // Incomplete pack (missing joiner)
        let inc_dir = dir.path().join("incomplete");
        std::fs::create_dir_all(&inc_dir).unwrap();
        std::fs::write(inc_dir.join("encoder.int8.onnx"), b"").unwrap();
        std::fs::write(inc_dir.join("decoder.int8.onnx"), b"").unwrap();
        std::fs::write(inc_dir.join("tokens.txt"), b"").unwrap();
        assert_ne!(detect_architecture(&inc_dir), AsrArchitecture::Transducer);
        assert!(!pack_looks_complete(&inc_dir));
    }

    #[test]
    fn realtime_punctuation_token_mapping() {
        // Individual token mapping
        assert_eq!(map_punctuation_token("comma"), Some(","));
        assert_eq!(map_punctuation_token("Comma"), Some(","));
        assert_eq!(map_punctuation_token("period"), Some("."));
        assert_eq!(map_punctuation_token("Period"), Some("."));
        assert_eq!(map_punctuation_token("question mark"), Some("?"));
        assert_eq!(map_punctuation_token("Question Mark"), Some("?"));
        assert_eq!(map_punctuation_token("exclamation mark"), Some("!"));
        assert_eq!(map_punctuation_token("Exclamation Mark"), Some("!"));
        assert_eq!(map_punctuation_token("hello"), None);
        assert_eq!(map_punctuation_token("command"), None);
        assert_eq!(map_punctuation_token("periodic"), None);

        // Streaming text formatting
        assert_eq!(
            format_streaming_punctuation("hello comma world"),
            "hello, world"
        );
        assert_eq!(
            format_streaming_punctuation("this is a test period next sentence"),
            "this is a test. next sentence"
        );
        assert_eq!(
            format_streaming_punctuation("how are you question mark"),
            "how are you?"
        );
        assert_eq!(
            format_streaming_punctuation("look at that exclamation mark"),
            "look at that!"
        );
        assert_eq!(
            format_streaming_punctuation(
                "hello comma world period are you there question mark yes exclamation mark"
            ),
            "hello, world. are you there? yes!"
        );
        assert_eq!(
            format_streaming_punctuation("command periodic questionable mark"),
            "command periodic questionable mark"
        );
        assert_eq!(
            format_streaming_punctuation("comma at the start"),
            ", at the start"
        );
        assert_eq!(format_streaming_punctuation("comma"), ",");
        assert_eq!(format_streaming_punctuation("period"), ".");
        assert_eq!(format_streaming_punctuation("question mark"), "?");
        assert_eq!(format_streaming_punctuation("exclamation mark"), "!");
        assert_eq!(format_streaming_punctuation(""), "");
        assert_eq!(format_streaming_punctuation("   "), "");
    }

    #[test]
    fn online_asr_engine_mock_and_missing_error_handling() {
        use tempfile::TempDir;

        // Missing model path
        let err = OnlineAsrEngine::load("missing-model-definitely-does-not-exist");
        assert!(err.is_err());
        assert!(OnlineAsrEngine::try_load("missing-model-definitely-does-not-exist").is_none());

        // Mock model directory with empty files:
        // Directory resolution and architecture detection succeed, but ONNX loading fails cleanly.
        let dir = TempDir::new().expect("tempdir");
        let mock_dir = dir.path().join("mock-streaming-zipformer");
        std::fs::create_dir_all(&mock_dir).unwrap();
        std::fs::write(mock_dir.join("encoder.int8.onnx"), b"").unwrap();
        std::fs::write(mock_dir.join("decoder.int8.onnx"), b"").unwrap();
        std::fs::write(mock_dir.join("joiner.int8.onnx"), b"").unwrap();
        std::fs::write(mock_dir.join("tokens.txt"), b"dummy 0\n").unwrap();

        assert_eq!(detect_architecture(&mock_dir), AsrArchitecture::Transducer);
        assert!(pack_looks_complete(&mock_dir));

        let load_res = OnlineAsrEngine::load(mock_dir.to_str().unwrap());
        assert!(
            load_res.is_err(),
            "empty mock ONNX files should fail to initialize OnlineRecognizer"
        );
    }

    #[test]
    fn online_streaming_with_real_model_if_present() {
        if let Ok(dir) = resolve_model_dir(DEFAULT_ONLINE_ASR_ID) {
            if pack_looks_complete(&dir) {
                let engine = OnlineAsrEngine::load(DEFAULT_ONLINE_ASR_ID).expect("load online engine");
                let mut stream = engine.create_stream().expect("create online stream");

                // 100ms synthetic mono f32 chunk (1600 samples at 16kHz)
                let samples = vec![0.0f32; 1600];
                let start = std::time::Instant::now();
                stream.accept_waveform(16000, &samples);
                let _partial = stream.decode(engine.recognizer());
                let elapsed = start.elapsed();

                // Latency requirement: sub-50ms token emission step
                assert!(elapsed.as_millis() < 50, "step latency should be <50ms, took {:?}", elapsed);

                let final_text = stream.finish(engine.recognizer());
                let _ = final_text;

                // Live WAV audio streaming verification if test_wavs/0.wav is present
                let test_wav = dir.join("test_wavs").join("0.wav");
                if test_wav.is_file() {
                    let wave = sherpa_onnx::Wave::read(test_wav.to_str().unwrap())
                        .expect("read test wav");
                    let mut stream = engine.create_stream().expect("create online stream");
                    let chunk_size = (wave.sample_rate() as usize) / 10; // 100ms chunks
                    let all_samples = wave.samples();
                    let mut partials = Vec::new();

                    for chunk in all_samples.chunks(chunk_size) {
                        stream.accept_waveform(wave.sample_rate() as u32, chunk);
                        if let Some(partial) = stream.decode(engine.recognizer()) {
                            if !partial.trim().is_empty() {
                                partials.push(partial);
                            }
                        }
                    }
                    let final_result = stream.finish(engine.recognizer());
                    println!("Live streaming decoded partials count: {}", partials.len());
                    println!("Live streaming final decoded: '{}'", final_result);

                    assert!(!partials.is_empty(), "expected streaming partial emissions");
                    assert!(
                        final_result.to_uppercase().contains("NIGHTFALL")
                            || final_result.to_uppercase().contains("YELLOW")
                            || final_result.to_uppercase().contains("LAMPS"),
                        "expected ground-truth keywords in decoded text: '{}'",
                        final_result
                    );

                    let whole_transcribe = engine
                        .transcribe(wave.samples(), wave.sample_rate())
                        .expect("transcribe whole waveform");
                    assert!(
                        whole_transcribe.to_uppercase().contains("NIGHTFALL")
                            || whole_transcribe.to_uppercase().contains("YELLOW")
                            || whole_transcribe.to_uppercase().contains("LAMPS"),
                        "expected ground-truth in whole transcribe: '{}'",
                        whole_transcribe
                    );
                    for name in &["test_hello_16k.wav", "test_welcome_16k.wav", "test_ultimate_16k.wav"] {
                        let p = std::path::Path::new("/tmp").join(name);
                        if p.is_file() {
                            let w = sherpa_onnx::Wave::read(p.to_str().unwrap()).unwrap();
                            let decoded = engine.transcribe(w.samples(), w.sample_rate()).unwrap();
                            println!("DIAGNOSTIC TEST: {} -> '{}'", name, decoded);
                        }
                    }
                }
            }
        }
    }
}

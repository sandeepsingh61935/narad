use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use sherpa_onnx::{
    OfflineMoonshineModelConfig, OfflineNemoEncDecCtcModelConfig,
    OfflineRecognizerConfig, OfflineTransducerModelConfig, OfflineWhisperModelConfig,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub label: String,
    pub archive: String,
    pub extract_dir: String,
    pub kind: String,
    pub language: Option<String>,
    pub task: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModelsFile {
    pub model: Vec<ModelEntry>,
}

pub fn load_catalog(manifest: &Path) -> Result<Vec<ModelEntry>> {
    let text = std::fs::read_to_string(manifest)
        .with_context(|| format!("read model manifest {}", manifest.display()))?;
    let parsed: ModelsFile = toml::from_str(&text).context("parse models.toml")?;
    Ok(parsed.model)
}

pub fn archive_url(archive: &str) -> String {
    format!(
        "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/{}",
        archive
    )
}

fn find_one(dir: &Path, patterns: &[&str]) -> Result<PathBuf> {
    let mut matches = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        for pat in patterns {
            if glob_match(pat, &name) {
                matches.push(entry.path());
                break;
            }
        }
    }
    matches.sort();
    matches
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("no file matching {:?} in {}", patterns, dir.display()))
}

/// Simple `*` glob for a single path component (no `/`). Supports multiple `*` segments.
fn glob_match(pat: &str, name: &str) -> bool {
    let parts: Vec<&str> = pat.split('*').collect();
    if parts.len() == 1 {
        return pat == name;
    }
    let mut pos = 0usize;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !name.starts_with(part) {
                return false;
            }
            pos = part.len();
            continue;
        }
        let is_last = i + 1 == parts.len();
        if is_last && !pat.ends_with('*') {
            return name[pos..].ends_with(part) && name.len() >= pos + part.len();
        }
        match name[pos..].find(part) {
            Some(idx) => pos += idx + part.len(),
            None => return false,
        }
    }
    true
}

fn tokens_path(dir: &Path) -> Result<PathBuf> {
    find_one(dir, &["tokens.txt", "*-tokens.txt"])
}

pub fn model_dir(models_root: &Path, entry: &ModelEntry) -> PathBuf {
    models_root.join(&entry.extract_dir)
}

pub fn ensure_model_present(models_root: &Path, entry: &ModelEntry) -> Result<PathBuf> {
    let dir = model_dir(models_root, entry);
    if !dir.is_dir() {
        bail!(
            "model {:?} not found at {}\n  download: scripts/download-models.sh {}\n  or: wget {} && tar xf {} -C {}",
            entry.id,
            dir.display(),
            entry.id,
            archive_url(&entry.archive),
            entry.archive,
            models_root.display()
        );
    }
    Ok(dir)
}

pub fn build_recognizer_config(
    entry: &ModelEntry,
    dir: &Path,
    num_threads: i32,
) -> Result<OfflineRecognizerConfig> {
    let mut config = OfflineRecognizerConfig::default();
    config.model_config.num_threads = num_threads;
    config.model_config.provider = Some("cpu".into());
    config.model_config.tokens = Some(tokens_path(dir)?.display().to_string());

    match entry.kind.as_str() {
        "nemo_transducer" => {
            config.model_config.transducer = OfflineTransducerModelConfig {
                encoder: Some(
                    find_one(dir, &["encoder.int8.onnx", "encoder.onnx"])?
                        .display()
                        .to_string(),
                ),
                decoder: Some(
                    find_one(dir, &["decoder.int8.onnx", "decoder.onnx"])?
                        .display()
                        .to_string(),
                ),
                joiner: Some(
                    find_one(dir, &["joiner.int8.onnx", "joiner.onnx"])?
                        .display()
                        .to_string(),
                ),
            };
        }
        "nemo_ctc" => {
            config.model_config.nemo_ctc = OfflineNemoEncDecCtcModelConfig {
                model: Some(
                    find_one(dir, &["model.int8.onnx", "model.onnx"])?
                        .display()
                        .to_string(),
                ),
            };
        }
        "moonshine_v1" => {
            config.model_config.moonshine = OfflineMoonshineModelConfig {
                preprocessor: Some(
                    find_one(dir, &["preprocess.onnx", "preprocessor.onnx"])?
                        .display()
                        .to_string(),
                ),
                encoder: Some(
                    find_one(dir, &["encode.int8.onnx", "encode.onnx", "encoder*.onnx"])?
                        .display()
                        .to_string(),
                ),
                uncached_decoder: Some(
                    find_one(
                        dir,
                        &[
                            "uncached_decode.int8.onnx",
                            "uncached_decode.onnx",
                            "uncached_decoder*.onnx",
                        ],
                    )?
                    .display()
                    .to_string(),
                ),
                cached_decoder: Some(
                    find_one(
                        dir,
                        &[
                            "cached_decode.int8.onnx",
                            "cached_decode.onnx",
                            "cached_decoder*.onnx",
                        ],
                    )?
                    .display()
                    .to_string(),
                ),
                merged_decoder: None,
            };
        }
        "moonshine_v2" => {
            config.model_config.moonshine = OfflineMoonshineModelConfig {
                encoder: Some(
                    find_one(dir, &["encoder_model.ort", "encoder*.ort", "encoder*.onnx"])?
                        .display()
                        .to_string(),
                ),
                merged_decoder: Some(
                    find_one(
                        dir,
                        &[
                            "decoder_model_merged.ort",
                            "decoder*merged.ort",
                            "merged_decoder*.onnx",
                        ],
                    )?
                    .display()
                    .to_string(),
                ),
                preprocessor: None,
                uncached_decoder: None,
                cached_decoder: None,
            };
        }
        "whisper" => {
            config.model_config.whisper = OfflineWhisperModelConfig {
                encoder: Some(
                    find_one(dir, &["*encoder*.onnx"])?
                        .display()
                        .to_string(),
                ),
                decoder: Some(
                    find_one(dir, &["*decoder*.onnx"])?
                        .display()
                        .to_string(),
                ),
                language: entry.language.clone().or(Some(String::new())),
                task: entry
                    .task
                    .clone()
                    .or(Some("transcribe".into())),
                tail_paddings: 0,
                enable_token_timestamps: false,
                enable_segment_timestamps: false,
            };
        }
        other => bail!("unknown model kind {:?} for {}", other, entry.id),
    }

    Ok(config)
}

pub fn list_models(entries: &[ModelEntry], models_root: &Path) {
    println!("id\tlabel\tpresent\tarchive_url");
    for e in entries {
        let present = model_dir(models_root, e).is_dir();
        println!(
            "{}\t{}\t{}\t{}",
            e.id,
            e.label,
            present,
            archive_url(&e.archive)
        );
    }
}

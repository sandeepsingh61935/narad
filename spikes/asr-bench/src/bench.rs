use crate::models::{build_recognizer_config, ensure_model_present, ModelEntry};
use anyhow::{Context, Result};
use sherpa_onnx::{OfflineRecognizer, Wave};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub struct ModelBenchResult {
    pub cold_load_ms: f64,
    pub rows: Vec<(String, f64)>,
}

pub fn bench_model(
    entry: &ModelEntry,
    models_root: &Path,
    wavs: &[PathBuf],
    num_threads: i32,
) -> Result<ModelBenchResult> {
    let model_dir = ensure_model_present(models_root, entry)?;

    let config = build_recognizer_config(entry, &model_dir, num_threads)?;
    let t_load = Instant::now();
    let recognizer =
        OfflineRecognizer::create(&config).context("OfflineRecognizer::create")?;
    let cold_load_ms = t_load.elapsed().as_secs_f64() * 1000.0;

    let mut rows = Vec::new();
    for wav in wavs {
        let wave = Wave::read(wav.to_str().unwrap())
            .ok_or_else(|| anyhow::anyhow!("failed to read wav {}", wav.display()))?;
        let stream = recognizer.create_stream();
        stream.accept_waveform(wave.sample_rate(), wave.samples());
        let t_dec = Instant::now();
        recognizer.decode(&stream);
        let decode_ms = t_dec.elapsed().as_secs_f64() * 1000.0;
        let label = wav.file_name().unwrap().to_string_lossy().to_string();
        rows.push((label, decode_ms));
    }

    Ok(ModelBenchResult {
        cold_load_ms,
        rows,
    })
}

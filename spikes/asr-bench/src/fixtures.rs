use anyhow::{Context, Result};
use sherpa_onnx::write;
use std::path::{Path, PathBuf};

pub const SAMPLE_RATE: i32 = 16_000;
pub const DURATIONS_SEC: [f64; 3] = [3.0, 5.0, 10.0];

pub fn fixture_paths(dir: &Path) -> Vec<PathBuf> {
    DURATIONS_SEC
        .iter()
        .flat_map(|&sec| {
            let base = format!("clip_{}s", sec as u32);
            [
                dir.join(format!("{}_silence.wav", base)),
                dir.join(format!("{}_tone440.wav", base)),
            ]
        })
        .collect()
}

pub fn generate_all(dir: &Path) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let mut written = Vec::new();
    for &sec in &DURATIONS_SEC {
        let n = (sec * SAMPLE_RATE as f64).round() as usize;
        let silence = vec![0.0f32; n];
        let silence_path = dir.join(format!("clip_{}s_silence.wav", sec as u32));
        write_wav(&silence_path, &silence)?;
        written.push(silence_path);

        let tone = tone_440hz(n, SAMPLE_RATE);
        let tone_path = dir.join(format!("clip_{}s_tone440.wav", sec as u32));
        write_wav(&tone_path, &tone)?;
        written.push(tone_path);
    }
    Ok(written)
}

fn write_wav(path: &Path, samples: &[f32]) -> Result<()> {
    if !write(path.to_str().unwrap(), samples, SAMPLE_RATE) {
        anyhow::bail!("failed to write wav {}", path.display());
    }
    Ok(())
}

fn tone_440hz(num_samples: usize, sample_rate: i32) -> Vec<f32> {
    let freq = 440.0;
    let amp = 0.2;
    (0..num_samples)
        .map(|i| {
            let t = i as f64 / sample_rate as f64;
            (amp * (2.0 * std::f64::consts::PI * freq * t).sin()) as f32
        })
        .collect()
}

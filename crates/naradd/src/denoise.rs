//! Optional RNNoise (FR-AUD-5) via `nnnoiseless`.
//!
//! The crate wants 48 kHz frames of `DenoiseState::FRAME_SIZE` (480) in the
//! signed 16-bit range. Capture and ASR stay at 16 kHz mono f32; this module
//! adapts only at that boundary.

use nnnoiseless::DenoiseState;

use crate::audio::{resample_to_16k, TARGET_SAMPLE_RATE};

const RNNOISE_RATE: i32 = 48_000;
/// `nnnoiseless` treats f32 samples as i16 PCM, not [-1, 1].
const I16_SCALE: f32 = 32768.0;

/// `false` returns `samples` with no denoise pass (FR-AUD-5 default).
pub fn apply(samples: Vec<f32>, enabled: bool) -> Vec<f32> {
    if !enabled {
        return samples;
    }
    suppress_16k(&samples)
}

/// Run `nnnoiseless` on 16 kHz mono PCM. Output length and rate match `samples`.
pub fn suppress_16k(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }
    let mut up = upsample_16k_to_48k(samples);
    let up_len = up.len();
    for sample in &mut up {
        *sample = (*sample * I16_SCALE).clamp(-32768.0, 32767.0);
    }
    let frame = DenoiseState::FRAME_SIZE;
    let rem = up.len() % frame;
    if rem != 0 {
        up.resize(up.len() + (frame - rem), 0.0);
    }
    let mut state = DenoiseState::new();
    let mut out_buf = [0.0f32; DenoiseState::FRAME_SIZE];
    let mut denoised_48k = Vec::with_capacity(up.len());
    for chunk in up.chunks_exact(frame) {
        state.process_frame(&mut out_buf, chunk);
        denoised_48k.extend_from_slice(&out_buf);
    }
    denoised_48k.truncate(up_len);
    for sample in &mut denoised_48k {
        *sample /= I16_SCALE;
    }
    let down = resample_to_16k(&denoised_48k, RNNOISE_RATE);
    if down.len() == samples.len() {
        return down;
    }
    // Rate conversion can round off by one sample. Keep the recognizer's length.
    fit_len(down, samples.len())
}

/// 16 kHz → 48 kHz, sample 0 of each triplet on the original grid.
fn upsample_16k_to_48k(samples: &[f32]) -> Vec<f32> {
    let mut out = Vec::with_capacity(samples.len() * 3);
    for i in 0..samples.len() {
        let a = samples[i];
        let b = samples.get(i + 1).copied().unwrap_or(a);
        out.push(a);
        out.push(a + (b - a) / 3.0);
        out.push(a + (b - a) * (2.0 / 3.0));
    }
    debug_assert_eq!(RNNOISE_RATE / TARGET_SAMPLE_RATE, 3);
    out
}

fn fit_len(mut samples: Vec<f32>, len: usize) -> Vec<f32> {
    if samples.len() > len {
        samples.truncate(len);
        return samples;
    }
    samples.resize(len, 0.0);
    samples
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_leaves_samples_unchanged() {
        let samples = vec![0.1f32, -0.25, 0.0, 0.5, -1.0];
        assert_eq!(apply(samples.clone(), false), samples);
    }

    #[test]
    fn on_invokes_denoise_for_a_short_buffer() {
        // Shorter than one 10 ms RNNoise frame (160 samples at 16 kHz).
        let samples: Vec<f32> = (0..32).map(|i| (i as f32 * 0.35).sin() * 0.4).collect();
        let denoised = suppress_16k(&samples);
        assert_eq!(apply(samples.clone(), true), denoised);
        assert_eq!(denoised.len(), samples.len());
        assert!(denoised.iter().all(|sample| sample.is_finite()));
        assert_ne!(denoised, samples);
    }

    #[test]
    fn empty_stays_empty_when_enabled() {
        assert!(apply(Vec::new(), true).is_empty());
    }
}

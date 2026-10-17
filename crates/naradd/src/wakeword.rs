//! Local wake word spotting engine (FR-ACT-6).
//!
//! Provides continuous acoustic keyword spotting for phrases like "Hey Narad",
//! running entirely offline on CPU with zero network dependency (NFR-S-1).
//! Includes an energy gate, spectral voice-band gate, and pattern matching
//! state machine with post-trigger refractory protection.

use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

/// Target sample rate for internal acoustic analysis (16 kHz mono).
pub const TARGET_SAMPLE_RATE: u32 = 16_000;

/// Default keyword phrase triggering the engine.
pub const DEFAULT_KEYWORD_PHRASE: &str = "hey narad";

/// Default confidence threshold (0.0 to 1.0) required to trigger detection.
pub const DEFAULT_CONFIDENCE_THRESHOLD: f32 = 0.70;

/// Minimum RMS energy required to pass the silence gate.
pub const DEFAULT_ENERGY_THRESHOLD: f32 = 0.015;

/// Number of samples in one analysis FFT frame (256 samples @ 16 kHz = 16 ms).
const FFT_SIZE: usize = 256;

/// Default window duration in samples for candidate phrase evaluation (~1.05 seconds @ 16 kHz).
const DEFAULT_WINDOW_SAMPLES: usize = 16_800;

/// Minimum candidate duration in samples (~0.7 seconds @ 16 kHz).
const MIN_WINDOW_SAMPLES: usize = 11_200;

/// Maximum buffer capacity kept in the spotter (~2.0 seconds @ 16 kHz).
const MAX_BUFFER_SAMPLES: usize = 32_000;

/// Number of time slices used for template envelope and spectral matching.
const NUM_TEMPLATE_SLICES: usize = 16;

/// Refractory cool-down duration in samples (~1.2 seconds @ 16 kHz) to prevent duplicate triggers.
const REFRACTORY_SAMPLES: usize = 19_200;

/// Maximum consecutive silence duration in samples (~400 ms @ 16 kHz) before stale buffer is cleared.
const MAX_SILENCE_BEFORE_CLEAR: usize = 6_400;

/// Lead-in audio kept before speech start (~50 ms @ 16 kHz = 800 samples).
const LEAD_IN_SAMPLES: usize = 800;

/// Detected wake word event with metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WakeWordDetection {
    /// Detected phrase (e.g. "hey narad").
    pub phrase: String,
    /// Model confidence in range [0.0, 1.0].
    pub confidence: f32,
    /// Audio stream timestamp in milliseconds where detection occurred.
    pub timestamp_ms: u64,
}

/// Lifecycle state of the wake word spotter state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpotterState {
    /// Actively listening and analyzing continuous audio.
    Listening,
    /// Refractory cool-down period following a successful detection.
    CoolingDown,
}

/// Local wake word spotter engine.
pub struct WakeWordSpotter {
    phrase: String,
    threshold: f32,
    energy_threshold: f32,
    state: SpotterState,
    buffer: Vec<f32>,
    samples_processed: u64,
    refractory_samples_remaining: usize,
    silence_samples_count: usize,
    template: PhraseTemplate,
}

/// Acoustic template representing the temporal envelope and spectral profile of a phrase.
#[derive(Debug, Clone)]
struct PhraseTemplate {
    /// Expected relative RMS energy across `NUM_TEMPLATE_SLICES` time slices.
    energy_envelope: [f32; NUM_TEMPLATE_SLICES],
    /// Expected 4-band spectral distribution across `NUM_TEMPLATE_SLICES` time slices.
    /// Band 0: Sub-bass / nasal (60 - 250 Hz)
    /// Band 1: Vowel F1 / voice fundamental (250 - 1500 Hz)
    /// Band 2: Vowel F2 / formants (1500 - 3500 Hz)
    /// Band 3: Fricatives / consonants (3500 - 7000 Hz)
    spectral_profile: [[f32; 4]; NUM_TEMPLATE_SLICES],
}

impl Default for WakeWordSpotter {
    fn default() -> Self {
        Self::new(DEFAULT_KEYWORD_PHRASE, DEFAULT_CONFIDENCE_THRESHOLD)
    }
}

impl WakeWordSpotter {
    /// Create a new `WakeWordSpotter` with custom phrase and confidence threshold.
    pub fn new(phrase: impl Into<String>, threshold: f32) -> Self {
        let phrase_str = phrase.into();
        let template = PhraseTemplate::build(&phrase_str);
        Self {
            phrase: phrase_str,
            threshold: threshold.clamp(0.0, 1.0),
            energy_threshold: DEFAULT_ENERGY_THRESHOLD,
            state: SpotterState::Listening,
            buffer: Vec::with_capacity(MAX_BUFFER_SAMPLES),
            samples_processed: 0,
            refractory_samples_remaining: 0,
            silence_samples_count: 0,
            template,
        }
    }

    /// Builder method to override the keyword phrase.
    pub fn with_phrase(mut self, phrase: impl Into<String>) -> Self {
        self.set_phrase(phrase);
        self
    }

    /// Builder method to set the confidence threshold.
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.set_threshold(threshold);
        self
    }

    /// Builder method to set the silence energy threshold.
    pub fn with_energy_threshold(mut self, energy_threshold: f32) -> Self {
        self.set_energy_threshold(energy_threshold);
        self
    }

    /// Keyword phrase being monitored.
    pub fn phrase(&self) -> &str {
        &self.phrase
    }

    /// Update the keyword phrase and rebuild acoustic template.
    pub fn set_phrase(&mut self, phrase: impl Into<String>) {
        self.phrase = phrase.into();
        self.template = PhraseTemplate::build(&self.phrase);
        self.reset();
    }

    /// Current confidence threshold.
    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    /// Set confidence threshold.
    pub fn set_threshold(&mut self, threshold: f32) {
        self.threshold = threshold.clamp(0.0, 1.0);
    }

    /// Current RMS silence threshold.
    pub fn energy_threshold(&self) -> f32 {
        self.energy_threshold
    }

    /// Set RMS silence threshold.
    pub fn set_energy_threshold(&mut self, threshold: f32) {
        self.energy_threshold = threshold.max(0.0);
    }

    /// Current state machine status.
    pub fn state(&self) -> SpotterState {
        self.state
    }

    /// Reset internal audio buffers, refractory timers, and state machine.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.state = SpotterState::Listening;
        self.refractory_samples_remaining = 0;
        self.silence_samples_count = 0;
        debug!("wake word spotter reset to listening state");
    }

    /// Process incoming continuous PCM audio samples.
    ///
    /// Accepts mono `f32` PCM samples at any specified `sample_rate`. If the
    /// sample rate differs from 16 kHz, samples are resampled internally.
    ///
    /// Returns `Some(WakeWordDetection)` when the phrase matches with confidence
    /// at or above `threshold`, and transitions into a refractory cool-down
    /// period to prevent duplicate triggers.
    pub fn process_pcm(&mut self, samples: &[f32], sample_rate: u32) -> Option<WakeWordDetection> {
        if samples.is_empty() || sample_rate == 0 {
            return None;
        }

        // Resample to 16 kHz if necessary.
        let pcm_16k: std::borrow::Cow<'_, [f32]> = if sample_rate == TARGET_SAMPLE_RATE {
            std::borrow::Cow::Borrowed(samples)
        } else {
            std::borrow::Cow::Owned(resample_linear(samples, sample_rate, TARGET_SAMPLE_RATE))
        };

        let num_samples_16k = pcm_16k.len();
        self.samples_processed += (num_samples_16k as u64 * sample_rate as u64) / TARGET_SAMPLE_RATE as u64;

        // If in refractory cool-down period, count down and suppress detections.
        if self.refractory_samples_remaining > 0 {
            if num_samples_16k >= self.refractory_samples_remaining {
                self.refractory_samples_remaining = 0;
                self.state = SpotterState::Listening;
                trace!("wake word refractory period ended; resumed listening");
            } else {
                self.refractory_samples_remaining -= num_samples_16k;
                return None;
            }
        }

        // Energy gate on incoming block.
        let block_rms = compute_rms(&pcm_16k);
        if block_rms < self.energy_threshold {
            self.silence_samples_count += num_samples_16k;
            if self.buffer.is_empty() {
                // Buffer is already empty; no need to accumulate leading silence.
                return None;
            }
            if self.silence_samples_count > MAX_SILENCE_BEFORE_CLEAR {
                // Extended silence: clear buffer.
                self.buffer.clear();
                self.silence_samples_count = 0;
                return None;
            }
            // During short pauses (like inter-word dip), retain silence frames
            // so the phrase envelope keeps natural timing.
            self.buffer.extend_from_slice(&pcm_16k);
        } else {
            self.silence_samples_count = 0;
            self.buffer.extend_from_slice(&pcm_16k);
        }

        // Cap buffer size.
        if self.buffer.len() > MAX_BUFFER_SAMPLES {
            let overflow = self.buffer.len() - MAX_BUFFER_SAMPLES;
            self.buffer.drain(0..overflow);
        }

        // Need at least minimum window size to evaluate phrase match.
        if self.buffer.len() < MIN_WINDOW_SAMPLES {
            return None;
        }

        // Evaluate candidate window. For continuous streaming, test the trailing window
        // ending at current buffer len. If buffer has grown, also test candidates aligned with recent speech onsets.
        let mut candidates_to_test = Vec::new();

        // 1. Trailing window (most recent audio)
        let eval_end = self.buffer.len();
        let eval_start = eval_end.saturating_sub(DEFAULT_WINDOW_SAMPLES);
        if eval_end - eval_start >= MIN_WINDOW_SAMPLES {
            candidates_to_test.push((eval_start, eval_end));
        }

        // 2. If buffer has multiple windows, also test speech onset window
        let speech_start = self
            .buffer
            .iter()
            .position(|&s| s.abs() >= self.energy_threshold)
            .unwrap_or(0);
        let onset_start = speech_start.saturating_sub(LEAD_IN_SAMPLES);
        let onset_end = (onset_start + DEFAULT_WINDOW_SAMPLES).min(self.buffer.len());
        if onset_end - onset_start >= MIN_WINDOW_SAMPLES && (onset_start, onset_end) != (eval_start, eval_end) {
            candidates_to_test.push((onset_start, onset_end));
        }

        for (c_start, c_end) in candidates_to_test {
            let candidate = &self.buffer[c_start..c_end];

            // Combined energy and spectral gate.
            if !self.passes_energy_and_spectral_gate(candidate) {
                continue;
            }

            // Evaluate acoustic template match.
            let confidence = self.evaluate_match(candidate);
            trace!(
                phrase = %self.phrase,
                confidence = format!("{:.3}", confidence),
                threshold = format!("{:.3}", self.threshold),
                "wake word match evaluation"
            );

            if confidence >= self.threshold {
                let timestamp_ms = (self.samples_processed * 1000) / sample_rate as u64;
                let detection = WakeWordDetection {
                    phrase: self.phrase.clone(),
                    confidence,
                    timestamp_ms,
                };

                // Transition to cool-down to prevent repeated triggers on the same utterance.
                self.state = SpotterState::CoolingDown;
                self.refractory_samples_remaining = REFRACTORY_SAMPLES;
                self.buffer.clear();
                self.silence_samples_count = 0;

                debug!(
                    phrase = %detection.phrase,
                    confidence = format!("{:.3}", detection.confidence),
                    timestamp_ms = detection.timestamp_ms,
                    "wake word triggered successfully"
                );
                return Some(detection);
            }
        }

        None
    }

    /// Combined energy and spectral gate. Rejects silence, DC hum, high-frequency hiss,
    /// and non-speech white noise before running full template matching.
    pub fn passes_energy_and_spectral_gate(&self, samples: &[f32]) -> bool {
        if samples.is_empty() {
            return false;
        }

        // 1. RMS Energy Gate
        let rms = compute_rms(samples);
        if rms < self.energy_threshold {
            return false;
        }

        // 2. Zero-Crossing Rate (ZCR) Gate:
        // Human speech typically stays within [0.02, 0.42].
        // High-frequency hiss/white noise is >= 0.45, while DC rumble is < 0.015.
        let zcr = compute_zcr(samples);
        if zcr < 0.015 || zcr > 0.44 {
            return false;
        }

        // 3. Spectral Voice-Band Ratio Gate:
        // Speech formant band: Band 1 (250-1500 Hz) + Band 2 (1500-3500 Hz).
        // In speech, formant bands dominate while high-frequency hiss/noise (Band 3: 3500-7000 Hz) is low (< 35%).
        // In flat white noise, Band 3 carries ~50% of spectral power.
        let check_centers = [
            samples.len() / 4,
            samples.len() / 2,
            (samples.len() * 3) / 4,
        ];

        let mut passes_spectral = false;
        for center in check_centers {
            let frame_start = center.saturating_sub(FFT_SIZE / 2);
            let frame_end = (frame_start + FFT_SIZE).min(samples.len());
            if frame_end - frame_start < FFT_SIZE {
                continue;
            }

            let bands = compute_4band_energy(&samples[frame_start..frame_end]);
            let total_band_power: f32 = bands.iter().sum();
            if total_band_power <= 1e-9 {
                continue;
            }

            let speech_band_power = bands[1] + bands[2];
            let speech_ratio = speech_band_power / total_band_power;
            let hiss_ratio = bands[3] / total_band_power;

            // Formants >= 35% and hiss < 38%
            if speech_ratio >= 0.35 && hiss_ratio < 0.38 {
                passes_spectral = true;
                break;
            }
        }

        passes_spectral
    }

    /// Evaluates candidate audio against the phrase template across slight alignment shifts.
    fn evaluate_match(&self, candidate: &[f32]) -> f32 {
        if candidate.len() < FFT_SIZE * NUM_TEMPLATE_SLICES {
            return 0.0;
        }

        let slice_len = candidate.len() / NUM_TEMPLATE_SLICES;
        let mut obs_energies = [0.0f32; NUM_TEMPLATE_SLICES];
        let mut obs_spectrals = [[0.0f32; 4]; NUM_TEMPLATE_SLICES];

        for i in 0..NUM_TEMPLATE_SLICES {
            let start = i * slice_len;
            let end = (start + slice_len).min(candidate.len());
            let slice = &candidate[start..end];
            obs_energies[i] = compute_rms(slice);

            // Compute spectral bands on a frame within the slice.
            let frame_len = slice.len().min(FFT_SIZE);
            let bands = compute_4band_energy(&slice[..frame_len]);
            let band_sum: f32 = bands.iter().sum();
            if band_sum > 1e-9 {
                for b in 0..4 {
                    obs_spectrals[i][b] = bands[b] / band_sum;
                }
            }
        }

        // Normalize observed energy envelope to [0.0, 1.0].
        let max_e = obs_energies
            .iter()
            .cloned()
            .fold(0.0f32, f32::max);
        if max_e > 1e-6 {
            for e in &mut obs_energies {
                *e /= max_e;
            }
        } else {
            return 0.0;
        }

        // Evaluate match score with slight alignment tolerance (+/- 1 slice).
        let mut best_score = 0.0f32;
        for shift in [-1isize, 0, 1] {
            let score = self.score_shifted(&obs_energies, &obs_spectrals, shift);
            if score > best_score {
                best_score = score;
            }
        }

        best_score
    }

    fn score_shifted(
        &self,
        energies: &[f32; NUM_TEMPLATE_SLICES],
        spectrals: &[[f32; 4]; NUM_TEMPLATE_SLICES],
        shift: isize,
    ) -> f32 {
        // Shift observed energies and spectrals into an aligned buffer.
        let mut aligned_e = [0.0f32; NUM_TEMPLATE_SLICES];
        let mut aligned_s = [[0.25f32; 4]; NUM_TEMPLATE_SLICES];

        for i in 0..NUM_TEMPLATE_SLICES {
            let src = i as isize + shift;
            if src >= 0 && (src as usize) < NUM_TEMPLATE_SLICES {
                aligned_e[i] = energies[src as usize];
                aligned_s[i] = spectrals[src as usize];
            }
        }

        // 1. Envelope correlation score.
        let env_score = pearson_correlation(&aligned_e, &self.template.energy_envelope).max(0.0);

        // 2. Inter-syllable / word valley check.
        let valley_score = check_cadence_valley(&aligned_e);

        // 3. Spectral trajectory similarity.
        let mut spec_sum = 0.0f32;
        let mut active_count = 0.0f32;
        for i in 0..NUM_TEMPLATE_SLICES {
            if self.template.energy_envelope[i] > 0.3 {
                let sim = cosine_similarity_4d(&aligned_s[i], &self.template.spectral_profile[i]);
                spec_sum += sim.max(0.0);
                active_count += 1.0;
            }
        }
        let spec_score = if active_count > 0.0 {
            spec_sum / active_count
        } else {
            0.0
        };

        // Weighted confidence calculation.
        let confidence = if self.phrase.to_lowercase().contains("narad") {
            0.45 * env_score + 0.35 * spec_score + 0.20 * valley_score
        } else {
            0.55 * env_score + 0.45 * spec_score
        };
        confidence.clamp(0.0, 1.0)
    }
}

impl PhraseTemplate {
    /// Build expected temporal energy envelope and spectral profile for a phrase.
    fn build(phrase: &str) -> Self {
        let norm = phrase.trim().to_lowercase();
        if norm.contains("narad") {
            Self::build_hey_narad()
        } else {
            Self::build_generic(&norm)
        }
    }

    /// Specially tuned acoustic profile for "Hey Narad":
    /// - Slice 0: Lead-in / breath onset
    /// - Slices 1..4: "Hey" (/heɪ/) strong F2 (~2000 Hz) and F1 (~500 Hz)
    /// - Slices 5..6: Inter-word dip between "Hey" and "Narad"
    /// - Slices 7..10: "Na-" (/nɑː/) strong F1 (~750 Hz) and nasal (~250 Hz)
    /// - Slices 11..14: "-rad" (/rəd/) liquid /r/ and schwa
    /// - Slice 15: Decay tail
    fn build_hey_narad() -> Self {
        let mut energy = [0.0f32; NUM_TEMPLATE_SLICES];
        let mut spec = [[0.25f32; 4]; NUM_TEMPLATE_SLICES];

        // Slice 0: Lead-in onset
        energy[0] = 0.10;
        spec[0] = [0.10, 0.20, 0.30, 0.40];

        // Slices 1..4: "Hey" (/heɪ/)
        energy[1] = 0.70;
        energy[2] = 0.95;
        energy[3] = 0.90;
        energy[4] = 0.65;
        for s in 1..=4 {
            // High Band 2 (F2 ~2000 Hz) and Band 1 (F1 ~500 Hz).
            spec[s] = [0.10, 0.40, 0.40, 0.10];
        }

        // Slices 5..6: Inter-word dip
        energy[5] = 0.12;
        energy[6] = 0.10;
        spec[5] = [0.25, 0.25, 0.25, 0.25];
        spec[6] = [0.25, 0.25, 0.25, 0.25];

        // Slices 7..10: "Na-" (/nɑː/)
        energy[7] = 0.75;
        energy[8] = 1.00;
        energy[9] = 0.95;
        energy[10] = 0.70;
        for s in 7..=10 {
            // Nasal murmur Band 0 (~250 Hz) + open vowel Band 1 (~750 Hz).
            spec[s] = [0.25, 0.50, 0.20, 0.05];
        }

        // Slices 11..14: "-rad" (/rəd/)
        energy[11] = 0.70;
        energy[12] = 0.65;
        energy[13] = 0.55;
        energy[14] = 0.35;
        for s in 11..=14 {
            // Liquid /r/ + schwa /ə/
            spec[s] = [0.15, 0.45, 0.30, 0.10];
        }

        // Slice 15: Decay tail
        energy[15] = 0.10;
        spec[15] = [0.25, 0.25, 0.25, 0.25];

        Self {
            energy_envelope: energy,
            spectral_profile: spec,
        }
    }

    /// Generic profile for arbitrary phrases: distributes energy across detected words/syllables.
    fn build_generic(phrase: &str) -> Self {
        let words: Vec<&str> = phrase.split_whitespace().collect();
        let word_count = words.len().max(1);

        let mut energy = [0.10f32; NUM_TEMPLATE_SLICES];
        let mut spec = [[0.15f32, 0.45, 0.30, 0.10]; NUM_TEMPLATE_SLICES];

        let span = NUM_TEMPLATE_SLICES / word_count;
        for (w, _) in words.iter().enumerate() {
            let start = w * span + 1;
            let end = (start + span.saturating_sub(1)).min(NUM_TEMPLATE_SLICES);
            for s in start..end {
                energy[s] = 0.85;
                spec[s] = [0.15, 0.50, 0.25, 0.10];
            }
            if end < NUM_TEMPLATE_SLICES {
                energy[end] = 0.20; // dip between words
            }
        }

        Self {
            energy_envelope: energy,
            spectral_profile: spec,
        }
    }
}

/// Computes Root-Mean-Square (RMS) amplitude of audio samples.
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples
        .iter()
        .map(|&s| if s.is_finite() { s * s } else { 0.0 })
        .sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Computes normalized Zero-Crossing Rate (ZCR) of audio samples.
pub fn compute_zcr(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let mut crossings = 0usize;
    for w in samples.windows(2) {
        if (w[0] >= 0.0 && w[1] < 0.0) || (w[0] < 0.0 && w[1] >= 0.0) {
            crossings += 1;
        }
    }
    crossings as f32 / (samples.len() - 1) as f32
}

/// Computes 4-band spectral energy from a 256-sample frame using in-place Radix-2 FFT:
/// - Band 0: Sub-bass & Nasal (< 250 Hz) - Bins 1..=4
/// - Band 1: Vowel F1 & Pitch (250 - 1500 Hz) - Bins 5..=24
/// - Band 2: Vowel F2 & Formants (1500 - 3500 Hz) - Bins 25..=56
/// - Band 3: Fricatives & Consonants (3500 - 7000 Hz) - Bins 57..=112
pub fn compute_4band_energy(samples: &[f32]) -> [f32; 4] {
    let mut real = [0.0f32; FFT_SIZE];
    let mut imag = [0.0f32; FFT_SIZE];

    let n = samples.len().min(FFT_SIZE);
    // Apply Hann window
    for i in 0..n {
        let s = if samples[i].is_finite() { samples[i] } else { 0.0 };
        let hann = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos());
        real[i] = s * hann;
    }

    radix2_fft(&mut real, &mut imag);

    let mut bands = [0.0f32; 4];
    for k in 1..=FFT_SIZE / 2 {
        let mag_sq = real[k] * real[k] + imag[k] * imag[k];
        match k {
            1..=4 => bands[0] += mag_sq,
            5..=24 => bands[1] += mag_sq,
            25..=56 => bands[2] += mag_sq,
            57..=112 => bands[3] += mag_sq,
            _ => {}
        }
    }
    bands
}

/// Radix-2 in-place Cooley-Tukey FFT for 256 points.
fn radix2_fft(real: &mut [f32; FFT_SIZE], imag: &mut [f32; FFT_SIZE]) {
    // 1. Bit-reversal permutation
    let mut j = 0usize;
    for i in 0..FFT_SIZE - 1 {
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
        let mut k = FFT_SIZE >> 1;
        while k <= j {
            j -= k;
            k >>= 1;
        }
        j += k;
    }

    // 2. Butterfly computations
    let mut len = 2usize;
    while len <= FFT_SIZE {
        let half_len = len >> 1;
        let angle = -2.0 * std::f32::consts::PI / len as f32;
        let w_step_re = angle.cos();
        let w_step_im = angle.sin();

        let mut i = 0usize;
        while i < FFT_SIZE {
            let mut w_re = 1.0f32;
            let mut w_im = 0.0f32;
            for k in 0..half_len {
                let u_re = real[i + k];
                let u_im = imag[i + k];

                let v_re = real[i + k + half_len] * w_re - imag[i + k + half_len] * w_im;
                let v_im = real[i + k + half_len] * w_im + imag[i + k + half_len] * w_re;

                real[i + k] = u_re + v_re;
                imag[i + k] = u_im + v_im;

                real[i + k + half_len] = u_re - v_re;
                imag[i + k + half_len] = u_im - v_im;

                let next_w_re = w_re * w_step_re - w_im * w_step_im;
                let next_w_im = w_re * w_step_im + w_im * w_step_re;
                w_re = next_w_re;
                w_im = next_w_im;
            }
            i += len;
        }
        len <<= 1;
    }
}

/// Pearson correlation coefficient between two equal-length slices.
fn pearson_correlation(x: &[f32; NUM_TEMPLATE_SLICES], y: &[f32; NUM_TEMPLATE_SLICES]) -> f32 {
    let mean_x: f32 = x.iter().sum::<f32>() / NUM_TEMPLATE_SLICES as f32;
    let mean_y: f32 = y.iter().sum::<f32>() / NUM_TEMPLATE_SLICES as f32;

    let mut num = 0.0f32;
    let mut den_x = 0.0f32;
    let mut den_y = 0.0f32;

    for i in 0..NUM_TEMPLATE_SLICES {
        let dx = x[i] - mean_x;
        let dy = y[i] - mean_y;
        num += dx * dy;
        den_x += dx * dx;
        den_y += dy * dy;
    }

    let den = (den_x * den_y).sqrt();
    if den > 1e-9 {
        num / den
    } else {
        0.0
    }
}

/// Cosine similarity between two 4-dimensional vectors.
fn cosine_similarity_4d(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..4 {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let den = (norm_a * norm_b).sqrt();
    if den > 1e-9 {
        dot / den
    } else {
        0.0
    }
}

/// Checks that a candidate envelope displays cadence peaks and a distinct inter-word valley.
fn check_cadence_valley(energies: &[f32; NUM_TEMPLATE_SLICES]) -> f32 {
    // Peak 1: Slices 1..4 ("Hey")
    let peak1 = energies[1..=4]
        .iter()
        .cloned()
        .fold(0.0f32, f32::max);

    // Valley: Slices 5..6 (inter-word dip)
    let valley = energies[5..=6]
        .iter()
        .cloned()
        .fold(1.0f32, f32::min);

    // Peak 2: Slices 7..10 ("Na-")
    let peak2 = energies[7..=10]
        .iter()
        .cloned()
        .fold(0.0f32, f32::max);

    if peak1 < 0.4 || peak2 < 0.4 {
        return 0.0;
    }

    // Valley must be lower than both peaks by a noticeable margin.
    if valley < peak1 * 0.60 && valley < peak2 * 0.60 {
        1.0
    } else if valley < peak1 * 0.80 && valley < peak2 * 0.80 {
        0.5
    } else {
        0.0
    }
}

/// Linear resampler from `from_rate` to `to_rate`.
fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = to_rate as f64 / from_rate as f64;
    let out_len = ((samples.len() as f64) * ratio).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_idx = i as f64 / ratio;
        let idx0 = src_idx.floor() as usize;
        let frac = (src_idx - idx0 as f64) as f32;
        let idx1 = (idx0 + 1).min(samples.len() - 1);
        let s0 = samples[idx0.min(samples.len() - 1)];
        let s1 = samples[idx1];
        out.push(s0 + frac * (s1 - s0));
    }
    out
}

/// Synthesize a reference acoustic pattern matching the keyword phrase for testing.
///
/// Produces realistic multi-formant audio with vocal pitch modulation, breath onset,
/// syllable envelope, and inter-word dip matching the target phrase acoustic template.
pub fn generate_wake_phrase_samples(phrase: &str, sample_rate: u32) -> Vec<f32> {
    let rate = sample_rate as f32;
    let total_secs = 1.05f32;
    let total_samples = (total_secs * rate) as usize;
    let mut out = vec![0.0f32; total_samples];

    let norm = phrase.trim().to_lowercase();
    let is_hey_narad = norm.contains("narad");
    let word_count = norm.split_whitespace().count().max(1);

    for i in 0..total_samples {
        let t = i as f32 / rate;
        let sample = if is_hey_narad {
            generate_hey_narad_sample(t)
        } else {
            generate_generic_phrase_sample(t, total_secs, word_count)
        };
        out[i] = sample;
    }

    out
}

fn generate_hey_narad_sample(t: f32) -> f32 {
    let pi = std::f32::consts::PI;

    // Slice 0 (0.00 - 0.065s): Lead-in onset
    if t < 0.065 {
        let env = (t / 0.065).sin();
        let noise = ((t * 12345.67).sin() * 43758.5453).fract() * 2.0 - 1.0;
        return noise * 0.08 * env;
    }

    // Slices 1..4 (0.065 - 0.33s): "Hey" (/heɪ/)
    // F0 ~ 130 Hz, F1 ~ 500 Hz, F2 ~ 2000 Hz
    if t < 0.33 {
        let dur = 0.265;
        let rel_t = (t - 0.065) / dur;
        let env = (rel_t * pi).sin().powf(1.1);
        let f0 = 130.0 + 10.0 * rel_t;
        let f1 = 500.0;
        let f2 = 1900.0 + 200.0 * rel_t;

        let osc = 0.4 * (2.0 * pi * f0 * t).sin()
            + 0.35 * (2.0 * pi * f1 * t).sin()
            + 0.25 * (2.0 * pi * f2 * t).sin();
        return osc * env * 0.25;
    }

    // Slices 5..6 (0.33 - 0.46s): Inter-word dip
    if t < 0.46 {
        return 0.003 * (2.0 * pi * 100.0 * t).sin();
    }

    // Slices 7..10 (0.46 - 0.72s): "Na-" (/nɑː/)
    // Nasal ~250 Hz, F1 ~ 750 Hz, F2 ~ 1250 Hz
    if t < 0.72 {
        let dur = 0.26;
        let rel_t = (t - 0.46) / dur;
        let env = (rel_t * pi).sin().powf(1.1);
        let f0 = 125.0 - 5.0 * rel_t;
        let f_nasal = 250.0;
        let f1 = 750.0;
        let f2 = 1250.0;

        let osc = 0.30 * (2.0 * pi * f0 * t).sin()
            + 0.25 * (2.0 * pi * f_nasal * t).sin()
            + 0.30 * (2.0 * pi * f1 * t).sin()
            + 0.15 * (2.0 * pi * f2 * t).sin();
        return osc * env * 0.28;
    }

    // Slices 11..14 (0.72 - 0.99s): "-rad" (/rəd/)
    // F0 ~ 115 Hz, F1 ~ 500 Hz, F2 ~ 1500 Hz
    if t < 0.99 {
        let dur = 0.27;
        let rel_t = (t - 0.72) / dur;
        let env = (rel_t * pi).sin();
        let f0 = 115.0 - 15.0 * rel_t;
        let f1 = 500.0;
        let f2 = 1500.0 - 100.0 * rel_t;

        let osc = 0.40 * (2.0 * pi * f0 * t).sin()
            + 0.35 * (2.0 * pi * f1 * t).sin()
            + 0.25 * (2.0 * pi * f2 * t).sin();
        return osc * env * 0.20;
    }

    // Slice 15: Tail silence
    0.0
}

fn generate_generic_phrase_sample(t: f32, total_secs: f32, word_count: usize) -> f32 {
    let pi = std::f32::consts::PI;
    let rel = (t / total_secs).clamp(0.0, 1.0);
    let words = word_count.max(1) as f32;
    let word_rel = (rel * words).fract();
    let env = (word_rel * pi).sin().max(0.0);
    let osc = 0.5 * (2.0 * pi * 130.0 * t).sin() + 0.5 * (2.0 * pi * 800.0 * t).sin();
    osc * env * 0.2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_spotter_configuration() {
        let spotter = WakeWordSpotter::default();
        assert_eq!(spotter.phrase(), DEFAULT_KEYWORD_PHRASE);
        assert!((spotter.threshold() - DEFAULT_CONFIDENCE_THRESHOLD).abs() < 1e-6);
        assert!((spotter.energy_threshold() - DEFAULT_ENERGY_THRESHOLD).abs() < 1e-6);
        assert_eq!(spotter.state(), SpotterState::Listening);
    }

    #[test]
    fn test_builder_and_mutators() {
        let mut spotter = WakeWordSpotter::new("hey computer", 0.80)
            .with_energy_threshold(0.02);
        assert_eq!(spotter.phrase(), "hey computer");
        assert!((spotter.threshold() - 0.80).abs() < 1e-6);
        assert!((spotter.energy_threshold() - 0.02).abs() < 1e-6);

        spotter.set_phrase("hey narad");
        assert_eq!(spotter.phrase(), "hey narad");

        spotter.set_threshold(0.65);
        assert!((spotter.threshold() - 0.65).abs() < 1e-6);

        spotter.set_energy_threshold(0.01);
        assert!((spotter.energy_threshold() - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_ignores_pure_silence() {
        let mut spotter = WakeWordSpotter::default();
        let silence = vec![0.0f32; 16_000];

        let detection = spotter.process_pcm(&silence, 16_000);
        assert!(detection.is_none(), "pure silence must not trigger wake word");
        assert_eq!(spotter.state(), SpotterState::Listening);
    }

    #[test]
    fn test_ignores_low_energy_noise() {
        let mut spotter = WakeWordSpotter::default();
        // Generate low-amplitude white noise below energy threshold (RMS ~ 0.005 < 0.015).
        let low_noise: Vec<f32> = (0..16_000)
            .map(|i| {
                let pseudo_random = ((i as f32 * 17.13).sin() * 43758.5453).fract();
                (pseudo_random * 2.0 - 1.0) * 0.008
            })
            .collect();

        assert!(compute_rms(&low_noise) < spotter.energy_threshold());
        let detection = spotter.process_pcm(&low_noise, 16_000);
        assert!(detection.is_none(), "low energy noise must be rejected by energy gate");
        assert_eq!(spotter.state(), SpotterState::Listening);
    }

    #[test]
    fn test_spectral_gate_rejects_white_noise() {
        let spotter = WakeWordSpotter::default();
        // High-energy white noise with RMS ~ 0.12 > energy_threshold,
        // but high ZCR (~0.50) and flat spectrum.
        let white_noise: Vec<f32> = (0..16_000)
            .map(|i| {
                let pseudo_random = ((i as f32 * 31.41).sin() * 43758.5453).fract();
                (pseudo_random * 2.0 - 1.0) * 0.20
            })
            .collect();

        assert!(compute_rms(&white_noise) > spotter.energy_threshold());
        assert!(!spotter.passes_energy_and_spectral_gate(&white_noise), "spectral gate must reject white noise");
    }

    #[test]
    fn test_spectral_gate_rejects_monotonic_tone() {
        let mut spotter = WakeWordSpotter::default();
        // Pure 440 Hz sine wave for 1.2 seconds.
        // Passes energy gate, but has no speech formants or syllable cadence.
        let tone: Vec<f32> = (0..19_200)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i as f32 / 16_000.0)).sin() * 0.25)
            .collect();

        let detection = spotter.process_pcm(&tone, 16_000);
        assert!(detection.is_none(), "pure monotonic tone must not trigger wake word");
    }

    #[test]
    fn test_accurately_detects_wake_phrase_pattern() {
        let mut spotter = WakeWordSpotter::default();
        let samples = generate_wake_phrase_samples("hey narad", 16_000);

        let detection = spotter.process_pcm(&samples, 16_000);
        assert!(detection.is_some(), "wake phrase 'hey narad' must trigger detection");

        let det = detection.unwrap();
        assert_eq!(det.phrase, "hey narad");
        assert!(det.confidence >= spotter.threshold(), "confidence must meet threshold");
        assert!(det.timestamp_ms > 0, "timestamp must reflect stream position");
        assert_eq!(spotter.state(), SpotterState::CoolingDown);
    }

    #[test]
    fn test_streaming_chunked_detection() {
        let mut spotter = WakeWordSpotter::default();
        let samples = generate_wake_phrase_samples("hey narad", 16_000);

        // Feed in 160-sample chunks (10 ms @ 16 kHz), simulating real-time capture.
        let chunk_size = 160;
        let mut detected = None;

        for chunk in samples.chunks(chunk_size) {
            if let Some(det) = spotter.process_pcm(chunk, 16_000) {
                detected = Some(det);
                break;
            }
        }

        assert!(detected.is_some(), "chunked streaming audio must trigger detection");
        let det = detected.unwrap();
        assert_eq!(det.phrase, "hey narad");
        assert!(det.confidence >= 0.70);
    }

    #[test]
    fn test_reset_state_machine_avoids_repeated_triggers() {
        let mut spotter = WakeWordSpotter::default();
        let samples = generate_wake_phrase_samples("hey narad", 16_000);

        // 1. Initial trigger.
        let first_detection = spotter.process_pcm(&samples, 16_000);
        assert!(first_detection.is_some(), "first phrase must trigger detection");
        assert_eq!(spotter.state(), SpotterState::CoolingDown);

        // 2. Feed second copy immediately: refractory period prevents duplicate trigger.
        let duplicate_detection = spotter.process_pcm(&samples, 16_000);
        assert!(duplicate_detection.is_none(), "immediate duplicate must be ignored by refractory protection");

        // 3. Explicit reset clears refractory cool-down and resets to listening.
        spotter.reset();
        assert_eq!(spotter.state(), SpotterState::Listening);

        // 4. Feed phrase again after reset: triggers successfully.
        let second_detection = spotter.process_pcm(&samples, 16_000);
        assert!(second_detection.is_some(), "phrase must trigger again after reset");
    }

    #[test]
    fn test_configurable_threshold_filtering() {
        let samples = generate_wake_phrase_samples("hey narad", 16_000);

        // Very high threshold (0.999) filters out borderline signals.
        let mut strict_spotter = WakeWordSpotter::default().with_threshold(0.999);
        assert!(strict_spotter.process_pcm(&samples, 16_000).is_none());

        // Standard threshold (0.70) triggers.
        let mut standard_spotter = WakeWordSpotter::default().with_threshold(0.70);
        assert!(standard_spotter.process_pcm(&samples, 16_000).is_some());
    }

    #[test]
    fn test_resampling_48k_to_16k() {
        let mut spotter = WakeWordSpotter::default();
        // Generate audio synthesized directly at 48 kHz.
        let samples_48k = generate_wake_phrase_samples("hey narad", 48_000);

        let detection = spotter.process_pcm(&samples_48k, 48_000);
        assert!(detection.is_some(), "48 kHz audio must be resampled and detected");
        assert_eq!(detection.unwrap().phrase, "hey narad");
    }

    #[test]
    fn test_detects_phrase_preceded_by_other_speech() {
        let mut spotter = WakeWordSpotter::default();
        // 1. Initial conversational speech preceding the wake word
        let generic = generate_wake_phrase_samples("conversation here", 16_000);
        // 2. Followed by the wake word
        let wake = generate_wake_phrase_samples("hey narad", 16_000);

        let mut combined = generic;
        combined.extend(wake);

        let chunk_size = 160;
        let mut detected = None;

        for chunk in combined.chunks(chunk_size) {
            if let Some(det) = spotter.process_pcm(chunk, 16_000) {
                detected = Some(det);
                break;
            }
        }

        assert!(detected.is_some(), "wake phrase preceded by speech must trigger");
        assert_eq!(detected.unwrap().phrase, "hey narad");
    }

    #[test]
    fn test_generic_phrase_matching() {
        let mut spotter = WakeWordSpotter::new("start recording", 0.65);
        let samples = generate_wake_phrase_samples("start recording", 16_000);

        let detection = spotter.process_pcm(&samples, 16_000);
        assert!(detection.is_some(), "generic phrase must be matched");
        assert_eq!(detection.unwrap().phrase, "start recording");
    }
}

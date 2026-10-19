//! Production-grade realistic acoustic testing suite (NFR-T-1, NFR-T-4, ARCHITECTURE §10.1 & §10.5).
//!
//! Covers:
//! - Multi-speaker acoustic simulation: high-speed speech (>180 wpm), whispered speech, natural hesitation fillers ("um... uh...").
//! - Calibrated background noise mixing: mechanical keyboard clatter, coffee shop murmur, office soundscape at 5 dB, 10 dB, 15 dB SNR.
//! - Dynamic SNR mixer: `mix_with_noise(speech, noise, snr_db)`.
//! - VAD boundary and RTF regression gate: verifies clean speech boundary trimming and Real-Time Factor (RTF) <= 0.05x on CPU.
//! - Endurance & memory footprint audit: runs repeated chunked decoding cycles to assert flat memory footprint (<150 MB).

use std::f32::consts::PI;
use std::time::{Duration, Instant};

use naradd::asr::{
    format_streaming_punctuation_with_prosody, ProsodicPunctuationAction,
    ProsodicPunctuationTrigger,
};
use naradd::audio::{
    contains_speech, trim_silence, ProsodicEvent, ProsodyTracker, TARGET_SAMPLE_RATE,
};
use naradd::cleanup::{apply_cleanup, clean_text, prune_speech_repairs};
use naradd::denoise;
use naradd::ibus::IbusEngineState;
use naradd::text::{apply_commands, disambiguate_homophones, is_literal_punctuation_context};

const SAMPLE_RATE: u32 = TARGET_SAMPLE_RATE as u32; // 16,000 Hz

// ============================================================================
// 1. DYNAMIC SNR MIXER & DSP UTILITIES
// ============================================================================

/// Compute Root Mean Square (RMS) energy of an audio buffer.
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Dynamic SNR mixer: mixes `speech` and `noise` to achieve the target `snr_db`.
///
/// Formula:
/// SNR_dB = 20 * log10(RMS_speech / RMS_noise_scaled)
/// RMS_noise_scaled = RMS_speech / 10^(snr_db / 20)
/// alpha = RMS_noise_scaled / RMS_noise
/// mixed[i] = speech[i] + alpha * noise[i % noise_len]
pub fn mix_with_noise(speech: &[f32], noise: &[f32], snr_db: f32) -> Vec<f32> {
    if speech.is_empty() {
        return Vec::new();
    }
    if noise.is_empty() {
        return speech.to_vec();
    }

    let speech_rms = compute_rms(speech);
    let noise_rms = compute_rms(noise);

    if speech_rms < 1e-7 || noise_rms < 1e-7 {
        return speech.to_vec();
    }

    let target_noise_rms = speech_rms / 10.0f32.powf(snr_db / 20.0);
    let scale = target_noise_rms / noise_rms;
    let noise_len = noise.len();

    speech
        .iter()
        .enumerate()
        .map(|(i, &s)| {
            let n = noise[i % noise_len];
            (s + scale * n).clamp(-1.0, 1.0)
        })
        .collect()
}

/// Compute measured SNR in dB between a clean speech signal and an isolated noise component.
pub fn compute_measured_snr_db(speech: &[f32], noise_component: &[f32]) -> f32 {
    let s_rms = compute_rms(speech);
    let n_rms = compute_rms(noise_component);
    if n_rms < 1e-9 {
        return 100.0;
    }
    if s_rms < 1e-9 {
        return -100.0;
    }
    20.0 * (s_rms / n_rms).log10()
}

// ============================================================================
// 2. DETERMINISTIC PSEUDO-RANDOM NUMBER GENERATOR (PRNG)
// ============================================================================

struct FastRng {
    state: u64,
}

impl FastRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.max(1),
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    /// Uniform random float in [-1.0, 1.0].
    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32 / (u32::MAX as f32 / 2.0)) - 1.0
    }

    /// Approximate Gaussian noise via central limit theorem (sum of 4 uniform randoms).
    fn next_gaussian(&mut self) -> f32 {
        (self.next_f32() + self.next_f32() + self.next_f32() + self.next_f32()) * 0.5
    }
}

// ============================================================================
// 3. MULTI-SPEAKER ACOUSTIC SIMULATION (HIGH-SPEED, WHISPERED, FILLERS)
// ============================================================================

/// Second-order IIR bandpass formant resonator filter.
///
/// Transfer function:
/// y[n] = 2*r*cos(omega)*y[n-1] - r^2*y[n-2] + (1-r)*x[n]
#[derive(Clone)]
struct FormantResonator {
    a1: f32,
    a2: f32,
    b0: f32,
    y1: f32,
    y2: f32,
}

impl FormantResonator {
    fn new(freq: f32, bandwidth: f32, sample_rate: f32) -> Self {
        let r = (-PI * bandwidth / sample_rate).exp();
        let omega = 2.0 * PI * freq / sample_rate;
        let a1 = 2.0 * r * omega.cos();
        let a2 = -r * r;
        let b0 = 1.0 - r;
        Self {
            a1,
            a2,
            b0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn process(&mut self, x: f32) -> f32 {
        let out = self.b0 * x + self.a1 * self.y1 + self.a2 * self.y2;
        self.y2 = self.y1;
        self.y1 = out;
        out
    }
}

/// Speaker acoustic profile defining fundamental pitch and vocal tract formant shifts.
#[derive(Debug, Clone, Copy)]
pub struct SpeakerProfile {
    pub name: &'static str,
    pub f0: f32,
    pub formant_multiplier: f32,
    pub amplitude_scale: f32,
}

pub const SPEAKER_MALE: SpeakerProfile = SpeakerProfile {
    name: "Male-Baritone",
    f0: 120.0,
    formant_multiplier: 1.0,
    amplitude_scale: 0.18,
};

pub const SPEAKER_FEMALE: SpeakerProfile = SpeakerProfile {
    name: "Female-Soprano",
    f0: 220.0,
    formant_multiplier: 1.18,
    amplitude_scale: 0.16,
};

pub const SPEAKER_CHILD: SpeakerProfile = SpeakerProfile {
    name: "Child-HighPitch",
    f0: 290.0,
    formant_multiplier: 1.30,
    amplitude_scale: 0.14,
};

/// Vowel formant presets (F1, F2, F3 frequencies in Hz, bandwidths ~80, 100, 120 Hz).
#[derive(Debug, Clone, Copy)]
pub enum FormantVowel {
    Ah, // /a/ (e.g. father): ~730, 1090, 2440 Hz
    Ee, // /i/ (e.g. see):    ~270, 2290, 3010 Hz
    Oh, // /o/ (e.g. go):     ~570,  840, 2410 Hz
    Schwa, // /ə/ (e.g. "uh"):~500, 1500, 2500 Hz
    NasalM, // /m/ (e.g. "um"):~280, 1000, 2200 Hz
}

impl FormantVowel {
    fn formants(&self, multiplier: f32) -> [(f32, f32); 3] {
        let base = match self {
            Self::Ah => [(730.0, 90.0), (1090.0, 110.0), (2440.0, 140.0)],
            Self::Ee => [(270.0, 70.0), (2290.0, 120.0), (3010.0, 150.0)],
            Self::Oh => [(570.0, 80.0), (840.0, 100.0), (2410.0, 130.0)],
            Self::Schwa => [(500.0, 80.0), (1500.0, 100.0), (2500.0, 140.0)],
            Self::NasalM => [(280.0, 100.0), (1000.0, 150.0), (2200.0, 200.0)],
        };
        [
            (base[0].0 * multiplier, base[0].1),
            (base[1].0 * multiplier, base[1].1),
            (base[2].0 * multiplier, base[2].1),
        ]
    }
}

/// Synthesize a speech phoneme with glottal pulse excitation or unvoiced aspiration noise.
fn synthesize_phoneme(
    vowel: FormantVowel,
    profile: &SpeakerProfile,
    duration_secs: f32,
    sample_rate: u32,
    is_whisper: bool,
    rng: &mut FastRng,
) -> Vec<f32> {
    let num_samples = (duration_secs * sample_rate as f32).round() as usize;
    let mut out = Vec::with_capacity(num_samples);

    let formants = vowel.formants(profile.formant_multiplier);
    let mut r1 = FormantResonator::new(formants[0].0, formants[0].1, sample_rate as f32);
    let mut r2 = FormantResonator::new(formants[1].0, formants[1].1, sample_rate as f32);
    let mut r3 = FormantResonator::new(formants[2].0, formants[2].1, sample_rate as f32);

    let f0 = profile.f0;
    let sr = sample_rate as f32;
    let mut phase = 0.0f32;

    for i in 0..num_samples {
        // Excitation source:
        let exc = if is_whisper {
            // Unvoiced aspiration turbulence (noise source)
            rng.next_gaussian() * 0.70
        } else {
            // Voiced glottal pulse train with harmonics
            phase += 2.0 * PI * f0 / sr;
            if phase > 2.0 * PI {
                phase -= 2.0 * PI;
            }
            // Sum of fundamental + first 6 harmonics with -6dB roll-off
            let mut pulse = phase.sin();
            pulse += 0.50 * (2.0 * phase).sin();
            pulse += 0.25 * (3.0 * phase).sin();
            pulse += 0.12 * (4.0 * phase).sin();
            pulse += 0.06 * (5.0 * phase).sin();
            pulse
        };

        // Formants in parallel
        let s = r1.process(exc) * 0.6 + r2.process(exc) * 0.3 + r3.process(exc) * 0.1;

        // Smooth raised-cosine window envelope (attack and release)
        let t = i as f32 / num_samples as f32;
        let env = if t < 0.1 {
            (1.0 - (PI * t / 0.1).cos()) * 0.5
        } else if t > 0.9 {
            (1.0 - (PI * (1.0 - t) / 0.1).cos()) * 0.5
        } else {
            1.0
        };

        let amp = if is_whisper {
            profile.amplitude_scale * 0.45 // Whispered speech RMS ~ 0.006 - 0.018
        } else {
            profile.amplitude_scale
        };

        out.push(s * env * amp);
    }
    out
}

/// Synthesize high-speed speech (>180 wpm).
///
/// Typical rapid speech has 5.5 to 7 syllables per second (syllable duration ~140ms).
/// At 200 wpm, an utterance of 8 words (~12 syllables) takes ~1.8 to 2.2 seconds.
pub fn synthesize_high_speed_speech(profile: &SpeakerProfile, target_wpm: f32) -> Vec<f32> {
    assert!(target_wpm >= 180.0, "target WPM must be >= 180");
    let mut rng = FastRng::new(101);

    // Words per second: WPM / 60
    // With ~1.5 syllables per word: syllables_per_sec = (WPM / 60) * 1.5
    let syllables_per_sec = (target_wpm / 60.0) * 1.5;
    let syllable_duration = 1.0 / syllables_per_sec; // ~140ms for 200 WPM

    let sequence = [
        FormantVowel::Ah,
        FormantVowel::Ee,
        FormantVowel::Oh,
        FormantVowel::Schwa,
        FormantVowel::Ah,
        FormantVowel::Oh,
        FormantVowel::Ee,
        FormantVowel::Schwa,
        FormantVowel::Ah,
        FormantVowel::Ee,
    ];

    let mut speech = Vec::new();
    for vowel in sequence {
        let chunk = synthesize_phoneme(
            vowel,
            profile,
            syllable_duration * 0.85,
            SAMPLE_RATE,
            false,
            &mut rng,
        );
        speech.extend_from_slice(&chunk);
        // Short micro-pause between syllables (15ms)
        let pause_len = (0.015 * SAMPLE_RATE as f32) as usize;
        speech.extend(vec![0.0f32; pause_len]);
    }
    speech
}

/// Synthesize whispered / low-energy speech.
///
/// Aspiration noise excited formants with strictly bounded RMS (<0.035, >0.003).
pub fn synthesize_whispered_speech(profile: &SpeakerProfile, duration_secs: f32) -> Vec<f32> {
    let mut rng = FastRng::new(202);
    let mut speech = Vec::new();

    let phonemes = [FormantVowel::Schwa, FormantVowel::Ee, FormantVowel::Ah, FormantVowel::Oh];
    let piece_dur = duration_secs / phonemes.len() as f32;

    for vowel in phonemes {
        let chunk = synthesize_phoneme(vowel, profile, piece_dur, SAMPLE_RATE, true, &mut rng);
        speech.extend_from_slice(&chunk);
    }
    speech
}

/// Synthesize natural hesitation fillers ("um... uh...") interspersed with speech.
///
/// Structure:
/// 1. Hesitation filler "um" (schwa -> nasal murmur /m/, 450ms)
/// 2. Thinking pause (300ms)
/// 3. Hesitation filler "uh" (elongated schwa with pitch intonation drop, 400ms)
/// 4. Thinking pause (250ms)
/// 5. Resumed fluent speech phrase (Ah -> Ee -> Oh, 700ms)
pub fn synthesize_hesitation_filler_speech(profile: &SpeakerProfile) -> Vec<f32> {
    let mut rng = FastRng::new(303);
    let mut speech = Vec::new();

    // 1. "um" = /ə/ (180ms) + /m/ (270ms)
    speech.extend(synthesize_phoneme(FormantVowel::Schwa, profile, 0.18, SAMPLE_RATE, false, &mut rng));
    speech.extend(synthesize_phoneme(FormantVowel::NasalM, profile, 0.27, SAMPLE_RATE, false, &mut rng));

    // 2. Hesitation silence
    speech.extend(vec![0.0f32; (0.30 * SAMPLE_RATE as f32) as usize]);

    // 3. "uh" = /ə/ with falling intonation (350ms)
    let mut uh_profile = *profile;
    uh_profile.f0 *= 0.88; // Slight drop in fundamental pitch
    speech.extend(synthesize_phoneme(FormantVowel::Schwa, &uh_profile, 0.35, SAMPLE_RATE, false, &mut rng));

    // 4. Hesitation silence
    speech.extend(vec![0.0f32; (0.25 * SAMPLE_RATE as f32) as usize]);

    // 5. Fluent phrase burst
    speech.extend(synthesize_phoneme(FormantVowel::Ah, profile, 0.22, SAMPLE_RATE, false, &mut rng));
    speech.extend(synthesize_phoneme(FormantVowel::Ee, profile, 0.24, SAMPLE_RATE, false, &mut rng));
    speech.extend(synthesize_phoneme(FormantVowel::Oh, profile, 0.26, SAMPLE_RATE, false, &mut rng));

    speech
}

// ============================================================================
// 4. REALISTIC BACKGROUND NOISE PROFILES
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmbientNoiseType {
    KeyboardClatter,
    CoffeeShopMurmur,
    OfficeSoundscape,
}

/// Generate realistic mechanical keyboard clatter.
///
/// Simulates intermittent rapid keystrokes with high-frequency slider impact (~3.2 kHz)
/// and bottom-out housing resonance (~1.1 kHz).
pub fn generate_keyboard_clatter(duration_secs: f32) -> Vec<f32> {
    let num_samples = (duration_secs * SAMPLE_RATE as f32).round() as usize;
    let mut out = vec![0.0f32; num_samples];
    let mut rng = FastRng::new(404);

    // Keystroke intervals (average 6.5 keystrokes per second with jitter)
    let mut sample_idx = (0.08 * SAMPLE_RATE as f32) as usize;
    while sample_idx < num_samples {
        let keystroke_len = (0.035 * SAMPLE_RATE as f32) as usize; // 35ms key event
        let impact_freq = 3200.0 + rng.next_gaussian() * 200.0;
        let clack_freq = 1100.0 + rng.next_gaussian() * 100.0;

        for k in 0..keystroke_len {
            if sample_idx + k >= num_samples {
                break;
            }
            let t = k as f32 / SAMPLE_RATE as f32;
            let decay = (-t * 120.0).exp(); // Fast exponential damping
            let click = (2.0 * PI * impact_freq * t).sin() * 0.7;
            let clack = (2.0 * PI * clack_freq * t).sin() * 0.4;
            let noise_burst = rng.next_gaussian() * 0.3;
            out[sample_idx + k] += (click + clack + noise_burst) * decay * 0.15;
        }

        // Advance to next keystroke: 120ms to 180ms
        let delta_ms = 0.12 + (rng.next_u32() % 60) as f32 * 0.001;
        sample_idx += (delta_ms * SAMPLE_RATE as f32) as usize;
    }

    out
}

/// Generate realistic coffee shop murmur.
///
/// Diffuse multi-talker babble (superposition of multiple low-frequency murmurs ~200-800Hz)
/// plus occasional ambient ceramic cutlery clinks (~4.8 kHz).
pub fn generate_coffee_shop_murmur(duration_secs: f32) -> Vec<f32> {
    let num_samples = (duration_secs * SAMPLE_RATE as f32).round() as usize;
    let mut out = vec![0.0f32; num_samples];
    let mut rng = FastRng::new(505);

    // 1. Diffuse crowd babble (superposition of 8 detuned low-frequency oscillator murmur bands)
    let murmur_freqs = [180.0, 240.0, 310.0, 420.0, 510.0, 630.0, 750.0, 880.0];
    for &freq in &murmur_freqs {
        let mut phase = rng.next_f32() * 2.0 * PI;
        let mut mod_phase = rng.next_f32() * 2.0 * PI;
        let mod_speed = 1.5 + rng.next_f32() * 0.5;

        for sample in out.iter_mut() {
            phase += 2.0 * PI * freq / SAMPLE_RATE as f32;
            mod_phase += 2.0 * PI * mod_speed / SAMPLE_RATE as f32;
            let env = 0.5 + 0.5 * mod_phase.sin();
            *sample += phase.sin() * env * 0.02;
        }
    }

    // 2. Diffuse room reverberation pink noise
    let mut pink_state = 0.0f32;
    for sample in out.iter_mut() {
        let white = rng.next_gaussian();
        pink_state = 0.95 * pink_state + 0.05 * white;
        *sample += pink_state * 0.03;
    }

    // 3. Occasional ceramic cup clinks at 0.5s and 1.8s
    let clink_times = [0.55, 1.75];
    for &t_clink in &clink_times {
        let start = (t_clink * SAMPLE_RATE as f32) as usize;
        let clink_len = (0.06 * SAMPLE_RATE as f32) as usize;
        for k in 0..clink_len {
            if start + k < num_samples {
                let t = k as f32 / SAMPLE_RATE as f32;
                let decay = (-t * 70.0).exp();
                let ring = (2.0 * PI * 4800.0 * t).sin() * 0.5 + (2.0 * PI * 6100.0 * t).sin() * 0.3;
                out[start + k] += ring * decay * 0.08;
            }
        }
    }

    out
}

/// Generate realistic office soundscape.
///
/// Low-frequency HVAC electrical hum (60 Hz fundamental + 120 Hz, 180 Hz harmonics)
/// plus air conditioning pink noise air stream and distant subtle movement.
pub fn generate_office_soundscape(duration_secs: f32) -> Vec<f32> {
    let num_samples = (duration_secs * SAMPLE_RATE as f32).round() as usize;
    let mut out = vec![0.0f32; num_samples];
    let mut rng = FastRng::new(606);

    // 1. 60 Hz electrical and ventilation blower hum
    let mut p60 = 0.0f32;
    let mut p120 = 0.0f32;
    let mut p180 = 0.0f32;
    let sr = SAMPLE_RATE as f32;

    for sample in out.iter_mut() {
        p60 += 2.0 * PI * 60.0 / sr;
        p120 += 2.0 * PI * 120.0 / sr;
        p180 += 2.0 * PI * 180.0 / sr;

        let hum = 0.04 * p60.sin() + 0.02 * p120.sin() + 0.01 * p180.sin();
        *sample += hum;
    }

    // 2. Air conditioning gentle pink noise
    let mut b0 = 0.0f32;
    let mut b1 = 0.0f32;
    let mut b2 = 0.0f32;
    for sample in out.iter_mut() {
        let white = rng.next_gaussian();
        b0 = 0.99765 * b0 + white * 0.0990460;
        b1 = 0.96300 * b1 + white * 0.2965164;
        b2 = 0.57000 * b2 + white * 1.0526913;
        let pink = b0 + b1 + b2 + white * 0.1848;
        *sample += pink * 0.012;
    }

    out
}

/// Helper to generate ambient noise by type.
pub fn generate_ambient_noise(noise_type: AmbientNoiseType, duration_secs: f32) -> Vec<f32> {
    match noise_type {
        AmbientNoiseType::KeyboardClatter => generate_keyboard_clatter(duration_secs),
        AmbientNoiseType::CoffeeShopMurmur => generate_coffee_shop_murmur(duration_secs),
        AmbientNoiseType::OfficeSoundscape => generate_office_soundscape(duration_secs),
    }
}

// ============================================================================
// 5. TEST SUITE IMPLEMENTATION
// ============================================================================

#[test]
fn test_snr_mixer_calibration() {
    let speech = synthesize_high_speed_speech(&SPEAKER_MALE, 190.0);
    assert!(!speech.is_empty());
    let speech_rms = compute_rms(&speech);
    assert!(speech_rms > 0.01, "speech should have meaningful energy");

    let noise = generate_keyboard_clatter(3.0);
    assert!(!noise.is_empty());

    // Test calibrated SNRs: 5 dB, 10 dB, 15 dB
    for &target_snr in &[5.0f32, 10.0f32, 15.0f32] {
        let mixed = mix_with_noise(&speech, &noise, target_snr);
        assert_eq!(mixed.len(), speech.len());

        // Extract noise component: mixed - speech
        let noise_component: Vec<f32> = mixed
            .iter()
            .zip(speech.iter())
            .map(|(&m, &s)| m - s)
            .collect();

        let measured_snr = compute_measured_snr_db(&speech, &noise_component);
        let error = (measured_snr - target_snr).abs();
        assert!(
            error < 0.2,
            "SNR calibration drift for target {} dB: measured {} dB (error {} dB)",
            target_snr,
            measured_snr,
            error
        );
    }
}

#[test]
fn test_multi_speaker_high_speed_speech_simulation() {
    // High-speed speech (>180 wpm) across male, female, and child speakers
    let speakers = [SPEAKER_MALE, SPEAKER_FEMALE, SPEAKER_CHILD];

    for speaker in &speakers {
        let target_wpm = 205.0; // >180 wpm
        let speech = synthesize_high_speed_speech(speaker, target_wpm);

        let duration = speech.len() as f32 / SAMPLE_RATE as f32;
        assert!(
            (1.0..=3.0).contains(&duration),
            "speech duration unexpected: {}s",
            duration
        );

        let rms = compute_rms(&speech);
        assert!(
            (0.05..=0.35).contains(&rms),
            "speaker {} RMS outside realistic speech range: {}",
            speaker.name,
            rms
        );

        // VAD must detect speech in the synthesized utterance
        assert!(
            contains_speech(&speech, TARGET_SAMPLE_RATE),
            "VAD failed to detect high-speed speech for {}",
            speaker.name
        );
    }
}

#[test]
fn test_whispered_low_energy_speech_simulation() {
    for speaker in &[SPEAKER_MALE, SPEAKER_FEMALE] {
        let whisper = synthesize_whispered_speech(speaker, 1.5);
        let rms = compute_rms(&whisper);

        // Whispered speech must be significantly lower energy than voiced speech (<0.035),
        // but distinctly non-zero (>0.003).
        assert!(
            rms > 0.003 && rms < 0.035,
            "whispered speech RMS outside expected low-energy bounds (got {}): expected [0.003, 0.035]",
            rms
        );

        // Whispered speech energy must be detectable by speech gate
        assert!(
            contains_speech(&whisper, TARGET_SAMPLE_RATE),
            "whispered speech should pass speech presence gate"
        );
    }
}

#[test]
fn test_natural_hesitation_fillers_simulation() {
    for speaker in &[SPEAKER_MALE, SPEAKER_FEMALE] {
        let filler_speech = synthesize_hesitation_filler_speech(speaker);
        let duration = filler_speech.len() as f32 / SAMPLE_RATE as f32;

        // Utterance with "um... uh..." fillers should be ~1.8 to 2.5 seconds
        assert!(
            (1.5..=3.0).contains(&duration),
            "hesitation speech duration unexpected: {}s",
            duration
        );

        // Speech detector must confirm speech frames are present
        assert!(
            contains_speech(&filler_speech, TARGET_SAMPLE_RATE),
            "VAD should detect speech in hesitation filler sequence"
        );

        // Trim silence should successfully retain the speech and filler components
        let trimmed = trim_silence(&filler_speech, TARGET_SAMPLE_RATE);
        assert!(
            trimmed.is_some(),
            "hesitation filler speech should pass trim_silence"
        );
    }
}

#[test]
fn test_ambient_noise_profiles_generation() {
    let dur = 2.0;

    // 1. Keyboard clatter
    let kb = generate_keyboard_clatter(dur);
    assert_eq!(kb.len(), (dur * SAMPLE_RATE as f32) as usize);
    let kb_rms = compute_rms(&kb);
    assert!(kb_rms > 0.005 && kb_rms < 0.1, "keyboard RMS: {}", kb_rms);

    // 2. Coffee shop murmur
    let cs = generate_coffee_shop_murmur(dur);
    assert_eq!(cs.len(), (dur * SAMPLE_RATE as f32) as usize);
    let cs_rms = compute_rms(&cs);
    assert!(cs_rms > 0.01 && cs_rms < 0.1, "coffee shop RMS: {}", cs_rms);

    // 3. Office soundscape
    let off = generate_office_soundscape(dur);
    assert_eq!(off.len(), (dur * SAMPLE_RATE as f32) as usize);
    let off_rms = compute_rms(&off);
    assert!(off_rms > 0.01 && off_rms < 0.1, "office RMS: {}", off_rms);
}

#[test]
fn test_vad_boundary_and_rtf_regression_gate() {
    // Test that noisy speech boundaries are cleanly trimmed and RTF <= 0.05x on CPU
    let speaker = SPEAKER_MALE;
    let clean_speech = synthesize_high_speed_speech(&speaker, 195.0);
    let speech_len = clean_speech.len();
    let speech_dur_secs = speech_len as f32 / SAMPLE_RATE as f32;

    let noise_profiles = [
        AmbientNoiseType::KeyboardClatter,
        AmbientNoiseType::CoffeeShopMurmur,
        AmbientNoiseType::OfficeSoundscape,
    ];
    let snr_levels = [15.0f32, 10.0f32, 5.0f32];

    for noise_type in noise_profiles {
        for snr_db in snr_levels {
            // Generate ambient noise
            let full_dur = speech_dur_secs + 2.0; // 1.0s leading silence/ambient, 1.0s trailing
            let noise = generate_ambient_noise(noise_type, full_dur);

            // Construct composite audio: [leading ambient] + [speech + noise at calibrated SNR] + [trailing ambient]
            let lead_samples = (1.00 * SAMPLE_RATE as f32) as usize;
            let trail_samples = (1.00 * SAMPLE_RATE as f32) as usize;

            let mut composite = Vec::with_capacity(lead_samples + speech_len + trail_samples);

            // Pre-speech ambient room noise (attenuated room baseline floor before speech begins)
            let ambient_lead = &noise[..lead_samples];
            let ambient_scale = 0.01;
            for &n in ambient_lead {
                composite.push(n * ambient_scale);
            }

            // Speech mixed with noise at calibrated SNR (e.g. 15 dB, 10 dB, 5 dB)
            let speech_noise = &noise[lead_samples..lead_samples + speech_len];
            let mixed_speech = mix_with_noise(&clean_speech, speech_noise, snr_db);
            composite.extend_from_slice(&mixed_speech);

            // Post-speech ambient room noise
            let trail_start = lead_samples + speech_len;
            let ambient_trail = &noise[trail_start..trail_start + trail_samples];
            for &n in ambient_trail {
                composite.push(n * ambient_scale);
            }

            let total_dur_secs = composite.len() as f64 / SAMPLE_RATE as f64;

            // Measure RTF of VAD boundary trimming on CPU
            let start_time = Instant::now();
            let trimmed_opt = trim_silence(&composite, TARGET_SAMPLE_RATE);
            let elapsed = start_time.elapsed();

            let elapsed_secs = elapsed.as_secs_f64();
            let rtf = elapsed_secs / total_dur_secs;

            // Assert Real-Time Factor (RTF) is <= 0.05x on CPU (NFR-T-1 / ARCHITECTURE §10.1)
            assert!(
                rtf <= 0.05,
                "RTF exceeded regression ceiling: {:?} for {:?} at {} dB SNR (RTF = {:.5}x, target <= 0.05x)",
                elapsed,
                noise_type,
                snr_db,
                rtf
            );

            // Assert clean boundary trimming
            assert!(
                trimmed_opt.is_some(),
                "VAD boundary detection failed for {:?} at {} dB SNR",
                noise_type,
                snr_db
            );

            let trimmed = trimmed_opt.unwrap();

            // Trimmed buffer must be strictly shorter than composite buffer (leading/trailing ambient trimmed)
            assert!(
                trimmed.len() < composite.len(),
                "VAD failed to trim leading/trailing noise: trimmed len {} == composite len {}",
                trimmed.len(),
                composite.len()
            );

            // Trimmed buffer must retain the speech utterance (plus padding)
            assert!(
                trimmed.len() >= speech_len / 2,
                "VAD trimmed too aggressively: trimmed {} < speech {}",
                trimmed.len(),
                speech_len
            );
        }
    }
}

#[test]
fn test_rtf_regression_gate_benchmark() {
    // 10 seconds continuous composite audio benchmark
    let benchmark_duration = 10.0f32;
    let speech = synthesize_high_speed_speech(&SPEAKER_FEMALE, 200.0);
    let noise = generate_office_soundscape(benchmark_duration);

    // Tile speech across benchmark duration with calibrated 10 dB SNR
    let num_samples = (benchmark_duration * SAMPLE_RATE as f32) as usize;
    let mut tiled_speech = Vec::with_capacity(num_samples);
    while tiled_speech.len() < num_samples {
        tiled_speech.extend_from_slice(&speech);
    }
    tiled_speech.truncate(num_samples);

    let noisy_audio = mix_with_noise(&tiled_speech, &noise, 10.0);
    let audio_dur_secs = noisy_audio.len() as f64 / SAMPLE_RATE as f64;

    let start = Instant::now();
    let trimmed = trim_silence(&noisy_audio, TARGET_SAMPLE_RATE);
    let elapsed = start.elapsed();

    assert!(trimmed.is_some());
    let rtf = elapsed.as_secs_f64() / audio_dur_secs;

    println!(
        "RTF Benchmark: processed {:.2}s of noisy audio in {:?} -> RTF = {:.6}x (gate <= 0.05x)",
        audio_dur_secs, elapsed, rtf
    );

    assert!(
        rtf <= 0.05,
        "RTF benchmark failed regression gate: {:.6}x > 0.05x",
        rtf
    );
}

#[test]
fn test_endurance_and_memory_audit() {
    // Repeated chunked decoding cycles to assert flat memory footprint (NFR-T-4 / ARCHITECTURE §10.5)
    // Runs 100 continuous 50ms chunk processing cycles simulating live streaming dictation.

    fn get_process_rss_bytes() -> usize {
        if let Ok(content) = std::fs::read_to_string("/proc/self/statm") {
            let parts: Vec<&str> = content.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(pages) = parts[1].parse::<usize>() {
                    return pages * 4096;
                }
            }
        }
        0
    }

    let chunk_samples = (0.05 * SAMPLE_RATE as f32) as usize; // 50ms chunks = 800 samples
    let chunk_template = synthesize_phoneme(
        FormantVowel::Ah,
        &SPEAKER_MALE,
        0.05,
        SAMPLE_RATE,
        false,
        &mut FastRng::new(707),
    );

    // Warm-up cycles to establish initial runtime/allocator baseline
    for _ in 0..15 {
        let _ = contains_speech(&chunk_template, TARGET_SAMPLE_RATE);
        let _ = denoise::suppress_16k(&chunk_template);
        let _ = trim_silence(&chunk_template, TARGET_SAMPLE_RATE);
    }

    let baseline_rss = get_process_rss_bytes();

    // 100 chunked decoding cycles
    let num_cycles = 100;
    let mut ring_buffer = Vec::with_capacity(chunk_samples * 20);

    for i in 0..num_cycles {
        // Dynamic chunk with noise
        let noise_chunk = generate_keyboard_clatter(0.05);
        let mixed_chunk = mix_with_noise(&chunk_template, &noise_chunk, 12.0);

        // 1. Live speech detection
        let _has_speech = contains_speech(&mixed_chunk, TARGET_SAMPLE_RATE);

        // 2. Neural noise suppression
        let denoised = denoise::suppress_16k(&mixed_chunk);

        // 3. Sliding chunk accumulator (simulate 1-second sliding window)
        ring_buffer.extend_from_slice(&denoised);
        if ring_buffer.len() > chunk_samples * 20 {
            ring_buffer.drain(0..chunk_samples);
        }

        // 4. VAD trimming on accumulated buffer
        let _trimmed = trim_silence(&ring_buffer, TARGET_SAMPLE_RATE);

        // Every 25 cycles, verify memory does not explode
        if i % 25 == 0 && baseline_rss > 0 {
            let current_rss = get_process_rss_bytes();
            let growth = current_rss.saturating_sub(baseline_rss);
            // Growth should remain strictly bounded (<15 MB)
            assert!(
                growth < 15 * 1024 * 1024,
                "Memory growth detected at cycle {}: grew by {} bytes",
                i,
                growth
            );
        }
    }

    let final_rss = get_process_rss_bytes();
    if baseline_rss > 0 && final_rss > 0 {
        let total_growth = final_rss.saturating_sub(baseline_rss);
        let total_growth_mb = total_growth as f64 / (1024.0 * 1024.0);
        let final_rss_mb = final_rss as f64 / (1024.0 * 1024.0);

        println!(
            "Endurance audit complete: {} cycles, baseline RSS: {:.2} MB, final RSS: {:.2} MB, net growth: {:.2} MB",
            num_cycles,
            baseline_rss as f64 / (1024.0 * 1024.0),
            final_rss_mb,
            total_growth_mb
        );

        // NFR-T-4 requirement: maintain flat RSS memory footprint (<150 MB); zero leaks
        assert!(
            final_rss_mb < 150.0,
            "Process RSS exceeded 150 MB ceiling: {:.2} MB",
            final_rss_mb
        );
        assert!(
            total_growth_mb < 15.0,
            "Memory leaked over {} cycles: net growth {:.2} MB",
            num_cycles,
            total_growth_mb
        );
    }
}

// ============================================================================
// 6. 5-FRICTION-POINT REALISTIC ACOUSTIC & PARITY TEST SUITE
// ============================================================================

#[test]
fn test_acoustic_friction_case_1_spoken_symbol_vs_literal_word() {
    // Friction Case 1: Spoken symbol vs. literal word.
    // In "The patient has colon cancer", "colon" is an anatomical noun preceding "cancer"
    // and must not be transcribed as the punctuation symbol ':'.
    let raw = "The patient has colon cancer";

    // Verify syntactic lookaround detects literal context:
    assert!(is_literal_punctuation_context(
        &["the", "patient", "has"],
        "colon",
        &["cancer"]
    ));

    // Verify apply_commands retains "colon" literally
    let after_commands = apply_commands(raw);
    assert_eq!(after_commands, "The patient has colon cancer");

    // Verify clean_text formats as proper sentence with terminal period
    let cleaned = clean_text(&after_commands);
    assert_eq!(cleaned, "The patient has colon cancer.");

    // Contrast: Delimiter context "We need three items colon apples" should convert to ':'
    let delimiter_raw = "We need three items colon apples";
    assert!(!is_literal_punctuation_context(
        &["three", "items"],
        "colon",
        &["apples"]
    ));
    let delimiter_res = apply_commands(delimiter_raw);
    assert_eq!(delimiter_res, "We need three items: apples");
}

#[test]
fn test_acoustic_friction_case_2_eager_escape() {
    // Friction Case 2: Eager escape gate.
    // Speaking "literal colon" immediately renders "colon" without emitting punctuation symbol ':'
    // and strips the eager escape modifier "literal" on the fly.
    let raw = "literal colon";
    let output = apply_commands(raw);
    assert_eq!(output, "colon");

    // Also verify "word colon" and in sentence context "please write literal colon on line one"
    assert_eq!(apply_commands("word colon"), "colon");
    assert_eq!(
        apply_commands("please write literal colon on line one"),
        "please write colon on line one"
    );

    // Verify live IBus pre-edit engine formats incoming text seamlessly
    let mut engine = IbusEngineState::new();
    let escaped = apply_commands("literal colon");
    engine.update_preedit(&escaped, escaped.len() as u32, true);
    assert_eq!(engine.preedit_text, "colon");
    assert!(engine.visible);

    // With surrounding text context at the start of a buffer, it capitalizes:
    engine.set_surrounding_text("", 0, 0);
    engine.update_preedit(&escaped, escaped.len() as u32, true);
    assert_eq!(engine.preedit_text, "Colon");
}

#[test]
fn test_acoustic_friction_case_3_conversational_speech_repair() {
    // Friction Case 3: Conversational speech repair.
    // In "let's meet at four no wait five pm", the false start "four" is abandoned
    // and pruned back to clausal boundary "at", retaining "five pm".
    let raw = "let's meet at four no wait five pm";

    let repaired = prune_speech_repairs(raw);
    assert_eq!(repaired, "let's meet at five pm");

    let cleaned = clean_text(raw);
    assert_eq!(cleaned, "Let's meet at five pm.");

    // Also verify without leading contraction
    let raw_simple = "meet at four no wait five pm";
    let repaired_simple = prune_speech_repairs(raw_simple);
    assert_eq!(repaired_simple, "meet at five pm");
    assert_eq!(clean_text(raw_simple), "Meet at five pm.");
}

#[test]
fn test_acoustic_friction_case_4_homophone_resolution() {
    // Friction Case 4: Homophone resolution (they're / their / there).
    // Disambiguates phonetically identical acoustic sequences via syntactic collocations:
    // - "They're" before progressive verb "going"
    // - "their" possessive determiner before noun "bags"
    // - "there" locative adverb
    let ambiguous_input = [
        "there", "going", "to", "put", "there", "bags", "over", "their",
    ];
    let disambiguated = disambiguate_homophones(&ambiguous_input);
    let expected = vec![
        "they're", "going", "to", "put", "their", "bags", "over", "there",
    ];
    assert_eq!(disambiguated, expected);

    // Verify end-to-end command and cleanup pipeline
    let raw_canonical = "They're going to put their bags over there";
    let formatted = apply_commands(raw_canonical);
    assert_eq!(formatted, "They're going to put their bags over there");

    // Verify full pipeline disambiguation on ambiguous raw input
    let raw_ambiguous = "there going to put there bags over their";
    let formatted_ambiguous = apply_commands(raw_ambiguous);
    assert_eq!(
        formatted_ambiguous,
        "they're going to put their bags over there"
    );

    let cleaned = clean_text(&formatted_ambiguous);
    assert_eq!(cleaned, "They're going to put their bags over there.");

    // Verify GroundingGuard safely whitelists this homophone resolution without flagging
    let guard_res = apply_cleanup(&formatted_ambiguous, "clean");
    assert_eq!(guard_res.text, "They're going to put their bags over there.");
    assert!(
        !guard_res.flagged,
        "Homophone resolution must not be flagged by GroundingGuard"
    );
}

#[test]
fn test_acoustic_friction_case_5_prosodic_question_vs_period() {
    // Friction Case 5: Prosodic question vs. period intonation.
    // - Terminal falling pitch (Delta F0 <= -35 Hz) + energy decay + silence >= 380ms -> '.'
    // - Terminal rising uptalk (Delta F0 >= +30 Hz) + silence >= 250ms -> '?'

    let sample_rate = SAMPLE_RATE as f32;

    // 1. Synthesize Falling Pitch Utterance (200 Hz -> 130 Hz with exponential energy decay)
    let dur_falling = 0.250f32;
    let n_falling = (sample_rate * dur_falling) as usize;
    let mut falling_audio = Vec::with_capacity(n_falling);
    let mut phase = 0.0f32;
    for i in 0..n_falling {
        let t = i as f32 / sample_rate;
        let frac = t / dur_falling;
        let freq = 200.0 - 70.0 * frac; // 200 Hz -> 130 Hz (Delta F0 = -70 Hz)
        let amp = 0.8 * (-3.0 * frac).exp();
        phase += 2.0 * PI * freq / sample_rate;
        falling_audio.push(amp * phase.sin());
    }

    let mut tracker_fall = ProsodyTracker::new();
    let events_fall = tracker_fall.process_chunk(&falling_audio);
    let has_falling = events_fall.contains(&ProsodicEvent::FallingIntonation);
    assert!(
        has_falling,
        "ProsodyTracker must detect FallingIntonation from falling pitch sweep"
    );

    let mut trigger_fall = ProsodicPunctuationTrigger::new("clean");
    trigger_fall.feed_event(ProsodicEvent::FallingIntonation);
    // Pause >= 380 ms triggers period
    let action_fall = trigger_fall.feed_pause(Duration::from_millis(400));
    assert_eq!(
        action_fall,
        Some(ProsodicPunctuationAction::period()),
        "Falling intonation followed by 400ms pause must trigger period ('.')"
    );
    let formatted_fall = format_streaming_punctuation_with_prosody(
        "The patient has colon cancer",
        action_fall,
    );
    assert_eq!(formatted_fall, "The patient has colon cancer.");

    // Also verify LibriSpeech all-caps output normalizes to sentence case with period
    let formatted_allcaps = format_streaming_punctuation_with_prosody(
        "THE PATIENT HAS COLON CANCER",
        action_fall,
    );
    assert_eq!(formatted_allcaps, "The patient has colon cancer.");

    // 2. Synthesize Rising Pitch Utterance (180 Hz -> 250 Hz uptalk)
    let dur_rising = 0.250f32;
    let n_rising = (sample_rate * dur_rising) as usize;
    let mut rising_audio = Vec::with_capacity(n_rising);
    phase = 0.0;
    for i in 0..n_rising {
        let t = i as f32 / sample_rate;
        let frac = t / dur_rising;
        let freq = 180.0 + 70.0 * frac; // 180 Hz -> 250 Hz (Delta F0 = +70 Hz)
        phase += 2.0 * PI * freq / sample_rate;
        rising_audio.push(0.6 * phase.sin());
    }

    let mut tracker_rise = ProsodyTracker::new();
    let events_rise = tracker_rise.process_chunk(&rising_audio);
    let has_rising = events_rise.contains(&ProsodicEvent::RisingIntonation);
    assert!(
        has_rising,
        "ProsodyTracker must detect RisingIntonation from rising pitch sweep"
    );

    let mut trigger_rise = ProsodicPunctuationTrigger::new("clean");
    trigger_rise.feed_event(ProsodicEvent::RisingIntonation);
    // Pause >= 250 ms triggers question mark
    let action_rise = trigger_rise.feed_pause(Duration::from_millis(300));
    assert_eq!(
        action_rise,
        Some(ProsodicPunctuationAction::question_mark()),
        "Rising intonation followed by 300ms pause must trigger question mark ('?')"
    );
    let formatted_rise =
        format_streaming_punctuation_with_prosody("Is the meeting at five", action_rise);
    assert_eq!(formatted_rise, "Is the meeting at five?");

    // Also verify LibriSpeech all-caps interrogative normalizes to sentence case with question mark
    let formatted_rise_allcaps =
        format_streaming_punctuation_with_prosody("IS THE MEETING AT FIVE", action_rise);
    assert_eq!(formatted_rise_allcaps, "Is the meeting at five?");
}

#[test]
fn test_five_friction_points_end_to_end_parity() {
    // Master parity suite asserting all 5 friction points simultaneously

    // Case 1: Spoken symbol vs. literal word
    assert_eq!(
        clean_text(&apply_commands("The patient has colon cancer")),
        "The patient has colon cancer."
    );

    // Case 2: Eager escape
    assert_eq!(apply_commands("literal colon"), "colon");

    // Case 3: Speech repair
    assert_eq!(
        prune_speech_repairs("let's meet at four no wait five pm"),
        "let's meet at five pm"
    );

    // Case 4: Homophone resolution
    assert_eq!(
        clean_text(&apply_commands("there going to put there bags over their")),
        "They're going to put their bags over there."
    );

    // Case 5: Prosodic question vs period
    let mut trigger_period = ProsodicPunctuationTrigger::new("clean");
    trigger_period.feed_event(ProsodicEvent::FallingIntonation);
    let action_dot = trigger_period.feed_pause(Duration::from_millis(400));
    assert_eq!(action_dot.map(|a| a.mark), Some('.'));

    let mut trigger_q = ProsodicPunctuationTrigger::new("clean");
    trigger_q.feed_event(ProsodicEvent::RisingIntonation);
    let action_q = trigger_q.feed_pause(Duration::from_millis(300));
    assert_eq!(action_q.map(|a| a.mark), Some('?'));
}

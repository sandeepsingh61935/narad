//! TOML configuration load/save (FR-ACT-7, FR-AUD-6; paths per ARCHITECTURE §5).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Full daemon settings file (`config.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NaradConfig {
    pub activation: ActivationConfig,
    pub audio: AudioConfig,
    pub general: GeneralConfig,
    pub models: ModelsConfig,
    pub history: HistoryConfig,
    /// Spoken → written phrases (FR-TXT-3). Empty until the user adds one.
    pub replacements: Vec<Replacement>,
    /// Post-correction names and jargon (FR-TXT-2). Applied after replacements.
    pub vocabulary: Vec<Replacement>,
    /// Per-app dictation profiles (FR-CTX-2).
    #[serde(default)]
    pub profiles: Vec<AppProfile>,
}

impl NaradConfig {
    /// Look up a profile for the given application identifier (FR-CTX-2).
    ///
    /// Matches exact identifier (case-insensitive) or domain suffix (e.g. "code" matches "com.visualstudio.code").
    pub fn find_profile(&self, app_id: &str) -> Option<&AppProfile> {
        let norm_target = app_id.trim().to_lowercase();
        if norm_target.is_empty() {
            return None;
        }
        self.profiles.iter().find(|p| {
            let norm_prof = p.app_id.trim().to_lowercase();
            if norm_prof.is_empty() {
                return false;
            }
            norm_prof == norm_target
                || norm_target.split('.').next_back() == Some(norm_prof.as_str())
                || norm_prof.split('.').next_back() == Some(norm_target.as_str())
        })
    }
}

impl Default for NaradConfig {
    fn default() -> Self {
        Self {
            activation: ActivationConfig::default(),
            audio: AudioConfig::default(),
            general: GeneralConfig::default(),
            models: ModelsConfig::default(),
            history: HistoryConfig::default(),
            replacements: Vec::new(),
            vocabulary: Vec::new(),
            profiles: Vec::new(),
        }
    }
}

/// Dictation profile for a specific application (FR-CTX-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProfile {
    pub app_id: String,
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paste_with_shift: Option<bool>,
}

/// One user replacement. Matching is case-insensitive and whole-phrase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replacement {
    pub spoken: String,
    pub written: String,
}

/// Hotkeys and hold/tap discrimination (FR-ACT-1, FR-ACT-7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivationConfig {
    /// Global dictate hotkey (hold = push-to-talk, tap = toggle).
    pub dictate_hotkey: String,
    /// Paste the newest final text (FR-INS-4). Press only; not hold/tap.
    pub paste_last_hotkey: String,
    /// Delete the last insertion (FR-INS-5). Press only; not hold/tap.
    #[serde(default = "default_undo_last_hotkey")]
    pub undo_last_hotkey: String,
    /// Selection command hotkey (FR-INS-6).
    #[serde(default = "default_selection_hotkey")]
    pub selection_hotkey: String,
    /// Mode hotkeys enabled (Ctrl+Alt+1..5) (FR-ACT-4).
    #[serde(default = "default_mode_hotkeys_enabled")]
    pub mode_hotkeys_enabled: bool,
    /// Press duration at or above this value is treated as hold; below is tap (FR-ACT-1).
    pub hold_tap_threshold_ms: u64,
}

fn default_undo_last_hotkey() -> String {
    "Ctrl+Alt+Z".to_string()
}

fn default_selection_hotkey() -> String {
    "Ctrl+Alt+R".to_string()
}

fn default_mode_hotkeys_enabled() -> bool {
    true
}

impl Default for ActivationConfig {
    fn default() -> Self {
        Self {
            dictate_hotkey: "Ctrl+Alt+Space".to_string(),
            paste_last_hotkey: "Ctrl+Alt+V".to_string(),
            undo_last_hotkey: default_undo_last_hotkey(),
            selection_hotkey: default_selection_hotkey(),
            mode_hotkeys_enabled: default_mode_hotkeys_enabled(),
            hold_tap_threshold_ms: 250,
        }
    }
}

/// Recording cues and limits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    /// Start/stop/cancel/error sound cues (FR-AUD-6).
    pub sounds_enabled: bool,
    /// Tap-mode and HUD auto-stop after this many seconds without speech (FR-AUD-3).
    /// `0` disables silence stop only. It does not restore the default.
    pub auto_stop_silence_secs: u32,
    /// Hard cap on a single recording in seconds (FR-AUD-3; enforced as a listen timer).
    /// `0` disables the cap only. It does not restore the default.
    pub max_duration_secs: u32,
    /// PipeWire source node name or serial (FR-AUD-4). Empty follows the system default.
    pub input_target: String,
    /// RNNoise on captured PCM before VAD/ASR (FR-AUD-5). Off leaves samples unchanged.
    pub noise_suppression: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sounds_enabled: true,
            auto_stop_silence_secs: 2,
            max_duration_secs: 300,
            input_target: String::new(),
            noise_suppression: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Active transcription/formatting mode name.
    pub active_mode: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            active_mode: "plain".to_string(),
        }
    }
}

fn default_asr_english() -> String {
    "parakeet-ctc-110m-int8".to_string()
}

/// Installed / active model packs (FR-MDL-6; REQUIREMENTS §8 decision #1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelsConfig {
    /// Default English ASR pack id under `~/.local/share/narad/models/asr/<id>/`.
    #[serde(default = "default_asr_english")]
    pub asr_english: String,
    /// Unload speech model after N minutes of inactivity (FR-MDL-5). None = keep loaded.
    #[serde(default)]
    pub idle_unload_minutes: Option<u32>,
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            asr_english: default_asr_english(),
            idle_unload_minutes: None,
        }
    }
}

/// History retention (FR-HIS-7). Audio stays off until the user enables it (NFR-S-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoryConfig {
    /// Store utterance audio. This task only persists the flag; it does not write files.
    pub retain_audio: bool,
    /// Delete history rows older than this many days. Absent means keep text forever.
    pub text_retention_days: Option<u32>,
    /// Unlink audio and clear `audio_path` after this many days. Absent means keep audio forever.
    /// Enabling `retain_audio` does not start a timer until this is set.
    pub audio_retention_days: Option<u32>,
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            retain_audio: false,
            text_retention_days: None,
            audio_retention_days: None,
        }
    }
}

/// `~/.config/narad/config.toml` or `$XDG_CONFIG_HOME/narad/config.toml`.
pub fn resolve_config_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("narad").join("config.toml"));
    }
    std::env::var("HOME").ok().map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("narad")
            .join("config.toml")
    })
}

/// Read and parse an existing config file.
pub fn load(path: &Path) -> Result<NaradConfig> {
    let text =
        fs::read_to_string(path).with_context(|| format!("read config {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse config {}", path.display()))
}

/// Serialize config to TOML and write atomically (create parent dirs).
pub fn save(path: &Path, config: &NaradConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create config dir {}", parent.display()))?;
    }
    let text = toml::to_string_pretty(config).context("serialize config to TOML")?;
    fs::write(path, text).with_context(|| format!("write config {}", path.display()))?;
    Ok(())
}

/// Load from `path`, or write [`NaradConfig::default()`] there if missing.
pub fn load_or_create(path: &Path) -> Result<NaradConfig> {
    if path.is_file() {
        return load(path);
    }
    let defaults = NaradConfig::default();
    save(path, &defaults)?;
    Ok(defaults)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn defaults_match_requirements() {
        let cfg = NaradConfig::default();
        assert_eq!(cfg.activation.dictate_hotkey, "Ctrl+Alt+Space");
        assert_eq!(cfg.activation.paste_last_hotkey, "Ctrl+Alt+V");
        assert_eq!(cfg.activation.undo_last_hotkey, "Ctrl+Alt+Z");
        assert_eq!(cfg.activation.selection_hotkey, "Ctrl+Alt+R");
        assert!(cfg.activation.mode_hotkeys_enabled);
        assert_eq!(cfg.activation.hold_tap_threshold_ms, 250);
        assert!(cfg.audio.sounds_enabled);
        assert_eq!(cfg.audio.auto_stop_silence_secs, 2);
        assert_eq!(cfg.audio.max_duration_secs, 300);
        assert_eq!(cfg.audio.input_target, "");
        assert!(!cfg.audio.noise_suppression);
        assert_eq!(cfg.general.active_mode, "plain");
        assert_eq!(cfg.models.asr_english, "parakeet-ctc-110m-int8");
        assert_eq!(cfg.models.idle_unload_minutes, None);
        assert!(!cfg.history.retain_audio);
        assert_eq!(cfg.history.text_retention_days, None);
        assert_eq!(cfg.history.audio_retention_days, None);
        assert!(cfg.replacements.is_empty());
        assert!(cfg.vocabulary.is_empty());
        assert!(cfg.profiles.is_empty());
    }

    #[test]
    fn selection_and_mode_hotkeys_config_round_trip() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[activation]
selection_hotkey = "Ctrl+Shift+S"
mode_hotkeys_enabled = false
"#,
        )
        .expect("write");
        let cfg = load(&path).expect("load");
        assert_eq!(cfg.activation.selection_hotkey, "Ctrl+Shift+S");
        assert!(!cfg.activation.mode_hotkeys_enabled);
        assert_eq!(cfg.activation.dictate_hotkey, "Ctrl+Alt+Space");
    }

    #[test]
    fn load_or_create_writes_defaults_when_missing() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        assert!(!path.exists());

        let cfg = load_or_create(&path).expect("load_or_create");
        assert_eq!(cfg, NaradConfig::default());
        assert!(path.is_file());

        let again = load(&path).expect("reload");
        assert_eq!(again, NaradConfig::default());
    }

    #[test]
    fn load_parses_custom_toml() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[activation]
dictate_hotkey = "Ctrl+Shift+D"
hold_tap_threshold_ms = 400

[audio]
sounds_enabled = false
auto_stop_silence_secs = 5
max_duration_secs = 120

[general]
active_mode = "email"

[models]
asr_english = "whisper-small-en"
idle_unload_minutes = 10
"#,
        )
        .expect("write fixture");

        let cfg = load(&path).expect("load");
        assert_eq!(cfg.activation.dictate_hotkey, "Ctrl+Shift+D");
        assert_eq!(cfg.activation.paste_last_hotkey, "Ctrl+Alt+V");
        assert_eq!(cfg.activation.hold_tap_threshold_ms, 400);
        assert!(!cfg.audio.sounds_enabled);
        assert_eq!(cfg.audio.auto_stop_silence_secs, 5);
        assert_eq!(cfg.audio.max_duration_secs, 120);
        assert_eq!(cfg.general.active_mode, "email");
        assert_eq!(cfg.models.asr_english, "whisper-small-en");
        assert_eq!(cfg.models.idle_unload_minutes, Some(10));
    }

    #[test]
    fn zero_audio_limits_stay_zero() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "[audio]\nauto_stop_silence_secs = 0\n").expect("write");
        let cfg = load(&path).expect("load");
        assert_eq!(cfg.audio.auto_stop_silence_secs, 0);
        assert_eq!(cfg.audio.max_duration_secs, 300);

        fs::write(&path, "[audio]\nmax_duration_secs = 0\n").expect("write");
        let cfg = load(&path).expect("load");
        assert_eq!(cfg.audio.auto_stop_silence_secs, 2);
        assert_eq!(cfg.audio.max_duration_secs, 0);
    }

    #[test]
    fn partial_file_uses_field_defaults() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "[general]\nactive_mode = \"bullet\"\n").expect("write");

        let cfg = load(&path).expect("load");
        assert_eq!(cfg.general.active_mode, "bullet");
        assert_eq!(cfg.activation.dictate_hotkey, "Ctrl+Alt+Space");
        assert_eq!(cfg.activation.paste_last_hotkey, "Ctrl+Alt+V");
        assert_eq!(cfg.activation.hold_tap_threshold_ms, 250);
        assert_eq!(cfg.models.asr_english, "parakeet-ctc-110m-int8");
        assert_eq!(cfg.models.idle_unload_minutes, None);
        assert!(!cfg.history.retain_audio);
        assert_eq!(cfg.history.text_retention_days, None);
        assert_eq!(cfg.history.audio_retention_days, None);
        assert_eq!(cfg.audio.input_target, "");
        assert!(!cfg.audio.noise_suppression);
        assert!(cfg.replacements.is_empty());
        assert!(cfg.vocabulary.is_empty());
    }

    #[test]
    fn replacements_round_trip_and_missing_key_stays_empty() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[[replacements]]
spoken = "hi there"
written = "hello"
"#,
        )
        .expect("write");
        let cfg = load(&path).expect("load");
        assert_eq!(
            cfg.replacements,
            vec![Replacement {
                spoken: "hi there".into(),
                written: "hello".into(),
            }]
        );
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        let again: NaradConfig = toml::from_str(&text).expect("parse");
        assert_eq!(again.replacements, cfg.replacements);

        fs::write(
            &path,
            r#"
[[vocabulary]]
spoken = "nar ad"
written = "Narad"
"#,
        )
        .expect("write");
        let cfg = load(&path).expect("load");
        assert_eq!(
            cfg.vocabulary,
            vec![Replacement {
                spoken: "nar ad".into(),
                written: "Narad".into(),
            }]
        );
        assert!(cfg.replacements.is_empty());
    }

    #[test]
    fn absent_input_target_is_empty_and_set_id_loads() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "[audio]\nsounds_enabled = true\n").expect("write");
        let cfg = load(&path).expect("load");
        assert_eq!(cfg.audio.input_target, "");

        fs::write(&path, "[audio]\ninput_target = \"alsa_input.usb\"\n").expect("write");
        let cfg = load(&path).expect("load");
        assert_eq!(cfg.audio.input_target, "alsa_input.usb");
        assert!(!cfg.audio.noise_suppression);
    }

    #[test]
    fn noise_suppression_defaults_off_and_round_trips() {
        let cfg = NaradConfig::default();
        assert!(!cfg.audio.noise_suppression);
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        assert!(text.contains("noise_suppression = false"));

        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "[audio]\nsounds_enabled = true\n").expect("write");
        let cfg = load(&path).expect("load");
        assert!(!cfg.audio.noise_suppression);

        fs::write(&path, "[audio]\nnoise_suppression = true\n").expect("write");
        let cfg = load(&path).expect("load");
        assert!(cfg.audio.noise_suppression);
    }

    #[test]
    fn default_retention_round_trips_blank() {
        let cfg = NaradConfig::default();
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        assert!(text.contains("retain_audio = false"));
        assert!(!text.contains("text_retention_days"));
        assert!(!text.contains("audio_retention_days"));
        let loaded: NaradConfig = toml::from_str(&text).expect("parse");
        assert_eq!(loaded.history, HistoryConfig::default());
    }

    #[test]
    fn retention_days_round_trip_when_set() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[history]
retain_audio = true
text_retention_days = 4
audio_retention_days = 2
"#,
        )
        .expect("write");
        let cfg = load(&path).expect("load");
        assert!(cfg.history.retain_audio);
        assert_eq!(cfg.history.text_retention_days, Some(4));
        assert_eq!(cfg.history.audio_retention_days, Some(2));
    }

    #[test]
    fn app_profiles_deserialization_and_matching() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
[[profiles]]
app_id = "code"
mode = "code"
paste_with_shift = false

[[profiles]]
app_id = "org.gnome.Terminal"
mode = "plain"
paste_with_shift = true

[[profiles]]
app_id = "slack"
mode = "plain"
"#,
        )
        .expect("write fixture");

        let cfg = load(&path).expect("load");
        assert_eq!(cfg.profiles.len(), 3);
        assert_eq!(cfg.profiles[0].app_id, "code");
        assert_eq!(cfg.profiles[0].mode, "code");
        assert_eq!(cfg.profiles[0].paste_with_shift, Some(false));
        assert_eq!(cfg.profiles[2].paste_with_shift, None);

        // find_profile matching tests:
        // Exact match
        let prof = cfg.find_profile("code").expect("find code");
        assert_eq!(prof.mode, "code");

        // Target has domain prefix, profile is short name ("code" matches "com.visualstudio.code")
        let prof = cfg.find_profile("com.visualstudio.code").expect("find vscode");
        assert_eq!(prof.mode, "code");

        // Case-insensitive & domain suffix matching ("slack" matches "com.slack.Slack")
        let prof = cfg.find_profile("com.slack.Slack").expect("find slack");
        assert_eq!(prof.mode, "plain");

        // Profile has domain prefix, target is short name ("terminal" matches "org.gnome.Terminal")
        let prof = cfg.find_profile("Terminal").expect("find terminal");
        assert_eq!(prof.mode, "plain");
        assert_eq!(prof.paste_with_shift, Some(true));

        // Unknown app returns None
        assert!(cfg.find_profile("unknown.app").is_none());
        assert!(cfg.find_profile("").is_none());
    }
}


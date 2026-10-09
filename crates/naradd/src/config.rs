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
}

impl Default for NaradConfig {
    fn default() -> Self {
        Self {
            activation: ActivationConfig::default(),
            audio: AudioConfig::default(),
            general: GeneralConfig::default(),
            models: ModelsConfig::default(),
        }
    }
}

/// Hotkeys and hold/tap discrimination (FR-ACT-1, FR-ACT-7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivationConfig {
    /// Global dictate hotkey (hold = push-to-talk, tap = toggle).
    pub dictate_hotkey: String,
    /// Press duration at or above this value is treated as hold; below is tap (FR-ACT-1).
    pub hold_tap_threshold_ms: u64,
}

impl Default for ActivationConfig {
    fn default() -> Self {
        Self {
            dictate_hotkey: "Ctrl+Alt+Space".to_string(),
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
    /// Phase 2 FR-AUD-3 unused: tap-mode auto-stop after N seconds of silence.
    pub auto_stop_silence_secs: u32,
    /// Hard cap on a single recording in seconds (FR-AUD-3; enforced as a listen timer).
    pub max_duration_secs: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sounds_enabled: true,
            auto_stop_silence_secs: 2,
            max_duration_secs: 300,
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

/// Installed / active model packs (FR-MDL-6; REQUIREMENTS §8 decision #1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelsConfig {
    /// Default English ASR pack id under `~/.local/share/narad/models/asr/<id>/`.
    pub asr_english: String,
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            asr_english: "parakeet-ctc-110m-int8".to_string(),
        }
    }
}

/// `~/.config/narad/config.toml` or `$XDG_CONFIG_HOME/narad/config.toml`.
pub fn resolve_config_path() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg).join("narad").join("config.toml"));
    }
    std::env::var("HOME")
        .ok()
        .map(|home| PathBuf::from(home).join(".config").join("narad").join("config.toml"))
}

/// Read and parse an existing config file.
pub fn load(path: &Path) -> Result<NaradConfig> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("read config {}", path.display()))?;
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
        assert_eq!(cfg.activation.hold_tap_threshold_ms, 250);
        assert!(cfg.audio.sounds_enabled);
        assert_eq!(cfg.audio.auto_stop_silence_secs, 2);
        assert_eq!(cfg.audio.max_duration_secs, 300);
        assert_eq!(cfg.general.active_mode, "plain");
        assert_eq!(cfg.models.asr_english, "parakeet-ctc-110m-int8");
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
"#,
        )
        .expect("write fixture");

        let cfg = load(&path).expect("load");
        assert_eq!(cfg.activation.dictate_hotkey, "Ctrl+Shift+D");
        assert_eq!(cfg.activation.hold_tap_threshold_ms, 400);
        assert!(!cfg.audio.sounds_enabled);
        assert_eq!(cfg.audio.auto_stop_silence_secs, 5);
        assert_eq!(cfg.audio.max_duration_secs, 120);
        assert_eq!(cfg.general.active_mode, "email");
        assert_eq!(cfg.models.asr_english, "whisper-small-en");
    }

    #[test]
    fn partial_file_uses_field_defaults() {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("config.toml");
        fs::write(&path, "[general]\nactive_mode = \"bullet\"\n").expect("write");

        let cfg = load(&path).expect("load");
        assert_eq!(cfg.general.active_mode, "bullet");
        assert_eq!(cfg.activation.dictate_hotkey, "Ctrl+Alt+Space");
        assert_eq!(cfg.activation.hold_tap_threshold_ms, 250);
        assert_eq!(cfg.models.asr_english, "parakeet-ctc-110m-int8");
    }
}

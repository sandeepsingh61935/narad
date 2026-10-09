//! Read/write `~/.config/narad/config.toml` (FR-HIS-9).
//! Mirrors `naradd::config` known sections; unknown keys are dropped on save.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Editable settings exposed to the Svelte Settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DashboardConfig {
    pub activation: ActivationConfig,
    pub audio: AudioConfig,
    pub general: GeneralConfig,
    pub models: ModelsConfig,
    pub history: HistoryConfig,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            activation: ActivationConfig::default(),
            audio: AudioConfig::default(),
            general: GeneralConfig::default(),
            models: ModelsConfig::default(),
            history: HistoryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivationConfig {
    pub dictate_hotkey: String,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub sounds_enabled: bool,
    /// Round-tripped; not edited in basic Settings UI.
    pub auto_stop_silence_secs: u32,
    /// Round-tripped; not edited in basic Settings UI.
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    pub active_mode: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            active_mode: "plain".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelsConfig {
    pub asr_english: String,
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self {
            asr_english: "parakeet-ctc-110m-int8".to_string(),
        }
    }
}

/// History retention. Mirrors `naradd` `HistoryConfig` so a settings save keeps the keys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HistoryConfig {
    pub retain_audio: bool,
    pub text_retention_days: Option<u32>,
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

/// `$XDG_CONFIG_HOME/narad/config.toml` or `$HOME/.config/narad/config.toml`.
pub fn resolve_config_path() -> Result<PathBuf, String> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg).join("narad").join("config.toml"));
        }
    }
    let home = std::env::var("HOME").map_err(|_| "HOME is unset".to_string())?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("narad")
        .join("config.toml"))
}

pub fn load_config() -> Result<DashboardConfig, String> {
    let path = resolve_config_path()?;
    if !path.is_file() {
        return Ok(DashboardConfig::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

pub fn save_config(config: &DashboardConfig) -> Result<(), String> {
    let path = resolve_config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let text = toml::to_string_pretty(config).map_err(|e| format!("serialize config: {e}"))?;
    fs::write(&path, text).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_retention_is_off_and_durations_are_blank() {
        let cfg = DashboardConfig::default();
        assert!(!cfg.history.retain_audio);
        assert_eq!(cfg.history.text_retention_days, None);
        assert_eq!(cfg.history.audio_retention_days, None);
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        assert!(text.contains("retain_audio = false"));
        assert!(!text.contains("text_retention_days"));
        assert!(!text.contains("audio_retention_days"));
        let loaded: DashboardConfig = toml::from_str(&text).expect("parse");
        assert_eq!(loaded.history, cfg.history);
    }

    #[test]
    fn reserialize_keeps_retention_keys() {
        let raw = r#"
[activation]
dictate_hotkey = "Ctrl+Alt+D"

[history]
retain_audio = true
text_retention_days = 4
"#;
        let cfg: DashboardConfig = toml::from_str(raw).expect("parse");
        assert!(cfg.history.retain_audio);
        assert_eq!(cfg.history.text_retention_days, Some(4));
        assert_eq!(cfg.history.audio_retention_days, None);
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        assert!(text.contains("retain_audio = true"));
        assert!(text.contains("text_retention_days = 4"));
        assert!(!text.contains("audio_retention_days"));
        let again: DashboardConfig = toml::from_str(&text).expect("reparse");
        assert_eq!(again.history, cfg.history);
        assert_eq!(again.activation.dictate_hotkey, "Ctrl+Alt+D");
        assert_eq!(again.activation.hold_tap_threshold_ms, 250);
    }
}

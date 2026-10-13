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
    /// Press-only paste of the newest final text (FR-INS-4). Round-tripped so a save does not drop it.
    pub paste_last_hotkey: String,
    /// Press-only undo of the last insertion (FR-INS-5). Round-tripped so a save does not drop it.
    #[serde(default = "default_undo_last_hotkey")]
    pub undo_last_hotkey: String,
    pub hold_tap_threshold_ms: u64,
}

fn default_undo_last_hotkey() -> String {
    "Ctrl+Alt+Z".to_string()
}

impl Default for ActivationConfig {
    fn default() -> Self {
        Self {
            dictate_hotkey: "Ctrl+Alt+Space".to_string(),
            paste_last_hotkey: "Ctrl+Alt+V".to_string(),
            undo_last_hotkey: "Ctrl+Alt+Z".to_string(),
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
    /// PipeWire source id (FR-AUD-4). Empty follows the system default.
    /// Round-tripped so a settings save does not drop a daemon-set target.
    pub input_target: String,
    /// RNNoise before VAD/ASR (FR-AUD-5). Round-tripped; no settings control yet.
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
        assert_eq!(again.activation.paste_last_hotkey, "Ctrl+Alt+V");
        assert_eq!(again.activation.hold_tap_threshold_ms, 250);
        assert_eq!(again.audio.input_target, "");
    }

    #[test]
    fn reserialize_keeps_paste_last_hotkey() {
        let raw = r#"
[activation]
dictate_hotkey = "Ctrl+Alt+Space"
paste_last_hotkey = "Ctrl+Shift+V"
"#;
        let cfg: DashboardConfig = toml::from_str(raw).expect("parse");
        assert_eq!(cfg.activation.dictate_hotkey, "Ctrl+Alt+Space");
        assert_eq!(cfg.activation.paste_last_hotkey, "Ctrl+Shift+V");
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        assert!(text.contains("dictate_hotkey = \"Ctrl+Alt+Space\""));
        assert!(text.contains("paste_last_hotkey = \"Ctrl+Shift+V\""));
        let again: DashboardConfig = toml::from_str(&text).expect("reparse");
        assert_eq!(again.activation, cfg.activation);
    }

    #[test]
    fn reserialize_keeps_input_target() {
        let raw = r#"
[audio]
sounds_enabled = false
input_target = "alsa_input.usb"

[activation]
dictate_hotkey = "Ctrl+Alt+D"
"#;
        let cfg: DashboardConfig = toml::from_str(raw).expect("parse");
        assert_eq!(cfg.audio.input_target, "alsa_input.usb");
        assert!(!cfg.audio.sounds_enabled);
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        assert!(text.contains("input_target = \"alsa_input.usb\""));
        let again: DashboardConfig = toml::from_str(&text).expect("reparse");
        assert_eq!(again.audio.input_target, "alsa_input.usb");
        assert_eq!(again.audio.auto_stop_silence_secs, 2);
        assert_eq!(again.audio.max_duration_secs, 300);
        assert!(!again.audio.noise_suppression);
    }

    #[test]
    fn reserialize_keeps_noise_suppression() {
        let raw = r#"
[audio]
sounds_enabled = false
noise_suppression = true
"#;
        let cfg: DashboardConfig = toml::from_str(raw).expect("parse");
        assert!(cfg.audio.noise_suppression);
        assert!(!cfg.audio.sounds_enabled);
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        assert!(text.contains("noise_suppression = true"));
        let again: DashboardConfig = toml::from_str(&text).expect("reparse");
        assert!(again.audio.noise_suppression);
        assert_eq!(again.audio.auto_stop_silence_secs, 2);
        assert_eq!(again.audio.max_duration_secs, 300);
        assert_eq!(again.audio.input_target, "");
    }

    #[test]
    fn default_noise_suppression_is_off() {
        let cfg = DashboardConfig::default();
        assert!(!cfg.audio.noise_suppression);
        let text = toml::to_string_pretty(&cfg).expect("serialize");
        assert!(text.contains("noise_suppression = false"));
        let loaded: DashboardConfig = toml::from_str(&text).expect("parse");
        assert!(!loaded.audio.noise_suppression);
    }
}

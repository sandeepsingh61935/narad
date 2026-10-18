//! Native IBus component manifest generator and engine state protocol bridge.
//!
//! Exposes inline pre-edit text updates, clause-commit triggers, and IME capability detection
//! for direct streaming injection on Linux Wayland desktop applications.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::info;

/// Component name registered on the session D-Bus.
pub const IBUS_COMPONENT_NAME: &str = "org.freedesktop.IBus.Narad";
/// Engine name switched on-demand by the GNOME desktop bridge.
pub const IBUS_ENGINE_NAME: &str = "narad";

/// IBus capability flag: client text widget supports pre-edit text (underlined transient draft).
pub const IBUS_CAP_PREEDIT_TEXT: u32 = 1 << 0;
/// IBus capability flag: client text widget supports auxiliary/candidate text windows.
pub const IBUS_CAP_AUXILIARY_TEXT: u32 = 1 << 1;
/// IBus capability flag: client text widget supports commit string operations.
pub const IBUS_CAP_COMMIT_TEXT: u32 = 1 << 2;

/// Checks if an active client input context capability bitmask supports inline pre-edit rendering.
pub fn is_ime_supported(context_capabilities: u32) -> bool {
    // If capabilities are unspecified (0), assume default IME support in standard GTK/Qt/Electron widgets.
    if context_capabilities == 0 {
        return true;
    }
    (context_capabilities & IBUS_CAP_PREEDIT_TEXT) != 0
}

/// Generates the standard IBus XML component definition string for Narad.
pub fn generate_component_xml(exec_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<component>
	<name>{IBUS_COMPONENT_NAME}</name>
	<description>Narad Voice Typing Component</description>
	<exec>{exec_path}</exec>
	<version>0.0.1</version>
	<author>Narad Team</author>
	<license>Apache-2.0</license>
	<homepage>https://narad.dev</homepage>
	<textdomain>narad</textdomain>
	<engines>
		<engine>
			<name>{IBUS_ENGINE_NAME}</name>
			<language>en</language>
			<license>Apache-2.0</license>
			<author>Narad Team</author>
			<icon>audio-input-microphone</icon>
			<layout>default</layout>
			<longname>Narad Voice Typing</longname>
			<description>Local real-time streaming voice typing</description>
			<rank>99</rank>
		</engine>
	</engines>
</component>
"#
    )
}

/// Resolves standard XDG component directory: `~/.local/share/ibus/component/`.
pub fn default_component_dir() -> Option<PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join("ibus").join("component"));
        }
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share/ibus/component"))
}

/// Writes `narad.xml` component manifest to `target_dir` (defaults to `~/.local/share/ibus/component/`).
pub fn write_ibus_component_manifest(target_dir: Option<&Path>) -> Result<PathBuf> {
    let dir = match target_dir {
        Some(d) => d.to_path_buf(),
        None => default_component_dir().context("cannot resolve ibus component dir")?,
    };

    fs::create_dir_all(&dir).with_context(|| format!("create directory {}", dir.display()))?;
    let path = dir.join("narad.xml");

    let current_exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/usr/bin/naradd".to_string());

    let xml = generate_component_xml(&current_exe);
    fs::write(&path, xml).with_context(|| format!("write {}", path.display()))?;
    info!(path = %path.display(), "wrote ibus component manifest");
    Ok(path)
}

/// In-memory engine state representing the active client text field pre-edit buffer and history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IbusEngineState {
    pub preedit_text: String,
    pub cursor_pos: u32,
    pub visible: bool,
    pub committed_buffer: Vec<String>,
}

impl Default for IbusEngineState {
    fn default() -> Self {
        Self {
            preedit_text: String::new(),
            cursor_pos: 0,
            visible: false,
            committed_buffer: Vec::new(),
        }
    }
}

impl IbusEngineState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the active pre-edit draft string and cursor position.
    pub fn update_preedit(&mut self, text: &str, cursor_pos: u32, visible: bool) {
        self.preedit_text = text.to_string();
        self.cursor_pos = cursor_pos;
        self.visible = visible;
    }

    /// Hides and clears the in-progress pre-edit text without committing.
    pub fn hide_preedit(&mut self) {
        self.preedit_text.clear();
        self.cursor_pos = 0;
        self.visible = false;
    }

    /// Commits the specified text into the document, clearing active pre-edit text.
    pub fn commit_text(&mut self, text: &str) -> String {
        self.hide_preedit();
        if !text.is_empty() {
            self.committed_buffer.push(text.to_string());
        }
        text.to_string()
    }

    /// Total count of committed clauses.
    pub fn committed_clause_count(&self) -> usize {
        self.committed_buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_component_xml_generation() {
        let xml = generate_component_xml("/opt/narad/bin/naradd");
        assert!(xml.contains("<name>org.freedesktop.IBus.Narad</name>"));
        assert!(xml.contains("<exec>/opt/narad/bin/naradd</exec>"));
        assert!(xml.contains("<name>narad</name>"));
        assert!(xml.contains("<language>en</language>"));
        assert!(xml.contains("</component>"));
    }

    #[test]
    fn test_write_ibus_component_manifest() {
        let tmp = tempdir().expect("tempdir");
        let path = write_ibus_component_manifest(Some(tmp.path())).expect("write manifest");
        assert!(path.is_file());
        assert_eq!(path.file_name().unwrap(), "narad.xml");

        let content = fs::read_to_string(&path).expect("read manifest");
        assert!(content.contains(IBUS_COMPONENT_NAME));
        assert!(content.contains(IBUS_ENGINE_NAME));
    }

    #[test]
    fn test_ime_capability_detection() {
        // Zero capabilities: default permissive
        assert!(is_ime_supported(0));
        // Explicit pre-edit support
        assert!(is_ime_supported(IBUS_CAP_PREEDIT_TEXT));
        assert!(is_ime_supported(IBUS_CAP_PREEDIT_TEXT | IBUS_CAP_COMMIT_TEXT));
        // Missing pre-edit support (e.g. raw terminal or game without pre-edit)
        assert!(!is_ime_supported(IBUS_CAP_COMMIT_TEXT));
        assert!(!is_ime_supported(IBUS_CAP_AUXILIARY_TEXT));
    }

    #[test]
    fn test_ibus_engine_state_transitions() {
        let mut state = IbusEngineState::new();
        assert_eq!(state.preedit_text, "");
        assert_eq!(state.cursor_pos, 0);
        assert!(!state.visible);

        // Update preedit draft
        state.update_preedit("Hello world", 11, true);
        assert_eq!(state.preedit_text, "Hello world");
        assert_eq!(state.cursor_pos, 11);
        assert!(state.visible);

        // Natural pause: clause commit
        let committed = state.commit_text("Hello world.");
        assert_eq!(committed, "Hello world.");
        assert_eq!(state.preedit_text, "");
        assert_eq!(state.cursor_pos, 0);
        assert!(!state.visible);
        assert_eq!(state.committed_clause_count(), 1);

        // Next clause preedit
        state.update_preedit("Next sentence", 13, true);
        assert_eq!(state.preedit_text, "Next sentence");

        // Discard / cancel clears preedit without committing
        state.hide_preedit();
        assert_eq!(state.preedit_text, "");
        assert_eq!(state.committed_clause_count(), 1);
    }
}

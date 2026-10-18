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
/// IBus capability flag: client text widget supports focus events.
pub const IBUS_CAP_FOCUS: u32 = 1 << 3;
/// IBus capability flag: client text widget supports property lists.
pub const IBUS_CAP_PROPERTY: u32 = 1 << 4;
/// IBus capability flag: client text widget supports surrounding text queries.
pub const IBUS_CAP_SURROUNDING_TEXT: u32 = 1 << 5;

/// Pre-edit underline attribute type (IBus standard).
pub const IBUS_ATTR_TYPE_UNDERLINE: u32 = 1;
pub const IBUS_ATTR_UNDERLINE_NONE: u32 = 0;
pub const IBUS_ATTR_UNDERLINE_SINGLE: u32 = 1;
pub const IBUS_ATTR_UNDERLINE_DOUBLE: u32 = 2;
pub const IBUS_ATTR_UNDERLINE_LOW: u32 = 3;
pub const IBUS_ATTR_UNDERLINE_ERROR: u32 = 4;

/// Linux input event keyvals for collision testing and physical input handling.
pub const KEYVAL_BACKSPACE: u32 = 0xff08; // XKB/IBus Backspace
pub const KEYVAL_RETURN: u32 = 0xff0d;    // XKB/IBus Return / Enter
pub const KEYVAL_ENTER: u32 = 0xff8d;     // Keypad Enter
pub const KEYVAL_ESCAPE: u32 = 0xff1b;    // Escape

/// Pre-edit text underline / styling attribute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IbusAttribute {
    pub attr_type: u32,
    pub value: u32,
    pub start_index: u32,
    pub end_index: u32,
}

impl IbusAttribute {
    pub fn underline(start: u32, end: u32) -> Self {
        Self {
            attr_type: IBUS_ATTR_TYPE_UNDERLINE,
            value: IBUS_ATTR_UNDERLINE_SINGLE,
            start_index: start,
            end_index: end,
        }
    }
}

/// Outcome of handling a physical key during input method operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicalKeyAction {
    /// No active pre-edit; the physical key passes through untouched.
    PassThrough,
    /// Pre-edit was active; it was atomically committed, and the physical key can now be applied.
    CommittedPreeditAndPassThrough { committed_text: String },
}

/// Formats text considering the surrounding text context before and after the cursor.
///
/// Rules:
/// 1. After period + space (or !/? + space, or empty buffer): capitalize first word of incoming text.
/// 2. Mid-sentence insertion: ensures proper spacing without duplicating spaces or broken punctuation.
pub fn format_surrounding_context(before: &str, after: &str, incoming: &str) -> String {
    if incoming.is_empty() {
        return String::new();
    }

    let mut result = incoming.to_string();

    // 1. Capitalization: After period + space (or !, ? + space, start of buffer, or after newline)
    let should_capitalize = if before.trim().is_empty() || before.ends_with('\n') {
        true
    } else {
        let trimmed_end = before.trim_end();
        let has_trailing_space = before.ends_with(' ') || before.ends_with('\t');
        let ends_with_sentence_punct = trimmed_end.ends_with('.') || trimmed_end.ends_with('?') || trimmed_end.ends_with('!');
        ends_with_sentence_punct && has_trailing_space
    };

    if should_capitalize {
        if let Some((idx, first_char)) = result.char_indices().find(|(_, c)| c.is_alphabetic()) {
            if first_char.is_lowercase() {
                let upper = first_char.to_uppercase().to_string();
                result.replace_range(idx..idx + first_char.len_utf8(), &upper);
            }
        }
    }

    // 2. Mid-sentence spacing:
    let is_first_char_punct = result.chars().next().map_or(false, |c| {
        c == '.' || c == ',' || c == '?' || c == '!' || c == ':' || c == ';' || c == ')' || c == ']' || c == '}'
    });

    if !before.is_empty() {
        let before_has_space = before.ends_with(' ') || before.ends_with('\t') || before.ends_with('\n');
        let incoming_has_leading_space = result.starts_with(' ') || result.starts_with('\t');

        if !before_has_space && !is_first_char_punct && !incoming_has_leading_space {
            // Need a space before incoming text
            result.insert(0, ' ');
        } else if before_has_space && incoming_has_leading_space {
            // Remove duplicate leading space
            result.remove(0);
        }
    }

    // Trailing space handling (if inserting mid-sentence before non-empty 'after'):
    if !after.is_empty() {
        let after_has_space = after.starts_with(' ') || after.starts_with('\t') || after.starts_with('\n');
        let after_starts_with_punct = after.starts_with('.') || after.starts_with(',') || after.starts_with('?') || after.starts_with('!') || after.starts_with(':') || after.starts_with(';');
        let result_has_trailing_space = result.ends_with(' ') || result.ends_with('\t');

        if !after_has_space && !after_starts_with_punct && !result_has_trailing_space {
            result.push(' ');
        } else if after_has_space && result_has_trailing_space {
            result.pop();
        }
    }

    result
}

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
    pub preedit_attrs: Vec<IbusAttribute>,
    pub committed_buffer: Vec<String>,
    pub capabilities: u32,
    pub is_terminal: bool,
    pub has_surrounding_context: bool,
    pub surrounding_text: String,
    pub surrounding_cursor_pos: u32,
    pub surrounding_anchor_pos: u32,
}

impl Default for IbusEngineState {
    fn default() -> Self {
        Self {
            preedit_text: String::new(),
            cursor_pos: 0,
            visible: false,
            preedit_attrs: Vec::new(),
            committed_buffer: Vec::new(),
            capabilities: IBUS_CAP_PREEDIT_TEXT | IBUS_CAP_COMMIT_TEXT | IBUS_CAP_SURROUNDING_TEXT,
            is_terminal: false,
            has_surrounding_context: false,
            surrounding_text: String::new(),
            surrounding_cursor_pos: 0,
            surrounding_anchor_pos: 0,
        }
    }
}

impl IbusEngineState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capabilities(capabilities: u32, is_terminal: bool) -> Self {
        Self {
            capabilities,
            is_terminal,
            ..Default::default()
        }
    }

    pub fn set_capabilities(&mut self, capabilities: u32) {
        self.capabilities = capabilities;
    }

    pub fn set_terminal(&mut self, is_terminal: bool) {
        self.is_terminal = is_terminal;
    }

    pub fn is_ime_supported(&self) -> bool {
        if self.is_terminal {
            return false;
        }
        is_ime_supported(self.capabilities)
    }

    pub fn set_surrounding_text(&mut self, text: &str, cursor_pos: u32, anchor_pos: u32) {
        self.surrounding_text = text.to_string();
        self.surrounding_cursor_pos = cursor_pos;
        self.surrounding_anchor_pos = anchor_pos;
        self.has_surrounding_context = true;
    }

    pub fn format_incoming(&self, text: &str) -> String {
        if !self.has_surrounding_context {
            return text.to_string();
        }
        let cursor_idx = (self.surrounding_cursor_pos as usize).min(self.surrounding_text.len());
        let before = &self.surrounding_text[..cursor_idx];
        let after = &self.surrounding_text[cursor_idx..];
        format_surrounding_context(before, after, text)
    }

    /// Updates the active pre-edit draft string and cursor position.
    /// If the target is a raw terminal or lacks pre-edit capability, pre-edit emission is bypassed.
    pub fn update_preedit(&mut self, text: &str, cursor_pos: u32, visible: bool) {
        if !self.is_ime_supported() {
            // VTE Terminal / raw terminal bypass: do not emit pre-edit underline or text
            self.hide_preedit();
            return;
        }

        let formatted = self.format_incoming(text);
        let len = formatted.len() as u32;
        self.preedit_text = formatted;
        self.cursor_pos = cursor_pos.min(len);
        self.visible = visible;
        if visible && len > 0 {
            self.preedit_attrs = vec![IbusAttribute::underline(0, len)];
        } else {
            self.preedit_attrs.clear();
        }
    }

    /// Hides and clears the in-progress pre-edit text without committing.
    pub fn hide_preedit(&mut self) {
        self.preedit_text.clear();
        self.cursor_pos = 0;
        self.visible = false;
        self.preedit_attrs.clear();
    }

    /// Commits the specified text into the document, clearing active pre-edit text.
    pub fn commit_text(&mut self, text: &str) -> String {
        self.hide_preedit();
        if text.is_empty() {
            return String::new();
        }
        let final_text = self.format_incoming(text);
        self.committed_buffer.push(final_text.clone());

        // Update internal surrounding text context
        if self.has_surrounding_context {
            let cursor_idx = (self.surrounding_cursor_pos as usize).min(self.surrounding_text.len());
            self.surrounding_text.insert_str(cursor_idx, &final_text);
            self.surrounding_cursor_pos += final_text.len() as u32;
            self.surrounding_anchor_pos = self.surrounding_cursor_pos;
        }

        final_text
    }

    /// Handles a physical key event during input.
    /// If pre-edit is active, atomically commits pre-edit text first so no keystrokes are dropped.
    pub fn handle_physical_key(&mut self, _keyval: u32, _keycode: u32, _state: u32) -> PhysicalKeyAction {
        if self.visible && !self.preedit_text.is_empty() {
            let active = self.preedit_text.clone();
            let committed = self.commit_text(&active);
            PhysicalKeyAction::CommittedPreeditAndPassThrough { committed_text: committed }
        } else {
            PhysicalKeyAction::PassThrough
        }
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
        assert_eq!(state.preedit_attrs.len(), 1);
        assert_eq!(state.preedit_attrs[0], IbusAttribute::underline(0, 11));

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

    #[test]
    fn test_format_surrounding_context() {
        // After period + space -> capitalizes
        assert_eq!(
            format_surrounding_context("Hello world. ", "", "this is next"),
            "This is next"
        );
        // Empty buffer -> capitalizes
        assert_eq!(
            format_surrounding_context("", "", "start of sentence"),
            "Start of sentence"
        );
        // Mid-sentence no space -> prepends space
        assert_eq!(
            format_surrounding_context("Hello", "", "world"),
            " world"
        );
        // Mid-sentence with space -> does not duplicate space
        assert_eq!(
            format_surrounding_context("Hello ", "", "world"),
            "world"
        );
    }
}

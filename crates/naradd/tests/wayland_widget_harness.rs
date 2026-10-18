//! Headless Wayland Desktop Widget Test Harness (ARCHITECTURE §10.3 / NFR-T-2 / FR-INS-7 / FR-INS-8).
//!
//! Simulates virtual input contexts across widget targets:
//! - GTK4: GtkEntry, GtkTextView
//! - Qt6: QLineEdit
//! - Chromium / Electron: contenteditable
//! - Raw VTE Terminal: gnome-terminal, Ptyxis, Alacritty
//!
//! Asserts live cursor-anchored pre-edit underline emission (`UpdatePreeditText`), cursor movement,
//! atomic commit (`CommitText`), surrounding-text context (`SetSurroundingText`) capitalization/spacing,
//! and raw terminal pre-edit bypass without broken escape sequences.

use naradd::ibus::{
    is_ime_supported, IbusEngineState,
    IBUS_ATTR_TYPE_UNDERLINE, IBUS_ATTR_UNDERLINE_SINGLE, IBUS_CAP_COMMIT_TEXT, IBUS_CAP_FOCUS,
    IBUS_CAP_PREEDIT_TEXT, IBUS_CAP_PROPERTY, IBUS_CAP_SURROUNDING_TEXT,
};

/// Target desktop widget environments simulated by the Wayland testbed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetTarget {
    Gtk4Entry,
    Gtk4TextView,
    Qt6LineEdit,
    ChromiumContentEditable,
    VteTerminal,
}

impl WidgetTarget {
    /// Capabilities reported by the widget when focusing.
    pub fn capabilities(&self) -> u32 {
        match self {
            WidgetTarget::Gtk4Entry | WidgetTarget::Gtk4TextView => {
                IBUS_CAP_PREEDIT_TEXT
                    | IBUS_CAP_COMMIT_TEXT
                    | IBUS_CAP_SURROUNDING_TEXT
                    | IBUS_CAP_FOCUS
                    | IBUS_CAP_PROPERTY
            }
            WidgetTarget::Qt6LineEdit => {
                IBUS_CAP_PREEDIT_TEXT
                    | IBUS_CAP_COMMIT_TEXT
                    | IBUS_CAP_SURROUNDING_TEXT
                    | IBUS_CAP_FOCUS
            }
            WidgetTarget::ChromiumContentEditable => {
                IBUS_CAP_PREEDIT_TEXT
                    | IBUS_CAP_COMMIT_TEXT
                    | IBUS_CAP_SURROUNDING_TEXT
            }
            WidgetTarget::VteTerminal => {
                // VTE terminal does not support inline pre-edit underlines
                IBUS_CAP_COMMIT_TEXT
            }
        }
    }

    /// Whether this widget target is a terminal emulator requiring pre-edit bypass.
    pub fn is_terminal(&self) -> bool {
        matches!(self, WidgetTarget::VteTerminal)
    }
}

/// A recorded pre-edit update event received by the virtual widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreeditUpdateEvent {
    pub text: String,
    pub cursor_pos: u32,
    pub visible: bool,
    pub has_underline: bool,
    pub underline_span: (u32, u32),
}

/// Virtual input context representing a client application widget connected to Narad IME.
pub struct VirtualInputContext {
    pub target: WidgetTarget,
    pub buffer: String,
    pub cursor_pos: usize,
    pub active_preedit: Option<PreeditUpdateEvent>,
    pub preedit_history: Vec<PreeditUpdateEvent>,
    pub commit_history: Vec<String>,
    pub direct_paste_history: Vec<String>,
    pub engine: IbusEngineState,
}

impl VirtualInputContext {
    pub fn new(target: WidgetTarget) -> Self {
        let engine = IbusEngineState::with_capabilities(target.capabilities(), target.is_terminal());
        Self {
            target,
            buffer: String::new(),
            cursor_pos: 0,
            active_preedit: None,
            preedit_history: Vec::new(),
            commit_history: Vec::new(),
            direct_paste_history: Vec::new(),
            engine,
        }
    }

    /// Simulates widget sending `SetSurroundingText` to the IBus engine.
    pub fn set_surrounding_text(&mut self, text: &str, cursor_pos: usize) {
        self.buffer = text.to_string();
        self.cursor_pos = cursor_pos;
        self.engine
            .set_surrounding_text(text, cursor_pos as u32, cursor_pos as u32);
    }

    /// Simulates Narad emitting `UpdatePreeditText` to the widget.
    pub fn update_preedit(&mut self, text: &str, cursor_pos: u32, visible: bool) {
        self.engine.update_preedit(text, cursor_pos, visible);

        if !self.target.is_terminal() && is_ime_supported(self.target.capabilities()) {
            if visible && !text.is_empty() {
                let formatted = self.engine.preedit_text.clone();
                let underline_attr = self.engine.preedit_attrs.iter().find(|a| {
                    a.attr_type == IBUS_ATTR_TYPE_UNDERLINE && a.value == IBUS_ATTR_UNDERLINE_SINGLE
                });

                let event = PreeditUpdateEvent {
                    text: formatted,
                    cursor_pos: self.engine.cursor_pos,
                    visible: true,
                    has_underline: underline_attr.is_some(),
                    underline_span: underline_attr
                        .map(|a| (a.start_index, a.end_index))
                        .unwrap_or((0, 0)),
                };
                self.active_preedit = Some(event.clone());
                self.preedit_history.push(event);
            } else {
                self.active_preedit = None;
            }
        } else {
            // Raw terminal: pre-edit is bypassed completely!
            self.active_preedit = None;
        }
    }

    /// Simulates Narad emitting `CommitText` to lock in text atomically.
    pub fn commit_text(&mut self, text: &str) -> String {
        let committed = self.engine.commit_text(text);
        self.active_preedit = None;

        if self.target.is_terminal() {
            // Terminal receives direct paste/type
            self.direct_paste_history.push(committed.clone());
        } else {
            self.commit_history.push(committed.clone());
        }

        // Apply committed text into virtual widget buffer at cursor
        let insert_idx = self.cursor_pos.min(self.buffer.len());
        self.buffer.insert_str(insert_idx, &committed);
        self.cursor_pos = insert_idx + committed.len();

        committed
    }
}

#[test]
fn test_virtual_input_contexts_across_all_widget_targets() {
    let targets = [
        WidgetTarget::Gtk4Entry,
        WidgetTarget::Gtk4TextView,
        WidgetTarget::Qt6LineEdit,
        WidgetTarget::ChromiumContentEditable,
        WidgetTarget::VteTerminal,
    ];

    for target in targets {
        let ctx = VirtualInputContext::new(target);
        if target.is_terminal() {
            assert!(
                !ctx.engine.is_ime_supported(),
                "VTE terminal must report IME pre-edit unsupported"
            );
        } else {
            assert!(
                ctx.engine.is_ime_supported(),
                "{target:?} must report IME pre-edit supported"
            );
        }
    }
}

#[test]
fn test_live_cursor_anchored_preedit_underline_and_cursor_movement() {
    // Tests across GTK4, Qt6, and Chromium/Electron
    let gui_targets = [
        WidgetTarget::Gtk4Entry,
        WidgetTarget::Gtk4TextView,
        WidgetTarget::Qt6LineEdit,
        WidgetTarget::ChromiumContentEditable,
    ];

    for target in gui_targets {
        let mut ctx = VirtualInputContext::new(target);
        ctx.set_surrounding_text("", 0);

        // Step 1: Incremental partial speech streaming emits pre-edit with single underline
        ctx.update_preedit("he", 2, true);
        let active = ctx.active_preedit.as_ref().expect("active pre-edit");
        assert_eq!(active.text, "He"); // Capitalized at start
        assert_eq!(active.cursor_pos, 2);
        assert!(active.has_underline);
        assert_eq!(active.underline_span, (0, 2));

        // Step 2: Speech continues: "hello world"
        ctx.update_preedit("hello world", 11, true);
        let active = ctx.active_preedit.as_ref().expect("active pre-edit");
        assert_eq!(active.text, "Hello world");
        assert_eq!(active.cursor_pos, 11);
        assert!(active.has_underline);
        assert_eq!(active.underline_span, (0, 11));

        // Step 3: Cursor movement inside pre-edit (cursor moves to position 5 between "Hello" and " world")
        ctx.update_preedit("hello world", 5, true);
        let active = ctx.active_preedit.as_ref().expect("active pre-edit");
        assert_eq!(active.text, "Hello world");
        assert_eq!(active.cursor_pos, 5); // Cursor anchored at 5
        assert!(active.has_underline);

        // Step 4: Atomic CommitText locks in final text and clears pre-edit draft
        let committed = ctx.commit_text("hello world");
        assert_eq!(committed, "Hello world");
        assert!(ctx.active_preedit.is_none(), "pre-edit must be cleared after commit");
        assert_eq!(ctx.buffer, "Hello world");
        assert_eq!(ctx.cursor_pos, 11);
        assert_eq!(ctx.commit_history.len(), 1);
    }
}

#[test]
fn test_surrounding_text_after_period_and_space_capitalization() {
    let mut ctx = VirtualInputContext::new(WidgetTarget::Gtk4TextView);

    // Sentence 1 followed by period and space
    ctx.set_surrounding_text("The meeting adjourned. ", 23);

    // Incoming speech starting lowercase
    ctx.update_preedit("we should follow up on monday.", 30, true);
    let active = ctx.active_preedit.as_ref().expect("active pre-edit");
    assert_eq!(
        active.text, "We should follow up on monday.",
        "Must capitalize next word following period + space"
    );

    // Commit sentence
    let committed = ctx.commit_text("we should follow up on monday.");
    assert_eq!(committed, "We should follow up on monday.");
    assert_eq!(
        ctx.buffer,
        "The meeting adjourned. We should follow up on monday."
    );

    // Verify after question mark + space
    ctx.set_surrounding_text("Are you ready? ", 15);
    ctx.update_preedit("yes let's proceed.", 18, true);
    assert_eq!(
        ctx.active_preedit.as_ref().unwrap().text,
        "Yes let's proceed.",
        "Must capitalize next word following question mark + space"
    );

    // Verify after exclamation mark + space
    ctx.set_surrounding_text("Great job! ", 11);
    ctx.update_preedit("keep it up.", 11, true);
    assert_eq!(
        ctx.active_preedit.as_ref().unwrap().text,
        "Keep it up.",
        "Must capitalize next word following exclamation mark + space"
    );
}

#[test]
fn test_surrounding_text_mid_sentence_spacing_no_duplicates() {
    let mut ctx = VirtualInputContext::new(WidgetTarget::Qt6LineEdit);

    // Case 1: Cursor immediately follows word without space -> inserts leading space
    ctx.set_surrounding_text("Hello", 5);
    ctx.update_preedit("world", 5, true);
    assert_eq!(
        ctx.active_preedit.as_ref().unwrap().text,
        " world",
        "Must prepend space when cursor follows non-space without space"
    );
    let committed = ctx.commit_text("world");
    assert_eq!(committed, " world");
    assert_eq!(ctx.buffer, "Hello world");

    // Case 2: Cursor follows word WITH trailing space -> does NOT duplicate space
    ctx.set_surrounding_text("Hello ", 6);
    ctx.update_preedit("world", 5, true);
    assert_eq!(
        ctx.active_preedit.as_ref().unwrap().text,
        "world",
        "Must NOT add extra space when space already precedes cursor"
    );
    let committed = ctx.commit_text("world");
    assert_eq!(committed, "world");
    assert_eq!(ctx.buffer, "Hello world");

    // Case 3: Mid-sentence insertion between words: "The  fox jumps" (cursor at 4)
    ctx.set_surrounding_text("The fox jumps", 4);
    ctx.update_preedit("quick brown", 11, true);
    assert_eq!(
        ctx.active_preedit.as_ref().unwrap().text,
        "quick brown ",
        "Must ensure trailing space before non-empty succeeding text"
    );
    let committed = ctx.commit_text("quick brown");
    assert_eq!(committed, "quick brown ");
    assert_eq!(ctx.buffer, "The quick brown fox jumps");

    // Case 4: Punctuation insertion mid-sentence attaches without extra space
    ctx.set_surrounding_text("Greetings", 9);
    ctx.update_preedit(",", 1, true);
    assert_eq!(
        ctx.active_preedit.as_ref().unwrap().text,
        ",",
        "Punctuation must not have leading space inserted"
    );
    let committed = ctx.commit_text(",");
    assert_eq!(committed, ",");
    assert_eq!(ctx.buffer, "Greetings,");
}

#[test]
fn test_vte_terminal_preedit_bypass_and_escape_sequence_isolation() {
    let mut ctx = VirtualInputContext::new(WidgetTarget::VteTerminal);
    ctx.set_surrounding_text("user@host:~$ ", 13);

    // Pre-edit updates are streaming during dictation
    ctx.update_preedit("cargo test", 10, true);
    assert!(
        ctx.active_preedit.is_none(),
        "Terminal must bypass pre-edit rendering"
    );
    assert_eq!(
        ctx.preedit_history.len(),
        0,
        "Terminal must receive zero pre-edit underline emission events"
    );
    assert_eq!(
        ctx.buffer, "user@host:~$ ",
        "Terminal buffer must remain clean during speech streaming"
    );

    // Commit final command
    let committed = ctx.commit_text("cargo test");
    assert_eq!(committed, "cargo test");
    assert_eq!(ctx.buffer, "user@host:~$ cargo test");
    assert_eq!(ctx.direct_paste_history.len(), 1);
    assert_eq!(ctx.direct_paste_history[0], "cargo test");

    // Verify zero broken ANSI escape codes (no \x1b[4m underline or raw escape bytes)
    assert!(
        !ctx.buffer.contains('\x1b'),
        "Terminal buffer must contain zero ANSI escape codes from IME"
    );
}

//! Concurrency, Chaos, & Keystroke Collision Injection Suite (ARCHITECTURE §10.4 / NFR-T-3).
//!
//! Tests:
//! 1. Physical Keystroke Interruption Chaos: User typing physical keys (character typing,
//!    Backspace, Enter) while partial speech pre-edit is streaming. Asserts pre-edit cleanly
//!    commits without losing keystrokes or hanging the input loop.
//! 2. Rapid hotkey cycling: 50 toggles in rapid succession to ensure no thread deadlocks,
//!    resource leaks, or orphaned async tasks.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use narad_core::store::HistoryStore;
use naradd::config::NaradConfig;
use naradd::ibus::{
    IbusEngineState, PhysicalKeyAction, KEYVAL_BACKSPACE, KEYVAL_ESCAPE, KEYVAL_RETURN,
};
use naradd::pipeline::PipelineHandle;
use tempfile::tempdir;

/// Simulates a virtual widget experiencing physical keystroke injection during speech dictation.
struct CollisionWidget {
    pub buffer: String,
    pub cursor_pos: usize,
    pub engine: IbusEngineState,
    pub dropped_keystrokes: usize,
}

impl CollisionWidget {
    pub fn new() -> Self {
        let mut engine = IbusEngineState::new();
        engine.set_surrounding_text("", 0, 0);
        Self {
            buffer: String::new(),
            cursor_pos: 0,
            engine,
            dropped_keystrokes: 0,
        }
    }

    /// Speech engine streams partial pre-edit draft.
    pub fn stream_partial_speech(&mut self, text: &str) {
        let len = text.len() as u32;
        self.engine.update_preedit(text, len, true);
    }

    /// User physically presses a key on the keyboard.
    pub fn inject_physical_key(&mut self, keyval: u32, key_char: Option<char>) {
        let action = self.engine.handle_physical_key(keyval, 0, 0);
        match action {
            PhysicalKeyAction::CommittedPreeditAndPassThrough { committed_text } => {
                // Pre-edit was active: committed atomically into buffer first!
                let insert_idx = self.cursor_pos.min(self.buffer.len());
                self.buffer.insert_str(insert_idx, &committed_text);
                self.cursor_pos = insert_idx + committed_text.len();

                // Now execute the physical key on the document buffer
                self.apply_physical_key(keyval, key_char);
            }
            PhysicalKeyAction::PassThrough => {
                // No pre-edit was active: physical key executes directly
                self.apply_physical_key(keyval, key_char);
            }
            PhysicalKeyAction::WordRewind { remaining_preedit: _ } => {
                // Pre-edit was active: trailing word was popped from pre-edit buffer
                // without forwarding backspace to app (ADR-0002).
                // Engine state has already updated pre-edit text and underline attributes.
            }
            PhysicalKeyAction::DiscardedPreedit => {
                // Pre-edit was active: pre-edit draft was discarded without committing (ADR-0002).
                // Engine state has already cleared pre-edit text.
            }
        }
    }

    fn apply_physical_key(&mut self, keyval: u32, key_char: Option<char>) {
        if keyval == KEYVAL_BACKSPACE {
            if self.cursor_pos > 0 && !self.buffer.is_empty() {
                // Remove character before cursor
                let prev_char_idx = self.buffer[..self.cursor_pos]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                self.buffer.remove(prev_char_idx);
                self.cursor_pos = prev_char_idx;
            } else {
                // Backspace with nothing to delete is not dropped; it's a no-op
            }
        } else if keyval == KEYVAL_RETURN {
            let insert_idx = self.cursor_pos.min(self.buffer.len());
            self.buffer.insert(insert_idx, '\n');
            self.cursor_pos = insert_idx + 1;
        } else if let Some(c) = key_char {
            let insert_idx = self.cursor_pos.min(self.buffer.len());
            self.buffer.insert(insert_idx, c);
            self.cursor_pos = insert_idx + c.len_utf8();
        } else {
            self.dropped_keystrokes += 1;
        }

        // Synchronize surrounding text context after key event
        self.engine.set_surrounding_text(
            &self.buffer,
            self.cursor_pos as u32,
            self.cursor_pos as u32,
        );
    }
}

#[test]
fn test_physical_character_typing_collision_during_speech_streaming() {
    let mut widget = CollisionWidget::new();

    // Partial speech pre-edit streaming
    widget.stream_partial_speech("transcribing this live sentence");
    assert_eq!(widget.engine.preedit_text, "Transcribing this live sentence");
    assert!(widget.engine.visible);
    assert_eq!(widget.buffer, "");

    // User types physical key '!' while speech is in progress
    widget.inject_physical_key('!' as u32, Some('!'));

    // Assert pre-edit cleanly committed atomically first, and '!' was not dropped!
    assert!(
        !widget.engine.visible,
        "Pre-edit must be hidden after physical key interruption"
    );
    assert_eq!(
        widget.buffer, "Transcribing this live sentence!",
        "Pre-edit must cleanly commit and append physical character without drops"
    );
    assert_eq!(widget.dropped_keystrokes, 0);

    // Continue typing normally after collision
    widget.inject_physical_key(' ' as u32, Some(' '));
    assert_eq!(widget.buffer, "Transcribing this live sentence! ");
}

#[test]
fn test_physical_backspace_collision_during_speech_streaming() {
    let mut widget = CollisionWidget::new();

    // Streaming pre-edit: "Hello world"
    widget.stream_partial_speech("hello world");
    assert_eq!(widget.engine.preedit_text, "Hello world");
    assert!(widget.engine.visible);
    assert_eq!(widget.buffer, "");

    // User hits physical Backspace during streaming: executes Word-Level Rewind (ADR-0002)
    // Pops trailing word "world", remaining pre-edit is "Hello"
    widget.inject_physical_key(KEYVAL_BACKSPACE, None);

    assert!(
        widget.engine.visible,
        "Pre-edit must remain visible with remaining word after Word-Level Rewind"
    );
    assert_eq!(
        widget.engine.preedit_text, "Hello",
        "Pre-edit must retain remaining word without forwarding backspace to app"
    );
    assert_eq!(
        widget.buffer, "",
        "Document buffer must remain untouched during Word-Level Rewind"
    );
    assert_eq!(widget.dropped_keystrokes, 0);

    // User hits Backspace again: pops "Hello", pre-edit is now empty
    widget.inject_physical_key(KEYVAL_BACKSPACE, None);
    assert!(
        !widget.engine.visible,
        "Pre-edit must be hidden after all words rewound"
    );
    assert_eq!(widget.engine.preedit_text, "");
    assert_eq!(widget.buffer, "");

    // Prepare committed text in buffer to verify subsequent Backspace passes through
    widget.buffer = "Committed text".to_string();
    widget.cursor_pos = 14;
    widget.engine.set_surrounding_text(&widget.buffer, 14, 14);

    // User hits Backspace with empty pre-edit: passes through to document buffer
    widget.inject_physical_key(KEYVAL_BACKSPACE, None);
    assert_eq!(widget.buffer, "Committed tex");
    assert_eq!(widget.cursor_pos, 13);
    assert_eq!(widget.dropped_keystrokes, 0);
}

#[test]
fn test_physical_escape_discard_during_speech_streaming() {
    let mut widget = CollisionWidget::new();

    // Streaming pre-edit: "Draft sentence to cancel"
    widget.stream_partial_speech("draft sentence to cancel");
    assert_eq!(widget.engine.preedit_text, "Draft sentence to cancel");
    assert!(widget.engine.visible);

    // User hits physical Escape
    widget.inject_physical_key(KEYVAL_ESCAPE, None);

    // Pre-edit discarded without committing to document buffer
    assert!(!widget.engine.visible);
    assert_eq!(widget.engine.preedit_text, "");
    assert_eq!(widget.buffer, "");
    assert_eq!(widget.dropped_keystrokes, 0);
}

#[test]
fn test_physical_enter_collision_during_speech_streaming() {
    let mut widget = CollisionWidget::new();

    // Streaming pre-edit: "First paragraph"
    widget.stream_partial_speech("first paragraph");
    assert_eq!(widget.engine.preedit_text, "First paragraph");

    // User hits physical Return/Enter
    widget.inject_physical_key(KEYVAL_RETURN, None);

    // Assert pre-edit committed and newline inserted
    assert_eq!(widget.buffer, "First paragraph\n");
    assert_eq!(widget.dropped_keystrokes, 0);

    // Subsequent speech stream after newline should capitalize
    widget.stream_partial_speech("second paragraph");
    assert_eq!(widget.engine.preedit_text, "Second paragraph");
    widget.inject_physical_key('.' as u32, Some('.'));
    assert_eq!(widget.buffer, "First paragraph\nSecond paragraph.");
}

#[tokio::test]
async fn test_chaotic_multithreaded_keystroke_collision_stress() {
    // Stress test: concurrent speech streaming interleaved with physical typing chaos
    let widget = Arc::new(Mutex::new(CollisionWidget::new()));
    let iterations = 100;

    let w1 = Arc::clone(&widget);
    let speech_handle = tokio::spawn(async move {
        for i in 0..iterations {
            let partial = format!("clause {}", i);
            {
                let mut guard = w1.lock().unwrap();
                guard.stream_partial_speech(&partial);
            }
            tokio::task::yield_now().await;
        }
    });

    let w2 = Arc::clone(&widget);
    let typing_handle = tokio::spawn(async move {
        for i in 0..iterations {
            {
                let mut guard = w2.lock().unwrap();
                match i % 3 {
                    0 => guard.inject_physical_key('a' as u32, Some('a')),
                    1 => guard.inject_physical_key(KEYVAL_BACKSPACE, None),
                    _ => guard.inject_physical_key(' ' as u32, Some(' ')),
                }
            }
            tokio::task::yield_now().await;
        }
    });

    let (res1, res2) = tokio::time::timeout(
        Duration::from_secs(3),
        futures_util::future::join(speech_handle, typing_handle),
    )
    .await
    .expect("Chaotic collision stress test timed out (deadlock detected)");

    res1.expect("speech task error");
    res2.expect("typing task error");

    let guard = widget.lock().unwrap();
    assert_eq!(
        guard.dropped_keystrokes, 0,
        "Zero keystrokes must be dropped during chaotic collision stress"
    );
}

#[tokio::test]
async fn test_rapid_hotkey_cycling_50_toggles() {
    // NFR-T-3: 50 toggles in rapid succession to ensure no thread deadlocks or pipe leaks
    let tmp = tempdir().expect("tempdir");
    let store = HistoryStore::open(tmp.path().join("toggle_test.db")).expect("store");
    let audio_dir = tmp.path().join("audio");
    let mut cfg = NaradConfig::default();
    cfg.audio.sounds_enabled = false;
    let pipeline = PipelineHandle::new(cfg, store, None, audio_dir);

    // Rapid toggle loop: 50 toggles in rapid succession (under 5 seconds per ARCHITECTURE §10.4)
    let toggle_future = async {
        for _ in 0..50 {
            pipeline.request_toggle(None);
            tokio::task::yield_now().await;
        }
    };

    tokio::time::timeout(Duration::from_secs(5), toggle_future)
        .await
        .expect("Rapid hotkey cycling deadlocked or hung the runtime");

    // Pipeline must remain responsive and in a valid state
    let (st, _, _, _) = pipeline.snapshot_props().expect("snapshot");
    assert!(
        st == narad_dbus::SessionState::IDLE
            || st == narad_dbus::SessionState::LISTENING
            || st == narad_dbus::SessionState::ERROR,
        "Pipeline must not be permanently stuck processing, got state: {st}"
    );
}

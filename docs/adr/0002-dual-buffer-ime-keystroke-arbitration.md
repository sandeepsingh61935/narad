# Dual-Buffer IME Keystroke Arbitration with Word-Level Backspace Rewind

- **Status**: accepted
- **Date**: 2026-10-19
- **Context**: In streaming voice dictation via IBus on Linux Wayland, users frequently type physical keys while speaking. Standard input methods either abort speech dictation on the first physical keypress or drop incoming keystrokes while the composition buffer is active. Furthermore, pressing Backspace while a voice hypothesis is active could either delete a single character or rewind spoken words.
- **Decision**: We adopted a non-blocking dual-buffer keystroke arbiter:
  1. Typing any printable character atomically commits the active pre-edit text to the document, inserts the typed character, and leaves the microphone active without session interruption.
  2. Pressing physical Backspace while pre-edit text is displayed intercepts the key and pops the trailing recognized word from the uncommitted pre-edit buffer (Word-Level Rewind). Once the pre-edit buffer is empty, subsequent backspaces pass through directly to the document.
- **Consequences**:
  - Eliminates speech interruption when users fix punctuation or insert numbers manually.
  - Matches the discrete word-level granularity of speech recognition hypotheses, preventing broken partial token prefixes.
  - Differs from traditional East Asian IMEs which perform character-level backspace inside composition strings.

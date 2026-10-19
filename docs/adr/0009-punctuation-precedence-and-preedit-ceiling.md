# Punctuation Precedence and Syntactic Pre-edit Length Soft Ceiling

- **Status**: accepted
- **Date**: 2026-10-19
- **Context**: In continuous streaming dictation, two edge cases emerge during long or command-heavy speech:
  1. Acoustic prosody may tentatively infer a punctuation mark (e.g., question mark or comma) right before the user explicitly speaks a punctuation command (e.g. "period").
  2. Users speaking continuously for 30–60 seconds without pausing generate very long uncommitted pre-edit strings, causing visual clipping, horizontal scroll jitter, and rendering lag in desktop widgets (GTK3/Qt5/browsers).
- **Decision**:
  1. Spoken punctuation commands have absolute precedence over acoustic prosody: when a spoken command is detected, it atomically pops any preceding prosodic punctuation attached to the tail of the open segment before applying the explicit mark.
  2. The pipeline enforces a syntactic pre-edit soft ceiling: whenever a terminal sentence boundary (`.` or `?`) is reached and the uncommitted pre-edit buffer exceeds 20 words (~120 characters), the pipeline automatically executes a Clause Commit for the completed sentence, resetting the pre-edit buffer to only the subsequent clause.
- **Consequences**:
  - Completely prevents concatenated double punctuation artifacts (e.g., `?.` or `,.`).
  - Keeps the Wayland pre-edit underline short, lightweight, and flicker-free during extended continuous dictation.
  - Commits only occur at syntactically closed sentence boundaries, never splitting mid-clause speech.

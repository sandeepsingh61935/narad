# Native Streaming Dictation Implementation Plan

- **Goal**: Implement real-time streaming speech-to-text, native IBus inline pre-edit, continuous dictation across pauses, and post-stop heuristic sentence transformation.
- **Spec**: `docs/superpowers/specs/2026-10-18-native-streaming-dictation-design.md`
- **Branch**: `feat/native-streaming-dictation`
- **Worktree**: `.worktrees/feat-native-streaming-dictation`

---

## Task 1: Streaming ASR Subsystem (`crates/naradd/src/asr.rs`)

### Description
1. In `crates/naradd/src/asr.rs`:
   - Introduce `OnlineAsrEngine` utilizing `sherpa_onnx::OnlineRecognizer` and `OnlineStream`.
   - Implement configuration for `sherpa-onnx-streaming-zipformer-en-2023-06-26-int8` (encoder, decoder, joiner, tokens).
   - Expose methods:
     - `create_stream(&self) -> Result<OnlineAsrStream>`
     - `OnlineAsrStream::accept_waveform(&mut self, sample_rate: u32, samples: &[f32])`
     - `OnlineAsrStream::decode(&mut self, recognizer: &OnlineRecognizer) -> Option<String>`
     - `OnlineAsrStream::finish(&mut self, recognizer: &OnlineRecognizer) -> String`
   - Implement real-time punctuation token mapping (`"comma"` $\rightarrow$ `","`, `"period"` $\rightarrow$ `"."`, `"question mark"` $\rightarrow$ `"?"`, `"exclamation mark"` $\rightarrow$ `"!"`).
2. Unit and integration tests in `crates/naradd/src/asr.rs`:
   - Test synthetic audio chunk ingestion and frame decoding.
   - Test punctuation conversion in streaming chunks.
   - Benchmark token emission latency (<50ms).

### Deliverables & File Claims
- `crates/naradd/src/asr.rs`

### Verification
```bash
cargo test -p naradd --lib asr
```

---

## Task 2: Continuous Pipeline State Machine & Pause Commits (`crates/naradd/src/pipeline.rs`)

### Description
1. In `crates/naradd/src/pipeline.rs`:
   - Decouple VAD silence detection from session auto-stop. Introduce `continuous_mode: bool` (default `true`).
   - Implement silence segmentation:
     - When silence is between 2.0s and 3.0s after a finished sentence: emit `ClauseCommit` event to commit completed text to the document, clear pre-edit, and keep the session active in `LISTENING`.
     - When continuous silence exceeds 30.0s: trigger safety auto-commit and transition to `IDLE`.
   - Update `request_stop(&self)`:
     - Runs `cleanup::apply_cleanup()` on the remaining active draft.
     - Commits the final transformed sentence.
     - Writes entry to SQLite history with `raw_text` and `final_text`.
   - Update `request_cancel(&self)`:
     - Wipes pre-edit buffer.
     - Drops audio buffer from memory without writing to SQLite history.
   - Implement physical keyboard interruption hook: physical keystroke immediately commits the current draft and yields the input context.
2. In `crates/naradd/src/ipc/daemon.rs` and `crates/narad-dbus/src/daemon_iface.rs`:
   - Add D-Bus signal: `PartialTranscript(text: &str)`.
   - Add D-Bus methods: `StopAndInsert()` and `Discard()`.

### Deliverables & File Claims
- `crates/naradd/src/pipeline.rs`
- `crates/naradd/src/ipc/daemon.rs`
- `crates/narad-dbus/src/daemon_iface.rs`

### Verification
```bash
cargo test -p naradd --lib pipeline
cargo test -p narad-dbus
```

---

## Task 3: Embedded IBus Engine Component (`crates/naradd/src/ibus.rs`)

### Description
1. In `crates/naradd/src/ibus.rs`:
   - Create IBus component manifest generator: writes `~/.local/share/ibus/component/narad.xml`.
   - Implement embedded IBus engine listener on a dedicated Tokio thread.
   - Implement IBus engine callbacks:
     - `update_preedit_text(text: &str, cursor_pos: u32, visible: bool)`
     - `commit_text(text: &str)`
     - `hide_preedit_text()`
   - Connect `pipeline` streaming tokens directly to `update_preedit_text()`.
   - Connect clause-commit and `StopAndInsert` to `commit_text()`.
   - Connect `Discard` to `hide_preedit_text()`.
   - Handle fallback: if focused application does not report IBus capabilities, flag non-IME fallback mode.
2. Unit tests in `crates/naradd/src/ibus.rs`:
   - Test manifest generation.
   - Test pre-edit and commit event sequencing with mock D-Bus connection.

### Deliverables & File Claims
- `crates/naradd/src/ibus.rs`
- `crates/naradd/src/main.rs` (spawning IBus thread on startup)

### Verification
```bash
cargo test -p naradd --lib ibus
```

---

## Task 4: GNOME Extension HUD & Input Source Guard (`extension/`)

### Description
1. In `extension/extension.js`:
   - On dictation activation: save current GNOME input source (e.g. `xkb:us::eng`) and set active input source to `narad`.
   - On dictation stop, cancel, or daemon disconnection: immediately restore previous input source.
   - Update Floating HUD Companion Pill:
     - Expand companion pill during `listening` state.
     - Render mini live audio waveform meter + elapsed timer (`0:12`).
     - Render clickable **"■ Stop & Insert"** button (calls D-Bus `StopAndInsert()`).
     - Render clickable **"× Discard"** button (calls D-Bus `Discard()`).
     - Preserve keyboard shortcuts: `Ctrl+Alt+Space` toggles stop, `Esc` triggers discard.
2. Verification tests in `dashboard/src/lib/hud-parity.test.ts`:
   - Validate state transitions and HUD button presence.

### Deliverables & File Claims
- `extension/extension.js`
- `extension/stylesheet.css`
- `dashboard/src/lib/hud-parity.test.ts`

### Verification
```bash
cd dashboard && bun test
```

---

## Whole-System End-to-End Verification

1. Compile and run full daemon test suite:
   ```bash
   cargo test --workspace
   ```
2. Run end-to-end simulation:
   - Stream audio vector into `OnlineAsrEngine` $\rightarrow$ verify partial pre-edit updates $\rightarrow$ trigger stop $\rightarrow$ verify cleaned text committed and saved to history.
3. Commit with conventional commit message:
   ```
   feat(streaming): implement native ibus pre-edit streaming and continuous dictation
   ```

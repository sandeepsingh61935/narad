# Native Streaming Dictation with Inline Pre-edit & Continuous Transformation Spec

- **Feature**: Real-time streaming speech-to-text, native IBus inline pre-edit, continuous dictation across pauses, and post-stop heuristic sentence transformation.
- **Phase**: Streaming Dictation & Input Method Integration (Phase 5 Extension)
- **Status**: Ready for Implementation
- **Triage Label**: `ready-for-agent`
- **Companion Spec**: [`docs/superpowers/specs/2026-10-19-acoustic-prosody-and-contextual-disambiguation-design.md`](./2026-10-19-acoustic-prosody-and-contextual-disambiguation-design.md) (Acoustic prosody, Joiner Trie biasing, and syntactic disambiguation)

---

## Problem Statement

Users trying to dictate into arbitrary text fields across the Linux desktop (such as the Antigravity chatbot, web browsers like Google search, code editors, and messaging clients) face severe friction with the current batch dictation approach:

1. **Premature Silence Cutoffs**: Narad currently enforces an aggressive 2-second silence auto-cutoff. When a user pauses to organize their thoughts mid-sentence, the recording terminates unexpectedly and dumps an incomplete fragment.
2. **Lack of Streaming Feedback**: Transcription only starts *after* recording has stopped. Users cannot see what the engine hears while they speak, leading to uncertainty and broken cadence.
3. **Fragile Keystroke/Clipboard Insertion**: Injecting text by synthesizing `Ctrl+V` through the RemoteDesktop portal fails when permissions are denied, triggers unwanted paste handlers in modern web applications, corrupts the user's system clipboard, and cannot support in-place word revisions.
4. **Spoken Clutter**: Spontaneous natural speech is riddled with filler words (*"um"*, *"uh"*, *"you know"*), repetitions, stuttered punctuation, and lowercase run-on sentences. Users want their spoken thoughts transformed into clean, articulate, well-structured sentences when they stop speaking.

---

## Solution

A native Linux desktop voice-typing experience matching modern AI voice inputs:

1. **Continuous Voice Input**: The microphone stays open through natural thinking pauses. It never cuts off unexpectedly and terminates only when the user explicitly clicks the **"■ Stop & Insert"** button (or presses the toggle hotkey), with a 30-second unattended silence failsafe.
2. **Native Inline Pre-edit Streaming**: As the user speaks, words stream in real time directly at the cursor inside any focused input field as native, underlined `preedit` text via an embedded IBus engine. Punctuation commands (*"period"*, *"comma"*) replace inline immediately.
3. **Sentence-Level Pause Commits**: When the user naturally pauses (2–3 seconds) after finishing a thought, that completed sentence automatically commits permanently into the document while the microphone remains active for the next sentence.
4. **Post-Stop Sentence Transformation**: When the user explicitly clicks Stop, a fast heuristic sentence transformation cleans up the final clause—stripping filler words, collapsing orphan punctuation, capitalizing sentences, and fixing grammar—before committing the finalized text into the text area.
5. **Strict Cancel Privacy**: Hitting `Esc` or clicking **"× Discard"** instantly clears the in-progress pre-edit draft from the text area, purges the audio from memory, and writes nothing to the SQLite history.

---

## User Stories

1. As a desktop user dictating into the Antigravity chatbot, I want my spoken words to appear in the chat prompt in real time as I speak, so that I have immediate visual confirmation of the transcription.
2. As a knowledge worker searching in Google, I want voice dictation to work inside the browser search input without having to switch windows or manually copy-paste.
3. As a writer pausing mid-thought to think of the right word, I want the microphone to remain active for several seconds without cutting me off, so that my natural dictation flow is uninterrupted.
4. As a user finishing a sentence and taking a short breath, I want completed sentences to lock into the document as normal text, so that my progress is permanently saved while I continue speaking.
5. As an editor, I want to see native underlined pre-edit text in the focused text box that updates in place as speech hypotheses improve, so that my input field does not flicker with messy backspaces.
6. As a user saying "comma" or "period", I want the corresponding punctuation mark to appear directly in the pre-edit draft, so that my dictated draft reads naturally.
7. As a busy user prone to speaking with fillers like "um", "uh", and "you know", I want those fillers automatically stripped when I click Stop, so that my inserted text is clean and professional.
8. As a user dictating an email or query, I want the final committed sentence to have proper sentence capitalization and punctuation spacing, so that I do not have to manually edit it.
9. As a power user, I want an explicit "■ Stop & Insert" button and a clear elapsed timer on the companion HUD pill, so that I have full visual control over when my dictation ends.
10. As a keyboard-first user, I want the global hotkey (`Ctrl+Alt+Space`) to seamlessly toggle between starting dictation and stopping/committing, so that my hands never have to leave the keyboard.
11. As a user who stumbles or changes their mind, I want to press `Esc` or click "× Discard" on the HUD pill, so that the pre-edit draft is erased from the document and no audio or transcript is saved to history.
12. As a multitasking user who forgets to stop dictation and walks away from my desk, I want the recording session to automatically commit and shut off after 30 seconds of total silence, so that my microphone is not left broadcasting indefinitely.
13. As a typist who decides to type physical keys while dictating, I want my physical keystrokes (or Enter) to immediately commit the active speech draft and let my keyboard input pass through, so that manual typing always takes priority.
14. As a system administrator on GNOME Wayland, I want Narad to hook directly into IBus on-demand, so that my everyday keyboard layout and shortcuts are unaffected when I am not dictating.
15. As a desktop user, I want an automatic watchdog that restores my default keyboard layout if the Narad daemon ever crashes, so that my system keyboard never gets stuck in an unusable state.
16. As a user working in a rare legacy application that does not support IBus pre-edit, I want the streaming text to display in the companion HUD pill and fall back to single-shot paste upon stop, so that my dictation never fails silently.
17. As an accessibility-focused user, I want clear visual state changes (idle, listening, committing, cancelled) on the floating HUD without relying solely on color, so that status is perceptible at a glance.
18. As a privacy-conscious user, I want all streaming speech recognition and heuristic cleanup to execute 100% locally on my machine without any network requests, so that my voice data remains completely confidential.
19. As a developer auditing the database, I want each completed dictation session recorded in SQLite history with both the raw transcript and the cleaned final text, so that I can inspect the exact transformations performed.
20. As a laptop user on battery, I want the streaming speech recognizer to consume minimal CPU (<8%) and memory (<150 MB), so that voice typing does not cause thermal throttling or drain battery life.

---

## Implementation Decisions

### Module Architecture & Boundary Decomposition

To enable parallel development, fast testing, and complete fault isolation, the feature is decomposed across four decoupled modules with explicit interfaces:

```
┌────────────────────────────────────────────────────────────────────────┐
│ Module 1: Streaming ASR Subsystem (crates/naradd/src/asr.rs)           │
│ • Wraps sherpa-onnx OnlineRecognizer & OnlineStream                    │
│ • Ingests raw audio frames, decodes incrementally, returns tokens      │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Stream Tokens
                                    ▼
┌────────────────────────────────────────────────────────────────────────┐
│ Module 2: Continuous Pipeline State Machine (crates/naradd/src/pipeline.rs)
│ • Owns session lifecycle (Start, Pause, Clause-Commit, Stop, Discard)  │
│ • Enforces 30s unattended failsafe; manages silence clause boundaries   │
│ • Runs sentence cleanup transformation heuristics on Stop              │
└───────────────────┬────────────────────────────────┬───────────────────┘
                    │ Pre-edit & Commit              │ Status & Controls
                    ▼                                ▼
┌──────────────────────────────────────┐  ┌──────────────────────────────┐
│ Module 3: Embedded IBus Engine       │  │ Module 4: GNOME Shell HUD    │
│ (crates/naradd/src/ibus.rs)          │  │ & Extension (extension/)     │
│ • Registers org.freedesktop.IBus.Narad│  │ • Input source switch/guard  │
│ • Emits set_preedit_text & commit    │  │ • Stop & Discard button UI   │
└──────────────────────────────────────┘  └──────────────────────────────┘
```

### Architectural Decisions

1. **ASR Model**: `sherpa-onnx-streaming-zipformer-en-2023-06-26-int8` is the primary streaming engine. It processes 16 kHz mono f32 samples in 50ms chunks, maintaining hidden states with sub-50ms latency.
2. **Process Model**: The IBus engine runs on a dedicated thread directly inside the `naradd` daemon process. This eliminates IPC round-trips between speech inference and text injection.
3. **Input Source Handshake**: 
   - Narad installs its component manifest to `~/.local/share/ibus/component/narad.xml`.
   - On dictation start, the GNOME extension switches the desktop input source to `narad`.
   - On session end (stop, cancel, or crash), the extension restores the previous input source.
4. **State Machine Transitions**:
   - `IDLE` $\rightarrow$ `LISTENING`: Triggered by hotkey or HUD click. Mic opens, IBus engine activated.
   - `LISTENING` $\rightarrow$ `CLAUSE_COMMITTED`: 2–3s silence detected after completed sentence. Pre-edit converted to committed text; mic remains open in `LISTENING`.
   - `LISTENING` $\rightarrow$ `COMMITTING`: User clicks Stop or taps hotkey. Remaining clause is cleaned via heuristics, committed to document, session transitions to `IDLE`.
   - `LISTENING` $\rightarrow$ `CANCELLED`: User presses `Esc` or clicks Discard. Pre-edit cleared, memory dropped, transitions to `IDLE`.
   - `LISTENING` $\rightarrow$ `AUTO_COMMITTED`: 30s continuous silence failsafe triggers commit and transition to `IDLE`.
5. **Real-time Punctuation Contract**:
   - Spoken tokens matching punctuation keywords (`"period"`, `"comma"`, `"question mark"`, `"exclamation mark"`) are immediately mapped to their respective glyphs (`.`, `,`, `?`, `!`) in the live pre-edit stream.
6. **Sentence Transformation Contract**:
   - The final pass uses `crates/naradd/src/cleanup.rs`:
     - Token-level filler removal: `"um"`, `"uh"`, `"er"`, `"ah"`, `"basically"`, `"you know"`, `"i mean"`.
     - Orphan punctuation and spacing collapse.
     - Sentence casing normalization.
     - Validated by the deterministic `GroundingGuard`.

---

## Testing Decisions

### What Makes a Good Test
- **External Behavioral Verification**: Tests must verify observable inputs and outputs (audio feed in $\rightarrow$ partial text out $\rightarrow$ commit text on stop $\rightarrow$ clean database row), never internal private variables.
- **Decoupled Mocking**: Testing the pipeline and ASR streaming must not require a physical microphone, live audio hardware, or an active Wayland compositor. Audio is injected as synthetic f32 sample vectors; IBus interactions are captured via mock D-Bus event sinks.
- **Deterministic Concurrency**: Multi-threaded streaming tests must use coordinated Tokio synchronization or deterministic tick counts rather than arbitrary `sleep()` durations.

### Modules Tested
1. **ASR Streaming Engine**: Test `accept_waveform` with known audio vectors; verify partial tokens match expected text; verify `input_finished()` emits final tail tokens.
2. **Pipeline State Machine**:
   - Test continuous listening across simulated 2s silence gaps (verify session does NOT auto-terminate).
   - Test 2.5s pause after sentence (verify clause auto-commits while session remains active).
   - Test 30s silence failsafe (verify automatic commit and transition to idle).
   - Test explicit `Stop` (verify cleanup heuristics applied and final text committed).
   - Test `Cancel`/`Esc` (verify pre-edit erased and 0 rows written to SQLite history).
   - Test physical typing event (verify active draft committed and input source yielded).
3. **IBus Protocol Bridge**: Test mock component registration, `UpdatePreeditText` event ordering, and `CommitText` UTF-8 payload validity.

### Prior Art in Codebase
- Pipeline lifecycle tests in [`crates/naradd/src/pipeline.rs`](file:///home/sandy/projects/narad/crates/naradd/src/pipeline.rs#L1250-L1600).
- Mock primary selection and backspace testing in [`crates/naradd/src/insert.rs`](file:///home/sandy/projects/narad/crates/naradd/src/insert.rs#L445-L535).
- GroundingGuard and cleanup heuristic tests in [`crates/naradd/src/cleanup.rs`](file:///home/sandy/projects/narad/crates/naradd/src/cleanup.rs#L350-L750).

---

## Out of Scope

- **Local LLM Model Inference**: Deep semantic restructuring via local LLMs (e.g. Qwen 2.5 1.5B via `llama-cpp-2`) is deferred to Phase 4 / future roadmap. The current implementation relies strictly on the fast heuristic cleanup engine.
- **Streaming Moonshine Architecture**: Alternative streaming architectures remain documented for future benchmarking; Zipformer int8 is the sole engine for this release.
- **Cloud ASR / Remote APIs**: Zero audio or telemetry leaves the local machine.
- **Windows / macOS Input Method Support**: All IME integration in this spec targets Linux GNOME 50 / Wayland via IBus.

---

## Further Notes

- **Parallel Implementation Slices**:
  - **Slice 1**: Streaming ASR engine (`crates/naradd/src/asr.rs`) with unit test verification.
  - **Slice 2**: Pipeline continuous state machine & silence clause-commit (`crates/naradd/src/pipeline.rs`).
  - **Slice 3**: Embedded IBus engine protocol & component registration (`crates/naradd/src/ibus.rs`).
  - **Slice 4**: GNOME Shell extension HUD Stop/Discard buttons & input source switching (`extension/`).

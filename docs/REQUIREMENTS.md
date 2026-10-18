# Narad — Requirements

Narad is a local, universal voice-to-text tool. Speak anywhere on the desktop; text appears at the cursor of whatever app is focused. Everything runs on the user's machine.

Status: **finalised v1 scope** (2026-10-09). Phases referenced below are defined in [ROADMAP.md](ROADMAP.md); design in [ARCHITECTURE.md](ARCHITECTURE.md).

## 1. Goals and principles

1. **Local only.** No audio, text, or telemetry leaves the machine. The only network use is an explicit, user-initiated model download.
2. **Fast.** Text should land at the cursor before the user notices a wait. Latency beats features when they conflict.
3. **Efficient.** Near-zero CPU when idle; memory bounded and user-controllable.
4. **Minimal design.** One floating HUD, one dashboard window. No clutter, no onboarding wizards beyond what's needed.
5. **Modular models.** Speech, translation, and cleanup models are installable packs, not hardcoded.
6. **Never lose speech.** Every utterance is saved to history (raw transcript always kept), even if insertion fails.

## 2. Target platform

| Item | v1 target |
|---|---|
| OS | Linux — Fedora 44 |
| Desktop | GNOME 50 on Wayland |
| Reference hardware | Intel i5-8250U (4C/8T, AVX2), 16 GB RAM, no GPU |
| Later | Other Linux desktops (KDE, X11), then Windows/macOS. Core must stay platform-agnostic. |

## 3. Glossary

- **Utterance** — one recording session, from activation to stop.
- **Raw transcript** — speech-recognition output, untouched.
- **Final text** — what is inserted, after dictionary, commands, translation, and cleanup.
- **Mode** — a named pipeline preset (e.g. *Plain*, *Email*, *Notes*, *Code*) controlling cleanup, formatting, and translation.
- **Model pack** — an installable model: speech (ASR), translation (MT), cleanup (LLM), or wake word.
- **Target app** — the app that had focus when dictation started.

## 4. Functional requirements

### 4.1 Activation

| ID | Requirement | Phase |
|---|---|---|
| FR-ACT-1 | Global dictation hotkey works in any app. **Hold** = push-to-talk (stops on release). **Tap** = toggle (stops on second tap or auto-stop). Hold vs tap is distinguished by a press-duration threshold (default 250 ms). | 1 |
| FR-ACT-2 | Floating HUD: clicking the anchor toggles dictation. | 1 |
| FR-ACT-3 | `Esc` cancels an in-progress recording (audio is discarded, nothing inserted, nothing saved). | 1 |
| FR-ACT-4 | Mode hotkeys start dictation directly in a given mode (e.g. `Ctrl+Alt+1..9`). | 4 |
| FR-ACT-5 | Spoken mode prefix (e.g. "email: …", "bullet list: …") selects a mode for that utterance. | 4 |
| FR-ACT-6 | Wake word ("Hey Narad"), fully local, **off by default**. | 5 |
| FR-ACT-7 | All hotkeys are user-configurable. | 1 |

Default hotkeys (proposed, configurable):

| Action | Default |
|---|---|
| Dictate (hold / tap) | `Ctrl+Alt+Space` |
| Selection command | `Ctrl+Alt+R` |
| Paste last transcription | `Ctrl+Alt+V` |
| Undo last insertion | `Ctrl+Alt+Z` |
| Dictate in mode N | `Ctrl+Alt+1..9` |
| Cancel (only while recording) | `Esc` |

### 4.2 Floating HUD

Visual/surface authority: **Libadwaita Dark Visual System** (ADR-006, `brand-spec.md`, `DESIGN-HANDOFF.md`): compact circular ~28px anchor + Raycast-style companion waveform HUD pill while active. IDs keep the `FR-DOT-*` prefix for roadmap continuity.

| ID | Requirement | Phase |
|---|---|---|
| FR-DOT-1 | Always-on-top floating HUD: compact circular anchor (~28px), draggable; position persists across restarts. | 1 |
| FR-DOT-2 | States: **idle** (neutral anchor); **listening** (Adwaita Blue anchor + mic-level pulse + companion pill with live waveform bars); **processing** (spinner on anchor + “Transcribing…” pill); **inserted** / **copied** / **cancelled** / **error** (brief outcome on anchor + companion toast/pill as needed). Status never by color alone. | 1 |
| FR-DOT-3 | Can be hidden (hotkey still works). | 1 |
| FR-DOT-4 | Right-click menu on the anchor: current mode, open dashboard, pause history, hide HUD, quit (with keycap accelerators). | 2 |
| FR-DOT-5 | While listening/processing, companion HUD shows current mode and language pair (plus elapsed time while listening). | 3 |

### 4.3 Recording and audio

| ID | Requirement | Phase |
|---|---|---|
| FR-AUD-1 | Mic stream opens only on activation (no always-on mic unless wake word is enabled). | 1 |
| FR-AUD-2 | Voice-activity detection trims leading/trailing silence before transcription. | 1 |
| FR-AUD-3 | Auto-stop after N seconds of silence in tap mode (default 2 s); hard maximum duration (default 5 min, unlimited in long-form mode). | 2 |
| FR-AUD-4 | Microphone picker; follows system default unless overridden. | 2 |
| FR-AUD-5 | Optional noise suppression. | 2 |
| FR-AUD-6 | Start/stop/cancel/error sound cues; can be turned off. | 1 |

### 4.4 Transcription

| ID | Requirement | Phase |
|---|---|---|
| FR-ASR-1 | Offline speech recognition using an installed speech model pack. | 1 |
| FR-ASR-2 | Model stays loaded in memory for instant start (subject to FR-MDL-5). | 1 |
| FR-ASR-3 | Multiple speech packs installable; one active per language/mode. | 3 |
| FR-ASR-4 | Spoken-language auto-detect when the active model supports it; otherwise fixed source language per mode. | 3 |
| FR-ASR-5 | Hallucination filter: drop known phantom phrases (e.g. "Thanks for watching") and output produced from silence/noise-only audio. | 1 |
| FR-ASR-6 | Transcribe dropped audio/video files from the dashboard (and via CLI). | 2 |
| FR-ASR-7 | Real-time online streaming speech recognition with sub-35ms partial token emissions via `sherpa_onnx::OnlineRecognizer` (Zipformer Transducer/CTC). | 1 |
| FR-ASR-8 | Universal BYOM (Bring-Your-Own-Model) dynamic discovery & runtime hardware profiler: auto-detects user-dropped models, benchmarks AVX2/AVX-512 SIMD cores, and tunes inference threads. | 2 |

### 4.5 Text processing pipeline

Order: raw transcript → voice commands → dictionary/replacements → translation (optional) → AI cleanup (optional) → final text.

| ID | Requirement | Phase |
|---|---|---|
| FR-TXT-1 | Voice commands: "new line", "new paragraph", spoken punctuation ("period", "comma", "question mark"), "scratch that" (removes previous segment in the same utterance). | 2 |
| FR-TXT-2 | Custom vocabulary (names, jargon) used for biasing where the model supports it, plus post-correction mapping. | 2 |
| FR-TXT-3 | Text replacements / snippets (e.g. "my email" → `me@example.com`, "insert signature" → multi-line block). | 2 |
| FR-TXT-4 | Code dictation: "camel case user name" → `userName`; also snake, kebab, pascal, constant case. | 2 |
| FR-TXT-5 | Translation A → B via installed translation pack or single-pass Whisper translation; skipped when A = B. | 3 |
| FR-TXT-6 | AI cleanup with a local LLM: filler removal, hallucination removal, formatting, restructure, rephrase. | 4 |
| FR-TXT-7 | Cleanup is per-mode: **off for Plain dictation**, on for modes that need it. | 4 |
| FR-TXT-8 | Cleanup guard: if LLM output adds content not grounded in the transcript (length/novel-word thresholds), fall back to the pre-cleanup text and flag the entry. | 4 |
| FR-VOC-3 | Auto-learning dictionary: semantic diff-match-patch detects post-insertion user edits and learns corrections into the local SQLite dictionary. | 2 |

Built-in modes (editable, user can add more):

| Mode | Cleanup | Behaviour |
|---|---|---|
| Plain | off | Raw + commands + dictionary. Fastest. |
| Clean | on | Remove fillers, fix punctuation/grammar, keep wording. |
| Email | on | Greeting/body/sign-off structure, polite tone. |
| Notes | on | Bullet points, concise. |
| Message | on | Casual, short. |
| Code | off | Code dictation rules, no auto-capitalisation/punctuation. |
| Translate → X | optional | Translation to target language X. |

### 4.6 Insertion

| ID | Requirement | Phase |
|---|---|---|
| FR-INS-1 | Insert final text at the cursor of the target app, all at once after stop or committed atomically from pre-edit. | 1 |
| FR-INS-2 | Fallback method (terminals / non-IBus apps): clipboard paste with the user's clipboard saved and restored afterwards. Terminals use `Ctrl+Shift+V`. | 1 |
| FR-INS-3 | If insertion fails or no text field is focused, text is copied to clipboard and the HUD shows a "copied" state (companion pill + paste keycap hint). | 1 |
| FR-INS-4 | Paste-last hotkey re-inserts the most recent final text. | 2 |
| FR-INS-5 | Undo-last hotkey removes the last inserted text (if focus/cursor unchanged). | 2 |
| FR-INS-6 | Selection command mode: select text, press selection hotkey, speak an instruction ("make this formal", "translate to Hindi", "shorten") → selection replaced. | 4 |
| FR-INS-7 | Primary streaming method: Native IBus Input Method Engine (`org.freedesktop.IBus.Engine`) shows underlined pre-edit tokens directly at the blinking cursor in real time; commits atomically on silence pause or hotkey release. | 1 |
| FR-INS-8 | Surrounding-text context via IBus `SetSurroundingText` dynamically determines capitalization, sentence boundaries, and word spacing without overwriting existing text. | 1 |

### 4.7 Dashboard and history

| ID | Requirement | Phase |
|---|---|---|
| FR-HIS-1 | Every utterance saved: timestamp, duration, raw transcript, final text, mode, models used, target app, language pair, insertion status. | 1 |
| FR-HIS-2 | Dashboard: single window, search bar, reverse-chronological list, entry detail. | 1 |
| FR-HIS-3 | Full-text search; filter by date, mode, app, language. | 1 / 2 |
| FR-HIS-4 | Entry actions: copy, re-insert, delete, toggle raw/final view. | 1 |
| FR-HIS-5 | Optional audio retention per entry; play back and re-transcribe with a different model/mode. | 2 |
| FR-HIS-6 | Pin, tag, multi-select delete, export (Markdown, JSON, TXT). | 2 |
| FR-HIS-7 | Retention policy: auto-delete text and/or audio after N days; pause history. | 2 |
| FR-HIS-8 | Stats: words dictated, speaking WPM, utterance count, estimated time saved vs typing; daily/weekly view. | 2 |
| FR-HIS-9 | Settings tab: hotkeys, mic, modes, dictionary, sounds, retention, models. | 1+ |
| FR-HIS-10 | Libadwaita dark window posture with Tauri CSD; fully keyboard-navigable. | 1 |

### 4.8 App context

| ID | Requirement | Phase |
|---|---|---|
| FR-CTX-1 | Record the target app (app id / window class) with each entry. | 2 |
| FR-CTX-2 | Per-app profiles: default mode per app (e.g. Slack → Message, VS Code → Code, Terminal → Plain with `Ctrl+Shift+V`). | 4 |
| FR-CTX-3 | Local screen, UI, and IDE context harvesting via AT-SPI2 accessibility tree (`org.a11y.Bus`) and directory symbol scanner for dynamic in-memory hotword biasing. | 2 |

### 4.9 Models

| ID | Requirement | Phase |
|---|---|---|
| FR-MDL-1 | Model manager in dashboard: browse catalog, download, verify checksum, delete, set active. | 3 |
| FR-MDL-2 | Pack types: speech (ASR), translation (MT), cleanup (LLM), wake word (KWS). | 3 |
| FR-MDL-3 | Downloads are the only network operation and require explicit user action. | 3 |
| FR-MDL-4 | First-run benchmark: time each installed speech model on this machine; recommend the best speed/accuracy trade-off. | 3 |
| FR-MDL-5 | Idle unloading: LLM unloaded after N minutes idle (default 5); speech model kept loaded by default (configurable). | 3 |
| FR-MDL-6 | A default English speech model ships with / is installed on first run so v1 works out of the box. | 1 |

### 4.10 Long-form and hands-free

| ID | Requirement | Phase |
|---|---|---|
| FR-LNG-1 | Long-form / meeting mode: continuous chunked transcription, saved as one document in history (not inserted). | 5 |
| FR-LNG-2 | Optional speaker labels (diarization) in long-form mode. | 5 |

### 4.11 CLI

| ID | Requirement | Phase |
|---|---|---|
| FR-CLI-1 | `narad toggle`, `narad start [--mode M]`, `narad stop`, `narad cancel`. | 1 |
| FR-CLI-2 | `narad last`, `narad history [--search Q] [--json]`. | 2 |
| FR-CLI-3 | `narad transcribe <file> [--mode M] [--to LANG]`. | 2 |
| FR-CLI-4 | `narad models list/install/remove/benchmark`. | 3 |

### 4.12 Agentic Extensibility

| ID | Requirement | Phase |
|---|---|---|
| FR-MCP-1 | Local Model Context Protocol (MCP) server over stdio/Unix socket for AI coding agents to control dictation, read history, and register custom vocabulary. | 2 |

## 5. Non-functional requirements

### 5.1 Performance (reference hardware; validated in Phase 0 spike)

| ID | Metric | Target |
|---|---|---|
| NFR-P-1 | Hotkey → recording started (sound cue) | < 80 ms — **measured PASS** 2026-10-09: median 0.608 ms / max 0.653 ms (i5-8250U; see `spikes/hotkeys/RESULTS.md`) |
| NFR-P-2 | Stop → text inserted, Plain mode, 5 s utterance | < 500 ms (stretch: < 300 ms) |
| NFR-P-3 | Stop → text inserted, cleanup mode, 5 s utterance | < 3 s |
| NFR-P-4 | Idle CPU (daemon + floating HUD) | < 0.5 % |
| NFR-P-5 | Idle RAM | ≤ loaded speech model + 150 MB; LLM only while loaded |
| NFR-P-6 | Dashboard cold open | < 1 s |
| NFR-P-7 | History search over 100k entries | < 50 ms |

### 5.2 Privacy and security

- NFR-S-1: The daemon contains no network code. Model downloads are performed by the dashboard/CLI only.
- NFR-S-2: All data stored under the user's XDG directories with `0700`/`0600` permissions.
- NFR-S-3: Audio retention is **off by default**.
- NFR-S-4: Clipboard contents are restored after paste-insertion and never logged.
- NFR-S-5: Downloaded models are verified against pinned SHA-256 checksums.

### 5.3 Reliability

- NFR-R-1: Daemon runs as a systemd user service with auto-restart.
- NFR-R-2: A crash during processing must not lose the recorded audio (spooled to disk until saved).
- NFR-R-3: If the desktop bridge (GNOME extension) is missing, hotkeys, insertion, and history still work; only the floating HUD and app context degrade.

### 5.4 Usability

- NFR-U-1: Works out of the box after install with one permission prompt (remote-desktop/input) and one shortcut confirmation.
- NFR-U-2: Every setting has a sensible default; no setting required to start dictating.

### 5.5 Realistic and Rigorous Testing Quality Gate

| ID | Metric | Target |
|---|---|---|
| NFR-T-1 | Acoustic human corpora coverage | Real LibriSpeech + diverse accents (US, UK, Indian, European) tested with calibrated noise injection at 5, 10, and 15 dB SNR; WER regression gate enforced. |
| NFR-T-2 | Live desktop widget integration | Headless Wayland (Weston) + IBus testbed passes pre-edit streaming across GTK4, Qt6, Chromium, and VTE terminal widgets. |
| NFR-T-3 | Concurrency & Chaos resilience | 50 hotkey cycles in 5s; typing collision during pre-edit; zero deadlock, zero dropped keystrokes. |
| NFR-T-4 | Endurance & Memory footprint | 30-min continuous transcription session maintains flat RSS memory footprint (<150 MB); zero leaks across 100 model swaps. |

## 6. Out of scope for v1

- Cloud ASR/LLM providers (100% on-device only).
- Windows/macOS/KDE/X11 builds (architecture must not block them).
- Multi-device sync.
- *(Note: Live word-by-word streaming into the target app was originally deferred in early Phase 0, but is graduated to core architecture via FR-ASR-7 and FR-INS-7 to achieve parity and superiority over Wispr Flow).*

## 7. Backlog (ideas not yet scheduled)

- Local read-back (TTS via Piper) of the last transcription for verification/accessibility.
- Low-confidence word highlighting in the dashboard, click to correct; corrections feed the dictionary automatically.
- "Inbox" capture: dictate with no target app → saved as a note in the dashboard.
- Encrypted history at rest (key in GNOME Keyring).
- Voice macros: phrase → sequence of keystrokes/actions.
- Ask mode: speak a question to the local LLM, answer inserted at cursor.
- Encrypted backup/restore of history and settings to a file.
- Mic level warning ("speaking too quietly") during recording.

## 8. Decisions (Phase 0)

| # | Topic | Status | Resolution |
|---|---|---|---|
| 1 | Default English ASR model | **Resolved (Phase 0)** | **`parakeet-ctc-110m-int8`** (Parakeet CTC 110M int8). Measured on i5-8250U: ~160–175 ms warm decode for 5 s synthetic clips (NFR-P-2 headroom). See `spikes/asr-bench/RESULTS.md`. TDT 0.6B / Whisper miss the 500 ms decode budget on this CPU. |
| 2 | Translation runtime | Deferred (post–Phase 0) | CTranslate2 (opus-mt / NLLB) vs Bergamot — not required to exit Phase 0. |
| 3 | Default cleanup LLM | Deferred (post–Phase 0) | Qwen3 1.7B vs Qwen2.5 1.5B vs Llama 3.2 1B (Q4) — Phase 4. |
| 4 | `Esc`-to-cancel binding | **Resolved (Phase 0)** | GNOME extension registers a **temporary** `Esc` keybinding **only while recording** (ARCHITECTURE §2). Not a permanent global grab; not the primary path on GNOME. Portal dynamic shortcut remains a fallback for non-GNOME desktops later. |

### Decision #1 — Default English ASR (locked)

- **Pack id:** `parakeet-ctc-110m-int8`
- **Evidence:** `spikes/asr-bench/RESULTS.md` (2026-10-09, synthetic fixtures, 8 threads).
- **NFR-P-2:** 5 s warm decode ≈ 160–175 ms on reference hardware; leaves room for insert/clipboard.
- **Not default:** Parakeet TDT 0.6B int8 (too slow on CPU), Moonshine (borderline), Whisper small.en (too slow). Optional downloads later via model manager.

### Decision #4 — Esc cancel (locked)

- **FR:** FR-ACT-3.
- **Mechanism:** Extension adds/removes an `Esc` grab for the recording window only; release on Stop/Cancel/error/idle.
- **Rationale:** A permanent global `Esc` breaks every app; GlobalShortcuts for Esc is awkward and still fights focused apps.
- **Out of scope for Phase 0 spike:** Implementing the grab (MVP / Phase 1 with daemon state).

# Narad Domain Glossary

Canonical domain vocabulary for the Narad voice-to-text stack across the daemon, GNOME Shell extension, and Tauri dashboard.

## Desktop & UI Surfaces

**Floating HUD**:
Raycast-style floating companion pill overlay rendered by GNOME Shell, displaying mic states, waveform bars, elapsed time, mode, and insertion feedback.
_Avoid_: Floating capsule, floating dot, floating indicator, status bar widget

**Libadwaita Dark Window**:
Desktop window canvas adhering to GNOME Libadwaita dark styling, 1px planar rules, warm charcoal neutrals (`--bg: oklch(21% 0.01 72)`, `--surface: oklch(24.5% 0.01 72)`), with no decorative glows or neon.
_Avoid_: Main window, dashboard window, dark theme modal, web view frame

**Adwaita Blue Accent**:
Single blue accent (`oklch(70% 0.125 250)` / sRGB `#3584e4`) used sparingly for active states, primary actions, and listening indicators, replacing legacy saffron.
_Avoid_: Saffron accent, orange accent, primary brand color, indigo accent

**Client-Side Decorations (CSD)**:
Integrated headerbar housing the window drag region, app title, offline indicator, and window controls (minimize/maximize/close) without native OS titlebars.
_Avoid_: Title bar, OS decorations, window chrome, frame header

**Provenance Stamp**:
Tabular mono stamp displaying model id, time, local/offline state, and keyboard shortcuts.
_Avoid_: Model badge, status chip, footer badge, system tag

**Master-Detail History**:
Dual-pane transcript browser with fast keyboard navigation (`/` search, `J`/`K` navigation, `Enter` insert, `C` copy, `Del` delete).
_Avoid_: History list, transcript viewer, split view, archive pane

## Voice & Pipeline

**Utterance**:
A single continuous voice recording session, from activation to stop.
_Avoid_: Session, recording clip, voice capture, dictation event

**Raw Transcript**:
The direct, unedited speech-recognition output emitted by the local ASR model.
_Avoid_: Draft text, dirty transcript, initial speech, raw audio

**Final Text**:
The processed text inserted into the target application or copied to the clipboard after voice commands, dictionary corrections, translation, and AI cleanup.
_Avoid_: Output text, result string, transcribed message, delivered text

**Mode**:
A named pipeline configuration preset (such as Plain, Clean, Email, Notes, Message, or Code) specifying formatting rules, cleanup prompts, and translation settings.
_Avoid_: Profile, preset, template, channel

**Model Pack**:
A locally installable on-device directory containing weights, tokenizer, and manifest for speech recognition (ASR), translation (MT), cleanup (LLM), or wake-word detection (KWS).
_Avoid_: Model bundle, AI plugin, weight file, neural package

**Target App**:
The desktop application window that had active input focus when dictation started and where final text is inserted.
_Avoid_: Active window, destination program, receiver app, client window

## Streaming Dictation & Acoustic Prosody

**Pre-edit**:
Transient, uncommitted text displayed with an underline directly in the focused application's text widget.
_Avoid_: Composition string, draft text, floating buffer

**Committed Text**:
Final text injected permanently into the document buffer or target widget.
_Avoid_: Inserted text, finalized buffer

**Acoustic Prosody**:
Vocal pitch intonation ($F_0$) and energy decay dynamics extracted from raw audio to infer clause boundaries and punctuation.
_Avoid_: Audio features, voice tone, inflection

**Syntactic Lookaround**:
Deterministic n-token window inspection of preceding and succeeding words to disambiguate polysemous symbols and homophones.
_Avoid_: Text heuristics, context parser

**Eager Escape**:
A spoken prefix (such as "literal" or "word") that immediately strips the prefix and forces literal noun output in pre-edit.
_Avoid_: Force literal, escape command

**Speech Repair**:
A conversational false-start where the speaker backtracks using verbal markers ("no wait", "actually no") to correct their utterance. Active in Clean and Code modes to excise false starts; bypassed in Plain mode to preserve literal verbatim speech.
_Avoid_: Scratch that, mistake correction

**Constituent Phrase Scope**:
The grammatical boundary window (e.g. compound numbers, currency, noun phrases) over which a speech repair marker excises an abandoned spoken fragment.
_Avoid_: Repair window, pop range

**Clause Commit**:
Automatic intermediate commit of a completed sentence or clause triggered by a natural pause without ending the dictation session.
_Avoid_: Auto-pause, silence stop

**Pre-edit Ceiling**:
A threshold (soft ceiling of ~20 words / 120 characters) at which completed sentences are committed at terminal punctuation ('.' or '?') while retaining subsequent clauses to prevent UI rendering degradation and horizontal scroll jitter during continuous speech.
_Avoid_: Buffer limit, max preedit

**Punctuation Precedence**:
The deterministic rule whereby an explicit spoken punctuation command atomically supersedes and overwrites any prior tentatively placed prosodic mark on the preceding token.
_Avoid_: Punctuation override, mark priority

**In-Stream Mode Transition**:
A spoken mode change event ("code mode", "email mode") that triggers an immediate clause commit of prior speech under the outgoing mode before formatting subsequent speech under the incoming mode.
_Avoid_: Dynamic mode switch, mid-sentence mode

**Grounding Guard**:
A deterministic gate checking length ratios and novel-word counts to prevent ungrounded alterations to user speech.
_Avoid_: Hallucination filter, safety check

**Word-Level Rewind**:
An input method action where physical Backspace intercepts and pops the trailing recognized word from an active pre-edit hypothesis in `IbusEngineState` without resetting the underlying ASR decoder or causing screen flicker.
_Avoid_: Character backspace, token delete

**Prefix Trie Biasing**:
A runtime trie indexing contextual hotwords and active application identifiers to additively boost logit scores in the Transducer Joiner layer without mutating base model weights, serialized directly to `OnlineRecognizer::create_stream_with_hotwords`.
_Avoid_: Hotword list, keyword booster

**Energy Decay Fallback**:
An acoustic heuristic that uses short-time log energy slope ($\frac{dE}{dt} < -0.15\text{ dB/ms}$) and extended silence pause to infer terminal sentence punctuation when pitch harmonics are unvoiced or degraded by noise.
_Avoid_: Energy VAD, silence punctuation

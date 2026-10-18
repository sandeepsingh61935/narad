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

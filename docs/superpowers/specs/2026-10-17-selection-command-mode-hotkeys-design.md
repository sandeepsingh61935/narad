# Selection Command Mode & Mode Hotkeys Design Spec (Phase 4 Surface 4)

- **Features**: Selection Command Mode (FR-INS-6), Mode Hotkeys (FR-ACT-4), Spoken Mode Prefixes (FR-ACT-5)
- **Phase**: Phase 4 — AI cleanup and modes
- **Status**: Draft

---

## 1. Problem & Normative Requirements

Per `docs/REQUIREMENTS.md`:
- **FR-INS-6**: Selection command mode: select text, press selection hotkey (`Ctrl+Alt+R`), speak an instruction ("make this formal", "shorten", "bullet points", "translate to Hindi") → selection replaced.
- **FR-ACT-4**: Mode hotkeys start dictation directly in a given mode (e.g. `Ctrl+Alt+1..9` / `Ctrl+Alt+1..5`).
- **FR-ACT-5**: Spoken mode prefix (e.g. "email: …", "bullet list: …", "code: …") selects a mode for that utterance.

Per `docs/ROADMAP.md`:
- "Selection command is a HUD state, not a dashboard costume."
- Mode editor supports hotkey and spoken prefix configuration.

---

## 2. Architecture & Data Flow

### A. Selection Command Flow (FR-INS-6)
1. User highlights text in any application (editor, browser, document).
2. User presses `Ctrl+Alt+R` (or D-Bus `SelectionCommand()` is called).
3. Daemon captures primary selection:
   - Reads via `wl-paste -n --primary` (or clipboard fallback).
   - If empty: signals `HudOutcome::Error` with detail `"No text selected"`.
   - If text captured: stores `selection_text` in session runtime, transitions to `LISTENING` with mode `"selection"`.
4. Companion HUD displays `"selection · EN"` and prompt `"Speak instruction…"`.
5. User speaks transformation instruction (e.g. `"make this formal"`, `"shorten"`, `"bullet points"`, `"code"`).
6. Audio recording completes:
   - Speech transcribed to instruction string.
   - Pure transform `cleanup::apply_selection_command(&selection, &instruction)` generates replacement text.
   - Text is inserted at the cursor (`insert_text`), replacing the highlighted text in the active window.
   - History entry is stored with `kind: "selection_command"`, `raw_text: instruction`, `final_text: transformed`.

### B. Spoken Mode Prefixes (FR-ACT-5)
- User starts dictating in any mode (e.g. normal `Ctrl+Alt+Space` in `plain` mode).
- User speaks a phrase beginning with a known prefix:
  - `"email: Sandy let's meet tomorrow"` → mode becomes `email`, formatted as email.
  - `"bullet list: buy milk buy eggs"` or `"bullet: ..."` → mode becomes `bullet`, formatted as bullet points.
  - `"code: fn main()"` → mode becomes `code`, formatted as code.
  - `"clean: ..."` → mode becomes `clean`, formatted as cleaned speech.
  - `"plain: ..."` → mode becomes `plain`, raw transcription.
- Pure parser `cleanup::extract_spoken_mode_prefix(raw)` identifies and strips the prefix, setting the utterance mode.

### C. Mode Hotkeys (FR-ACT-4)
- Global hotkeys via desktop portal:
  - `Ctrl+Alt+1` → Dictate in `plain` mode
  - `Ctrl+Alt+2` → Dictate in `clean` mode
  - `Ctrl+Alt+3` → Dictate in `email` mode
  - `Ctrl+Alt+4` → Dictate in `bullet` mode
  - `Ctrl+Alt+5` → Dictate in `code` mode
  - `Ctrl+Alt+R` → Selection command mode
- Configured in `ActivationConfig`:
  - `selection_hotkey: "Ctrl+Alt+R"`
  - `mode_hotkeys_enabled: true`
- Settings page `/settings` (Hotkeys) allows viewing and changing triggers, persisting through `saveDashboardConfig`.

---

## 3. Configuration Contract

In `NaradConfig` (`crates/naradd/src/config.rs`) & `DashboardConfig` (`dashboard/src-tauri/src/config.rs`):
```rust
pub struct ActivationConfig {
    pub dictate_hotkey: String,
    pub paste_last_hotkey: String,
    pub undo_last_hotkey: String,
    pub selection_hotkey: String,
    pub mode_hotkeys_enabled: bool,
    pub hold_tap_threshold_ms: u64,
}
```

Defaults:
- `dictate_hotkey`: `"Ctrl+Alt+Space"`
- `paste_last_hotkey`: `"Ctrl+Alt+V"`
- `undo_last_hotkey`: `"Ctrl+Alt+Z"`
- `selection_hotkey`: `"Ctrl+Alt+R"`
- `mode_hotkeys_enabled`: `true`
- `hold_tap_threshold_ms`: `250`

---

## 4. D-Bus Interface (`crates/narad-dbus/src/daemon_iface.rs`)

```rust
#[zbus::proxy(
    interface = "dev.narad.Daemon",
    default_service = "dev.narad.Daemon",
    default_path = "/dev/narad/Daemon"
)]
pub trait Daemon {
    ...
    /// Selection command mode (FR-INS-6): transforms current selection with spoken instruction.
    async fn selection_command(&self) -> zbus::Result<()>;
}
```

---

## 5. Selection Command Transformations

Pure heuristic and prompt rules in `cleanup.rs`:
- **Formal / Professional**: Removes contractions (`don't` → `do not`, `can't` → `cannot`), capitalizes sentences, formats cleanly.
- **Shorten / Concise / Summarize**: Removes verbose clauses and conversational fillers while keeping core semantic tokens.
- **Bullets / Bullet List / List**: Converts sentences or commas/conjunctions to bulleted points (`• `).
- **Code**: Wraps in code format or preserves precise code syntax without capitalization.
- **Email**: Formats greeting, body, and sign-off.
- **Clean / Fix grammar**: Standard filler removal and sentence capitalization.
- **Uppercase / Lowercase / Title case**: Case transformations.

---

## 6. Verification Plan

- `cargo test -p naradd cleanup`
- `cargo test -p naradd hotkeys`
- `cargo test -p naradd pipeline`
- `cargo test -p narad-dbus`
- `cargo test -p narad-dashboard-tauri`
- `cd dashboard && bun test`
- HUD parity assertions in `dashboard/src/lib/hud-parity.test.ts` for selection mode.

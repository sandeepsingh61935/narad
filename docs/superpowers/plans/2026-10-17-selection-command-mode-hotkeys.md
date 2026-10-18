# Selection Command Mode & Mode Hotkeys Implementation Plan (Phase 4 Surface 4)

- **Goal**: Implement Selection Command Mode (FR-INS-6), Mode Hotkeys (FR-ACT-4), and Spoken Mode Prefixes (FR-ACT-5).
- **Spec**: `docs/superpowers/specs/2026-10-17-selection-command-mode-hotkeys-design.md`
- **Branch**: `feat/selection-command-mode-hotkeys`
- **Worktree**: `.worktrees/feat-selection-command-mode-hotkeys`

---

## Task 1: Spoken Prefix & Selection Command Engine, and D-Bus IPC

### Description
1. In `crates/naradd/src/cleanup.rs`:
   - Implement `extract_spoken_mode_prefix(text: &str) -> Option<(&'static str, &str)>`:
     - Checks prefixes: `"plain:"`, `"clean:"`, `"email:"`, `"bullet:"`, `"bullet list:"`, `"code:"`.
     - Returns mapped mode name (`"plain"`, `"clean"`, `"email"`, `"bullet"`, `"code"`) and remaining trimmed text.
   - Implement `apply_selection_command(selection: &str, instruction: &str) -> String`:
     - Dispatches commands: formal, shorten, bullet, code, email, clean/grammar, uppercase/lowercase.
   - Exhaustive unit tests in `cleanup.rs`.
2. In `crates/narad-dbus/src/daemon_iface.rs`:
   - Add `async fn selection_command(&self) -> zbus::Result<()>;`.
3. In `crates/naradd/src/ipc/daemon.rs`:
   - Implement `selection_command` calling `self.pipeline.request_selection_command()`.
   - Update tests in `naradd/src/ipc/daemon.rs` and `narad-dbus`.

### Deliverables
- `crates/naradd/src/cleanup.rs`
- `crates/narad-dbus/src/daemon_iface.rs`
- `crates/naradd/src/ipc/daemon.rs`

---

## Task 2: Pipeline Selection Command Flow, Primary Selection & Hotkeys

### Description
1. In `crates/naradd/src/insert.rs`:
   - Implement `read_primary_selection() -> Option<String>` (via `wl-paste -n --primary` with clipboard fallback).
2. In `crates/naradd/src/config.rs`:
   - Add `selection_hotkey: String` (default `"Ctrl+Alt+R"`) and `mode_hotkeys_enabled: bool` (default `true`) to `ActivationConfig`.
   - Update tests in `config.rs`.
3. In `crates/naradd/src/pipeline.rs`:
   - In `SessionRuntime`: add `selection_context: Option<String>`.
   - In `set_mode`: allow `"clean"` in addition to `"plain"`, `"email"`, `"bullet"`, `"code"`.
   - Implement `request_selection_command(&self)`:
     - Captures selection via `insert::read_primary_selection()`.
     - If empty: signals error with detail `"No text selected"`.
     - If non-empty: stores `selection_context` and begins listening in mode `"selection"`.
   - In `accepted_transcript(&self, raw: &str, mode: &str)`:
     - Check `extract_spoken_mode_prefix`: if present, override mode and strip prefix.
     - If `st.selection_context` is present: apply `cleanup::apply_selection_command`, replace selection, and record history entry with `kind: "selection_command"`.
   - Unit tests for selection command lifecycle, spoken prefix parsing, and mode hotkey triggers.
4. In `crates/naradd/src/hotkeys.rs`:
   - Register shortcuts:
     - `SELECTION_ID`: `Ctrl+Alt+R` (calling `pipeline.request_selection_command()`).
     - Mode shortcuts: `Ctrl+Alt+1..5` (calling `pipeline.request_start(Some(mode.to_string()))`).
   - Unit tests in `hotkeys.rs`.

### Deliverables
- `crates/naradd/src/insert.rs`
- `crates/naradd/src/config.rs`
- `crates/naradd/src/pipeline.rs`
- `crates/naradd/src/hotkeys.rs`

---

## Task 3: Dashboard Hotkeys Settings, HUD Parity & Whole-Branch Verification

### Description
1. Update `dashboard/src-tauri/src/config.rs`:
   - Add `selection_hotkey: String` and `mode_hotkeys_enabled: bool` to `ActivationConfig`.
   - Unit tests for round-trip serialization.
2. Update `dashboard/src/lib/config.ts` and `dashboard/src/lib/prepare-config.ts`:
   - Add `selection_hotkey` and `mode_hotkeys_enabled` to `DashboardConfig` and `defaultConfig()`.
   - Sanitize in `prepareConfigForSave`.
   - Unit tests in `prepare-config.test.ts`.
3. Update `dashboard/src/routes/settings/+page.svelte`:
   - Wire `Selection Command` hotkey card to `config.activation.selection_hotkey`.
   - Wire `Dictate in Mode 1–5` mode hotkeys description and quick mode switching toggle.
   - Sentence-case copy, standard focus rings, touch targets >= 36px.
4. Update `extension/extension.js` & `dashboard/src/lib/hud-parity.test.ts`:
   - Add `clean` mode to HUD right-click menu and shortcuts.
   - Verify HUD companion pill displays `selection · EN` and `clean · EN`.
5. Full verification:
   - `cargo test -p naradd`
   - `cargo test -p narad-dbus`
   - `cargo test -p narad-dashboard-tauri`
   - `cd dashboard && bun test`
6. Commit and prepare for whole-branch review.

### Deliverables
- `dashboard/src-tauri/src/config.rs`
- `dashboard/src/lib/config.ts`
- `dashboard/src/lib/prepare-config.ts`
- `dashboard/src/lib/prepare-config.test.ts`
- `dashboard/src/routes/settings/+page.svelte`
- `extension/extension.js`
- `dashboard/src/lib/hud-parity.test.ts`

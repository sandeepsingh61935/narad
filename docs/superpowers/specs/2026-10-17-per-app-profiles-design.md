# Per-App Profiles Design Spec (Phase 4 Surface 3)

- **Feature**: Per-App Profiles (FR-CTX-2)
- **Phase**: Phase 4 — AI cleanup and modes
- **Status**: Draft

---

## 1. Problem & Normative Requirements

Per `docs/REQUIREMENTS.md`:
- **FR-CTX-1**: Record the target app (app id / window class) with each entry. (Phase 2, complete)
- **FR-CTX-2**: Per-app profiles: default mode per app (e.g. Slack → Message/Email, VS Code → Code, Terminal → Plain with `Ctrl+Shift+V`). (Phase 4)

Users frequently dictate into different applications with differing formatting expectations:
- Code editors (VS Code, Zed, Sublime) expect code formatting (`code` mode).
- Communication tools (Slack, Teams, Discord, Mail) expect message or email formatting (`email` mode or `clean` mode).
- Terminal emulators (GNOME Terminal, Ptyxis, Alacritty, Kitty) expect literal plain text (`plain` mode) and require `Ctrl+Shift+V` to paste.
- Document and note-taking apps (Obsidian, LibreOffice, Google Docs) often prefer bullet lists (`bullet` mode) or standard cleaned text.

Currently, the user must manually change the mode on the HUD or hotkey each time they switch between windows.
With per-app profiles, Narad detects window focus changes reported by the GNOME Shell extension via D-Bus (`ReportFocus`) and automatically selects the configured default mode and paste behavior for the focused application.

---

## 2. Architecture & Data Flow

```
+------------------------------------+
|  GNOME Shell Extension             |
|  (notify::focus-window)            |
+-----------------+------------------+
                  |
                  | D-Bus: ReportFocus(app_id)
                  v
+-----------------+------------------+
|  Narad Daemon (naradd)             |
|  - pipeline.rs: report_focus()     |
|  - config.toml: [[profiles]]       |
|    - match app_id / wm_class       |
|    - set session mode = profile.mode|
|    - if paste_with_shift: true ->  |
|      insert uses Ctrl+Shift+V      |
+-----------------+------------------+
                  |
                  | D-Bus PropertiesChanged / Mode
                  v
+-----------------+------------------+
|  HUD Extension & Dashboard         |
|  - HUD companion mode cue updates  |
|  - Settings -> App profiles UI     |
+------------------------------------+
```

### Profile Matching Rules
When `report_focus(app_id)` is invoked with an app id string (or wm_class):
1. If `app_id` is empty (`""`), focus is unknown; mode remains or falls back to `config.general.active_mode`.
2. Matching is case-insensitive.
3. A profile matches if:
   - `profile.app_id.to_lowercase() == focus_app.to_lowercase()`, OR
   - `focus_app.to_lowercase().split('.').next_back() == Some(profile.app_id.to_lowercase())` (e.g. profile `"slack"` matches focused `"com.slack.Slack"` or `"slack"`), OR
   - `profile.app_id.to_lowercase().split('.').next_back() == Some(focus_app.to_lowercase())` (e.g. profile `"com.visualstudio.code"` matches focused `"code"`).
4. If a match is found:
   - Mode is updated to `profile.mode`.
   - If the user had previously manually overridden the mode for a different application window, that override is cleared upon switching to a new application.
5. If no profile matches:
   - Mode falls back to `config.general.active_mode`.
6. Manual session override:
   - Calling `SetMode(mode)` or triggering a mode hotkey while inside an app temporarily overrides the mode for that app session until focus changes to another app.

### Terminal / Shift Paste Override
When pasting (`insert_text`):
- If the focused app has a profile with `paste_with_shift: true`, `Ctrl+Shift+V` is used.
- If `paste_with_shift: false`, standard `Ctrl+V` is used.
- If `paste_with_shift: None` (or not configured in profile), falls back to the built-in `TERMINAL_APPS` list (`org.gnome.Terminal`, `gnome-terminal`, `org.gnome.Console`, `kgx`, `org.gnome.Ptyxis`, `ptyxis`).

---

## 3. Configuration Contract (`config.toml`)

```toml
[general]
active_mode = "plain"

[[profiles]]
app_id = "code"
mode = "code"

[[profiles]]
app_id = "slack"
mode = "email"

[[profiles]]
app_id = "kitty"
mode = "plain"
paste_with_shift = true
```

In Rust (`crates/naradd/src/config.rs` & `dashboard/src-tauri/src/config.rs`):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppProfile {
    pub app_id: String,
    pub mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paste_with_shift: Option<bool>,
}
```

In TypeScript (`dashboard/src/lib/config.ts`):
```typescript
export type AppProfile = {
  app_id: string;
  mode: string;
  paste_with_shift?: boolean | null;
};
```

---

## 4. UI Specification: Dashboard App Profiles (`/settings/profiles`)

- **Navigation**: Unlocks `App profiles` link in `dashboard/src/routes/settings/+layout.svelte` with `href: "/settings/profiles"`.
- **Page Layout**:
  - Header:
    - Title: `App profiles`
    - Subtitle: `Default dictation mode and paste shortcut for specific applications.`
  - Profile List:
    - Cards for each configured profile.
    - App identification: `App ID or class` (e.g. `code`, `slack`, `org.gnome.Terminal`).
    - Mode selector dropdown: `plain`, `email`, `bullet`, `code`.
    - Checkbox / switch: `Paste with Ctrl+Shift+V` (for terminal emulators or command line utilities).
    - Remove profile button (accessible, min 36px hit target, with icon `delete`).
  - Add Profile Section:
    - Input: `Application ID or name` with placeholder `e.g. code, slack, obsidian`.
    - Quick add chips for popular suggestions: `VS Code`, `Slack`, `Terminal`, `Obsidian`, `Discord`.
    - Mode selector dropdown.
    - Optional `Paste with Ctrl+Shift+V` checkbox.
    - Add button `Add profile`.
  - Auto-save:
    - Operates through `saveDashboardConfig`.
    - No manual "Save" button.
    - Quiet Precision feedback (`Saved. The daemon reads this config on its next start.` notification).

---

## 5. Verification Plan

- `cargo test -p naradd config`
- `cargo test -p naradd pipeline`
- `cargo test -p naradd insert`
- `cargo test -p narad-dbus`
- `cd dashboard && bun test`
- New test suite `dashboard/src/lib/profiles.test.ts` testing profile matching, validation, defaults, and serialization.

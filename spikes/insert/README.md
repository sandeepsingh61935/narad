# spike-insert (Phase 0)

Throwaway spike: prove **clipboard + paste** into the focused app on GNOME Wayland using xdg-desktop-portal (`ashpd`) and libei (`reis`).

## Prerequisites

- Fedora 44 / GNOME 50 Wayland session
- `xdg-desktop-portal` + `xdg-desktop-portal-gnome` running (default on GNOME)
- Build: `cargo build -p spike-insert` from the repo root

## Run

1. Open a text field in a target app (see smoke-test list below).
2. From the repo root:

```bash
cargo run -p spike-insert
```

Defaults: text `Hello from spike-insert`, **3 s** delay before paste, paste via **libei** (`ConnectToEIS`).

Useful flags:

```bash
# Custom text and longer focus window
cargo run -p spike-insert -- --text "नमस्ते spike" --delay-secs 5

# GNOME Terminal / VTE (Ctrl+Shift+V)
cargo run -p spike-insert -- --terminal

# Portal D-Bus keycodes instead of libei client (debug / comparison)
cargo run -p spike-insert -- --paste portal

# Force a fresh permission dialog (ignore saved restore token)
cargo run -p spike-insert -- --forget-token
```

Set `RUST_LOG=info` (or `debug`) for portal steps.

## Portal prompts (first run)

Expect GNOME dialogs when no valid **restore token** is stored:

1. **Remote Desktop** — grant **keyboard** control (required for paste). The spike requests `PersistMode::ExplicitlyRevoked` so the portal may return a **restore token** for later runs.
2. **Clipboard** — enabled by calling `Clipboard.RequestClipboard` *before* `RemoteDesktop.Start`. Approve clipboard if the dialog offers it.

On success, a restore token is written to:

`$XDG_DATA_HOME/narad/spike-insert/restore_token`  
(default: `~/.local/share/narad/spike-insert/restore_token`)

Re-run without `--forget-token` to check whether GNOME skips the dialog. If a restored session comes back with `clipboard_enabled=false` (token from an older run), the spike clears the token and retries once with a fresh dialog.

## What the binary does

1. `RemoteDesktop.CreateSession` → `SelectDevices` (keyboard) → **`Clipboard.RequestClipboard`** → `Start`
2. After Start (and only if `clipboard_enabled=true`): `SetSelection` (`text/plain`), with a background handler for `SelectionTransfer` / `SelectionWrite`
3. After the delay, paste:
   - **Default (`--paste libei`)**: `RemoteDesktop.ConnectToEIS` → `reis` libei handshake → synthetic **Ctrl+V** (or **Ctrl+Shift+V** with `--terminal`)
   - **`--paste portal`**: `RemoteDesktop.NotifyKeyboardKeycode` (no `reis` client)

## Troubleshooting

- **`Clipboard.RequestClipboard` / `Invalid state`** — RequestClipboard was called after Start. Fixed order is RequestClipboard → Start → SetSelection.
- **`clipboard_enabled=false`** — Old restore token without clipboard, or clipboard denied in the dialog. Use `--forget-token` and approve again (auto-retry also clears a bad token once).

## Smoke-test apps

| App | Notes |
|-----|--------|
| **gedit** | Plain `Ctrl+V` |
| **Firefox** | Focus address bar or a text input; `Ctrl+V` |
| **GNOME Terminal** | Use `--terminal` (`Ctrl+Shift+V`) |
| **VS Code** | Focus editor; `Ctrl+V` |

## Live paste status

This agent environment cannot complete interactive portal dialogs. **UNKNOWN** whether paste succeeds until you run the binary locally and confirm text appears in the focused field.

## Out of scope (this spike)

- Saving/restoring the user's prior clipboard contents
- Undo, terminal auto-detection, daemon integration

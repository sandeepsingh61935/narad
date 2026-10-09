# spike-hotkeys

Phase 0 throwaway spike: bind **Ctrl+Alt+Space** via `org.freedesktop.portal.GlobalShortcuts` and print **Activated** / **Deactivated** on press and release.

## Prerequisites

- GNOME 50 (or another desktop) with **xdg-desktop-portal** ≥ **1.20** and a backend that implements GlobalShortcuts (e.g. **xdg-desktop-portal-gnome** ≥ 48).
- Session D-Bus and a graphical session (portal dialogs are UI-driven).

Host (non-Flatpak) binaries must register an app id before `CreateSession`. On start this spike:

1. Writes `~/.local/share/applications/dev.narad.SpikeHotkeys.desktop` with an **absolute** `Exec=` pointing at the running binary.
2. Calls `org.freedesktop.host.portal.Registry.Register` for `dev.narad.SpikeHotkeys`.

## Run

From the repo root:

```bash
cargo run -p spike-hotkeys
```

Or after a release build:

```bash
cargo build -p spike-hotkeys
./target/debug/spike-hotkeys
```

Optional logging:

```bash
RUST_LOG=info cargo run -p spike-hotkeys
```

## Expected portal prompts

1. **Bind shortcuts** — After startup, the portal may show a dialog listing the shortcut (“Narad spike: probe push-to-talk hotkey”) with preferred trigger **Ctrl+Alt+Space**. Approve the binding or choose an equivalent trigger in the UI. Dismissing without binding causes the spike to exit with an error.
2. **No separate permission prompt** for Activated/Deactivated once the shortcut is bound; events arrive over D-Bus while the process keeps running.

## Expected interactive output

After a successful bind:

```
bound shortcut id=narad-push-to-talk trigger=…
Listening for CTRL+ALT+space — hold and release; expect Activated then Deactivated. Ctrl+C to exit.
```

While holding **Ctrl+Alt+Space**:

```
Activated: shortcut_id=narad-push-to-talk
```

On release:

```
Deactivated: shortcut_id=narad-push-to-talk
```

## Troubleshooting

- **`CreateSession failed` / `An app id is required`** — Portal ≥ 1.20 rejected an unregistered host process. This spike registers automatically; rebuild/re-run so the `.desktop` + `Registry.Register` path runs. Confirm with `RUST_LOG=info` that you see `registered host app` then `CreateSession OK`.
- **`App info not found for 'dev.narad.SpikeHotkeys'`** — GIO could not load the `.desktop` (usually a non-absolute `Exec=`). The spike rewrites `Exec=` from `current_exe()`; ensure `update-desktop-database` can run, or log out/in once.
- **BindShortcuts errors** — Confirm GlobalShortcuts exists: `busctl --user introspect org.freedesktop.portal.Desktop /org/freedesktop/portal/desktop`.
- **No events** — Another app may own the same global shortcut; change the trigger in the bind dialog or free the conflicting binding in system settings.
- **Headless / SSH** — Portal UI will not appear; run on the local Wayland session.

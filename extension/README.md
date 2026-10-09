# Narad GNOME Shell extension

Floating, always-on-top dictation **HUD** for **GNOME Shell 50** on Wayland, aligned with the Stitch **Quiet Precision** sheet (`narad_floating_hud_component_sheet`).

- **Idle:** ~28px circular anchor (warm charcoal / saffron hover core)
- **Listening / processing / copied / error:** companion frosted pill (waveform, mode, language, toasts)
- Driven by `dev.narad.Daemon` over D-Bus when available; demo click-cycle if the daemon is missing

## Requirements

- GNOME Shell **50** (Fedora 44 default)
- Session type **Wayland** (recommended; this is the target platform)
- Optional: `naradd` on the session bus for live state / Toggle / Esc→Cancel

## Install

Pick one method.

### Symlink (good for development)

```bash
mkdir -p ~/.local/share/gnome-shell/extensions
rm -rf ~/.local/share/gnome-shell/extensions/narad@narad.dev
ln -sfn "$(pwd)/extension" ~/.local/share/gnome-shell/extensions/narad@narad.dev
```

Run from the repo root, or replace `$(pwd)/extension` with the absolute path to this directory. Remove any existing directory first so `ln` does not nest a symlink inside it.

### Copy

```bash
rm -rf ~/.local/share/gnome-shell/extensions/narad@narad.dev
cp -a extension ~/.local/share/gnome-shell/extensions/narad@narad.dev
```

### Pack archive (optional)

From the repo root:

```bash
gnome-extensions pack extension/ -f
gnome-extensions install --force narad@narad.dev.shell-extension.zip
```

## Enable

After installing or updating the files:

```bash
gnome-extensions enable narad@narad.dev
```

If the extension does not load, restart GNOME Shell:

- **Wayland:** log out and back in, or reboot.
- **X11 (legacy):** Alt+F2 → `r` → Enter (not available on Wayland).

List state:

```bash
gnome-extensions info narad@narad.dev
gnome-extensions list --enabled | grep narad
```

## Manual verify (Slices D + E)

1. Install/symlink + enable; **log out/in** on Wayland so Shell rescans the extension.
2. Confirm Quiet Precision HUD near top-right; hover idle shows `Ctrl+Alt+Space`.
3. **Without daemon:** click cycles demo states; right-click hides the HUD locally.
4. Start daemon: `cargo run -p naradd` (session bus).
5. Journal should show `[narad] connected to dev.narad.Daemon`.
6. **Click** HUD → `Toggle` → listening (saffron + pill); click again → idle.
   Or: `narad toggle` / `narad stop` and watch HUD follow `StateChanged`.
7. While **listening**, press **Esc** → `Cancel` → idle. Esc must **not** be grabbed when idle.
8. **Right-click** → `Properties.Set HudVisible=false` (daemon owns state; HUD hides). Restore with:
   `busctl --user set-property dev.narad.Daemon /dev/narad/Daemon dev.narad.Daemon HudVisible b true`
9. **Drag** the HUD; position is saved to `~/.config/narad/hud-position.json` and restored on enable (FR-DOT-1).
10. Listening pill shows **mode + elapsed** only (no Phase-3 lang/model fakes). Brief **inserted** / **cancelled** outcomes follow daemon `StateChanged`.

Logs:

```bash
journalctl -f -o cat /usr/bin/gnome-shell
```

## Behaviour

| Input | Daemon present | No daemon |
|---|---|---|
| Primary click | `Toggle(Mode)` | Demo state cycle |
| Right-click | `HudVisible=false` | Hide locally |
| Esc while listening | `Cancel()` | Local → idle |
| Esc otherwise | Not bound | Not bound |
| `StateChanged` / `Level` | Drive HUD / waveform | — |
| `HudVisible` property | Show/hide chrome | — |

## Wayland notes

- HUD uses `Main.layoutManager.addChrome(actor, { trackFullscreen: true })` (Shell 50; always-on-top; hides in fullscreen). Do not pass `affectsInputRegion` (removed upstream).
- Drag uses a **stage grab** so release is tracked if the pointer leaves the HUD.
- Esc uses a **temporary** `grab_accelerator('Escape')` only while `listening` (decision #4 / FR-ACT-3). Never a permanent global Esc.

## Layout

| File | Role |
|------|------|
| `metadata.json` | Extension UUID `narad@narad.dev`, Shell 50 |
| `stylesheet.css` | Quiet Precision tokens (saffron, charcoal, pill) |
| `extension.js` | HUD UI, D-Bus client, Esc grab, drag |

UUID and install directory name must stay `narad@narad.dev`.

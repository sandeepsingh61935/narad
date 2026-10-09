# Packaging — daily driver install (NFR-R-1)

Install the dictation daemon as a **systemd user service** and optionally the GNOME Shell HUD extension.

## Prerequisites

- Fedora / GNOME Wayland session (target: GNOME Shell 50)
- Rust toolchain (`cargo`) for building `naradd` / `narad` CLI
- Session already logged in (user systemd + D-Bus available)
- Default ASR pack under `~/.local/share/narad/models/asr/parakeet-ctc-110m-int8` (daemon auto-symlinks from `spikes/asr-bench/models/…` when present)

### Dashboard (optional)

```bash
sudo dnf install -y webkit2gtk4.1-devel libsoup3-devel openssl-devel
cd dashboard && bun install && bun run tauri dev
```

## 1. Daemon (systemd --user)

From the repo root:

```bash
./packaging/install-user.sh
```

What it does:

1. `cargo build -p naradd --release` (or use `--debug` to install `target/debug/naradd`)
2. Installs the binary to `~/.local/bin/naradd`
3. Installs `packaging/narad.service` to `~/.config/systemd/user/narad.service`
4. Seeds `~/.config/narad/config.toml` from `config.toml.example` if missing
5. `systemctl --user daemon-reload && enable --now narad.service`
6. Prints `systemctl --user status`

Fast iteration without a release build:

```bash
cargo build -p naradd
./packaging/install-user.sh --debug
```

### Unit behaviour

| Directive | Value | Why |
|-----------|-------|-----|
| `ExecStart` | `%h/.local/bin/naradd` | Per-user binary, no root |
| `Restart` | `on-failure` | Auto-restart after crashes (NFR-R-1) |
| `RestartSec` | `2` | Brief backoff before restart |
| `After` | `graphical-session.target` | Wait for desktop session |
| `WantedBy` | `default.target` | Start at login for user services |
| `EnvironmentFile` | `-%h/.config/narad/env` | Optional overrides (e.g. `RUST_LOG=debug`) |

### Useful commands

```bash
systemctl --user status narad.service
journalctl --user -u narad.service -f
systemctl --user restart narad.service
systemctl --user disable --now narad.service   # stop + un-enable
```

Optional env file (create if needed):

```bash
mkdir -p ~/.config/narad
echo 'RUST_LOG=info' > ~/.config/narad/env
systemctl --user restart narad.service
```

Config lives at `~/.config/narad/config.toml` (see `config.toml.example`). The daemon also creates XDG data under `~/.local/share/narad/`.

## 2. GNOME Shell extension (HUD)

```bash
./packaging/install-extension.sh
```

Symlinks `extension/` → `~/.local/share/gnome-shell/extensions/narad@narad.dev` and runs `gnome-extensions enable`.

**Wayland:** log out and back in (or reboot) after the first install so Shell rescans the extension. Then:

```bash
gnome-extensions info narad@narad.dev
gnome-extensions list --enabled | grep narad
```

More detail: [`extension/README.md`](../extension/README.md).

## 3. First-run checklist

1. Daemon active: `systemctl --user is-active narad.service` → `active`
2. D-Bus name claimed: `busctl --user status dev.narad.Daemon` (exactly one `naradd`; PID matches the service)
3. Extension enabled (after logout/in if needed)
4. Portal / remote-desktop permission when first binding the hotkey (`Ctrl+Alt+Space` by default)
5. Dictate into a text field; Esc cancels while listening

### Single-instance note

`naradd` claims `dev.narad.Daemon` with replace-existing disabled. A second launch (including GNOME opening `dev.narad.Narad.desktop` for the portal Registry) exits immediately if the name is already owned. Prefer `systemctl --user` for start/stop; do not run a second `naradd` by hand while the service is active.

## Layout

```
packaging/
  narad.service           # systemd user unit
  install-user.sh         # build + install + enable daemon
  install-extension.sh    # symlink + enable GNOME extension
  config.toml.example     # seed for ~/.config/narad/config.toml
  README.md               # this file
```

#!/usr/bin/env bash
# Install Narad dashboard desktop icon, launcher, and application entry.
# Usage: packaging/install-desktop.sh [--debug]
set -euo pipefail

DEBUG=0
for arg in "$@"; do
  case "$arg" in
    --debug) DEBUG=1 ;;
    -h|--help)
      echo "Usage: $0 [--debug]"
      echo "  (default) Install release dashboard binary and desktop launcher"
      echo "  --debug   Install debug dashboard binary instead"
      exit 0
      ;;
    *)
      echo "unknown argument: $arg" >&2
      exit 2
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DESKTOP_SRC="$SCRIPT_DIR/dev.narad.Dashboard.desktop"
ICONS_SRC="$REPO_ROOT/dashboard/src-tauri/icons"

BIN_DIR="${HOME}/.local/bin"
APPS_DIR="${HOME}/.local/share/applications"
ICONS_DIR="${HOME}/.local/share/icons/hicolor"
DESKTOP_DIR="${HOME}/Desktop"

if [[ ! -f "$DESKTOP_SRC" ]]; then
  echo "missing desktop entry source: $DESKTOP_SRC" >&2
  exit 1
fi

cd "$REPO_ROOT"

# 1. Locate or build dashboard binary
if [[ "$DEBUG" -eq 1 ]]; then
  BIN_SRC="$REPO_ROOT/dashboard/src-tauri/target/debug/dashboard"
  if [[ ! -x "$BIN_SRC" ]]; then
    echo "debug dashboard not found at $BIN_SRC; building..."
    (cd "$REPO_ROOT/dashboard" && bun run tauri build --debug --no-bundle)
  fi
else
  BIN_SRC="$REPO_ROOT/dashboard/src-tauri/target/release/dashboard"
  if [[ ! -x "$BIN_SRC" ]]; then
    echo "building release dashboard..."
    (cd "$REPO_ROOT/dashboard" && bun run tauri build --no-bundle)
  fi
fi

if [[ ! -x "$BIN_SRC" ]]; then
  echo "binary not executable: $BIN_SRC" >&2
  exit 1
fi

mkdir -p "$BIN_DIR"
DASHBOARD_BIN="$BIN_DIR/narad-dashboard-bin"
install -m 0755 "$BIN_SRC" "$DASHBOARD_BIN"
echo "installed $DASHBOARD_BIN"

# 2. Install wrapper launcher script
DASHBOARD_LAUNCHER="$BIN_DIR/narad-dashboard"
cat <<'EOF' > "$DASHBOARD_LAUNCHER"
#!/usr/bin/env bash
# Narad application launcher: ensures daemon is active then starts dashboard
set -e

# Attempt to start user systemd service if present
if command -v systemctl >/dev/null 2>&1; then
  if systemctl --user list-unit-files narad.service 2>/dev/null | grep -q narad.service; then
    if ! systemctl --user is-active --quiet narad.service 2>/dev/null; then
      systemctl --user start narad.service 2>/dev/null || true
    fi
  fi
fi

# Fallback: start daemon directly if not already running and binary exists
if ! pgrep -x naradd >/dev/null 2>&1; then
  if [[ -x "${HOME}/.local/bin/naradd" ]]; then
    "${HOME}/.local/bin/naradd" >/dev/null 2>&1 &
  fi
fi

# Ensure smooth rendering on Linux Wayland (Intel/Nvidia hybrid / Mesa)
export WEBKIT_DISABLE_DMABUF_RENDERER=1

exec "${HOME}/.local/bin/narad-dashboard-bin" "$@"
EOF
chmod 0755 "$DASHBOARD_LAUNCHER"
echo "installed $DASHBOARD_LAUNCHER"

# 3. Install icon sizes
mkdir -p "$ICONS_DIR/512x512/apps" "$ICONS_DIR/256x256/apps" "$ICONS_DIR/128x128/apps" "$ICONS_DIR/32x32/apps"
if [[ -f "$ICONS_SRC/icon.png" ]]; then
  install -m 0644 "$ICONS_SRC/icon.png" "$ICONS_DIR/512x512/apps/dev.narad.Dashboard.png"
fi
if [[ -f "$ICONS_SRC/128x128@2x.png" ]]; then
  install -m 0644 "$ICONS_SRC/128x128@2x.png" "$ICONS_DIR/256x256/apps/dev.narad.Dashboard.png"
fi
if [[ -f "$ICONS_SRC/128x128.png" ]]; then
  install -m 0644 "$ICONS_SRC/128x128.png" "$ICONS_DIR/128x128/apps/dev.narad.Dashboard.png"
fi
if [[ -f "$ICONS_SRC/32x32.png" ]]; then
  install -m 0644 "$ICONS_SRC/32x32.png" "$ICONS_DIR/32x32/apps/dev.narad.Dashboard.png"
fi
echo "installed icons into $ICONS_DIR"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
  gtk-update-icon-cache -f -t "$ICONS_DIR" 2>/dev/null || true
fi

# 4. Install XDG application entry
mkdir -p "$APPS_DIR"
APP_DESKTOP="$APPS_DIR/dev.narad.Dashboard.desktop"
sed -e "s|^Exec=.*|Exec=${DASHBOARD_LAUNCHER} %U|" \
    -e "s|^TryExec=.*|TryExec=${DASHBOARD_LAUNCHER}|" \
    "$DESKTOP_SRC" > "$APP_DESKTOP"
chmod 0644 "$APP_DESKTOP"
echo "installed $APP_DESKTOP"

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database "$APPS_DIR" 2>/dev/null || true
fi

# 5. Install Desktop icon shortcut
mkdir -p "$DESKTOP_DIR"
DESKTOP_SHORTCUT="$DESKTOP_DIR/dev.narad.Dashboard.desktop"
cp "$APP_DESKTOP" "$DESKTOP_SHORTCUT"
chmod 0755 "$DESKTOP_SHORTCUT"
if command -v gio >/dev/null 2>&1; then
  gio set "$DESKTOP_SHORTCUT" metadata::trusted true 2>/dev/null || true
fi
echo "installed $DESKTOP_SHORTCUT"

echo
echo "Narad desktop icon installed successfully."
echo "You can launch Narad from:"
echo "  1. Desktop folder: $DESKTOP_SHORTCUT"
echo "  2. GNOME Applications Overview (search 'Narad')"
echo "  3. Terminal: narad-dashboard"

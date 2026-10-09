#!/usr/bin/env bash
# Symlink the GNOME Shell extension and enable it.
# Usage: packaging/install-extension.sh
# Note: on Wayland, log out and back in (or reboot) may be required before the
# extension loads after first install.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
EXT_SRC="$REPO_ROOT/extension"
UUID="narad@narad.dev"
EXT_DST="${HOME}/.local/share/gnome-shell/extensions/${UUID}"

if [[ ! -f "$EXT_SRC/metadata.json" ]]; then
  echo "extension source missing: $EXT_SRC/metadata.json" >&2
  exit 1
fi

mkdir -p "$(dirname "$EXT_DST")"
# Remove existing dir/symlink so ln does not nest inside it.
rm -rf "$EXT_DST"
ln -sfn "$EXT_SRC" "$EXT_DST"
echo "symlinked $EXT_DST -> $EXT_SRC"

if command -v gnome-extensions >/dev/null 2>&1; then
  if gnome-extensions enable "$UUID" 2>/tmp/narad-ext-enable.err; then
    echo "enabled $UUID"
  else
    echo "enable deferred (GNOME Shell has not rescanned yet — normal on Wayland):"
    cat /tmp/narad-ext-enable.err 2>/dev/null || true
    rm -f /tmp/narad-ext-enable.err
  fi
  echo
  echo "=== extension info ==="
  gnome-extensions info "$UUID" 2>/dev/null || echo "(not visible until logout/in)"
else
  echo "gnome-extensions CLI not found; enable manually after logout:" >&2
  echo "  gnome-extensions enable $UUID" >&2
fi

echo
echo "On Wayland, log out and back in (or reboot) so GNOME Shell rescans extensions,"
echo "then run: gnome-extensions enable $UUID"
echo "Verify:   gnome-extensions list --enabled | grep narad"

#!/usr/bin/env bash
# Install naradd as a systemd --user service (NFR-R-1).
# Usage: packaging/install-user.sh [--debug]
set -euo pipefail

DEBUG=0
for arg in "$@"; do
  case "$arg" in
    --debug) DEBUG=1 ;;
    -h|--help)
      echo "Usage: $0 [--debug]"
      echo "  (default) cargo build -p naradd --release, install that binary"
      echo "  --debug   skip release build; copy target/debug/naradd if present"
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
UNIT_SRC="$SCRIPT_DIR/narad.service"
BIN_DST="${HOME}/.local/bin/naradd"
UNIT_DST="${HOME}/.config/systemd/user/narad.service"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/narad"
CONFIG_EXAMPLE="$SCRIPT_DIR/config.toml.example"

if [[ ! -f "$UNIT_SRC" ]]; then
  echo "missing unit file: $UNIT_SRC" >&2
  exit 1
fi

cd "$REPO_ROOT"

if [[ "$DEBUG" -eq 1 ]]; then
  BIN_SRC="$REPO_ROOT/target/debug/naradd"
  if [[ ! -x "$BIN_SRC" ]]; then
    echo "debug binary not found at $BIN_SRC; building..."
    cargo build -p naradd
  fi
else
  echo "building release naradd..."
  cargo build -p naradd --release
  BIN_SRC="$REPO_ROOT/target/release/naradd"
fi

if [[ ! -x "$BIN_SRC" ]]; then
  echo "binary not executable: $BIN_SRC" >&2
  exit 1
fi

mkdir -p "$(dirname "$BIN_DST")"
install -m 0755 "$BIN_SRC" "$BIN_DST"
echo "installed $BIN_DST"

# Thin CLI client (FR-CLI-1)
if [[ "$DEBUG" -eq 1 ]]; then
  cargo build -p narad-cli
  CLI_SRC="$REPO_ROOT/target/debug/narad"
else
  cargo build -p narad-cli --release
  CLI_SRC="$REPO_ROOT/target/release/narad"
fi
CLI_DST="${HOME}/.local/bin/narad"
install -m 0755 "$CLI_SRC" "$CLI_DST"
echo "installed $CLI_DST"

mkdir -p "$(dirname "$UNIT_DST")"
install -m 0644 "$UNIT_SRC" "$UNIT_DST"
echo "installed $UNIT_DST"

mkdir -p "$CONFIG_DIR"
if [[ ! -f "$CONFIG_DIR/config.toml" && -f "$CONFIG_EXAMPLE" ]]; then
  install -m 0600 "$CONFIG_EXAMPLE" "$CONFIG_DIR/config.toml"
  echo "seeded $CONFIG_DIR/config.toml from example (edit as needed)"
fi

systemctl --user daemon-reload
systemctl --user enable --now narad.service

echo
echo "=== narad.service status ==="
systemctl --user --no-pager status narad.service || true
echo
echo "Useful commands:"
echo "  systemctl --user status narad.service"
echo "  journalctl --user -u narad.service -f"
echo "  systemctl --user restart narad.service"

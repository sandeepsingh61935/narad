#!/bin/sh
# Install Narad commit-check hooks. Does not touch graphify post-commit/post-checkout.
set -eu
ROOT=$(git rev-parse --show-toplevel)
HOOKS=$(git rev-parse --git-path hooks)
mkdir -p "$HOOKS"
for name in commit-msg; do
    src="$ROOT/scripts/githooks/$name"
    dst="$HOOKS/$name"
    if [ -e "$dst" ] && ! grep -q 'narad-commit-check' "$dst" 2>/dev/null; then
        echo "narad-commit-check: $dst exists and is not ours — not clobbering" >&2
        exit 1
    fi
    cp "$src" "$dst"
    chmod +x "$dst"
    echo "installed $dst"
done

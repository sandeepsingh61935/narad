# Narad dashboard

Tauri v2 + SvelteKit (SPA) History shell. Thin client over the `naradd` session D-Bus API (`dev.narad.Daemon`).

Phase 1 slice: History list + search, entry detail, copy / re-insert / delete, raw↔final toggle
(`list_history` → `History`, `reinsert_entry` → `ReinsertEntry`, `delete_entries` → `DeleteEntries`).
Stats, Models, Settings, and model downloads are out of scope here.

## Prerequisites

- Rust toolchain (`cargo`)
- Node.js 20+ with **pnpm** (or npm)
- Linux Tauri deps: webkit2gtk, rsvg2 — see [Tauri Linux prerequisites](https://tauri.app/start/prerequisites/)
- `naradd` running on the session bus for live history data

## Setup

```bash
cd dashboard
pnpm install
# or: npm install
```

## Develop

```bash
cd dashboard
pnpm tauri dev
# or: npm run tauri dev
# or: cargo tauri dev   # after `pnpm install` so @tauri-apps/cli is available
```

Frontend alone (no native shell / no D-Bus invoke):

```bash
pnpm dev
```

## Build

```bash
pnpm tauri build
```

## Architecture note

Rust commands connect to the session bus via `narad-dbus` (`list_history`, `reinsert_entry`, `delete_entries`).
Copy uses the webview Clipboard API. Do not invent D-Bus methods here — extend `crates/narad-dbus` first.

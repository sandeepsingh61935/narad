# Phase 1 — parallel slice cards

Use after Phase 0 exits: ASR default (decision #1) chosen, Esc (decision #4) locked, `narad-dbus` = `HudVisible` contract.

Each card = one agent session / one PR. Cite FR ids; verify with the listed exit.

Shared rules: `AGENTS.md`, `docs/AI_DEVELOPMENT.md`. Do not invent D-Bus names — use `crates/narad-dbus`.

---

## Slice A — Daemon IPC live on session bus

```
Goal:     naradd owns Start/Stop/Cancel/Toggle + StateChanged + HudVisible on session bus
In:       FR-ACT-1/2 (toggle surface), naradd ipc, narad-dbus
Out:      cargo run -p naradd &; narad status shows mode/paused/hud_visible; Toggle flips StateChanged
Not:      Real mic, ASR, paste, extension
```

## Slice B — Hotkeys → daemon

```
Goal:     GlobalShortcuts hold/release calls daemon Start/Stop (portal glue from spike-hotkeys)
In:       FR-ACT-1, spikes/hotkeys lessons, naradd hotkeys module
Out:      Hold Ctrl+Alt+Space → listening; release → stop (stub); RUST_LOG shows portal + dbus
Not:      ASR, insert, Esc grab
```

## Slice C — Insert path in daemon

```
Goal:     Stop (or stub “final text ready”) pastes via Clipboard+RemoteDesktop/libei
In:       FR-INS-1/2/3, spikes/insert, naradd inserter trait
Out:      Manual: focused gedit receives fixed test string after Stop; clipboard_enabled=true
Not:      Real ASR output; undo/paste-last
```

## Slice D — Extension HUD ↔ daemon

```
Goal:     HUD states driven by StateChanged/Level; click → Toggle; HudVisible property
In:       FR-DOT-1/2/3, extension/, narad-dbus
Out:      Daemon Toggle updates HUD listening/idle; hide sets HudVisible=false
Not:      Esc grab (Slice E), waveform from real mic RMS (stub Level ok)
```

## Slice E — Esc cancel (decision #4)

```
Goal:     Extension temporary Esc grab only while listening → Cancel
In:       FR-ACT-3, REQUIREMENTS §8 #4, ARCHITECTURE §2
Out:      Listening + Esc → idle, nothing inserted; Esc does not bind when idle
Not:      Portal Esc shortcut
```

## Slice F — History store + History D-Bus

```
Goal:     Persist entries; History/ReinsertEntry/DeleteEntries work
In:       FR-HIS-1/2/3/4/9/10, narad-core store, naradd
Out:      cargo test -p narad-core; CLI or dbus-send History returns rows after stub EntryAdded
Not:      Dashboard UI, audio files retention policy extras
```

## Slice G — CLI thin client

```
Goal:     narad CLI covers Phase 1 daemon ops used in smoke tests
In:       FR-CLI-1, narad-cli, narad-dbus
Out:      narad start|stop|cancel|toggle|status (+ history when Slice F lands)
Not:      Models/transcribe commands (Phase 2+)
```

## Slice H — Dashboard shell (Stitch)

```
Goal:     Tauri+Svelte shell; History view reads daemon; Quiet Precision tokens
In:       Stitch export + docs/DESIGN.md, FR-HIS views, dashboard/
Out:      App opens <1s goal (NFR-P-6 stretch); lists history from daemon; no invented API
Not:      Stats/Models/Settings full polish; first-run wizard
```

## Slice I — Mic + VAD + ASR (Phase 1 core)

```
Goal:     Hold dictation records mic → VAD trim → sherpa-onnx ASR (parakeet-ctc-110m-int8) → text → insert
In:       FR-AUD-1/2/6, FR-ASR-1/2/5, FR-MDL-6
Out:      Speak English → text pastes; silence filtered; model warm-loaded from XDG models dir
Not:      Model manager UI; translation; LLM cleanup
```

---

## Suggested fan-out order

1. **A** first (everyone depends on a live bus) — **done** (naradd + `narad status`/`toggle` verified).
2. Parallel wave (in flight): **B+C+I** (daemon pipeline) ∥ **D+E** (extension) ∥ **G** (CLI) ∥ **H** (dashboard).
3. **F** largely present (History/DeleteEntries); reinsert completes with insert path.
4. Phase 0 NFR-P-1 timing after hotkeys land.

## Phase 0 blockers before starting these

- [x] Decision #1: default ASR = `parakeet-ctc-110m-int8` (`spikes/asr-bench/RESULTS.md`)
- [x] Decision #4: Esc → extension temporary grab
- [x] `HudVisible` in narad-dbus / naradd / narad-cli

**Status:** Slice A verified; four subagents finishing B–I / D–E / G / H (no-commit mode).

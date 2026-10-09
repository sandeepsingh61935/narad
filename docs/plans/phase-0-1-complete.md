# Plan: Finish Phase 0 exit + Phase 1 MVP

**Mode:** no git commits (user choice). Subagents edit the shared tree; controller verifies with cargo/manual checks.

**Sources of truth:** `docs/REQUIREMENTS.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/PHASE1_PARALLEL_SLICES.md`, `AGENTS.md`.

**Global constraints:**
- Do not invent D-Bus names — use `crates/narad-dbus` (`HudVisible`, not DotVisible).
- Default ASR pack id: `parakeet-ctc-110m-int8`.
- Esc cancel = extension temporary grab (decision #4); implement in Slice E only.
- No network in daemon (NFR-S-1).
- Match existing code style; no drive-by refactors.
- Do not spawn nested subagents.

## Task 0 — Phase 0 NFR-P-1 (after Slice B)

Measure hotkey→listen cue latency; record in `spikes/hotkeys/RESULTS.md` or REQUIREMENTS NFR table; mark Phase 0 exit in ROADMAP if <80 ms or document measured value + gap.

## Task A — Daemon IPC live

Per PHASE1_PARALLEL_SLICES Slice A. Prove `naradd` + `narad status` / Toggle / StateChanged on session bus.

## Task F — History D-Bus

Slice F. History/ReinsertEntry/DeleteEntries + EntryAdded; `cargo test -p narad-core`.

## Task G — CLI

Slice G. history/reinsert/delete subcommands when F ready.

## Task B — Hotkeys → daemon

Slice B. Portal Registry + GlobalShortcuts → Start/Stop. Reuse spike-hotkeys lessons (`dev.narad.Narad` or daemon app id + desktop file).

## Task C — Insert path

Slice C. On Stop with stub text (or after ASR), Clipboard+RemoteDesktop paste. RequestClipboard before Start.

## Task I — Mic + VAD + ASR (Phase 1 core)

```
Goal:     Hold dictation records mic → VAD trim → sherpa-onnx ASR (parakeet-ctc-110m-int8) → final text → insert
In:       FR-AUD-1/2/6, FR-ASR-1/2/5, FR-MDL-6, models path under XDG
Out:      cargo check; manual: speak → English text pastes; hallucination filter for silence
Not:      Model manager UI; translation; LLM cleanup
```

## Task D — Extension HUD ↔ daemon

Slice D. StateChanged drives HUD; click Toggle; HudVisible.

## Task E — Esc cancel

Slice E. Temporary Esc while listening → Cancel.

## Task H — Dashboard History shell

Slice H. Tauri+Svelte; History from daemon; Quiet Precision. Minimal.

## Task Z — Phase 1 exit checklist

Update ROADMAP Phase 0/1 status notes; list remaining manual verification for human (portal dialogs, logout for extension).

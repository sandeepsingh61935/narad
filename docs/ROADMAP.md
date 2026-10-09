# Narad — Roadmap

Each phase ends in something usable. Phase numbers match the `Phase` column in [REQUIREMENTS.md](REQUIREMENTS.md).

## Phase 0 — Tech spike (de-risk)

Throwaway-quality code proving the four risky pieces on the reference machine.

1. GlobalShortcuts portal: bind `Ctrl+Alt+Space`, receive press **and** release.
2. RemoteDesktop portal + libei: paste into gedit, Firefox, GNOME Terminal, VS Code; restore token avoids re-prompt after restart.
3. GNOME extension: draggable always-on-top floating HUD (idle anchor + listening companion pill), toggles state over D-Bus.
4. sherpa-onnx benchmark: Parakeet 0.6B int8, Parakeet 110M, Moonshine base, Whisper small on 3 s / 5 s / 10 s clips.

**Exit:** measured decode numbers in `spikes/asr-bench/RESULTS.md`; default ASR = `parakeet-ctc-110m-int8` (decision #1); Esc = extension temporary grab (decision #4). **NFR-P-1** measured (PASS): hotkey press → listening median **0.608 ms** / max **0.653 ms** on i5-8250U (`spikes/hotkeys/RESULTS.md`).

## Phase 1 — MVP (core loop)

Daemon + hotkey (hold/tap) + floating HUD + record + VAD + ASR + paste insert + history + minimal dashboard.

- FR-ACT-1/2/3/7, FR-DOT-1/2/3, FR-AUD-1/2/6, FR-ASR-1/2/5, FR-INS-1/2/3
- FR-HIS-1/2/3 (search)/4/9 (basic)/10, FR-MDL-6, FR-CLI-1
- systemd user service, config file

**Exit:** usable as a daily dictation tool in English. **Met (2026-10-09)** — user Phase 1 smoke passed (hold dictate → paste, Esc cancel, HUD click toggle, history).

**Implementation status (2026-10-09) — Phase 1 complete:**
- Core loop: hotkey → mic → VAD → `parakeet-ctc-110m-int8` → history → portal paste; review fixes (clipboard restore, writable `HudVisible`, HUD position, sound cues, FR-MDL-6, History detail/actions, inserted/cancelled).
- **NFR-P-1** PASS; `cargo test -p naradd` 13 ok / `narad-dbus` 4 ok.
- **systemd** `narad.service` enabled; CLI OK; single-instance (bus name + `NoDisplay` desktop).
- Extension: `addChrome({ trackFullscreen: true })` for Shell 50; **State ACTIVE** after session reload.
- Dashboard: Tauri/Svelte History + Settings buildable (`webkit2gtk` present).
- Soft gaps (Phase 2+): terminal `Ctrl+Shift+V`; ASR quality on CTC 110M; paste-last / undo / voice commands / etc.

## Phase 2 — Daily driver

- Paste-last, undo-last (FR-INS-4/5)
- Voice commands, dictionary, replacements, code dictation (FR-TXT-1..4)
- Auto-stop, max duration, mic picker, noise suppression (FR-AUD-3/4/5)
- App context recording (FR-CTX-1)
- Audio retention, retention policy, pause, history extras, export, stats (FR-HIS-5..8)
- File transcription (FR-ASR-6), HUD context menu (FR-DOT-4), CLI history/transcribe (FR-CLI-2/3)

## Phase 3 — Models and languages

- Model manager + catalog + checksums (FR-MDL-1..3)
- Multiple ASR packs, language auto-detect (FR-ASR-3/4)
- Translation packs A → B (FR-TXT-5)
- First-run benchmark + recommendation, idle unloading (FR-MDL-4/5)
- HUD companion mode/language (FR-DOT-5), CLI models (FR-CLI-4)

## Phase 4 — AI cleanup and modes

- Local LLM cleanup with per-mode prompts and grounding guard (FR-TXT-6/7/8)
- Built-in modes + user-defined modes, mode hotkeys, spoken prefixes (FR-ACT-4/5)
- Selection command mode (FR-INS-6)
- Per-app profiles (FR-CTX-2)

## Phase 5 — Hands-free and long-form

- Wake word (FR-ACT-6)
- Long-form / meeting mode with optional speaker labels (FR-LNG-1/2)

## Later

Backlog items in REQUIREMENTS §7; other Linux desktops; Windows/macOS.

# Narad — Roadmap

Each phase ends in something usable. Phase numbers match the `Phase` column in [REQUIREMENTS.md](REQUIREMENTS.md).

## How a phase ships

A phase is not “all daemon work, then a UI pass.” The HUD and the dashboard controls are part of the phase that names them. Three beats, in this order, for each user-visible requirement:

1. **Behavior.** Daemon, store, or config change, with a test. No new control on screen.
2. **Thin UI.** The smallest HUD or dashboard control that operates that behavior with live data. Labels come from [DESIGN.md](DESIGN.md). A number, model id, or status appears only when the daemon returned it.
3. **Visual pass.** After beat 2 works on the real surface. Impeccable, Operate mode, on that surface only:
   - `distill`, then `critique`, then `audit`
   - `clarify` when the labels are the defect
   - `harden` when empty, error, or permission states are the defect
   - `shape` once, before beat 1, only for a surface this phase introduces (the Phase 3 catalog, the Phase 4 mode editor)
   - `bolder`, `delight`, and `overdrive` are out of scope for Narad

Impeccable’s web detector and `live` apply to dashboard Svelte. The HUD is the GNOME extension; review it against DESIGN §3, not the HTML detector.

A screen whose requirement sits in a later phase is not linked in the shell and is not visually polished. The Stitch export is the picture of the 28px anchor and the companion pill. It is not a reason to build a later phase early. Next slice: [superpowers/plans/2026-10-15-dashboard-shell-honesty.md](superpowers/plans/2026-10-15-dashboard-shell-honesty.md).

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
- Contextual symbol protection, eager escape, speech repairs, homophone collocations, punctuation precedence (FR-TXT-9..12)
- Acoustic prosody auto-punctuation (FR-AUD-7), Prefix Trie Joiner biasing (FR-ASR-9)
- Dual-buffer simultaneous typing, Word-Level Rewind, syntactic pre-edit ceiling (FR-INS-9/10)
- Auto-stop, max duration, mic picker, noise suppression (FR-AUD-3/4/5)
- App context recording (FR-CTX-1)
- Audio retention, retention policy, pause, history extras, export, stats (FR-HIS-5..8)
- File transcription (FR-ASR-6), HUD context menu (FR-DOT-4), CLI history/transcribe (FR-CLI-2/3)

**UI in this phase:** History, Stats, Hotkeys, Audio, Dictionary, Privacy, and the HUD including its right-click menu. Those pages stay. The Models item, the static model catalog, and `/setup`’s model picker are Phase 3 and come out of the window until then. Per-app profiles and the cleanup mode editor stay Phase 4. The active-mode list already wired to config may stay; it does not grow prompts, temperature, or a Save button.

**Impeccable:** not on the Stitch paste. After the shell shows only these surfaces, `critique` then `audit` on `dashboard/src/routes/+layout.svelte`. Each later Phase 2 page gets `distill` → `critique` → `audit` only once it reads live daemon state.

**Exit:** daily English dictation plus the Phase 2 behaviors above, each operable from the HUD or the dashboard without a costume page. Not met.

## Phase 3 — Models and languages

- Model manager + catalog + checksums (FR-MDL-1..3)
- Multiple ASR packs, language auto-detect (FR-ASR-3/4)
- Translation packs A → B (FR-TXT-5)
- First-run benchmark + recommendation, idle unloading (FR-MDL-4/5)
- HUD companion mode/language (FR-DOT-5), CLI models (FR-CLI-4)

**UI in this phase:** Models page and the first-run benchmark. The catalog lists installed packs and measured latency on this machine. Decision #1 stays the default (`parakeet-ctc-110m-int8`). Decision #2 (translation runtime) is settled in REQUIREMENTS §8 before a translation row is shown.

**Impeccable:** `shape` the catalog once, against FR-MDL, before implementation. Then the three beats. `clarify` and `harden` for download, verify, failed, and active.

## Phase 4 — AI cleanup and modes

- Local LLM cleanup with per-mode prompts and grounding guard (FR-TXT-6/7/8)
- Built-in modes + user-defined modes, mode hotkeys, spoken prefixes (FR-ACT-4/5)
- Selection command mode (FR-INS-6)
- Per-app profiles (FR-CTX-2)

**UI in this phase:** mode editor (hotkey, spoken prefix, measured extra delay, before/after on a real sample) and per-app default mode. Decision #3 (cleanup LLM) is settled in REQUIREMENTS §8 before a model name is shown. Selection command is a HUD state, not a dashboard costume.

**Impeccable:** `shape` the mode editor once. Then the three beats. Duplicate a built-in mode; the editor does not add a Save button.

## Phase 5 — Hands-free

- Wake word (FR-ACT-6)

**UI in this phase:** wake-word off by default, with a HUD cue while it is listening for the word. HUD cue reviewed against DESIGN §3.

## Later

Backlog items in REQUIREMENTS §7; other Linux desktops; Windows/macOS.

# Narad — Architecture

Companion to [REQUIREMENTS.md](REQUIREMENTS.md).

## 1. Overview

```
 ┌──────────────────────── GNOME Shell (Wayland) ────────────────────────┐
 │  narad@narad.dev extension  ("desktop bridge")                        │
 │   • floating HUD (anchor + companion pill; always on top, draggable) │
 │   • focused app id, PRIMARY selection, transient Esc grab             │
 └───────────────┬───────────────────────────────────────────────────────┘
                 │ D-Bus (session bus)  dev.narad.Daemon
 ┌───────────────┴──────────────────────────────────────────┐
 │  naradd  (Rust daemon, systemd user service)             │
 │                                                          │
 │  Hotkeys ── xdg-desktop-portal GlobalShortcuts           │
 │  Audio   ── PipeWire capture → resample 16 kHz mono      │
 │  Pipeline: VAD → ASR → commands → dictionary →           │
 │            translate? → cleanup? → final text            │
 │  Insert  ── clipboard + RemoteDesktop portal (libei)     │
 │  Store   ── SQLite + FTS5, audio spool (Opus)            │
 └───────▲──────────────────────────────────▲───────────────┘
         │ D-Bus                            │ D-Bus
 ┌───────┴────────┐                 ┌───────┴────────┐
 │ narad-dashboard│                 │  narad (CLI)   │
 │ Tauri v2 +     │                 │                │
 │ Svelte         │                 │                │
 │ (+ model       │                 │                │
 │  downloads)    │                 │                │
 └────────────────┘                 └────────────────┘
```

Three processes, one contract (D-Bus interface). The daemon owns all state and inference; the dashboard and CLI are thin clients. The GNOME extension is the only GNOME-specific piece, which keeps porting to other desktops contained.

## 2. Why these platform choices (GNOME 50 / Wayland)

| Need | Choice | Reason / rejected alternatives |
|---|---|---|
| Global hotkey with **hold** and **tap** | `org.freedesktop.portal.GlobalShortcuts` | Emits `Activated` *and* `Deactivated`, so push-to-talk works. GNOME custom keybindings only fire on press. Supported by xdg-desktop-portal-gnome ≥ 48. |
| Insert text into any app | Clipboard set + `Ctrl+V` via `org.freedesktop.portal.RemoteDesktop` (libei keyboard), with persisted restore token | `wtype` unsupported on GNOME (no virtual-keyboard protocol). `ydotool` needs root/uinput setup. Typing keysyms breaks on non-Latin scripts; paste handles any Unicode. |
| Always-on-top floating HUD | GNOME Shell extension (`Main.layoutManager.addChrome`) | Wayland clients cannot position themselves or force always-on-top. XWayland hack is fragile. Circular ~28px anchor + Raycast-style companion waveform pill (Libadwaita dark + Adwaita Blue). |
| Focused app, selected text | Same extension (`global.display.focus_window`, `St.Clipboard` PRIMARY) | No portal exposes these. |
| Cancel with `Esc` | Extension adds a keybinding only while recording (**decision #4 locked**) | A permanent global `Esc` would break every app. Portal Esc shortcut is a non-GNOME fallback later. |

## 3. Components

### 3.1 `naradd` (daemon)

Rust, single binary, `tokio` runtime with dedicated inference threads.

| Module | Responsibility | Key crates / libs |
|---|---|---|
| `ipc` | D-Bus service `dev.narad.Daemon` (methods, signals, properties) | `zbus` |
| `hotkeys` | Portal session, bind shortcuts, hold/tap state machine | `ashpd` |
| `audio` | PipeWire capture, ring buffer, resample to 16 kHz mono f32, level meter | `pipewire` (or `cpal`), `rubato` |
| `denoise` | Optional RNNoise | `nnnoiseless` |
| `vad` | Silero VAD: trim, auto-stop, silence/noise rejection | `sherpa-onnx` |
| `asr` | Offline recognisers (Parakeet TDT, Whisper, SenseVoice, Moonshine) behind one trait | `sherpa-onnx` |
| `text` | Voice commands, dictionary, replacements, code-case rules, hallucination filter | pure Rust |
| `translate` | MT pack runner | `ct2rs` (CTranslate2) — or Bergamot, see open decision |
| `cleanup` | LLM prompts per mode, grounding guard | `llama-cpp-2` (llama.cpp, GGUF) |
| `insert` | Clipboard save/set/restore, paste keystroke via libei, terminal detection, undo | `ashpd`, `reis` |
| `store` | SQLite schema, FTS5 search, retention jobs, audio spool | `rusqlite` |
| `models` | Registry of installed packs, load/unload, idle timers, benchmark | — |
| `config` | TOML config, live reload | `serde`, `toml`, `notify` |

`sherpa-onnx` is deliberately the single runtime for VAD, ASR, wake-word spotting, and diarization: one dependency, one ONNX Runtime, CPU-optimised int8 models.

### 3.2 GNOME extension (`extension/`)

GJS, ~300 lines target. Draws the floating HUD (idle anchor; listening/processing companion pill), forwards clicks, reports focus changes and PRIMARY selection on request, grabs `Esc` during recording. Listens to daemon signals for state and mic level.

### 3.3 `narad-dashboard`

Tauri v2 + Svelte (small bundle, fast cold start). Talks to the daemon over D-Bus via a thin Rust command layer. Owns model downloads (the only network code in the system). Views: **History**, **Stats**, **Models**, **Settings**.

### 3.4 `narad` CLI

Rust, `clap`, shares the D-Bus client crate with the dashboard.

## 4. Hot path: hotkey → text

```
t0  hotkey Activated (portal)
    ├─ open PipeWire stream, play start cue, signal state=listening
    ├─ ask extension for focused app id (async, cached on focus-change)
t1  hotkey Deactivated (hold) | second tap | VAD auto-stop
    ├─ close stream, signal state=processing
    ├─ VAD trim → reject if no speech
    ├─ ASR (model already resident)            ← dominant cost
    ├─ commands → dictionary → hallucination filter
    ├─ [translate] [cleanup + guard]           ← only if mode enables
    ├─ insert: save clipboard → set text → Ctrl+V / Ctrl+Shift+V → restore clipboard
    ├─ write history row (async, off hot path)
t2  signal state=idle
```

Latency rules:
- Models are loaded at startup, never on the hot path (except after idle unload).
- Audio is accumulated in memory; no disk I/O before insertion. Spooling for crash safety happens on a background thread.
- History writes, stats, and audio encoding happen after insertion.
- Inference threads pinned to physical cores (4 on the reference CPU).

## 5. Data

Locations (XDG):

| Path | Content |
|---|---|
| `~/.config/narad/config.toml` | Settings, hotkeys, modes, per-app profiles |
| `~/.local/share/narad/narad.db` | History, dictionary, stats |
| `~/.local/share/narad/audio/` | Retained audio (`<entry-id>.opus`) |
| `~/.local/share/narad/models/<type>/<id>/` | Model packs + `manifest.toml` |
| `$XDG_RUNTIME_DIR/narad/spool/` | In-flight audio for crash recovery |

Core tables (sketch):

```sql
entries(id, created_at, duration_ms, mode, src_lang, dst_lang,
        asr_model, mt_model, llm_model, app_id,
        raw_text, final_text, insert_status, cleanup_flagged,
        audio_path, pinned)
entries_fts(raw_text, final_text)        -- FTS5, external content
tags(id, name);  entry_tags(entry_id, tag_id)
dictionary(id, kind /* vocab|replace|snippet */, spoken, written)
```

Model pack manifest (default English ASR — REQUIREMENTS §8 decision #1):

```toml
id = "parakeet-ctc-110m-int8"
type = "asr"            # asr | mt | llm | kws
runtime = "sherpa-onnx"
languages = ["en"]      # mt: pairs = [["en","hi"]]
size_mb = 126           # on-disk pack dir on reference machine; verify on install
sha256 = "…"
files = ["model.int8.onnx", "tokens.txt"]
# Other packs (e.g. Parakeet TDT 0.6B, Whisper) use different file sets; user-selectable in Phase 3.
```

## 6. D-Bus interface (sketch)

```
dev.narad.Daemon
  Methods:  Start(mode s) · Stop() · Cancel() · Toggle(mode s)
            PasteLast() · UndoLast() · SelectionCommand()
            Transcribe(path s, mode s, dst_lang s) → (id x)
            History(query s, offset u, limit u) → a(...)
            ReinsertEntry(id x) · DeleteEntries(ids ax)
            ListModels() · SetActiveModel(type s, id s) · Benchmark()
  Signals:  StateChanged(state s)           # idle|listening|processing|error|copied
            Level(rms d)                    # ~30 Hz while listening
            EntryAdded(id x)
  Properties: Mode s · Paused b · HudVisible b
```

## 7. Repository layout (planned)

```
narad/
  Cargo.toml              # workspace
  crates/
    naradd/               # daemon
    narad-core/           # pipeline, text rules, store (platform-agnostic)
    narad-dbus/           # shared D-Bus client/types
    narad-cli/
  dashboard/              # Tauri v2 + Svelte
  extension/              # GNOME Shell extension
  packaging/              # systemd unit, RPM spec
  docs/
```

`narad-core` has no Linux/GNOME dependencies so it can be reused on other platforms; platform specifics live in `naradd` behind traits (`HotkeySource`, `Inserter`, `DesktopBridge`).

## 8. Architecture Decision Records (ADRs)

### ADR-006: Libadwaita Dark Visual System, CSD Window Posture & Offline Typography

- **Status:** Accepted (2026-10-18)
- **Context:** The legacy UI prototype used "Quiet Precision" with a saffron accent (`#E08A1E`), Inter / JetBrains Mono web fonts, and dual light/dark themes. GNOME ecosystem users expect a cohesive Libadwaita dark experience, offline typography, and a Raycast-style floating companion pill HUD that integrates naturally with GNOME Shell 50 on Wayland.
- **Decision:** Complete cutover to the Libadwaita dark visual system:
  1. **Tokens:** Warm charcoal neutrals (`--bg: oklch(21% 0.01 72)`, `--surface: oklch(24.5% 0.01 72)`, `--raised: oklch(28.8% 0.01 72)`, `--border: oklch(36% 0.01 72)`) with 1px planar rules and no decorative glow or glass.
  2. **Accent:** Single Adwaita Blue accent (`oklch(70% 0.125 250)` / sRGB `#3584e4`) replacing legacy saffron; used sparingly for active states, primary actions, and listening indicators.
  3. **Typography:** Bundled offline IBM Plex Sans (display/body) and IBM Plex Mono (provenance stamps, timestamps, model IDs, counts, keycaps).
  4. **Window Posture:** Tauri v2 Client-Side Decorations (CSD) with integrated 46px headerbar, custom window controls (minimize/maximize/close), drag region, and offline provenance badge, replacing native OS titlebars.
  5. **Shell Compatibility:** GNOME Shell St uses exact sRGB hex mappings (`#3584e4`, `#1b1813`, `#23201b`, `#403c37`, `#efebe1`) due to St engine parser constraints.
- **Consequences:** 100% offline consistency across distros, zero network dependencies for font assets, seamless GNOME 50 desktop integration, and removal of dual-theme maintenance overhead.

# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

## Users

Primary audience for this design pass: **broader GNOME desktop users** on Linux (Fedora/GNOME Wayland target), who want system-wide voice-to-text without leaving the focused app. Typical situation: typing into mail, chat, docs, or code; hands on keyboard; low patience for setup friction.

Secondary (product owner / power user): heavy hotkey dictation all day; dashboard mainly for history, models, and settings.

## Product Purpose

Narad is a **local, universal voice-to-text** tool: speak anywhere on the desktop; text appears at the cursor of the focused app. Success is text landing before the user notices a wait, with clear feedback that recording started, stopped, and where the text went — without audio or transcripts leaving the machine.

## Positioning

Everything runs on-device. The only network use is an explicit, user-initiated model download. The primary product chrome is a **floating waveform HUD capsule** (Stitch Quiet Precision), with a quiet Tauri dashboard as archive/control room; most usage never opens the dashboard.

## Surface decision (2026-10-09)

- **Authority:** Stitch export at `/home/sandy/Downloads/stitch_narad_offline_voice_to_text_desktop_app/` (`quiet_precision/DESIGN.md` + `narad_floating_hud_component_sheet/`).
- **Chosen floating surface:** Compact ~28px circular anchor + companion waveform HUD pill while active (Stitch sheet). Quiet Precision tokens (saffron accent, warm charcoal, Inter / JetBrains Mono).
- **Synced:** `docs/DESIGN.md`, FR-DOT-* / related rows in `docs/REQUIREMENTS.md`, `docs/ARCHITECTURE.md` (incl. D-Bus `HudVisible`), `docs/STITCH_PROMPTS.md`, `docs/ROADMAP.md`.

## Operating Context

- OS/desktop: Linux — Fedora 44, GNOME 50 on Wayland (v1); other desktops later.
- Surfaces: GNOME Shell extension (floating HUD, hotkey UX) + Tauri v2 / Svelte dashboard + CLI; Rust daemon owns state and inference over D-Bus.
- UX behavior authority: `docs/DESIGN.md`. Visual design (colors, type, components) deferred to the Stitch project (`docs/STITCH_PROMPTS.md`).
- Stitch prototype (pending access): https://stitch.withgoogle.com/preview/6703747760591777930?node-id=de92b7b12ed045609b949b4879de87e5

## Capabilities and Constraints

- Local-only; near-zero idle CPU; memory bounded and user-controllable.
- Hold vs tap dictation; Esc cancels; clipboard fallback when no text field focused.
- Floating HUD states: idle (anchor); listening (saffron + waveform companion pill); processing; inserted/copied/cancelled/error outcome pills.
- Dashboard is archive/control room (history, models, settings) — out of scope for this UI pass.
- Open: Stitch screens not yet visible from agent tooling (Google sign-in). Visual tokens remain Stitch-owned until export/screenshots are available.

## Brand Commitments

- Name: Narad.
- Voice: plain, short, sentence case; no hype; buttons are verbs; errors say what happened and what to do (`docs/DESIGN.md` §6).

## Evidence on Hand

- Specs: `docs/REQUIREMENTS.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/DESIGN.md`, `docs/STITCH_PROMPTS.md`.
- Extension stub: `extension/` (HUD surface lives here; Phase 0 spike is the idle circular anchor).
- Dashboard folder reserved (`dashboard/`) but empty of app UI.
- Stitch local export (visual authority): `/home/sandy/Downloads/stitch_narad_offline_voice_to_text_desktop_app/`
  - Design system: `quiet_precision/DESIGN.md` (“Quiet Precision”)
  - Floating HUD / indicator sheet: `narad_floating_hud_component_sheet/`
  - First-run, empty/edge, history, models, settings, dictionary, stats, privacy screens also present
- Stitch preview URL (auth-gated for agents): https://stitch.withgoogle.com/preview/6703747760591777930?node-id=de92b7b12ed045609b949b4879de87e5

## Product Principles

1. The floating HUD is the product; the dashboard is secondary.
2. Feedback before latency — status must be obvious within a blink.
3. Keyboard first; never block the dictation path with modals.
4. Trust is visible: local-only, models, and utterance fate are discoverable.
5. Progressive disclosure: defaults work; advanced options stay out of the way.

## Accessibility & Inclusion

Status never by color alone (icon, shape, or label). Visible focus, comfortable targets including the HUD anchor, reduced-motion alternative for listening waveform/pulse, screen-reader announcements for recording start/stop and insertion result. Content stress cases include Hindi/Devanagari, Hinglish, and long dictations.

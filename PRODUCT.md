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

## Surface decision (2026-10-15)

- **Behavior authority:** `docs/REQUIREMENTS.md` and `docs/DESIGN.md`. Phase order: `docs/ROADMAP.md`.
- **Visual authority:** Quiet Precision tokens and the ~28px circular anchor plus companion waveform pill. The Stitch HTML is a picture of that HUD, not a spec for dashboard behavior.
- **Chosen floating surface:** Compact ~28px circular anchor + companion waveform HUD pill while active. Saffron accent, warm charcoal, Inter / JetBrains Mono. No glow, no glass.
- **Impeccable:** Operate mode. `shape` only before a new phase surface. `distill`, `critique`, and `audit` after that surface reads live data. Do not run `bolder`, `delight`, or `overdrive`.

## Operating Context

- OS/desktop: Linux — Fedora 44, GNOME 50 on Wayland (v1); other desktops later.
- Surfaces: GNOME Shell extension (floating HUD, hotkey UX) + Tauri v2 / Svelte dashboard + CLI; Rust daemon owns state and inference over D-Bus.
- UX behavior authority: `docs/DESIGN.md`. Quiet Precision tokens are the visual system. `docs/STITCH_PROMPTS.md` is how the export was prompted, not a requirement.

## Capabilities and Constraints

- Local-only; near-zero idle CPU; memory bounded and user-controllable.
- Hold vs tap dictation; Esc cancels; clipboard fallback when no text field focused.
- Floating HUD states: idle (anchor); listening (saffron + waveform companion pill); processing; inserted/copied/cancelled/error outcome pills.
- Dashboard is the archive and control room. Phase 2 shows History, Stats, and Settings. Models waits for Phase 3. The cleanup mode editor waits for Phase 4.

## Brand Commitments

- Name: Narad.
- Voice: plain, short, sentence case; no hype; buttons are verbs; errors say what happened and what to do (`docs/DESIGN.md` §6).

## Evidence on Hand

- Specs: `docs/REQUIREMENTS.md`, `docs/ARCHITECTURE.md`, `docs/ROADMAP.md`, `docs/DESIGN.md`.
- HUD: `extension/`. Dashboard: `dashboard/` (Tauri + Svelte). History, stats, dictionary, audio, and privacy read the daemon or config. The Models route and `/setup` are still Stitch pastes and are scheduled for removal in the shell-honesty plan.
- Stitch export, HUD picture only: `/home/sandy/Downloads/stitch_narad_offline_voice_to_text_desktop_app/` (`quiet_precision/DESIGN.md`, `narad_floating_hud_component_sheet/`).

## Product Principles

1. The floating HUD is the product; the dashboard is secondary.
2. Feedback before latency — status must be obvious within a blink.
3. Keyboard first; never block the dictation path with modals.
4. Trust is visible: local-only, models, and utterance fate are discoverable.
5. Progressive disclosure: defaults work; advanced options stay out of the way.

## Accessibility & Inclusion

Status never by color alone (icon, shape, or label). Visible focus, comfortable targets including the HUD anchor, reduced-motion alternative for listening waveform/pulse, screen-reader announcements for recording start/stop and insertion result. Content stress cases include Hindi/Devanagari, Hinglish, and long dictations.

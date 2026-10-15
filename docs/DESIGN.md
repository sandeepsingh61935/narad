# Narad — UX Guidelines

How Narad should behave and be structured. Visual tokens come from Quiet Precision (warm charcoal, saffron `#E08A1E`, Inter, JetBrains Mono, libadwaita radii, no glow or glass). Prompts that produced the Stitch export: [STITCH_PROMPTS.md](STITCH_PROMPTS.md).

**Visual / surface authority:** the 28px circular anchor and the companion pill while dictating, as drawn in the Stitch export `quiet_precision/DESIGN.md` and `narad_floating_hud_component_sheet/`. That export is not behavior authority. Where a Stitch screen disagrees with this file or [REQUIREMENTS.md](REQUIREMENTS.md), those docs win. Visual polish runs inside the phase that owns the surface, after the control uses live daemon state ([ROADMAP.md](ROADMAP.md), “How a phase ships”).

## 1. Principles

1. **The floating HUD is the product.** Most usage never opens the dashboard. Design the always-on-top indicator and its dictation feedback first; the dashboard is a quiet archive and control room.
2. **Feedback before latency.** The user must know within a blink that recording started, stopped, and where the text went.
3. **Keyboard first.** Every action is reachable without a mouse, and shortcuts are shown next to the actions they trigger.
4. **Never block.** Toasts with Undo instead of confirmation dialogs; nothing modal on the dictation path.
5. **Trust is visible.** Local-only status, loaded models, and what happened to each utterance are always discoverable.
6. **Progressive disclosure.** Defaults work; advanced options exist but stay out of the way.

## 2. Core flows to design (not just screens)

| Flow | Steps | Must answer for the user |
|---|---|---|
| Dictate (hold) | press → listening → release → processing → inserted | Is it hearing me? Did it finish? Where did the text go? |
| Dictate (tap) | tap → listening → silence auto-stop or tap → inserted | When will it stop? How do I stop/cancel? |
| Cancel | listening → Esc → discarded | Was anything saved or typed? (No.) |
| No text field focused | processing → copied to clipboard | Where is my text? |
| Correct a mistake | open dashboard → find entry → edit → "add to dictionary?" | Will it get this right next time? |
| Undo / paste last | hotkey → text removed / re-inserted | What exactly was undone? |
| Selection command | select text → hotkey → speak instruction → replaced | What will be changed? Can I revert? |
| Switch mode | HUD menu / mode hotkey / spoken prefix | Which mode am I in right now? |
| First run | permission → shortcut → model → try it | Is it set up? What do I press? |
| Install a model | browse → download → verify → set active | Size, speed on *my* machine, is it safe? |
| Translate | speak A → text in B | Which language pair is active? |

## 3. Floating HUD (primary surface)

Stitch model: a **compact circular anchor** (~28px) that stays out of the way when idle, plus a **companion frosted HUD pill** that appears while dictating / processing / confirming outcome. Visual tokens: Quiet Precision (warm charcoal surfaces, saffron `#E08A1E` accent, Inter + JetBrains Mono, libadwaita radii, planar borders — no decorative AI glow meshes).

| State | Anchor | Companion HUD |
|---|---|---|
| **Idle** | Neutral 28px circle; hover shows hotkey hint | None |
| **Listening** | Saffron fill + mic icon + volume pulse ring | Waveform bars · mode · language pair · elapsed time |
| **Processing** | Spinner ring around core | “Transcribing…” · active model id |
| **Inserted** | Brief success cue | Optional short confirmation, then collapse to idle |
| **Copied** | Check / success fill | “Copied to clipboard” · reason (e.g. no focus) · paste keycap |
| **Cancelled** | Returns to idle; no history entry | Brief “Cancelled” or none |
| **Error** | Alert glyph / crimson treatment | Plain error + one recovery action (e.g. “Check audio settings”) |
| **History paused** | Distinct paused cue | Optional “History paused” |
| **Hidden** | Not drawn; hotkeys still work | — |

Status must never rely on color alone — pair it with an icon, a shape, or a label. Reduced-motion: replace waveform bounce / pulse ring with a static listening glyph and a non-animated spinner alternative.

Other surfaces:

| Surface | States |
|---|---|
| History | empty (first use) · loading · results · no search results · entry selected · editing · multi-select · audio playing |
| Models | none installed · downloading (progress, cancel) · verifying · installed · active · download failed · benchmark running/results |
| App-wide | daemon stopped · GNOME extension missing (HUD unavailable, hotkey still works) · microphone missing · permission denied · AI model unloaded (idle) |

## 4. Interaction rules

- **Keyboard map (dashboard):** `Ctrl+K` command palette · `/` focus search · `J/K` next/prev entry · `Enter` re-insert · `C` copy · `E` edit · `P` pin · `Del` delete (Undo toast) · `Esc` close pane/palette.
- **Command palette** reaches every action, mode, model, setting, and history search.
- **Edit to correct:** editing an entry suggests adding changed words to the dictionary.
- **Undo over confirm:** destructive actions happen immediately with an Undo toast. Only "Delete all history" asks for typed confirmation.
- **Latency honesty:** modes with AI cleanup show their measured extra delay.
- **Mode always visible:** current mode (and language pair when relevant) shown on the companion HUD while listening/processing and in the dashboard status area.
- **Raw is never lost:** any cleaned/translated entry can be toggled to its raw transcript.
- **HUD etiquette:** draggable (anchor), snaps to screen edges, remembers position, hides in fullscreen apps; companion pill must not cover the text the user is typing into.
- **Context menu (right-click anchor):** active mode selector, open dashboard, pause history, hide HUD, quit — with keycap accelerators (Stitch Adwaita popover).
- **Downloads:** show source, size, and verification result; this is the only network action and the UI says so.

## 5. Information hierarchy

- **History row:** text preview first; then time, app, mode; duration and models are secondary.
- **Entry detail:** final text → actions (copy, re-insert) → raw/final toggle → audio → metadata.
- **Stats:** one headline number (time saved), then trends, then breakdowns.
- **Models:** "what's active and how fast it is on this machine" before the catalog.
- **Settings:** grouped by user intent (Shortcuts, Microphone, Modes, Words, Privacy), not by internal component.

## 6. Content and tone

- Plain, short, sentence case. No hype, no "AI magic".
- Buttons are verbs: "Re-insert", "Download", "Run benchmark".
- Errors state what happened and what to do: "Microphone unavailable. Choose another in Settings → Audio."
- Design with realistic and stress-case content: long dictations, Hindi/Devanagari text, mixed Hinglish, code, very short one-word entries, thousands of entries.

## 7. Accessibility

- Visible keyboard focus on every interactive element.
- No information by color alone.
- Comfortable pointer targets, including the HUD anchor (~28px minimum with hover affordance).
- Layout holds at larger text sizes and narrow window widths.
- Reduced-motion alternative for listening waveform / pulse.
- Screen-reader announcements for recording start/stop and insertion result.

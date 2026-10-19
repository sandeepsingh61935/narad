# Narad — Design & UX Guidelines

How Narad behaves and looks. The visual contract is defined by the **Libadwaita Dark Visual System** (`brand-spec.md`, `DESIGN-HANDOFF.md`, `DESIGN-MANIFEST.json`, and ADR-006): a warm-charcoal Libadwaita desktop canvas, 1px planar rules, Adwaita Blue accent used sparingly, bundled offline IBM Plex typography, and a Raycast-style floating companion pill overlay rendered by GNOME Shell.

## 1. Design Principles

1. **The floating HUD is the product.** Most interactions never open the dashboard window. Design the always-on-top indicator and its dictation feedback first; the dashboard is a quiet archive, model manager, and control room.
2. **Quiet chrome, singular focus.** The window chrome stays quiet with neutral charcoal surfaces; the listening pill is the only loud instrument. A single blue accent appears at most twice per screen.
3. **Feedback before latency.** The user must know within a blink that recording started, stopped, and where the text went.
4. **Keyboard first.** Every action is reachable without a mouse, primary actions show their keycaps, and shortcuts are visible next to their targets.
5. **Never block.** Toasts with Undo affordance instead of blocking confirmation dialogs; no modal interruptions on the dictation hot path.
6. **Provenance and trust are visible.** Offline status, active model ID, local elapsed time, and utterance fates are always inspectable. No audio or text leaves the user's machine.
7. **Progressive disclosure.** Sensible defaults work out of the box; advanced configurations exist but stay out of the way.

## 2. Design Tokens

The visual system uses warm-charcoal neutrals warmed off Adwaita's cool gray, paired with Adwaita Blue. 1px planar borders replace decorative gradients, glass, glow, or neon.

### Color Tokens

| Token | OKLCH Value | sRGB Hex Fallback | Purpose / Usage |
|---|---|---|---|
| `--desktop` | `oklch(15.5% 0.012 65)` | `#100b07` | Stage behind the window / desktop surface |
| `--bg` | `oklch(21% 0.01 72)` | `#1b1813` | Main Libadwaita window canvas background |
| `--surface` | `oklch(24.5% 0.01 72)` | `#23201b` | Sidebar, input fields, popovers, grouped containers |
| `--raised` | `oklch(28.8% 0.01 72)` | `#2e2a26` | Card backgrounds, list selections, active items |
| `--fg` | `oklch(94% 0.014 85)` | `#efebe1` | Primary text and foreground glyphs |
| `--muted` | `oklch(72% 0.016 78)` | `#aaa49a` | Secondary text, shortcut keycaps, metadata |
| `--border` | `oklch(36% 0.01 72)` | `#403c37` | 1px planar boundaries, dividers, card borders |
| `--accent` | `oklch(70% 0.125 250)` | `#3584e4` | Adwaita Blue: active navigation icons, primary buttons, listening waveform bars |
| `--ok` | `oklch(72% 0.10 155)` | `#6fb789` | Status: inserted success, positive feedback |
| `--warn` | `oklch(80% 0.10 85)` | `#dbb970` | Status: warning, attention required |
| `--danger` | `oklch(66% 0.15 28)` | `#df695c` | Status: error, destructive actions |

> [!NOTE]
> GNOME Shell St uses exact sRGB hex mappings (`#3584e4`, `#1b1813`, `#23201b`, `#403c37`, `#efebe1`) because St lacks `oklch()` color-space support.

### Geometry, Radii & Posture

- **Window Canvas:** Width `min(1120px, 100%)`, height `min(760px, calc(100vh - 32px))`, border radius `12px` (`--window-radius`).
- **Controls & Cards:** Border radius `8px` (`--radius`).
- **HUD Posture:** 9999px full pill radius only on the floating HUD overlay and search input.
- **Rules & Planar Borders:** Strict 1px planar borders (`var(--border)`). No decorative drop shadows or glow (`--shadow-pop: none`).
- **Client-Side Decorations (CSD):** 46px integrated headerbar with window drag area, title, offline provenance stamp, and window controls (`wc-min`, `wc-max`, `wc-close`).

### Motion & Easing

- **Enter Duration:** `--dur: 180ms` with easing `--ease: cubic-bezier(0.23, 1, 0.32, 1)`.
- **Exit Duration:** `--dur-out: 140ms`.
- **Reduced Motion:** When `prefers-reduced-motion: reduce` is detected, animations (waveform bar bounce, spinner rotation, pulse rings) are replaced with static glyphs and immediate opacity transitions.

## 3. Typography Contract

Narad packages offline fonts directly into the application distribution, ensuring 100% offline visual consistency without external CDN requests.

- **Display & Body Font:**
  `"IBM Plex Sans", "Cantarell", "Adwaita Sans", "Noto Sans", sans-serif`
  Base sizing: 15px / 1.5 line height, letter-spacing 0, antialiased.
- **Monospace Font:**
  `"IBM Plex Mono", "Noto Sans Mono", ui-monospace, monospace`
  Used for model IDs, timestamps, keyboard shortcuts (`.kbd`), provenance stamps, tabular statistics, and code dictation output.
- **Tabular Figures:** Numbers in timers, counts, and stats use tabular figures (`font-variant-numeric: tabular-nums`) with tight tracking.

## 4. Raycast-Style Floating HUD (Primary Surface)

The primary surface is an always-on-top GNOME Shell companion overlay (`Main.layoutManager.addChrome`). When idle, it rests as an unobtrusive 28px circular anchor. When dictating or processing, it expands smoothly into a Raycast-style floating pill displaying microphone level, waveform bars, elapsed time, mode, and insertion feedback.

### HUD State Machine

| State | Anchor / Visual | Companion Pill Content | Interaction & Feedback |
|---|---|---|---|
| **Idle** | Neutral 28px circular anchor with microphone glyph (`.hud.is-idle`) | Collapsed; hover reveals global hotkey hint (`Ctrl+Alt+Space`) | Click toggles dictation; right-click opens Adwaita popover menu. |
| **Listening** | Adwaita Blue fill + active mic icon | 4 animated waveform bars in Adwaita Blue (`--accent`), "Listening" label, mode stamp ("Hold" / "Tap"), and elapsed timer (`0:00`) | Hold release or second tap stops; `Esc` cancels and discards audio. |
| **Transcribing** | Spinner ring around core icon (`.spinner`) | "Transcribing" label, active local model stamp (e.g. `small · local` or `parakeet · local`) | Inference active; audio stream closed. |
| **Inserted** | Success checkmark in `--ok` green | "Inserted at cursor" label, brief Undo button affordance (`.hud-undo`) | Outcome confirmed; text placed via clipboard + paste portal. |
| **Copied** | Success checkmark in `--ok` green | "Copied to clipboard" label, reason note (e.g. "No focused input"), and paste keycap hint (`Ctrl+V`) | Triggered if no editable cursor was detected. |
| **Cancelled** | Collapses back to idle anchor | Brief "Cancelled" toast or silent collapse; no transcript or audio saved | Triggered by pressing `Esc` during recording. |
| **Error** | Alert glyph in `--danger` red | Plain error description (e.g. "Microphone unavailable") + single recovery action | Status paired with icon; right-click offers audio device settings. |
| **Paused** | Paused indicator badge on anchor | "History paused" banner / badge | Transcriptions still insert at cursor but are not written to history. |

### Anchor Context Menu (Right-Click)

A Libadwaita popover menu anchored to the floating pill:
- **Hold to talk** (`1`): Set activation posture to push-to-talk.
- **Tap to toggle** (`2`): Set activation posture to toggle recording.
- **Pause history** (`P`): Toggle recording transcripts to SQLite.
- **Open dashboard** (`Enter`): Bring the Tauri dashboard to focus.
- **Stats** (`S`): Open local statistics view.
- **Quit** (`Q`): Terminate the daemon and extension HUD.

## 5. The 9 Core Screens

The application UI is structured across 9 distinct user-facing surfaces (`DESIGN-MANIFEST.json`):

1. **Launcher / Overview (`index.html`):**
   The central desktop hub. Displays an overview of Narad's offline capabilities, active model stamp (`small · local`), an interactive miniature HUD preview widget, and links to all primary surfaces.
2. **Floating HUD (`narad-hud.html`):**
   Full desktop stage demonstrating the 28px circular anchor, active waveform companion pill, right-click menu, focused editor interaction, and live state transitions.
3. **Master-Detail History (`narad-history.html`):**
   Dual-pane transcript browser. Left pane houses search filter (`/`) and transcript list; right pane shows transcript details, audio player with interactive scrubber waveform (`.wave`), raw speech vs AI-cleaned text toggle, and "Keep clip" retention control.
4. **Local Statistics (`narad-stats.html`):**
   Honest local analytics: word counts, speaking duration, audio clips kept, delivery destination breakdown (target apps), and custom vocabulary hits. No speculative "time saved" or vanity metrics.
5. **Speech Models (`narad-models.html`):**
   Local model catalog and manager. Inspect installed ASR models (Parakeet CTC 110M default, Whisper variants), download progress, SHA-256 verification status, on-disk size, and benchmark execution.
6. **Custom Words (`narad-words.html`):**
   Domain vocabulary and acronym dictionary (e.g., `drm_ioctl_kernel`, `Libadwaita`, `oklch`, `Wayland`). Maps spoken phonetics to precise written terms to bias speech decoding.
7. **Settings (`narad-settings.html`):**
   System preferences grouped by intent: activation mode (hold vs tap), global shortcuts, insertion target (insert at cursor vs copy to clipboard), audio input device picker, and autostart on login.
8. **First-run Setup (`narad-setup.html`):**
   Frictionless 3-step setup wizard: microphone permission and input detection, shortcut confirmation, and a 3-second live dictation test.
9. **Dictation Sequence (`narad-dictation.html`):**
   Interactive walkthrough and motion timeline demonstrating the 14-second desktop dictation cycle from shortcut press to caret delivery.

## 6. Interaction & Keyboard Map

Narad adheres to a strict keyboard-first posture. Every primary action displays its keycap (`.kbd`).

### Global Shortcuts

- **Dictate (Hold / Tap):** `Ctrl+Alt+Space` (configurable)
- **Cancel Recording:** `Esc` (registered only while recording is active)
- **Selection Command:** `Ctrl+Alt+R` (speak instruction on selected text)
- **Paste Last Transcription:** `Ctrl+Alt+V`
- **Undo Last Insertion:** `Ctrl+Alt+Z`
- **Mode Quick Switch:** `Ctrl+Alt+1..9`

### Master-Detail History Navigation

- `/`: Focus search input
- `J` or `ArrowDown`: Select next transcript in list
- `K` or `ArrowUp`: Select previous transcript in list
- `Enter`: Re-insert selected transcript at current cursor
- `C`: Copy selected transcript to clipboard
- `Del` or `Backspace`: Delete transcript (triggers non-blocking Undo toast)
- `Esc`: Clear search input focus or close detail view

### Global Dashboard & Popover Controls

- `Ctrl+K`: Global command palette
- `1` / `2`: Select Hold / Tap mode in HUD menu
- `P`: Toggle Pause History in HUD menu
- `Esc`: Dismiss HUD popover or active modal

## 7. Information Hierarchy & Provenance

- **Provenance Stamp:** Every surface displays provenance in monospace (`IBM Plex Mono`): model identifier, timestamp, local/offline indicator, and shortcut keycaps (`data-model-stamp`, `data-od-id="offline-stamp"`).
- **History Detail Hierarchy:** Final text first → primary action ("Insert at cursor" in Adwaita Blue) → raw/cleaned version toggle → audio scrubber player → metadata line (target app, time, model).
- **Honest Analytics:** Only real, measured counts are recorded. The interface never invents speculative "time saved" or "typing speed" approximations.

## 8. Accessibility Rules

- **No Status by Color Alone:** Every state change pairs color with an icon glyph, a shape alteration, or a textual label.
- **High-Contrast Focus Indicators:** All interactive elements (`button`, `input`, `a`, `.nav-item`, `.row`) provide visible focus rings using 2px high-contrast outlines.
- **Screen Reader Support:** Recording state changes and insertion results announce status via `aria-live="polite"` live regions (`role="status"`). Menus and listboxes use standard WAI-ARIA attributes (`role="menu"`, `role="listbox"`, `aria-checked`, `aria-pressed`).
- **Target Sizes:** Minimum interactive hit target size is 28px with generous hover and padding envelopes.
- **Reduced Motion:** Complete support for `prefers-reduced-motion: reduce`: waveform bars stay static, spinner rotation is replaced with a static loader icon, and animations default to subtle opacity changes.

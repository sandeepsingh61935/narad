# Mode Editor Design Brief (Phase 4)

> **Authority:** `docs/ROADMAP.md` Phase 4 ("shape the mode editor once. Then the three beats. Duplicate a built-in mode; the editor does not add a Save button."), `docs/REQUIREMENTS.md` (FR-ACT-4/5, FR-TXT-6/7/8, FR-CTX-2, Decision #3), `docs/DESIGN.md` §3/§4/§5/§6/§7.

## 1. Job and Audience
- **Visitor Mode:** `Operate`.
- **User Context:** The user wants to inspect how dictation text is cleaned up and formatted in different modes, customize prompt instructions, bind mode hotkeys (`Ctrl+Alt+1..9`), configure spoken prefixes (e.g. "email:"), duplicate an existing mode to create a tailored variation, or set a default mode per application.
- **Primary Need:** Transparent, instant control over dictation modes without hidden LLM magic. Clear visibility into whether cleanup is on or off, prompt instructions, and a live before/after diff preview on a real utterance sample.

## 2. Outcome and Proof
- **Visual Truth First:** The active mode and mode list sit clearly in the view.
- **Proof over Assertion:** Before/after transformation on a real sample shows exactly what cleanup does. No synthetic or invented performance percentages.
- **Honest Persistence:** Changes persist via existing `saveDashboardConfig`. The editor does NOT add a manual Save button (ROADMAP.md).
- **Grounding Guard:** Clear indication of grounding guard behavior (FR-TXT-8: falls back to raw transcript if LLM introduces ungrounded words or excessive length).

## 3. Selected Direction (Quiet Precision)
- **Design Tokens:** Warm charcoal surfaces (`#1c1b1a`, `bg-surface`, `bg-surface-container-low`, `bg-surface-container-high`), saffron accent (`#e08a1e`), Inter typography, JetBrains Mono for hotkeys/prefixes/prompts, libadwaita radii (`rounded-xl`), planar borders (`border border-outline-variant/30`).
- **Hierarchy:**
  1. **Header:** Title "Modes", subtitle "Speech processing modes, cleanup prompts, and shortcuts."
  2. **Active Mode Selector:** Quick active mode card showing the currently selected session mode with a saffron badge.
  3. **Mode List & Actions:**
     - List of built-in modes (`Plain`, `Clean`, `Email`, `Notes`, `Code`, `Bullet`).
     - "Duplicate mode" button on each mode to clone into a user-defined preset.
  4. **Mode Detail / Editor Drawer:**
     - Mode name & description.
     - Cleanup toggle (Quiet Precision switch: `on` / `off`). Off for Plain and Code.
     - Mode hotkey keycap (e.g. `Ctrl+Alt+1`).
     - Spoken prefix input (e.g. `email: `).
     - Cleanup prompt instructions (editable text area with monospace font).
     - Live Before / After sample preview card.

## 4. Scope and Boundaries
- **In Scope for Phase 4:**
  - Built-in modes + user-defined modes.
  - Mode hotkeys and spoken prefixes.
  - Local cleanup prompts and grounding guard thresholds.
  - Per-app default mode mapping.
  - Pure sentence-case copy.
- **Explicit Anti-Goals:**
  - No manual Save button (auto-saves via `saveDashboardConfig`).
  - No remote cloud LLM endpoints (100% local only).
  - No `bolder`, `delight`, or `overdrive`.

## 5. Interaction and Accessibility
- **Focus Rings:** `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container`.
- **Hit Targets:** Minimum 36px interactive heights (`min-h-[36px]`).
- **Live Announcements:** `role="status"` on auto-save notices, `role="alert"` on validation errors.

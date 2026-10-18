# Plan: Mode Editor & Mode Management (Phase 4 Surface 1)

> **Authority:** `docs/ROADMAP.md` Phase 4 ("UI in this phase: mode editor (hotkey, spoken prefix, measured extra delay, before/after on a real sample)... Duplicate a built-in mode; the editor does not add a Save button."), `docs/REQUIREMENTS.md` (FR-ACT-4, FR-ACT-5, FR-TXT-6, FR-TXT-7, Decision #3), `docs/DESIGN.md` §3/§4/§5/§6/§7, `docs/superpowers/specs/2026-10-16-mode-editor-design.md`.

## Goal
Implement Phase 4 Surface 1: Mode Editor and mode management (FR-ACT-4/5, FR-TXT-6/7). Provide a pure mode definition contract with built-in presets (`Plain`, `Clean`, `Email`, `Bullet`, `Code`), mode duplication to create custom presets, cleanup prompt configuration, hotkey & spoken prefix binding, and before/after transformation preview on real sample speech in `dashboard/src/routes/settings/modes/+page.svelte` without a manual Save button.

## Global Constraints
- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Decision #3 (cleanup LLM) is local only; zero cloud dependencies.
- Copy is sentence case: "Active mode", "Duplicate mode", "Cleanup prompts", "Before and after", "Spoken prefix", "Mode hotkey". No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- The editor does NOT add a manual Save button; changes persist through the existing `saveDashboardConfig` pipeline (ROADMAP.md).
- Do not run Impeccable bolder, delight, or overdrive.
- One commit per task, on feature branch `feat/mode-editor`, never push.

---

## Tasks

### Task 1: Mode definition contract and sample transformation module
**Files:**
- Create `dashboard/src/lib/modes.ts`
- Create `dashboard/src/lib/modes.test.ts`

**Brief:**
Define the pure TypeScript mode definition contract, built-in presets, and duplication logic:
- `ModeDefinition`: `{ id: string, name: string, description: string, cleanup: boolean, prompt: string, hotkey: string, spoken_prefix: string, is_builtin: boolean }`.
- `BUILTIN_MODES`: Array containing `plain`, `clean`, `email`, `bullet`, `code` presets with accurate descriptions, prompt templates, hotkeys (`Ctrl+Alt+1..5`), and spoken prefixes.
- `duplicateMode(source: ModeDefinition, newId: string, newName: string): ModeDefinition`: clones a mode into a user-defined preset (`is_builtin: false`).
- `validateMode(mode: ModeDefinition): { valid: boolean, error?: string }`: checks non-empty id, valid slug, non-empty name.
- `previewSampleTransform(rawSample: string, mode: ModeDefinition): { before: string, after: string }`: deterministic before/after preview for sample text.
- Unit tests in `dashboard/src/lib/modes.test.ts`.

**Test & Commit:**
`node --test dashboard/src/lib/modes.test.ts`
`cd dashboard && bun test`
Commit: `feat(dashboard): create mode definitions contract and transformation heuristics`

---

### Task 2: Mode editor UI wiring with duplication and before/after preview
**Files:**
- Modify `dashboard/src/routes/settings/modes/+page.svelte`
- Modify `dashboard/src/lib/modes.test.ts`

**Brief:**
1. In `dashboard/src/routes/settings/modes/+page.svelte`:
   - Replace the bare dropdown with a full Mode Management view:
     - Header: "Modes", subtitle "Speech processing modes, cleanup prompts, and shortcuts."
     - Active mode selection card with saffron "Active" badge.
     - Mode list displaying each mode card (name, description, cleanup pill, hotkey keycap, spoken prefix badge).
     - "Duplicate" button to clone any mode into a custom preset.
     - Selected mode detail drawer/editor:
       - Mode name & description.
       - Cleanup toggle (Quiet Precision switch: `on` / `off`).
       - Mode hotkey keycap / input (`Ctrl+Alt+1..9`).
       - Spoken prefix input (e.g. `email:`).
       - Cleanup prompt textarea (`font-mono`).
       - Before / After live preview card on sample text.
     - Auto-save via `saveDashboardConfig` on change without a manual Save button.
2. In `dashboard/src/lib/modes.test.ts`:
   - Add integration tests verifying mode list, duplication, and UI contract properties.

**Test & Commit:**
`node --test dashboard/src/lib/modes.test.ts`
`cd dashboard && bun test`
Commit: `feat(dashboard): wire mode editor with duplication and before-after sample preview`

---

### Task 3: Impeccable critique and audit pass
**Files:**
- Modify `dashboard/src/routes/settings/modes/+page.svelte`

**Brief:**
Operate mode critique and audit pass on the mode cards, duplicate button, and detail drawer against `docs/DESIGN.md` §6 & §7:
- Keyboard focus rings: `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container`.
- Minimum 36px hit targets (`min-h-[36px]`).
- Disabled states on interactive inputs during save.
- Accessible live announcements: `role="status"` on auto-save, `role="alert"` on error.
- Contrast & typography tokens: Quiet Precision tokens and pure sentence-case copy.

**Test & Commit:**
`node --test dashboard/src/lib/modes.test.ts`
`cd dashboard && bun test`
Commit: `fix(dashboard): apply mode editor critique, action buttons, and focus styling`

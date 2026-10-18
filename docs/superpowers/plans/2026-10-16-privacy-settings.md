# Privacy Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Phase 2 Surface 5 — implement Privacy and retention settings (retention days inputs, pause history switch, retain audio toggle, and clear history with typed confirmation) with validation, Quiet Precision switches, honest daemon data (no costume metrics), and Impeccable styling.

**Architecture:** `docs/REQUIREMENTS.md` (FR-HIS-7, FR-HIS-4, FR-HIS-5, FR-HIS-9, NFR-S-3), `docs/ROADMAP.md` (Phase 2 Surface 5, "How a phase ships"), and `docs/DESIGN.md` §4, §6, §7 are the normative authorities.
Beat 1 creates `dashboard/src/lib/privacy-settings.ts` with pure validation functions for retention days inputs, display formatting, and typed deletion phrase confirmation.
Beat 2 updates `dashboard/src/routes/settings/privacy/+page.svelte` to wire Quiet Precision switches for `retain_audio` and `paused`, wire retention validation, wire the typed confirmation dialog for `delete_all_history`, and remove costume fake metrics/mockups.
Beat 3 runs Impeccable in Operate mode (`distill` → `critique` → `audit`) on the Privacy settings surface for focus rings, hit targets (`min-h-[36px]`), contrast tokens, and status announcements.

**Tech Stack:** Svelte 5, TypeScript, Bun / Node test runner, Tauri commands (`load_config`, `save_config`, `session_status`, `set_paused`, `delete_all_history`).

## Global Constraints

- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Daemon owns state. The dashboard is a thin client. This plan adds no D-Bus method and no new dependency.
- Retention schema note: retention settings in `DashboardConfig` and `config.toml` live under `history` (`DashboardConfig["history"]` with fields `retain_audio: boolean`, `text_retention_days: number | null`, `audio_retention_days: number | null`), mirroring `naradd` and `dashboard/src-tauri/src/config.rs`.
- Copy is sentence case: "Keep audio recordings", "Pause history", "Auto-delete text transcripts", "Auto-delete retained audio", "Clear history…", "Delete all history".
- No forbidden jargon: No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- No costume numbers or mockups: numbers appear only when the daemon returned them (`docs/ROADMAP.md`). Remove hardcoded storage breakdown numbers (2.45 GB, 4%, 14%, 82%, 1,936 entries) and inert export mockup cards.
- Settings that already save do so through the existing config path (`saveDashboardConfig`). No manual Save button.
- Destructive deletion: "Delete all history" requires typed confirmation (`docs/DESIGN.md` §4) matching exact phrase `delete all history` (`deleteAllConfirmed` from `$lib/screen-state`) before calling Tauri command `delete_all_history`.
- Do not run Impeccable `bolder`, `delight`, or `overdrive`.
- One commit per task, on a feature branch, never push.

## File structure

- `dashboard/src/lib/privacy-settings.ts` — validation and formatting for retention days, confirmation predicate for history deletion.
- `dashboard/src/lib/privacy-settings.test.ts` — unit tests for privacy settings validation, formatting, and page assertions.
- `dashboard/src/routes/settings/privacy/+page.svelte` — Privacy settings page with Quiet Precision switches, retention inputs, and clear history confirmation dialog.

---

### Task 1: Privacy settings validation and confirmation contract module

**Files:**
- Create: `dashboard/src/lib/privacy-settings.ts`
- Create: `dashboard/src/lib/privacy-settings.test.ts`
- Test: `node --test dashboard/src/lib/privacy-settings.test.ts`

**Interfaces:**
- Consumes: `DashboardConfig["history"]` shape from `$lib/config`, `deleteAllConfirmed` from `$lib/screen-state`.
- Produces: `parseRetentionDays`, `formatRetentionDays`, `validatePrivacyDraft`, `isDeleteAllConfirmed`, `RETENTION_ERROR`, `DELETE_ALL_CONFIRMATION_PHRASE`.

- [ ] **Step 1: Write the failing test**

Create `dashboard/src/lib/privacy-settings.test.ts`:

```ts
import assert from "node:assert/strict";
import test from "node:test";
import {
  formatRetentionDays,
  isDeleteAllConfirmed,
  parseRetentionDays,
  validatePrivacyDraft,
  RETENTION_ERROR,
} from "./privacy-settings.ts";

test("parseRetentionDays parses whole non-negative numbers and treats blank as keep", () => {
  assert.equal(parseRetentionDays(""), null);
  assert.equal(parseRetentionDays("   "), null);
  assert.equal(parseRetentionDays(null), null);
  assert.equal(parseRetentionDays(undefined), null);
  assert.equal(parseRetentionDays("0"), 0);
  assert.equal(parseRetentionDays("4"), 4);
  assert.equal(parseRetentionDays("  30  "), 30);
  assert.equal(parseRetentionDays(30), 30);
  assert.equal(parseRetentionDays("-1"), undefined);
  assert.equal(parseRetentionDays("2.5"), undefined);
  assert.equal(parseRetentionDays("abc"), undefined);
  assert.equal(parseRetentionDays(-5), undefined);
});

test("formatRetentionDays formats null as blank and numbers as string", () => {
  assert.equal(formatRetentionDays(null), "");
  assert.equal(formatRetentionDays(0), "0");
  assert.equal(formatRetentionDays(30), "30");
});

test("validatePrivacyDraft validates retention days inputs", () => {
  const valid = validatePrivacyDraft({
    textDaysRaw: "30",
    audioDaysRaw: " ",
  });
  assert.equal(valid.ok, true);
  if (valid.ok) {
    assert.equal(valid.textRetentionDays, 30);
    assert.equal(valid.audioRetentionDays, null);
  }

  const invalid = validatePrivacyDraft({
    textDaysRaw: "not-a-number",
    audioDaysRaw: "10",
  });
  assert.equal(invalid.ok, false);
  if (!invalid.ok) {
    assert.equal(invalid.error, RETENTION_ERROR);
  }
});

test("isDeleteAllConfirmed requires exact trimmed phrase", () => {
  assert.equal(isDeleteAllConfirmed("delete all history"), true);
  assert.equal(isDeleteAllConfirmed("  delete all history  "), true);
  assert.equal(isDeleteAllConfirmed("delete all"), false);
  assert.equal(isDeleteAllConfirmed("Delete all history"), false);
  assert.equal(isDeleteAllConfirmed(""), false);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/privacy-settings.test.ts`

Expected: FAIL. `privacy-settings.ts` does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Create `dashboard/src/lib/privacy-settings.ts`:

```ts
import { deleteAllConfirmed } from "./screen-state";

export type ValidatedPrivacyDraft =
  | {
      ok: true;
      textRetentionDays: number | null;
      audioRetentionDays: number | null;
    }
  | {
      ok: false;
      error: string;
    };

export const RETENTION_ERROR =
  "Retention must be a whole number of days, or blank to keep.";

export const DELETE_ALL_CONFIRMATION_PHRASE = "delete all history";

/** Blank or missing means keep forever (null). Non-negative whole numbers return days. Negative/non-numeric returns undefined. */
export function parseRetentionDays(raw: unknown): number | null | undefined {
  if (raw == null) return null;
  if (typeof raw === "string") {
    const trimmed = raw.trim();
    if (trimmed === "") return null;
    if (!/^\d+$/.test(trimmed)) return undefined;
    return Number(trimmed);
  }
  if (typeof raw !== "number" || !Number.isFinite(raw) || raw < 0) {
    return undefined;
  }
  return Math.round(raw);
}

export function formatRetentionDays(value: number | null): string {
  return value == null ? "" : String(value);
}

export function validatePrivacyDraft(draft: {
  textDaysRaw: string;
  audioDaysRaw: string;
}): ValidatedPrivacyDraft {
  const textRetentionDays = parseRetentionDays(draft.textDaysRaw);
  const audioRetentionDays = parseRetentionDays(draft.audioDaysRaw);
  if (textRetentionDays === undefined || audioRetentionDays === undefined) {
    return {
      ok: false,
      error: RETENTION_ERROR,
    };
  }
  return {
    ok: true,
    textRetentionDays,
    audioRetentionDays,
  };
}

export function isDeleteAllConfirmed(phrase: string): boolean {
  return deleteAllConfirmed(phrase);
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/privacy-settings.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests pass).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/lib/privacy-settings.ts dashboard/src/lib/privacy-settings.test.ts
git commit -m "feat(dashboard): create privacy settings validation module"
```

---

### Task 2: Privacy settings UI wiring (switches, inputs, honest copy, typed clear history)

**Files:**
- Modify: `dashboard/src/routes/settings/privacy/+page.svelte`
- Modify: `dashboard/src/lib/privacy-settings.test.ts`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes:
  - `validatePrivacyDraft`, `formatRetentionDays`, `isDeleteAllConfirmed` from `$lib/privacy-settings`.
  - `saveDashboardConfig`, `SAVED_NOTICE` from `$lib/settings-form`.
  - `invoke` for `"load_config"`, `"save_config"`, `"session_status"`, `"set_paused"`, `"delete_all_history"`.
- Produces: Privacy settings page with Quiet Precision switch toggles, retention days inputs with contract validation, typed clear history modal/sheet, and no costume mockups.

- [ ] **Step 1: Write integration test assertions**

In `dashboard/src/lib/privacy-settings.test.ts`, add page source assertions:

```ts
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pageSource = readFileSync(
  join(here, "../routes/settings/privacy/+page.svelte"),
  "utf8"
);

test("privacy page includes switches, delete_all_history, and honest copy", () => {
  // Uses Quiet Precision switch role
  assert.match(pageSource, /role="switch"/);
  // Uses contract validation
  assert.match(pageSource, /validatePrivacyDraft/);
  // Invokes delete_all_history
  assert.match(pageSource, /delete_all_history/);
  // Preserves required contract keys for prepare-config.test.ts
  assert.match(pageSource, /WAV/);
  assert.match(pageSource, /set_paused/);
  assert.match(pageSource, /session_status/);
  assert.match(pageSource, /retain_audio/);
  assert.match(pageSource, /text_retention_days/);
  assert.match(pageSource, /audio_retention_days/);
  assert.match(pageSource, /SAVED_NOTICE/);
  // Drops forbidden jargon and costume copy
  assert.equal(pageSource.includes("telemetry"), false);
  assert.equal(pageSource.includes("acoustic"), false);
  assert.equal(pageSource.includes("2.45 GB"), false);
  assert.equal(pageSource.includes("1,936 entries"), false);
  assert.equal(pageSource.includes("FLAC"), false);
  assert.equal(pageSource.includes("30 days"), false);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/privacy-settings.test.ts`

Expected: FAIL. `privacy/+page.svelte` currently uses checkboxes, contains banned words ("telemetry", "acoustic"), and has fake storage metrics ("2.45 GB").

- [ ] **Step 3: Update `dashboard/src/routes/settings/privacy/+page.svelte`**

1. Import `validatePrivacyDraft`, `formatRetentionDays`, `isDeleteAllConfirmed` from `$lib/privacy-settings`.
2. Clean trust banner copy:
   - Heading: "Everything stays on this computer."
   - Subtitle: "Narad processes all speech locally on your device. Audio never leaves this computer, and models run without network access."
   - Replace jargon tags with clean trust badges ("On-device processing", "No network access", "Standard local storage").
3. Replace raw checkbox inputs for `retain_audio` and `paused` with Quiet Precision switch toggle buttons:
   ```svelte
   <button
     type="button"
     role="switch"
     aria-checked={config.history.retain_audio}
     aria-label="Keep audio recordings"
     disabled={!loaded}
     onclick={() => {
       config.history.retain_audio = !config.history.retain_audio;
       commit();
     }}
     class="w-11 h-6 rounded-full transition-colors relative flex items-center p-0.5 border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 {config.history.retain_audio ? 'bg-primary' : 'bg-surface-container-highest'}"
   >
     <span
       class="w-5 h-5 rounded-full bg-surface-bright shadow-sm transform transition-transform {config.history.retain_audio ? 'translate-x-5' : 'translate-x-0'}"
     ></span>
   </button>
   ```
   Do the same for `paused`:
   ```svelte
   <button
     type="button"
     role="switch"
     aria-checked={paused === true}
     aria-label="Pause history"
     disabled={paused == null || pauseBusy}
     onclick={() => void setPaused(!paused)}
     class="w-11 h-6 rounded-full transition-colors relative flex items-center p-0.5 border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 {paused ? 'bg-primary' : 'bg-surface-container-highest'}"
   >
     <span
       class="w-5 h-5 rounded-full bg-surface-bright shadow-sm transform transition-transform {paused ? 'translate-x-5' : 'translate-x-0'}"
     ></span>
   </button>
   ```
4. Wire `validatePrivacyDraft` in `commit()`:
   ```ts
   function commit() {
     if (!loaded) return;
     const validated = validatePrivacyDraft({
       textDaysRaw: textDays,
       audioDaysRaw: audioDays,
     });
     if (!validated.ok) {
       error = validated.error;
       notice = null;
       return;
     }
     const toSave: DashboardConfig = {
       ...config,
       history: {
         retain_audio: config.history.retain_audio,
         text_retention_days: validated.textRetentionDays,
         audio_retention_days: validated.audioRetentionDays,
       },
     };
     error = null;
     saveChain = saveChain.then(
       () => persist(toSave),
       () => persist(toSave),
     );
   }
   ```
5. Remove fake "Local Storage Breakdown" progress bar and "Data Portability" mockup cards.
6. Replace with an honest "Clear history" section:
   - State variables: `let showClearDialog = $state(false);`, `let clearConfirm = $state("");`, `let clearBusy = $state(false);`
   - Trigger button: "Clear history…" opens the confirmation dialog.
   - Confirmation dialog (`role="dialog"`, `aria-modal="true"`, `aria-labelledby="clear-dialog-title"`):
     - Title: "Clear all history"
     - Description: "This will permanently delete all transcription entries and stored audio files. To confirm, type delete all history below."
     - Confirmation input: `<input type="text" autocomplete="off" spellcheck="false" placeholder="delete all history" bind:value={clearConfirm} />`
     - Action buttons:
       - Cancel button: closes dialog, clears `clearConfirm`.
       - Delete button: "Delete all history", `disabled={!isDeleteAllConfirmed(clearConfirm) || clearBusy}`, onclick calls `handleDeleteAll()`.
   - `handleDeleteAll()`:
     ```ts
     async function handleDeleteAll() {
       if (!isDeleteAllConfirmed(clearConfirm) || clearBusy) return;
       clearBusy = true;
       error = null;
       try {
         await invoke("delete_all_history");
         showClearDialog = false;
         clearConfirm = "";
         notice = "All history cleared.";
       } catch (caught) {
         error = caught instanceof Error ? caught.message : String(caught);
       } finally {
         clearBusy = false;
       }
     }
     ```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/privacy-settings.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests pass, including `prepare-config.test.ts` and `shell-honesty.test.ts`).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/routes/settings/privacy/+page.svelte dashboard/src/lib/privacy-settings.test.ts
git commit -m "feat(dashboard): wire privacy settings switches and history deletion"
```

---

### Task 3: Impeccable critique and audit of the Privacy settings surface

**Files:**
- Modify: `dashboard/src/routes/settings/privacy/+page.svelte`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes: Privacy settings page markup.
- Produces: High-contrast focus rings, accessible hit targets (`min-h-[36px]`), ARIA roles and labels, Quiet Precision design token styling.

- [ ] **Step 1: Baseline verification**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 2: Run Critique and Audit in Operate mode on the Privacy surface**

Review against `docs/DESIGN.md` §6 & §7:
- Focus indicators: Ensure all interactive inputs, switch buttons, and action buttons have explicit `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container`.
- Hit targets: Minimum 36px interactive heights (`min-h-[36px]`) on inputs (`text-retention-days`, `audio-retention-days`, dialog confirmation input) and buttons.
- Contrast and tokens: `border border-outline-variant/30`, `bg-surface-container-high`, `bg-surface-container-low`, `text-on-surface`, `text-on-surface-variant`.
- Status announcements: `role="status"` on notice, `role="alert"` on error.
- Keyboard accessibility: Pressing `Escape` while the clear history dialog is open dismisses the dialog.

- [ ] **Step 3: Apply surviving defects**

Apply focus rings, `min-h-[36px]`, escape key listener, and token classes to `dashboard/src/routes/settings/privacy/+page.svelte`.

- [ ] **Step 4: Verify tests pass**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/routes/settings/privacy/+page.svelte
git commit -m "fix(dashboard): apply privacy settings critique and focus styling"
```

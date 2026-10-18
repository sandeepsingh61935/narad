# Audio Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Phase 2 Surface 4 — implement Audio settings (sound cues toggle, noise suppression switch, silence stop, max duration cap, and microphone target) with validation, Quiet Precision switches, and Impeccable styling.

**Architecture:** `docs/REQUIREMENTS.md` (FR-AUD-3, FR-AUD-4, FR-AUD-5, FR-AUD-6, FR-HIS-9), `docs/ROADMAP.md` (Phase 2 Surface 4, "How a phase ships"), and `docs/DESIGN.md` §6/§7 are the normative authorities.
Beat 1 creates `dashboard/src/lib/audio-settings.ts` with pure validation functions for audio configuration.
Beat 2 updates `dashboard/src/routes/settings/audio/+page.svelte` to include sound cues toggle, Quiet Precision switch components for boolean settings, and live auto-persisting.
Beat 3 runs Impeccable in Operate mode (`distill` → `critique` → `audit`) on the Audio settings surface for focus rings, hit targets (`min-h-[36px]`), and token fidelity.

**Tech Stack:** Svelte 5, TypeScript, Bun / Node test runner.

## Global Constraints

- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Daemon owns state. The dashboard is a thin client. This plan adds no D-Bus method and no dependency.
- Copy is sentence case: "Sound cues", "Noise suppression", "Microphone", "Silence stop", "Maximum duration". No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- Settings that already save do so through the existing config path (`saveDashboardConfig`). This plan does not add a manual Save button.
- Do not run Impeccable `bolder`, `delight`, or `overdrive`.
- One commit per task, on a feature branch, never push.

## File structure

- `dashboard/src/lib/audio-settings.ts` — validation and normalization for audio seconds and target strings.
- `dashboard/src/lib/audio-settings.test.ts` — unit tests for audio settings validation.
- `dashboard/src/routes/settings/audio/+page.svelte` — Audio settings page with sound cues, switches, inputs, and validation feedback.

---

### Task 1: Audio settings validation contract module

**Files:**
- Create: `dashboard/src/lib/audio-settings.ts`
- Create: `dashboard/src/lib/audio-settings.test.ts`
- Test: `dashboard/src/lib/audio-settings.test.ts`

**Interfaces:**
- Consumes: `DashboardConfig["audio"]` shape from `$lib/config`.
- Produces: `parseAudioSeconds`, `validateAudioConfig`.

- [ ] **Step 1: Write the failing test**

Create `dashboard/src/lib/audio-settings.test.ts`:

```ts
import assert from "node:assert/strict";
import test from "node:test";
import { parseAudioSeconds, validateAudioConfig } from "./audio-settings.ts";

test("parseAudioSeconds parses non-negative integers and allows 0 for off", () => {
  assert.equal(parseAudioSeconds("2"), 2);
  assert.equal(parseAudioSeconds("300"), 300);
  assert.equal(parseAudioSeconds("0"), 0);
  assert.equal(parseAudioSeconds("  15  "), 15);
  assert.equal(parseAudioSeconds(""), 0);
  assert.equal(parseAudioSeconds("-1"), undefined);
  assert.equal(parseAudioSeconds("2.5"), undefined);
  assert.equal(parseAudioSeconds("abc"), undefined);
});

test("validateAudioConfig validates complete audio draft", () => {
  const valid = validateAudioConfig({
    silenceRaw: "2",
    capRaw: "300",
    inputTarget: " alsa_input.pci ",
  });
  assert.equal(valid.ok, true);
  if (valid.ok) {
    assert.equal(valid.silenceSecs, 2);
    assert.equal(valid.capSecs, 300);
    assert.equal(valid.inputTarget, "alsa_input.pci");
  }

  const invalid = validateAudioConfig({
    silenceRaw: "not-a-number",
    capRaw: "300",
    inputTarget: "",
  });
  assert.equal(invalid.ok, false);
  if (!invalid.ok) {
    assert.match(invalid.error, /whole seconds/i);
  }
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/audio-settings.test.ts`

Expected: FAIL. `audio-settings.ts` does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Create `dashboard/src/lib/audio-settings.ts`:

```ts
export type ValidatedAudioConfig =
  | {
      ok: true;
      silenceSecs: number;
      capSecs: number;
      inputTarget: string;
    }
  | {
      ok: false;
      error: string;
    };

export function parseAudioSeconds(raw: unknown): number | undefined {
  if (raw == null) return 0;
  const str = String(raw).trim();
  if (str === "") return 0;
  if (!/^\d+$/.test(str)) return undefined;
  return Number(str);
}

export function validateAudioConfig(draft: {
  silenceRaw: string;
  capRaw: string;
  inputTarget: string;
}): ValidatedAudioConfig {
  const silenceSecs = parseAudioSeconds(draft.silenceRaw);
  const capSecs = parseAudioSeconds(draft.capRaw);
  if (silenceSecs === undefined || capSecs === undefined) {
    return {
      ok: false,
      error: "Silence stop and maximum duration must be whole seconds. 0 turns that limit off.",
    };
  }
  return {
    ok: true,
    silenceSecs,
    capSecs,
    inputTarget: draft.inputTarget.trim(),
  };
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/audio-settings.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests pass).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/lib/audio-settings.ts dashboard/src/lib/audio-settings.test.ts
git commit -m "feat(dashboard): create audio settings validation module"
```

---

### Task 2: Audio settings controls (Sound cues, Quiet Precision switches, inputs)

**Files:**
- Modify: `dashboard/src/routes/settings/audio/+page.svelte`
- Modify: `dashboard/src/lib/audio-settings.test.ts`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes: `validateAudioConfig` from `$lib/audio-settings`, `saveDashboardConfig` from `$lib/settings-form`.
- Produces: Audio settings page with sound cues toggle, Quiet Precision switches, and live persistence.

- [ ] **Step 1: Write integration test assertions**

In `dashboard/src/lib/audio-settings.test.ts`, add page source assertions:

```ts
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pageSource = readFileSync(join(here, "../routes/settings/audio/+page.svelte"), "utf8");

test("audio page includes sound cues toggle and Quiet Precision switches", () => {
  assert.match(pageSource, /Sound cues/);
  assert.match(pageSource, /sounds_enabled/);
  assert.match(pageSource, /validateAudioConfig/);
  assert.match(pageSource, /role="switch"/);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/audio-settings.test.ts`

Expected: FAIL. `audio/+page.svelte` does not yet contain `Sound cues` or `validateAudioConfig`.

- [ ] **Step 3: Update `dashboard/src/routes/settings/audio/+page.svelte`**

1. Import `validateAudioConfig` from `$lib/audio-settings`.
2. Add `sounds_enabled` row with Quiet Precision switch toggle button (`role="switch"`, `aria-checked`).
3. Replace raw checkbox on `noise_suppression` with Quiet Precision switch toggle button.
4. In `commit()`:
   Use `validateAudioConfig`:
   ```ts
   const validated = validateAudioConfig({
     silenceRaw: silence,
     capRaw: cap,
     inputTarget: config.audio.input_target,
   });
   if (!validated.ok) {
     error = validated.error;
     notice = null;
     return;
   }
   const toSave: DashboardConfig = {
     ...config,
     audio: {
       ...config.audio,
       auto_stop_silence_secs: validated.silenceSecs,
       max_duration_secs: validated.capSecs,
       input_target: validated.inputTarget,
     },
   };
   ```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/audio-settings.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests pass).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/routes/settings/audio/+page.svelte dashboard/src/lib/audio-settings.test.ts
git commit -m "feat(dashboard): add sound cues and accessible switches to audio settings"
```

---

### Task 3: Impeccable critique and audit of the Audio settings surface

**Files:**
- Modify: `dashboard/src/routes/settings/audio/+page.svelte`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes: Audio settings page markup.
- Produces: High-contrast focus rings, accessible hit targets (`min-h-[36px]`), ARIA roles and labels, Quiet Precision design token styling.

- [ ] **Step 1: Baseline verification**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 2: Run Critique and Audit in Operate mode on the Audio surface**

Review against `docs/DESIGN.md` §6 & §7:
- Focus indicators: Ensure all interactive inputs and switch buttons have explicit `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container`.
- Hit targets: Minimum 36px interactive heights (`min-h-[36px]`) on inputs.
- Contrast and tokens: `border-outline-variant/30`, `bg-surface-container-high` or `bg-surface-container`.
- Status announcements: `role="status"` on save notice, `role="alert"` on validation error.

- [ ] **Step 3: Apply surviving defects**

Apply focus rings and `min-h-[36px]` styling to `dashboard/src/routes/settings/audio/+page.svelte`.

- [ ] **Step 4: Verify tests pass**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/routes/settings/audio/+page.svelte
git commit -m "fix(dashboard): apply audio settings critique and focus styling"
```

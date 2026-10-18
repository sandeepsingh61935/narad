# Dictionary Settings Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Phase 2 Surface 3 — implement the Dictionary settings surface (vocabulary & replacements) under the Settings layout (`/settings/dictionary`) with pair validation, sentence-case copy, and Impeccable Quiet Precision styling.

**Architecture:** `docs/REQUIREMENTS.md` (FR-TXT-2, FR-TXT-3, FR-HIS-9), `docs/ROADMAP.md` (Phase 2 Surface 3, "How a phase ships"), and `docs/DESIGN.md` §6/§7 are the normative authorities.
Beat 1 creates `dashboard/src/lib/dictionary.ts` with helper and validation functions for vocabulary and replacement pairs.
Beat 2 moves/routes Dictionary under `/settings/dictionary` inside the Settings layout with live config loading/saving via `saveDashboardConfig` and sentence-case controls.
Beat 3 runs Impeccable in Operate mode (`distill` → `critique` → `audit`) on the Dictionary surface for focus rings, hit targets (`min-h-[36px]`), and token fidelity.

**Tech Stack:** Svelte 5, TypeScript, Bun / Node test runner.

## Global Constraints

- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Daemon owns state. The dashboard is a thin client. This plan adds no D-Bus method and no dependency.
- Copy is sentence case: "Add vocabulary", "Add replacement", "Remove", "Spoken", "Written". No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- Settings that already save do so through the existing config path (`saveDashboardConfig`). This plan does not add a manual Save button.
- Do not run Impeccable `bolder`, `delight`, or `overdrive`.
- One commit per task, on a feature branch, never push.

## File structure

- `dashboard/src/lib/dictionary.ts` — validation, normalization, and merge helpers for vocabulary and replacement pairs.
- `dashboard/src/lib/dictionary.test.ts` — unit tests for dictionary validation and merge helpers.
- `dashboard/src/routes/settings/dictionary/+page.svelte` — Dictionary settings page under the Settings layout.
- `dashboard/src/routes/settings/+layout.svelte` — updates Dictionary href to `/settings/dictionary`.
- `dashboard/src/routes/dictionary/+page.svelte` — maintains backward compatibility for tests and redirects.

---

### Task 1: Dictionary contract module and pair validation

**Files:**
- Create: `dashboard/src/lib/dictionary.ts`
- Create: `dashboard/src/lib/dictionary.test.ts`
- Test: `dashboard/src/lib/dictionary.test.ts`

**Interfaces:**
- Consumes: `type Replacement` from `$lib/config`.
- Produces: `cleanDictionaryPairs`, `findDuplicateSpoken`, and `validateDictionaryPair`.

- [ ] **Step 1: Write the failing test**

Create `dashboard/src/lib/dictionary.test.ts`:

```ts
import assert from "node:assert/strict";
import test from "node:test";
import {
  cleanDictionaryPairs,
  findDuplicateSpoken,
  validateDictionaryPair,
} from "./dictionary.ts";

test("cleanDictionaryPairs trims phrases and drops blank spoken phrases", () => {
  const input = [
    { spoken: "  my email  ", written: " me@example.com " },
    { spoken: "   ", written: "ignored" },
    { spoken: "kubernetes", written: "k8s" },
  ];
  const cleaned = cleanDictionaryPairs(input);
  assert.deepEqual(cleaned, [
    { spoken: "my email", written: "me@example.com" },
    { spoken: "kubernetes", written: "k8s" },
  ]);
});

test("findDuplicateSpoken detects case-insensitive duplicates", () => {
  const pairs = [
    { spoken: "Kubernetes", written: "K8s" },
    { spoken: "docker", written: "Docker" },
    { spoken: "kubernetes", written: "k8s" },
  ];
  const dupes = findDuplicateSpoken(pairs);
  assert.equal(dupes.has("kubernetes"), true);
  assert.equal(dupes.has("docker"), false);
});

test("validateDictionaryPair validates single pair", () => {
  assert.equal(validateDictionaryPair({ spoken: "", written: "test" }).valid, false);
  assert.equal(validateDictionaryPair({ spoken: "hello", written: "world" }).valid, true);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/dictionary.test.ts`

Expected: FAIL. `dictionary.ts` does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Create `dashboard/src/lib/dictionary.ts`:

```ts
import type { Replacement } from "./config";

export type PairValidation = {
  valid: boolean;
  error?: string;
};

export function cleanDictionaryPairs(pairs: readonly Replacement[]): Replacement[] {
  return pairs
    .map((pair) => ({
      spoken: pair.spoken.trim(),
      written: pair.written.trim(),
    }))
    .filter((pair) => pair.spoken.length > 0);
}

export function findDuplicateSpoken(pairs: readonly Replacement[]): Set<string> {
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const pair of pairs) {
    const key = pair.spoken.trim().toLowerCase();
    if (key.length === 0) continue;
    if (seen.has(key)) {
      duplicates.add(key);
    } else {
      seen.add(key);
    }
  }
  return duplicates;
}

export function validateDictionaryPair(pair: Replacement): PairValidation {
  const spoken = pair.spoken.trim();
  if (spoken.length === 0) {
    return { valid: false, error: "Spoken phrase cannot be empty." };
  }
  return { valid: true };
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/dictionary.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests green).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/lib/dictionary.ts dashboard/src/lib/dictionary.test.ts
git commit -m "feat(dashboard): create dictionary contract module"
```

---

### Task 2: Route Dictionary under Settings layout with sentence-case controls

**Files:**
- Create: `dashboard/src/routes/settings/dictionary/+page.svelte`
- Modify: `dashboard/src/routes/settings/+layout.svelte`
- Modify: `dashboard/src/routes/dictionary/+page.svelte`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes: `cleanDictionaryPairs`, `validateDictionaryPair` from `$lib/dictionary`, `saveDashboardConfig` from `$lib/settings-form`.
- Produces: Integrated Dictionary settings page rendered inside the Settings shell.

- [ ] **Step 1: Write integration test assertions**

In `dashboard/src/lib/dictionary.test.ts`, add page source assertions:

```ts
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

test("dictionary exists in settings layout and has sentence case placeholders", () => {
  const layout = readFileSync(join(here, "../routes/settings/+layout.svelte"), "utf8");
  assert.match(layout, /href:\s*["']\/settings\/dictionary["']/);

  const settingsDictPath = join(here, "../routes/settings/dictionary/+page.svelte");
  assert.equal(existsSync(settingsDictPath), true);
  const settingsDict = readFileSync(settingsDictPath, "utf8");
  assert.match(settingsDict, /placeholder="Spoken"/);
  assert.match(settingsDict, /placeholder="Written"/);
  assert.match(settingsDict, /Add vocabulary/);
  assert.match(settingsDict, /Add replacement/);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/dictionary.test.ts`

Expected: FAIL. `settings/dictionary/+page.svelte` does not exist yet.

- [ ] **Step 3: Implement settings dictionary and update navigation**

1. Create `dashboard/src/routes/settings/dictionary/+page.svelte`:
   - Load config via `load_config`.
   - On blur or removal, automatically persist via `saveDashboardConfig`.
   - Use sentence case placeholders (`placeholder="Spoken"`, `placeholder="Written"`).
   - Verb buttons: `Add vocabulary`, `Add replacement`, `Remove`.
   - Empty states: `No vocabulary yet.`, `No replacements yet.`.

2. Update `dashboard/src/routes/settings/+layout.svelte`:
   Update item href:
   `{ label: "Dictionary", icon: "spellcheck", href: "/settings/dictionary" }`

3. Update `dashboard/src/routes/dictionary/+page.svelte`:
   Ensure it imports or aligns with sentence-case controls so existing tests in `prepare-config.test.ts` continue to pass cleanly.

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/dictionary.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests pass).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/routes/settings/dictionary/+page.svelte dashboard/src/routes/settings/+layout.svelte dashboard/src/routes/dictionary/+page.svelte dashboard/src/lib/dictionary.test.ts
git commit -m "feat(dashboard): route dictionary under settings with sentence-case controls"
```

---

### Task 3: Impeccable critique and audit of the Dictionary surface

**Files:**
- Modify: `dashboard/src/routes/settings/dictionary/+page.svelte`
- Modify: `dashboard/src/routes/dictionary/+page.svelte`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes: Dictionary page markup.
- Produces: High-contrast focus rings, accessible hit targets (`min-h-[36px]`), ARIA roles and labels, Quiet Precision design token styling.

- [ ] **Step 1: Baseline verification**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 2: Run Critique and Audit in Operate mode on the Dictionary surface**

Review against `docs/DESIGN.md` §6 & §7:
- Focus indicators: Ensure all interactive inputs and buttons have explicit `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container`.
- Hit targets: Minimum 36px interactive heights (`min-h-[36px]`) on buttons and inputs.
- Screen reader accessibility: Accessible labels (`aria-label`) on all inputs and removal buttons.
- Save status: Notification alert/status with `role="status"` on auto-save (`SAVED_NOTICE`).

- [ ] **Step 3: Apply surviving defects**

Add focus-visible rings and `min-h-[36px]` classes to the inputs and action buttons in `dashboard/src/routes/settings/dictionary/+page.svelte` and `dashboard/src/routes/dictionary/+page.svelte`.

- [ ] **Step 4: Verify tests pass**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/routes/settings/dictionary/+page.svelte dashboard/src/routes/dictionary/+page.svelte
git commit -m "fix(dashboard): apply dictionary critique and focus styling"
```

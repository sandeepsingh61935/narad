# Model Catalog Implementation Plan (Phase 3 Surface 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Phase 3 Surface 1 — implement the Model Catalog & Management surface under the Settings layout (`/settings/models`) with model pack metadata, pinned checksums, active model selection, and Quiet Precision styling.

**Architecture:** `docs/ROADMAP.md` (Phase 3), `docs/REQUIREMENTS.md` (FR-MDL-1, FR-MDL-2, FR-MDL-6, NFR-S-1, NFR-S-5), `docs/superpowers/specs/2026-10-16-model-catalog-design.md`, and `docs/DESIGN.md` §3/§5/§6/§7 are the normative authorities.
Beat 1 creates `dashboard/src/lib/models.ts` with pure catalog metadata, pinned checksums, and active model validation.
Beat 2 routes Models under the Settings layout (`dashboard/src/routes/settings/models/+page.svelte`) and updates `dashboard/src/routes/settings/+layout.svelte` navigation.
Beat 3 runs Impeccable in Operate mode (`distill` → `critique` → `audit`) on the Models settings surface for focus rings, hit targets (`min-h-[36px]`), contrast tokens, and accessibility.

**Tech Stack:** Svelte 5, TypeScript, Bun / Node test runner.

## Global Constraints

- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Daemon owns state. The dashboard is a thin client. This plan adds no D-Bus method and no dependency.
- Copy is sentence case: "Active model", "Installed models", "Available models", "Set active", "Speech recognition". No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- Downloads are the only network operation; no network code in daemon (NFR-S-1).
- Pinned SHA-256 checksums for all catalog packs (NFR-S-5).
- Settings that already save do so through the existing config path (`saveDashboardConfig`). This plan does not add a manual Save button.
- Do not run Impeccable `bolder`, `delight`, or `overdrive`.
- One commit per task, on a feature branch, never push.

## File structure

- `dashboard/src/lib/models.ts` — model catalog metadata, pinned SHA-256 checksums, and model selection helpers.
- `dashboard/src/lib/models.test.ts` — unit tests for catalog structure, active model verification, and settings navigation.
- `dashboard/src/routes/settings/models/+page.svelte` — Models settings page with active model banner, installed list, and catalog cards.
- `dashboard/src/routes/settings/+layout.svelte` — Settings layout with navigation linking `/settings/models`.

---

### Task 1: Model catalog contract module and pinned metadata

**Files:**
- Create: `dashboard/src/lib/models.ts`
- Create: `dashboard/src/lib/models.test.ts`
- Test: `node --test dashboard/src/lib/models.test.ts`

**Interfaces:**
- Consumes: `DashboardConfig["models"]` shape from `$lib/config`.
- Produces: `MODEL_CATALOG`, `DEFAULT_SPEECH_MODEL_ID`, `type ModelPack`, `isModelActive`, `findModelById`.

- [ ] **Step 1: Write the failing test**

Create `dashboard/src/lib/models.test.ts`:

```ts
import assert from "node:assert/strict";
import test from "node:test";
import {
  DEFAULT_SPEECH_MODEL_ID,
  MODEL_CATALOG,
  findModelById,
  isModelActive,
} from "./models.ts";

test("catalog defines default speech pack matching Decision #1", () => {
  assert.equal(DEFAULT_SPEECH_MODEL_ID, "parakeet-ctc-110m-int8");
  const defaultPack = findModelById(DEFAULT_SPEECH_MODEL_ID);
  assert.ok(defaultPack);
  assert.equal(defaultPack.type, "asr");
  assert.equal(defaultPack.isDefault, true);
  assert.equal(typeof defaultPack.sizeMb, "number");
  assert.match(defaultPack.sha256, /^[a-f0-9]{64}$/);
});

test("catalog includes expected Phase 3 ASR models with pinned checksums", () => {
  const ids = MODEL_CATALOG.map((m) => m.id);
  assert.ok(ids.includes("parakeet-ctc-110m-int8"));
  assert.ok(ids.includes("parakeet-tdt-0.6b-int8"));
  assert.ok(ids.includes("moonshine-base-en-int8"));
  assert.ok(ids.includes("whisper-small-en"));

  for (const pack of MODEL_CATALOG) {
    assert.match(pack.id, /^[a-z0-9.-]+$/);
    assert.match(pack.name, /\S+/);
    assert.match(pack.sha256, /^[a-f0-9]{64}$/);
  }
});

test("isModelActive correctly compares with config", () => {
  const config = {
    models: {
      asr_english: "parakeet-ctc-110m-int8",
    },
  };
  assert.equal(isModelActive("parakeet-ctc-110m-int8", config), true);
  assert.equal(isModelActive("whisper-small-en", config), false);
  assert.equal(isModelActive("", config), false);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/models.test.ts`

Expected: FAIL. `models.ts` does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Create `dashboard/src/lib/models.ts`:

```ts
export type ModelType = "asr" | "mt" | "llm" | "kws";

export type ModelPack = {
  id: string;
  name: string;
  description: string;
  type: ModelType;
  languages: string[];
  sizeMb: number;
  sha256: string;
  isDefault?: boolean;
};

export const DEFAULT_SPEECH_MODEL_ID = "parakeet-ctc-110m-int8";

export const MODEL_CATALOG: ModelPack[] = [
  {
    id: "parakeet-ctc-110m-int8",
    name: "Parakeet CTC 110M int8",
    description: "Fast, low-latency English speech recognition. Lowest CPU usage.",
    type: "asr",
    languages: ["en"],
    sizeMb: 126,
    sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    isDefault: true,
  },
  {
    id: "parakeet-tdt-0.6b-int8",
    name: "Parakeet TDT 0.6B int8",
    description: "High-accuracy English speech recognition with transducer decoding.",
    type: "asr",
    languages: ["en"],
    sizeMb: 640,
    sha256: "d544062145da5776dcfd7589d38c642eef72535084920fcad4e39ec0ef6e36d4",
  },
  {
    id: "moonshine-base-en-int8",
    name: "Moonshine Base en int8",
    description: "Encoder-decoder speech model optimized for variable-length speech.",
    type: "asr",
    languages: ["en"],
    sizeMb: 180,
    sha256: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9",
  },
  {
    id: "whisper-small-en",
    name: "Whisper Small en",
    description: "Robust OpenAI Whisper architecture for English transcription.",
    type: "asr",
    languages: ["en"],
    sizeMb: 480,
    sha256: "722f4625be25633a65096ab6cfd3fc6b13b41bf3735165971a8fb18037ca0f80",
  },
];

export function findModelById(id: string): ModelPack | undefined {
  return MODEL_CATALOG.find((m) => m.id === id);
}

export function isModelActive(
  modelId: string,
  config: { models?: { asr_english?: string } } | null | undefined
): boolean {
  if (!modelId || !config?.models?.asr_english) return false;
  return modelId === config.models.asr_english;
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/models.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests pass).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/lib/models.ts dashboard/src/lib/models.test.ts
git commit -m "feat(dashboard): create model catalog contract module"
```

---

### Task 2: Models page and Settings layout navigation

**Files:**
- Create: `dashboard/src/routes/settings/models/+page.svelte`
- Modify: `dashboard/src/routes/settings/+layout.svelte`
- Modify: `dashboard/src/lib/models.test.ts`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes: `MODEL_CATALOG`, `DEFAULT_SPEECH_MODEL_ID`, `isModelActive` from `$lib/models`; `saveDashboardConfig`, `SAVED_NOTICE` from `$lib/settings-form`.
- Produces: Models page at `/settings/models`, linking from `/settings/+layout.svelte`.

- [ ] **Step 1: Write integration test assertions**

In `dashboard/src/lib/models.test.ts`, add page and layout assertions:

```ts
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const layoutSource = readFileSync(
  join(here, "../routes/settings/+layout.svelte"),
  "utf8"
);
const pageSource = readFileSync(
  join(here, "../routes/settings/models/+page.svelte"),
  "utf8"
);

test("settings layout links to /settings/models", () => {
  assert.match(layoutSource, /\/settings\/models/);
  assert.match(layoutSource, /Models/);
});

test("models page lists active model, catalog, and sentence case copy", () => {
  assert.match(pageSource, /Active model/);
  assert.match(pageSource, /MODEL_CATALOG/);
  assert.match(pageSource, /saveDashboardConfig/);
  assert.match(pageSource, /Set active/);
  // Zero forbidden jargon
  assert.equal(pageSource.includes("telemetry"), false);
  assert.equal(pageSource.includes("acoustic"), false);
  assert.equal(pageSource.includes("CUDA"), false);
  assert.equal(pageSource.includes("TensorRT"), false);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/models.test.ts`

Expected: FAIL. `/settings/models/+page.svelte` does not exist yet.

- [ ] **Step 3: Update `dashboard/src/routes/settings/+layout.svelte`**

In `dashboard/src/routes/settings/+layout.svelte`, add Models navigation tab:

```svelte
<script lang="ts">
  import { page } from "$app/state";
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();

  const nav = [
    { label: "Shortcuts", icon: "keyboard", href: "/settings/hotkeys" },
    { label: "Audio", icon: "mic", href: "/settings/audio" },
    { label: "Models", icon: "memory", href: "/settings/models" },
    { label: "Dictionary", icon: "spellcheck", href: "/settings/dictionary" },
    { label: "Modes", icon: "tune", href: "/settings/modes" },
    { label: "Privacy", icon: "shield", href: "/settings/privacy" },
  ];
</script>
```

- [ ] **Step 4: Create `dashboard/src/routes/settings/models/+page.svelte`**

Create `dashboard/src/routes/settings/models/+page.svelte` with:
- Active model card: Current active model ID and name, saffron "Active" badge, info note.
- Honest network notice: "Model downloads are the only network operation in Narad. All speech processing runs 100% locally on your device."
- Catalog list of speech recognition packs:
  - Card for each pack from `MODEL_CATALOG`.
  - Pack name, size in MB, language badge (`EN`), description.
  - If active: saffron "Active" pill indicator.
  - If not active: "Set active" button that updates `config.models.asr_english` and calls `saveDashboardConfig`.
- Live status announcements: `role="status"` on save notice, `role="alert"` on error.

- [ ] **Step 5: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/models.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests pass).

- [ ] **Step 6: Commit changes**

Run:
```bash
git add dashboard/src/routes/settings/models/+page.svelte dashboard/src/routes/settings/+layout.svelte dashboard/src/lib/models.test.ts
git commit -m "feat(dashboard): route models under settings with active selection"
```

---

### Task 3: Impeccable critique and audit of the Models settings surface

**Files:**
- Modify: `dashboard/src/routes/settings/models/+page.svelte`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes: Models settings page markup.
- Produces: High-contrast focus rings, accessible hit targets (`min-h-[36px]`), ARIA roles and labels, Quiet Precision design token styling.

- [ ] **Step 1: Baseline verification**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 2: Run Critique and Audit in Operate mode on the Models surface**

Review against `docs/DESIGN.md` §6 & §7 and `docs/superpowers/specs/2026-10-16-model-catalog-design.md`:
- Focus indicators: Ensure all interactive buttons have explicit `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container`.
- Hit targets: Minimum 36px interactive heights (`min-h-[36px]`) on buttons.
- Contrast and tokens: `border border-outline-variant/30`, `bg-surface-container-high` or `bg-surface-container-low`, `text-on-surface`, `text-on-surface-variant`.
- Accessible status announcements: `role="status"` on save notice, `role="alert"` on error.
- Semantic card landmarks and lists: `role="list"` and `role="listitem"` on catalog cards.

- [ ] **Step 3: Apply surviving defects**

Apply focus rings, `min-h-[36px]`, tokens, and ARIA roles to `dashboard/src/routes/settings/models/+page.svelte`.

- [ ] **Step 4: Verify tests pass**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/routes/settings/models/+page.svelte
git commit -m "fix(dashboard): apply models settings critique and focus styling"
```

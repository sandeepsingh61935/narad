# History Search Contract Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Phase 2 Surface 2 — implement the approved search contract for History (date, mode, app, language) in the dashboard with strict validation, mode selector, and Impeccable UX polish against Quiet Precision.

**Architecture:** `docs/REQUIREMENTS.md` (FR-HIS-2, FR-HIS-3), `docs/ROADMAP.md` (Phase 2 Surface 2, "How a phase ships"), and `docs/DESIGN.md` §3/§5 are the authorities.
Beat 1 creates a pure `history-search.ts` contract module with tests for filter normalization, date validation, mode filtering, and filter state.
Beat 2 wires `prepareHistoryFilter` into `dashboard/src/routes/+page.svelte`, upgrading the mode input to a mode selector (`plain`, `email`, `bullet`, `code`), and handling filter errors/reversed ranges cleanly.
Beat 3 runs Impeccable in Operate mode (`distill` → `critique` → `audit`) on the History search & filter controls.

**Tech Stack:** Svelte 5 dashboard, TypeScript, Bun / Node test runner.

## Global Constraints

- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Daemon owns state. The dashboard is a thin client. This plan adds no D-Bus method and no dependency.
- Search filters: `from_day` (YYYY-MM-DD), `to_day` (YYYY-MM-DD), `mode` (`plain`, `email`, `bullet`, `code`, or all), `app_id`, `src_lang`.
- Copy is sentence case. No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- Do not run Impeccable `bolder`, `delight`, or `overdrive`.
- One commit per task, on a feature branch, never push.

## File structure

- `dashboard/src/lib/history-search.ts` — pure filter contract, validation, normalization, and cache key generation.
- `dashboard/src/lib/history-search.test.ts` — unit tests for filter contract and validation.
- `dashboard/src/routes/+page.svelte` — History page search and filter bar using the contract and mode selector.

---

### Task 1: History search contract module

**Files:**
- Create: `dashboard/src/lib/history-search.ts`
- Create: `dashboard/src/lib/history-search.test.ts`
- Test: `dashboard/src/lib/history-search.test.ts`

**Interfaces:**
- Consumes: `ACTIVE_MODES` from `$lib/config`.
- Produces: `prepareHistoryFilter`, `isFilterNarrowed`, `filterCacheKey`, and `HistoryFilterDraft` types.

- [ ] **Step 1: Write the failing test**

Create `dashboard/src/lib/history-search.test.ts`:

```ts
import assert from "node:assert/strict";
import test from "node:test";
import {
  filterCacheKey,
  isFilterNarrowed,
  prepareHistoryFilter,
  type HistoryFilterDraft,
} from "./history-search.ts";

test("prepareHistoryFilter trims all fields and keeps valid dates and modes", () => {
  const draft: HistoryFilterDraft = {
    query: "  meeting notes  ",
    from_day: " 2026-10-01 ",
    to_day: " 2026-10-15 ",
    mode: " email ",
    app_id: " org.gnome.TextEditor ",
    src_lang: " en ",
  };
  const res = prepareHistoryFilter(draft);
  assert.equal(res.ok, true);
  if (res.ok) {
    assert.deepEqual(res.filter, {
      query: "meeting notes",
      from_day: "2026-10-01",
      to_day: "2026-10-15",
      mode: "email",
      app_id: "org.gnome.TextEditor",
      src_lang: "en",
    });
    assert.equal(res.isReversed, false);
  }
});

test("prepareHistoryFilter flags invalid date format", () => {
  const res = prepareHistoryFilter({
    query: "",
    from_day: "10/01/2026",
    to_day: "",
    mode: "",
    app_id: "",
    src_lang: "",
  });
  assert.equal(res.ok, false);
  if (!res.ok) {
    assert.match(res.error, /invalid date/i);
  }
});

test("prepareHistoryFilter detects reversed date range without throwing", () => {
  const res = prepareHistoryFilter({
    query: "",
    from_day: "2026-10-15",
    to_day: "2026-10-01",
    mode: "",
    app_id: "",
    src_lang: "",
  });
  assert.equal(res.ok, true);
  if (res.ok) {
    assert.equal(res.isReversed, true);
  }
});

test("prepareHistoryFilter rejects unknown mode", () => {
  const res = prepareHistoryFilter({
    query: "",
    from_day: "",
    to_day: "",
    mode: "supermode",
    app_id: "",
    src_lang: "",
  });
  assert.equal(res.ok, false);
  if (!res.ok) {
    assert.match(res.error, /unknown mode/i);
  }
});

test("isFilterNarrowed detects any active filter field", () => {
  assert.equal(
    isFilterNarrowed({
      from_day: "",
      to_day: "",
      mode: "",
      app_id: "",
      src_lang: "",
    }),
    false,
  );
  assert.equal(
    isFilterNarrowed({
      from_day: "2026-10-01",
      to_day: "",
      mode: "",
      app_id: "",
      src_lang: "",
    }),
    true,
  );
  assert.equal(
    isFilterNarrowed({
      from_day: "",
      to_day: "",
      mode: "code",
      app_id: "",
      src_lang: "",
    }),
    true,
  );
});

test("filterCacheKey generates unique key for cache busting", () => {
  const key1 = filterCacheKey({
    query: "hello",
    from_day: "2026-10-01",
    to_day: "2026-10-02",
    mode: "plain",
    app_id: "gedit",
    src_lang: "en",
  });
  const key2 = filterCacheKey({
    query: "hello",
    from_day: "2026-10-01",
    to_day: "2026-10-03",
    mode: "plain",
    app_id: "gedit",
    src_lang: "en",
  });
  assert.notEqual(key1, key2);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/history-search.test.ts`

Expected: FAIL. `history-search.ts` does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Create `dashboard/src/lib/history-search.ts`:

```ts
import { ACTIVE_MODES, type ActiveMode } from "./config";

export type HistoryFilterDraft = {
  query?: string;
  from_day?: string;
  to_day?: string;
  mode?: string;
  app_id?: string;
  src_lang?: string;
};

export type ValidatedHistoryFilter = {
  query: string;
  from_day: string;
  to_day: string;
  mode: string;
  app_id: string;
  src_lang: string;
};

export type PreparedHistoryFilter =
  | {
      ok: true;
      filter: ValidatedHistoryFilter;
      isReversed: boolean;
    }
  | {
      ok: false;
      error: string;
    };

const DATE_REGEX = /^\d{4}-\d{2}-\d{2}$/;

function isValidDayString(str: string): boolean {
  if (!DATE_REGEX.test(str)) return false;
  const [y, m, d] = str.split("-").map(Number);
  if (m < 1 || m > 12 || d < 1 || d > 31) return false;
  const date = new Date(Date.UTC(y, m - 1, d));
  return (
    date.getUTCFullYear() === y &&
    date.getUTCMonth() === m - 1 &&
    date.getUTCDate() === d
  );
}

export function prepareHistoryFilter(draft: HistoryFilterDraft): PreparedHistoryFilter {
  const query = (draft.query ?? "").trim();
  const from_day = (draft.from_day ?? "").trim();
  const to_day = (draft.to_day ?? "").trim();
  const mode = (draft.mode ?? "").trim();
  const app_id = (draft.app_id ?? "").trim();
  const src_lang = (draft.src_lang ?? "").trim();

  if (from_day.length > 0 && !isValidDayString(from_day)) {
    return { ok: false, error: `Invalid date: ${from_day}` };
  }
  if (to_day.length > 0 && !isValidDayString(to_day)) {
    return { ok: false, error: `Invalid date: ${to_day}` };
  }

  let isReversed = false;
  if (from_day.length > 0 && to_day.length > 0) {
    if (from_day > to_day) {
      isReversed = true;
    }
  }

  if (mode.length > 0 && !(ACTIVE_MODES as readonly string[]).includes(mode)) {
    return { ok: false, error: `Unknown mode: ${mode}` };
  }

  return {
    ok: true,
    filter: {
      query,
      from_day,
      to_day,
      mode,
      app_id,
      src_lang,
    },
    isReversed,
  };
}

export function isFilterNarrowed(
  filter: Omit<ValidatedHistoryFilter, "query">,
): boolean {
  return (
    filter.from_day.length > 0 ||
    filter.to_day.length > 0 ||
    filter.mode.length > 0 ||
    filter.app_id.length > 0 ||
    filter.src_lang.length > 0
  );
}

export function filterCacheKey(filter: ValidatedHistoryFilter): string {
  return [
    filter.query,
    filter.from_day,
    filter.to_day,
    filter.mode,
    filter.app_id,
    filter.src_lang,
  ].join("\0");
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/history-search.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests pass).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/lib/history-search.ts dashboard/src/lib/history-search.test.ts
git commit -m "feat(dashboard): create history search contract module"
```

---

### Task 2: Wire search contract and mode selector to History page

**Files:**
- Modify: `dashboard/src/routes/+page.svelte`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes: `prepareHistoryFilter`, `isFilterNarrowed`, `filterCacheKey` from `$lib/history-search`, and `ACTIVE_MODES` from `$lib/config`.
- Produces: Integrated history search and filter bar with `<select>` for mode filtering, date range validation feedback, and active filter handling.

- [ ] **Step 1: Write integration test assertions**

In `dashboard/src/lib/history-search.test.ts`, add test for page source integration:

```ts
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pageSource = readFileSync(join(here, "../routes/+page.svelte"), "utf8");

test("history page uses prepareHistoryFilter and active mode options", () => {
  assert.match(pageSource, /prepareHistoryFilter/);
  assert.match(pageSource, /ACTIVE_MODES/);
  assert.match(pageSource, /All modes/);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/history-search.test.ts`

Expected: FAIL. `+page.svelte` does not import `prepareHistoryFilter` or have `All modes`.

- [ ] **Step 3: Update `dashboard/src/routes/+page.svelte`**

1. Import `prepareHistoryFilter`, `isFilterNarrowed`, `filterCacheKey` from `$lib/history-search`.
2. In `loadHistory()`:
   Use `prepareHistoryFilter`:
   ```ts
   const prepared = prepareHistoryFilter({
     query,
     from_day: fromDay,
     to_day: toDay,
     mode: modeFilter,
     app_id: appFilter,
     src_lang: langFilter,
   });
   if (!prepared.ok) {
     actionError = prepared.error;
     loading = false;
     return;
   }
   const filters = prepared.filter;
   const narrowed = isFilterNarrowed(filters);
   const nextKey = filterCacheKey(filters);
   if (nextKey !== appliedKey) {
     checkedIds = [];
     appliedKey = nextKey;
   }
   if (prepared.isReversed) {
     entries = [];
     appliedQuery = filters.query;
     appliedNarrowed = true;
     reachable = true;
     loading = false;
     return;
   }
   ```
3. In `filterBar()`:
   Replace the raw mode `<input>` with:
   ```svelte
   <select
     class="w-28 px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/30 shrink-0 font-body-muted text-body-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
     aria-label="Filter by mode"
     bind:value={modeFilter}
     onchange={onFilterCommit}
   >
     <option value="">All modes</option>
     {#each ACTIVE_MODES as m}
       <option value={m}>{m.charAt(0).toUpperCase() + m.slice(1)}</option>
     {/each}
   </select>
   ```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/history-search.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests pass).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/routes/+page.svelte dashboard/src/lib/history-search.test.ts
git commit -m "feat(dashboard): wire search contract and mode select to history"
```

---

### Task 3: Impeccable critique and audit of the History search surface

**Files:**
- Modify: `dashboard/src/routes/+page.svelte`
- Test: `cd dashboard && bun test`

**Interfaces:**
- Consumes: History search & filter markup in `dashboard/src/routes/+page.svelte`.
- Produces: Polished interactive controls with focus-visible rings, accessible labels, clear hit targets, and Quiet Precision token compliance.

- [ ] **Step 1: Baseline verification**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 2: Run Critique and Audit in Operate mode on the History search surface**

Review against `docs/DESIGN.md` §5 & §7:
- Focus indicators: Ensure all filter inputs (`fromDay`, `toDay`, `modeFilter`, `appFilter`, `langFilter`) have explicit `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary`.
- Hit targets: Minimum 36px interactive heights (`min-h-[36px]`).
- Visual tokens: Consistent `bg-surface-container` and `border-outline-variant/30`.
- Clear button / search feedback: Ensure search form has clear accessibility attributes (`role="search"`, aria-labels).

- [ ] **Step 3: Apply surviving defects**

Apply focus rings and `min-h-[36px]` sizing consistently across the filter bar controls in `dashboard/src/routes/+page.svelte`.

- [ ] **Step 4: Verify tests pass**

Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/routes/+page.svelte
git commit -m "fix(dashboard): apply history search critique and focus styling"
```

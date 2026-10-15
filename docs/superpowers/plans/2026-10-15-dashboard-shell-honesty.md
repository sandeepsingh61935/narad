# Dashboard Shell Honesty Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The running dashboard links only Phase 2 surfaces and stops showing costume chrome from later phases.

**Architecture:** `docs/ROADMAP.md` (“How a phase ships”) is the spec. This plan is beat 2 for the shell only: delete unlinked fiction, keep History, Stats, and Settings. No new daemon method. Impeccable `critique` and `audit` run after the shell is honest, in Task 3, and do not add controls.

**Tech Stack:** Svelte 5 dashboard, Node’s built-in test runner, existing `node:assert/strict` source checks in `dashboard/src/lib/`.

## Global Constraints

- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Daemon owns state. The dashboard is a thin client. This plan adds no D-Bus method and no dependency.
- Phase 2 shell pages: History (`/`), Stats (`/stats`), Settings (`/settings`, including Audio, Dictionary, Privacy).
- Phase 3 screens (Models, `/setup` model picker) are not linked and their static route files are removed.
- Phase 4 cleanup editor and per-app profiles are not added.
- Copy is sentence case. No “telemetry”, “acoustic”, “lexical”, invented percents, CUDA, TensorRT, or gsettings.
- Hotkeys use the GlobalShortcuts portal, not `org.gnome.shell.keybindings`.
- Settings that already save do so through the existing config path. This plan does not add a Save button.
- Do not run Impeccable `bolder`, `delight`, or `overdrive`.
- One commit per task, on a feature branch, never push.

## File structure

- `dashboard/src/lib/shell-honesty.test.ts` — reads route source and fails while costume chrome or later-phase routes remain.
- `dashboard/src/routes/+layout.svelte` — primary nav and window chrome. History, Stats, Settings only.
- `dashboard/src/routes/models/+page.svelte` — static Stitch catalog. Deleted.
- `dashboard/src/routes/setup/+page.svelte` — static first-run with a fake Whisper picker. Deleted.
- `dashboard/src/routes/settings/+page.svelte` — hotkey editor. Loses the unwired conflict banner, the gsettings line, and the unwired reset button.

Out of this plan: wiring a live “Ready” line, History visual polish, dictionary behavior, Models rebuilt for Phase 3, mode-editor design. Those stay in `docs/ROADMAP.md`.

---

### Task 1: Phase 2 shell only

**Files:**
- Create: `dashboard/src/lib/shell-honesty.test.ts`
- Modify: `dashboard/src/routes/+layout.svelte`
- Delete: `dashboard/src/routes/models/+page.svelte`
- Delete: `dashboard/src/routes/setup/+page.svelte`
- Test: `dashboard/src/lib/shell-honesty.test.ts`

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces: a layout whose nav is History, Stats, Settings. No `/models` route, no `/setup` route. Later tasks must not reintroduce those paths.

- [ ] **Step 1: Write the failing test**

Create `dashboard/src/lib/shell-honesty.test.ts`:

```ts
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import test from "node:test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const layout = readFileSync(join(here, "../routes/+layout.svelte"), "utf8");

test("the shell links History, Stats, and Settings only", () => {
  assert.match(layout, /label: "History"/);
  assert.match(layout, /label: "Stats"/);
  assert.match(layout, /label: "Settings"/);
  assert.equal(layout.includes("/models"), false);
  assert.equal(layout.includes("/setup"), false);
  assert.equal(layout.includes("Buffer RAM"), false);
  assert.equal(layout.includes("124 MB"), false);
  assert.equal(layout.includes("Parakeet EN"), false);
  assert.equal(layout.includes(">Record<"), false);
  assert.equal(layout.includes(">person<"), false);
  assert.equal(layout.includes("Search transcripts"), false);
});

test("phase 3 costume routes are not in the tree", () => {
  assert.equal(existsSync(join(here, "../routes/models/+page.svelte")), false);
  assert.equal(existsSync(join(here, "../routes/setup/+page.svelte")), false);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/shell-honesty.test.ts`

Expected: FAIL. The layout still contains `/models`, `Buffer RAM`, and `>Record<`. Both route files still exist.

- [ ] **Step 3: Write the minimal implementation**

Replace `dashboard/src/routes/+layout.svelte` with:

```svelte
<script lang="ts">
  import { page } from "$app/state";
  import "../app.css";

  let { children } = $props();

  const links = [
    { href: "/", label: "History", icon: "history" },
    { href: "/stats", label: "Stats", icon: "bar_chart" },
    { href: "/settings", label: "Settings", icon: "settings" },
  ] as const;

  const activeClass =
    "flex items-center gap-space-md px-space-md py-space-sm rounded-lg transition-colors bg-surface-container-high text-on-surface font-medium";
  const idleClass =
    "flex items-center gap-space-md px-space-md py-space-sm rounded-lg text-on-surface-variant hover:bg-surface-container hover:text-on-surface transition-colors";

  function isActive(href: string): boolean {
    const path = page.url.pathname;
    if (href === "/") return path === "/";
    return path === href || path.startsWith(`${href}/`);
  }
</script>

<aside
  class="fixed left-0 top-0 h-full w-64 bg-surface-container-low border-r border-outline-variant flex flex-col z-30 select-none"
>
  <div class="flex flex-col">
    <div
      class="h-14 px-space-md flex items-center gap-space-sm border-b border-outline-variant"
    >
      <span class="font-headline-md text-headline-md tracking-tight text-on-surface">Narad</span>
    </div>
    <nav
      class="flex flex-col p-space-sm gap-space-xs mt-space-xs"
      aria-label="Primary"
    >
      {#each links as link}
        <a
          href={link.href}
          class={isActive(link.href) ? activeClass : idleClass}
          aria-current={isActive(link.href) ? "page" : undefined}
        >
          <span class="material-symbols-outlined text-[18px]">{link.icon}</span>
          <span class="font-body-default text-body-default">{link.label}</span>
        </a>
      {/each}
    </nav>
  </div>
</aside>
<main class="ml-64 h-screen overflow-auto">
  {@render children()}
</main>
```

The removed header search is not the history search. History keeps the field labeled “Search transcripts” on `dashboard/src/routes/+page.svelte`.

Delete these files:

- `dashboard/src/routes/models/+page.svelte`
- `dashboard/src/routes/setup/+page.svelte`

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/shell-honesty.test.ts dashboard/src/lib/prepare-config.test.ts dashboard/src/lib/screen-state.test.ts`

Expected: PASS. `prepare-config.test.ts` and `screen-state.test.ts` do not read the deleted routes.

- [ ] **Step 5: Commit**

```bash
git add dashboard/src/lib/shell-honesty.test.ts dashboard/src/routes/+layout.svelte dashboard/src/routes/models/+page.svelte dashboard/src/routes/setup/+page.svelte
git commit -m "fix(dashboard): drop phase 3 costume from the shell"
```

---

### Task 2: Hotkeys page stops claiming gsettings

**Files:**
- Modify: `dashboard/src/lib/shell-honesty.test.ts`
- Modify: `dashboard/src/routes/settings/+page.svelte`
- Test: `dashboard/src/lib/shell-honesty.test.ts`

**Interfaces:**
- Consumes: Task 1’s test file and the Phase 2 shell.
- Produces: `settings/+page.svelte` with no gsettings sentence, no “No Keybinding Conflicts” banner, and no unwired “Reset Hotkeys to Default” button. Hotkey editing via `beginHotkeyEdit` stays.

- [ ] **Step 1: Write the failing test**

Append to `dashboard/src/lib/shell-honesty.test.ts`:

```ts
const hotkeys = readFileSync(join(here, "../routes/settings/+page.svelte"), "utf8");

test("hotkeys copy does not invent a gsettings backend", () => {
  assert.equal(hotkeys.includes("gsettings"), false);
  assert.equal(hotkeys.includes("org.gnome.shell.keybindings"), false);
  assert.equal(hotkeys.includes("No Keybinding Conflicts"), false);
  assert.equal(hotkeys.includes("Reset Hotkeys to Default"), false);
  assert.match(hotkeys, /beginHotkeyEdit/);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/shell-honesty.test.ts`

Expected: FAIL on `gsettings` and `No Keybinding Conflicts`. The layout tests from Task 1 still pass.

- [ ] **Step 3: Write the minimal implementation**

In `dashboard/src/routes/settings/+page.svelte`, delete the block that starts with `<div class="flex flex-col gap-space-sm pb-8">` and ends just before `{/if}` (the conflict banner, the gsettings line, and the reset button). Leave `{/if}` and the surrounding section in place. Do not change `beginHotkeyEdit`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/shell-honesty.test.ts dashboard/src/lib/prepare-config.test.ts`

Expected: PASS. The hotkeys page still contains `SAVED_NOTICE` if that string is already there; this task does not remove it. If `prepare-config.test.ts` fails because it required the deleted block, stop. That test currently reads modes, privacy, audio, and dictionary, not this footer.

- [ ] **Step 5: Commit**

```bash
git add dashboard/src/lib/shell-honesty.test.ts dashboard/src/routes/settings/+page.svelte
git commit -m "fix(dashboard): remove the false hotkey-backend line"
```

---

### Task 3: Impeccable critique and audit of the shell

**Files:**
- Modify: only files Task 1 and Task 2 already own, and only if the critique names a defect in those files.
- Test: `dashboard/src/lib/shell-honesty.test.ts`

**Interfaces:**
- Consumes: the honest shell from Tasks 1 and 2.
- Produces: no new route, no new control, no new copy the daemon does not serve. A critique finding that asks for Models, a RAM meter, an account menu, or a Record button is rejected.

This is the Impeccable call for this slice. Do not run it before Task 2’s commit. Do not run `shape`, `bolder`, `delight`, `overdrive`, `colorize`, or `live`.

- [ ] **Step 1: Run the existing test so the baseline is green**

Run: `node --test dashboard/src/lib/shell-honesty.test.ts`

Expected: PASS.

- [ ] **Step 2: Critique, then audit**

Follow the impeccable skill in Operate mode for these two files only:

- `dashboard/src/routes/+layout.svelte`
- `dashboard/src/routes/settings/+page.svelte`

Run `critique`, then `audit`. The HUD is not part of this task.

- [ ] **Step 3: Apply only defects that survive the constraints**

Apply a finding when it is contrast, focus, hit target, or a label already required by `docs/DESIGN.md`. Skip a finding that adds a control, a metric, a page, or a font load. Re-run:

`node --test dashboard/src/lib/shell-honesty.test.ts dashboard/src/lib/prepare-config.test.ts dashboard/src/lib/screen-state.test.ts`

Expected: PASS.

- [ ] **Step 4: Commit when Step 3 changed a file**

```bash
git add dashboard/src/routes/+layout.svelte dashboard/src/routes/settings/+page.svelte
git commit -m "fix(dashboard): apply the shell critique"
```

If Step 3 changed nothing, do not create an empty commit. Leave the checkbox note “critique applied: none”.

---

## Self-review

- Spec coverage: ROADMAP “Phase 2 UI in this phase” is Tasks 1 and 2. The Impeccable line for the shell is Task 3. Phase 3 `shape`, Phase 4 `shape`, and Phase 5 `shape` are named in `docs/ROADMAP.md` and are not tasks here.
- History search stays on `+page.svelte`. The header duplicate is removed in Task 1.
- Placeholder scan: no TBD steps. Task 3’s skip rule is an explicit reject list, not an open implementation.
- The active-mode settings page is untouched, matching the roadmap line that the wired mode list may stay.

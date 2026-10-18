# Floating HUD States Parity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete Phase 2 Surface 1 — achieve full parity for floating HUD states in FR-DOT and DESIGN §3 (processing anchor non-color-alone icon, right-click menu keycap accelerators, dynamic pause history sync, screen-state parity, and Impeccable HUD review).

**Architecture:** `docs/REQUIREMENTS.md` (FR-DOT-2, FR-DOT-4, FR-HIS-7), `docs/ROADMAP.md` (Phase 2 Surface 1, "How a phase ships"), and `docs/DESIGN.md` §3/§4 are the normative authorities.
Beat 1 updates `screen-state.ts` to model paused states in `HudLine`.
Beat 2 implements the right-click menu keycap accelerators, dynamic pause sync, and processing anchor icon in `extension/extension.js`.
Beat 3 applies the Impeccable visual pass (Operate mode: `distill` → `critique` → `audit`) on the HUD surface (`extension/stylesheet.css` and `extension/extension.js`) against DESIGN §3.

**Tech Stack:** GNOME Shell extension (JavaScript, St, Clutter), CSS, Node test runner (`node --test`), TypeScript.

## Global Constraints

- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Daemon owns state. The HUD is a thin client. This plan adds no D-Bus method and no dependency.
- Phase 2 HUD states: idle, listening, processing, inserted, copied, cancelled, error, undone, and paused history cue.
- Status never by color alone: anchor and pill always pair color with an icon, shape, or label.
- Reduced motion: static listening glyph & non-animated spinner fallback when animations are disabled.
- Copy is sentence case. No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- Hotkeys: Mode 1 (`plain`) is `Ctrl+Alt+1`, Mode 2 (`email`) is `Ctrl+Alt+2`, Mode 3 (`bullet`) is `Ctrl+Alt+3`, Mode 4 (`code`) is `Ctrl+Alt+4`.
- Right-click menu items: active mode selector with check ornament and keycap accelerators, separator, Open dashboard, Pause history / Resume history, Hide HUD, Quit.
- Do not run Impeccable `bolder`, `delight`, or `overdrive`.
- One commit per task, on a feature branch, never push.

## File structure

- `dashboard/src/lib/screen-state.ts` — adds `paused?: boolean` to `HudInput` and `HudLine` to model paused HUD state parity.
- `dashboard/src/lib/screen-state.test.ts` — tests `hudLine` with paused states (idle and listening).
- `dashboard/src/lib/hud-parity.test.ts` — asserts extension source contains keycap accelerators (`Ctrl+Alt+1..4`), pause label sync (`Resume history`), and processing anchor non-color-alone icon (`emblem-synchronizing-symbolic`).
- `extension/extension.js` — GNOME Shell extension: right-click menu keycap badges, dynamic pause menu label sync, processing anchor icon.
- `extension/stylesheet.css` — HUD styling: keycaps in popover menu, contrast, and alignment.

---

### Task 1: Screen-state HUD paused parity

**Files:**
- Modify: `dashboard/src/lib/screen-state.ts`
- Modify: `dashboard/src/lib/screen-state.test.ts`
- Test: `dashboard/src/lib/screen-state.test.ts`

**Interfaces:**
- Consumes: `HudInput` in `screen-state.ts`.
- Produces: `HudLine` supporting `paused?: boolean` where idle + paused produces primary `"History paused"` pill text, and listening/processing + paused retains `paused: true`.

- [ ] **Step 1: Write the failing test**

Append to `dashboard/src/lib/screen-state.test.ts`:

```ts
test("HUD line reflects history paused state", () => {
  assert.deepEqual(hudLine({ state: "idle", paused: true }), {
    state: "idle",
    pill: "History paused",
    paused: true,
  });
  assert.deepEqual(
    hudLine({ state: "listening", mode: "plain", elapsed: "0:12", paused: true }),
    {
      state: "listening",
      mode: "plain",
      elapsed: "0:12",
      paused: true,
    },
  );
  assert.deepEqual(
    hudLine({ state: "processing", paused: true }),
    {
      state: "processing",
      primary: "Transcribing…",
      paused: true,
    },
  );
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/screen-state.test.ts`

Expected: FAIL. `hudLine` ignores `paused: true` and returns `{ state: "idle", pill: null }`.

- [ ] **Step 3: Write the minimal implementation**

In `dashboard/src/lib/screen-state.ts`, update `HudLine`, `HudInput`, and `hudLine`:

```ts
export type HudLine =
  | { state: "idle"; pill: null; paused?: false }
  | { state: "idle"; pill: "History paused"; paused: true }
  | { state: "listening"; mode: string; elapsed: string; paused?: boolean }
  | { state: "processing"; primary: "Transcribing…"; paused?: boolean }
  | { state: "inserted"; primary: "Inserted"; paused?: boolean }
  | { state: "copied"; primary: "Copied to clipboard"; reason: "No text field focused"; paused?: boolean }
  | { state: "cancelled"; primary: "Cancelled"; paused?: boolean }
  | { state: "error"; primary: string; recovery: string; paused?: boolean }
  | { state: "undone"; primary: "Undone"; deleted: string; paused?: boolean };

export type HudInput = {
  state: HudLine["state"];
  mode?: string;
  elapsed?: string;
  errorPrimary?: string | null;
  /** Text removed by undo. Only the undone line reads this. */
  deleted?: string | null;
  paused?: boolean;
};

export function hudLine(input: HudInput): HudLine {
  const paused = input.paused;
  switch (input.state) {
    case "idle":
      if (paused) {
        return { state: "idle", pill: "History paused", paused: true };
      }
      return { state: "idle", pill: null };
    case "listening":
      return {
        state: "listening",
        mode: input.mode ?? "",
        elapsed: input.elapsed ?? "",
        ...(paused ? { paused: true } : {}),
      };
    case "processing":
      return {
        state: "processing",
        primary: "Transcribing…",
        ...(paused ? { paused: true } : {}),
      };
    case "inserted":
      return {
        state: "inserted",
        primary: "Inserted",
        ...(paused ? { paused: true } : {}),
      };
    case "copied":
      return {
        state: "copied",
        primary: "Copied to clipboard",
        reason: "No text field focused",
        ...(paused ? { paused: true } : {}),
      };
    case "cancelled":
      return {
        state: "cancelled",
        primary: "Cancelled",
        ...(paused ? { paused: true } : {}),
      };
    case "error":
      return {
        state: "error",
        primary:
          input.errorPrimary != null && input.errorPrimary.length > 0
            ? input.errorPrimary
            : "Microphone unavailable",
        recovery: "Check audio settings",
        ...(paused ? { paused: true } : {}),
      };
    case "undone":
      return {
        state: "undone",
        primary: "Undone",
        deleted: input.deleted ?? "",
        ...(paused ? { paused: true } : {}),
      };
  }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/screen-state.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests green).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/lib/screen-state.ts dashboard/src/lib/screen-state.test.ts
git commit -m "fix(dashboard): model paused HUD state parity"
```

---

### Task 2: HUD state implementation in GNOME Shell extension

**Files:**
- Modify: `dashboard/src/lib/hud-parity.test.ts`
- Modify: `extension/extension.js`
- Test: `dashboard/src/lib/hud-parity.test.ts`

**Interfaces:**
- Consumes: `extension/extension.js` source code.
- Produces: Popover menu with mode keycaps (`Ctrl+Alt+1..4`), dynamic `Resume history` / `Pause history` label sync on `_applyPaused`, and anchor non-color-alone icon in `processing` state (`emblem-synchronizing-symbolic`).

- [ ] **Step 1: Write the failing test**

In `dashboard/src/lib/hud-parity.test.ts`, add tests for keycaps, pause sync, and processing anchor icon:

```ts
test("right-click menu includes keycap accelerators for modes", () => {
  assert.match(extensionSource, /Ctrl\+Alt\+1/);
  assert.match(extensionSource, /Ctrl\+Alt\+2/);
  assert.match(extensionSource, /Ctrl\+Alt\+3/);
  assert.match(extensionSource, /Ctrl\+Alt\+4/);
  assert.match(extensionSource, /narad-hud-keycap/);
});

test("right-click menu dynamically updates Pause history label", () => {
  assert.match(extensionSource, /Resume history/);
  assert.match(extensionSource, /_syncDotMenuPause/);
});

test("processing anchor state does not rely on color alone", () => {
  const procStart = extensionSource.indexOf("case 'processing':");
  assert.ok(procStart >= 0, "missing processing case");
  const procEnd = extensionSource.indexOf("case 'inserted':", procStart);
  assert.ok(procEnd > procStart, "processing case does not end before inserted");
  const procBlock = extensionSource.slice(procStart, procEnd);
  assert.match(procBlock, /emblem-synchronizing-symbolic/);
  assert.match(procBlock, /_setAnchorChild\(this\._anchorIcon\)/);
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `node --test dashboard/src/lib/hud-parity.test.ts`

Expected: FAIL. Extension source does not yet contain `Ctrl+Alt+1`, `Resume history`, `_syncDotMenuPause`, or `_setAnchorChild(this._anchorIcon)` in processing.

- [ ] **Step 3: Write the minimal implementation**

In `extension/extension.js`:
1. In `_buildDotMenu()`:
   Assign keycaps to mode items:
   ```js
        const modeShortcuts = {
            plain: 'Ctrl+Alt+1',
            email: 'Ctrl+Alt+2',
            bullet: 'Ctrl+Alt+3',
            code: 'Ctrl+Alt+4',
        };
        this._modeItems = [];
        for (const mode of ['plain', 'email', 'bullet', 'code']) {
            const item = new PopupMenu.PopupMenuItem(mode);
            const accel = new St.Label({
                text: modeShortcuts[mode],
                style_class: 'narad-hud-keycap',
                x_align: Clutter.ActorAlign.END,
                x_expand: true,
            });
            item.add_child(accel);
            item.connect('activate', () => this._setModeProperty(mode));
            this._menu.addMenuItem(item);
            this._modeItems.push([mode, item]);
        }
        this._menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._menu.addAction('Open dashboard', () => this._openDashboard());
        this._pauseItem = this._menu.addAction('Pause history', () => this._onPauseHistory());
        this._syncDotMenuPause();
        this._menu.addAction('Hide HUD', () => this._onHideClick());
        this._menu.addAction('Quit', () => this._quitDaemon());
   ```

2. Add `_syncDotMenuPause()` method:
   ```js
    _syncDotMenuPause() {
        if (!this._pauseItem || !this._pauseItem.label)
            return;
        this._pauseItem.label.text = this._paused ? 'Resume history' : 'Pause history';
    }
   ```

3. In `_applyPaused(paused)`:
   Call `this._syncDotMenuPause();`:
   ```js
    _applyPaused(paused) {
        this._paused = !!paused;
        this._syncDotMenuPause();
        if (!this._anchor)
            return;
        if (this._state === 'idle')
            this._applyState('idle');
        else
            this._refreshPausedCue();
    }
   ```

4. In `_applyState('processing')`:
   Update anchor child to `this._anchorIcon` with `emblem-synchronizing-symbolic`:
   ```js
        case 'processing':
            this._anchor.add_style_class_name('narad-hud-anchor-processing');
            this._anchorIcon.icon_name = 'emblem-synchronizing-symbolic';
            this._setAnchorChild(this._anchorIcon);
            this._pill.visible = true;
            this._pillIcon.icon_name = 'emblem-synchronizing-symbolic';
            this._pillIcon.visible = true;
            this._pillPrimary.text = 'Transcribing…';
            break;
   ```

- [ ] **Step 4: Run the test to verify it passes**

Run: `node --test dashboard/src/lib/hud-parity.test.ts`
Also run: `cd dashboard && bun test`

Expected: PASS (all tests green).

- [ ] **Step 5: Commit changes**

Run:
```bash
git add dashboard/src/lib/hud-parity.test.ts extension/extension.js
git commit -m "fix(extension): complete FR-DOT HUD states and keycap accelerators"
```

---

### Task 3: Impeccable critique and audit of the HUD

**Files:**
- Modify: `extension/stylesheet.css`
- Modify: `extension/extension.js` (if needed for ARIA/labels/tokens)
- Test: `node --test dashboard/src/lib/hud-parity.test.ts` and `cd dashboard && bun test`

**Interfaces:**
- Consumes: `extension/stylesheet.css`, `extension/extension.js`, `docs/DESIGN.md` §3/§4.
- Produces: Polished styling for menu keycaps, anchor processing icon, contrast and spacing compliant with Quiet Precision and Adwaita popover conventions.

- [ ] **Step 1: Baseline verification**

Run: `node --test dashboard/src/lib/hud-parity.test.ts`
Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 2: Run Critique and Audit in Operate mode on the HUD surface**

Review against `docs/DESIGN.md` §3 & §7:
- Quiet Precision tokens: warm charcoal, saffron `#e08a1e`, `#1c1b1a`, `#e6e2df`, `#9e9a94`, `#4e9a64`, `#d9534f`.
- Processing anchor styling in `stylesheet.css`: Ensure `narad-hud-anchor-processing StIcon` has appropriate icon-size (14px) and color (`#1c1b1a`), matching the listening anchor icon treatment.
- Menu keycap styling in `stylesheet.css`: Ensure `.narad-hud-keycap` has correct margins/padding when displayed inside `PopupMenu.PopupMenuItem`, so it aligns cleanly to the right edge with proper contrast against the dark Adwaita menu background.
- Reduced motion: Ensure `_animationsEnabled()` check protects animated spinners/waveforms.
- Hit target: Anchor size is 28px (`ANCHOR_SIZE`), with hover background feedback (`rgba(255, 255, 255, 0.15)`).

- [ ] **Step 3: Apply surviving defects**

In `extension/stylesheet.css`:
1. Add styling for `narad-hud-anchor-processing StIcon`:
```css
.narad-hud-anchor-processing StIcon {
    icon-size: 14px;
    color: #1c1b1a;
}
```
2. Ensure popover keycap styling has clean margin in menu item:
```css
.popup-menu-item .narad-hud-keycap {
    margin-left: 12px;
}
```

- [ ] **Step 4: Verify tests pass**

Run: `node --test dashboard/src/lib/hud-parity.test.ts`
Run: `cd dashboard && bun test`

Expected: PASS.

- [ ] **Step 5: Commit changes**

Run:
```bash
git add extension/stylesheet.css extension/extension.js
git commit -m "fix(extension): apply HUD critique and styling polish"
```
*(If no styling defects survive, do not create an empty commit.)*

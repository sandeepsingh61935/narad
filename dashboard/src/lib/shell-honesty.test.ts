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

const hotkeys = readFileSync(join(here, "../routes/settings/+page.svelte"), "utf8");

test("hotkeys copy does not invent a gsettings backend", () => {
  assert.equal(hotkeys.includes("gsettings"), false);
  assert.equal(hotkeys.includes("org.gnome.shell.keybindings"), false);
  assert.equal(hotkeys.includes("No Keybinding Conflicts"), false);
  assert.equal(hotkeys.includes("Reset Hotkeys to Default"), false);
  assert.match(hotkeys, /beginHotkeyEdit/);
});


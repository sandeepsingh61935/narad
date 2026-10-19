import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

const historyPage = readFileSync(join(here, "../routes/+page.svelte"), "utf8");
const hudPage = readFileSync(join(here, "../routes/hud/+page.svelte"), "utf8");
const wordsPage = readFileSync(join(here, "../routes/words/+page.svelte"), "utf8");
const dictPage = readFileSync(join(here, "../routes/dictionary/+page.svelte"), "utf8");

test("history page has master-detail split layout and required controls", () => {
  assert.ok(historyPage.includes('class="split"'));
  assert.ok(historyPage.includes('class="list-pane"'));
  assert.ok(historyPage.includes('class="detail-pane"'));
  assert.ok(historyPage.includes('id="pause-banner"'));
  assert.ok(historyPage.includes('id="count"'));
  assert.ok(historyPage.includes('id="q"'));
  assert.ok(historyPage.includes('id="insert"'));
  assert.ok(historyPage.includes('id="copy"'));
  assert.ok(historyPage.includes('id="delete"'));
  assert.ok(historyPage.includes('id="play"'));
  assert.ok(historyPage.includes('id="wave"'));
  assert.ok(historyPage.includes('id="scrub"'));
  assert.ok(historyPage.includes('id="clock"'));
  assert.ok(historyPage.includes('id="show-raw"'));
  assert.ok(historyPage.includes('id="show-clean"'));
  assert.ok(historyPage.includes('id="body"'));
  assert.ok(historyPage.includes("Add to dictionary"));
  assert.ok(historyPage.includes("data-od-id=\"history-list\""));
  assert.ok(historyPage.includes("data-od-id=\"transcript-detail\""));
});

test("hud page includes desktop simulator, all 5 states, context menu, and placement settings", () => {
  assert.ok(existsSync(join(here, "../routes/hud/+page.svelte")));
  assert.ok(hudPage.includes("desktop-stage"));
  assert.ok(hudPage.includes("hud-anchor"));
  assert.ok(hudPage.includes("hud-stem"));

  // 5 HUD states
  assert.ok(hudPage.includes("hud-idle"));
  assert.ok(hudPage.includes("hud-listen"));
  assert.ok(hudPage.includes("hud-proc"));
  assert.ok(hudPage.includes("hud-result"));
  assert.ok(hudPage.includes("hud-error"));

  // Context menu with mode accelerators
  assert.ok(hudPage.includes('id="hud-menu"'));
  assert.ok(hudPage.includes("Ctrl+Alt+1"));
  assert.ok(hudPage.includes("Ctrl+Alt+2"));
  assert.ok(hudPage.includes("Ctrl+Alt+3"));
  assert.ok(hudPage.includes("Ctrl+Alt+4"));
  assert.ok(hudPage.includes("Ctrl+Alt+5"));

  // Placement settings
  assert.ok(hudPage.includes("Edge snapping"));
  assert.ok(hudPage.includes("Auto-hide in fullscreen"));
  assert.ok(hudPage.includes("Always on top"));
});

test("custom words page includes word list, add modal, empty state, and dictionary alias", () => {
  assert.ok(existsSync(join(here, "../routes/words/+page.svelte")));
  assert.ok(wordsPage.includes('id="word-list"'));
  assert.ok(wordsPage.includes('id="word-empty"'));
  assert.ok(wordsPage.includes('id="scrim"'));
  assert.ok(wordsPage.includes('id="word-dialog-title"'));
  assert.ok(wordsPage.includes('id="word"'));
  assert.ok(wordsPage.includes('id="spoken"'));
  assert.ok(wordsPage.includes("validateDictionaryPair"));
  assert.ok(wordsPage.includes("cleanDictionaryPairs"));

  // /dictionary redirects / aliases to /words
  assert.ok(dictPage.includes("/words"));
  assert.ok(dictPage.includes("goto"));
});

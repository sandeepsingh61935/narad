import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

test("Seam 1: app.html must be 100% offline with zero external Google Fonts links", () => {
  const appHtml = readFileSync(join(here, "../app.html"), "utf8");
  assert.equal(
    appHtml.includes("fonts.googleapis.com"),
    false,
    "app.html must not load fonts from fonts.googleapis.com",
  );
  assert.equal(
    appHtml.includes("Material+Symbols"),
    false,
    "app.html must not load Material Symbols from external CDN",
  );
});

test("Seam 2: +layout.svelte wraps app in stage canvas with dynamic sidebar footer and statusbar", () => {
  const layout = readFileSync(join(here, "../routes/+layout.svelte"), "utf8");
  assert.ok(layout.includes('class="stage') || layout.includes("stage"), "+layout.svelte should contain a .stage wrapper");
  assert.ok(layout.includes('class="statusbar"'), "+layout.svelte should render a .statusbar");
  assert.ok(
    layout.includes("getSidebarFooter"),
    "+layout.svelte should have dynamic sidebar footer per route",
  );
});

test("Seam 3: Settings must NOT have rogue secondary sidebar +layout.svelte or stitch.css", () => {
  const rogueLayout = existsSync(join(here, "../routes/settings/+layout.svelte"));
  assert.equal(
    rogueLayout,
    false,
    "settings/+layout.svelte must be removed so settings is not wrapped in a duplicate sidebar",
  );
  const settingsPage = readFileSync(join(here, "../routes/settings/+page.svelte"), "utf8");
  assert.ok(
    settingsPage.includes("Hold to talk"),
    "settings page should have Hold to talk radio card from prototype",
  );
  assert.ok(
    settingsPage.includes("Tap to toggle"),
    "settings page should have Tap to toggle radio card from prototype",
  );
  assert.equal(
    settingsPage.includes("PORTAL SESSION"),
    false,
    "settings page should not have legacy PORTAL SESSION badge",
  );
});

test("Seam 4: History master-detail does not have mode dropdown clutter or word tags noise", () => {
  const historyPage = readFileSync(join(here, "../routes/+page.svelte"), "utf8");
  assert.equal(
    historyPage.includes("<select"),
    false,
    "history page left pane must not contain a mode select dropdown",
  );
  assert.equal(
    historyPage.includes("Word tags"),
    false,
    "history page detail pane must not contain unnecessary Word tags section",
  );
  assert.ok(
    historyPage.includes("Inserted at cursor") || historyPage.includes("delivery_dest"),
    "history list rows must show delivery destination",
  );
});

test("Seam 5: Custom words page has no duplicate h2 header and matches prototype actions", () => {
  const wordsPage = readFileSync(join(here, "../routes/words/+page.svelte"), "utf8");
  assert.equal(
    wordsPage.includes("<h2>Custom words</h2>"),
    false,
    "words page should not have duplicate h2 Custom words title",
  );
  assert.ok(
    wordsPage.includes("Add a spelling the engine should keep"),
    "words page should display the prototype explanatory copy",
  );
  assert.ok(wordsPage.includes("Add word"), "words page should have Add word button");
});

test("Seam 6: Speech models page matches Whisper model catalog without synthetic benchmark bloat", () => {
  const modelsPage = readFileSync(join(here, "../routes/models/+page.svelte"), "utf8");
  assert.ok(modelsPage.includes("tiny"), "models page should include tiny model");
  assert.ok(modelsPage.includes("base"), "models page should include base model");
  assert.ok(modelsPage.includes("small"), "models page should include small model");
  assert.ok(modelsPage.includes("medium"), "models page should include medium model");
  assert.ok(modelsPage.includes("large-v3"), "models page should include large-v3 model");
  assert.equal(
    modelsPage.includes("Run benchmark"),
    false,
    "models page should not have bulky Run benchmark button",
  );
  assert.ok(
    modelsPage.includes("Weights never leave this machine") ||
      modelsPage.includes("Speed and memory show up here"),
    "models page should have prototype copy",
  );
});

test("Seam 7: Stats page matches prototype 4 metrics and SVG timeline without accuracy circles", () => {
  const statsPage = readFileSync(join(here, "../routes/stats/+page.svelte"), "utf8");
  assert.ok(statsPage.includes("Pace"), "stats page should include Pace metric");
  assert.ok(statsPage.includes("Clips"), "stats page should include Clips metric");
  assert.equal(
    statsPage.includes("ACCURACY"),
    false,
    "stats page should not contain generic accuracy circle widget",
  );
  assert.equal(
    statsPage.includes("DELIVERY DESTINATION BREAKDOWN"),
    false,
    "stats page should not contain chunky breakdown card",
  );
  assert.ok(
    statsPage.includes("Counts stay on this machine") ||
      statsPage.includes("Counted from transcripts saved on this machine"),
    "stats page should match prototype copy",
  );
});

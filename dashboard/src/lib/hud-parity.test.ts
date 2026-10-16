import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const extensionSource = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "../../../extension/extension.js"),
  "utf8",
);

test("the Shell HUD uses the screen-state sentences", () => {
  const sentences = [
    "Transcribing…",
    "Inserted",
    "Copied to clipboard",
    "No text field focused",
    "Cancelled",
    "Microphone unavailable",
    "Check audio settings",
  ];
  for (const sentence of sentences)
    assert.ok(extensionSource.includes(sentence), `missing HUD sentence: ${sentence}`);
});

test("right-click opens the dot menu and does not hide", () => {
  const start = extensionSource.indexOf("button === Clutter.BUTTON_SECONDARY");
  assert.ok(start >= 0, "missing secondary-button branch");
  const end = extensionSource.indexOf("button !== Clutter.BUTTON_PRIMARY", start);
  assert.ok(end > start, "secondary-button branch does not end before primary");
  const branch = extensionSource.slice(start, end);
  assert.equal(branch.includes("_onHideClick"), false);
  assert.match(branch, /_menu\.open\(/);

  const literals = [
    "plain",
    "email",
    "bullet",
    "code",
    "Open dashboard",
    "Pause history",
    "Hide HUD",
    "Quit",
    "gio launch dev.narad.Dashboard",
    "systemctl --user stop narad.service",
  ];
  for (const literal of literals)
    assert.ok(extensionSource.includes(literal), `missing ${literal}`);
  assert.equal(extensionSource.includes("systemctl --user disable"), false);
  assert.match(
    extensionSource,
    /<property name="Mode" type="s" access="readwrite"\/>/,
  );
});

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

test("companion HUD displays mode and language pair during listening and processing (FR-DOT-5)", () => {
  const listenStart = extensionSource.indexOf("case 'listening':");
  const procStart = extensionSource.indexOf("case 'processing':");
  const insertedStart = extensionSource.indexOf("case 'inserted':");
  assert.ok(listenStart >= 0 && procStart > listenStart && insertedStart > procStart);

  const listenBlock = extensionSource.slice(listenStart, procStart);
  const procBlock = extensionSource.slice(procStart, insertedStart);

  assert.match(
    listenBlock,
    /this\._pillPrimary\.text\s*=\s*`\$\{this\._mode\s*\|\|\s*DEFAULT_MODE\}\s*·\s*EN`/,
  );
  assert.match(
    procBlock,
    /this\._pillSecondary\.text\s*=\s*`\$\{this\._mode\s*\|\|\s*DEFAULT_MODE\}\s*·\s*EN`/,
  );
});


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
    "clean",
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
  assert.match(
    extensionSource,
    /<method name="SelectionCommand"\/>/,
  );
});

test("right-click menu includes keycap accelerators for modes", () => {
  assert.match(extensionSource, /Ctrl\+Alt\+1/);
  assert.match(extensionSource, /Ctrl\+Alt\+2/);
  assert.match(extensionSource, /Ctrl\+Alt\+3/);
  assert.match(extensionSource, /Ctrl\+Alt\+4/);
  assert.match(extensionSource, /Ctrl\+Alt\+5/);
  assert.match(extensionSource, /narad-hud-keycap/);
});

test("clean mode is in right-click menu with accelerator and selection mode is supported", () => {
  assert.match(extensionSource, /clean:\s*'Ctrl\+Alt\+2'/);
  assert.match(extensionSource, /'plain',\s*'clean',\s*'email',\s*'bullet',\s*'code'/);
  assert.match(
    extensionSource,
    /this\._pillPrimary\.text\s*=\s*`\$\{this\._mode\s*\|\|\s*DEFAULT_MODE\}\s*·\s*EN`/,
  );
  assert.match(
    extensionSource,
    /this\._pillSecondary\.text\s*=\s*`\$\{this\._mode\s*\|\|\s*DEFAULT_MODE\}\s*·\s*EN`/,
  );
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

test("companion HUD includes clickable Stop & Insert and Discard buttons in listening state", () => {
  assert.match(extensionSource, /name:\s*['"]narad-stop-btn['"]/);
  assert.match(extensionSource, /name:\s*['"]narad-discard-btn['"]/);
  assert.match(extensionSource, /label:\s*['"]■ Stop & Insert['"]/);
  assert.match(extensionSource, /label:\s*['"]× Discard['"]/);

  const listenStart = extensionSource.indexOf("case 'listening':");
  const procStart = extensionSource.indexOf("case 'processing':");
  assert.ok(listenStart >= 0 && procStart > listenStart);
  const listenBlock = extensionSource.slice(listenStart, procStart);

  assert.match(listenBlock, /this\._stopBtn\.visible\s*=\s*true/);
  assert.match(listenBlock, /this\._discardBtn\.visible\s*=\s*true/);

  // In base state reset, buttons are hidden
  const applyStateStart = extensionSource.indexOf("_applyState(state)");
  const switchStart = extensionSource.indexOf("switch (state)", applyStateStart);
  const baseBlock = extensionSource.slice(applyStateStart, switchStart);
  assert.match(baseBlock, /this\._stopBtn\.visible\s*=\s*false/);
  assert.match(baseBlock, /this\._discardBtn\.visible\s*=\s*false/);

  // Handlers wire to daemon D-Bus StopAndInsert/Stop and Discard/Cancel
  assert.match(extensionSource, /<method name="Stop"\/>/);
  assert.match(extensionSource, /<method name="StopAndInsert"\/>/);
  assert.match(extensionSource, /<method name="Discard"\/>/);
  assert.match(extensionSource, /_onStopClick/);
  assert.match(extensionSource, /StopAndInsertRemote/);
  assert.match(extensionSource, /_onDiscardClick/);
  assert.match(extensionSource, /DiscardRemote/);
});

test("extension enforces input source switching and crash watchdog recovery contract", () => {
  assert.match(extensionSource, /originalInputSource/);
  assert.match(extensionSource, /_activateNaradInputSource/);
  assert.match(extensionSource, /_restoreOriginalInputSource/);
  assert.match(extensionSource, /_syncInputSource/);

  // Activates narad on listening/recording
  const syncStart = extensionSource.indexOf("_syncInputSource(state) {");
  assert.ok(syncStart >= 0, "missing _syncInputSource definition");
  const syncEnd = extensionSource.indexOf("_onStopClick()", syncStart);
  assert.ok(syncEnd > syncStart, "_syncInputSource definition must precede _onStopClick");
  const syncFn = extensionSource.slice(syncStart, syncEnd);
  assert.match(syncFn, /listening/);
  assert.match(syncFn, /_activateNaradInputSource/);

  // Restores on idle, cancelled, inserted
  assert.match(syncFn, /idle/);
  assert.match(syncFn, /cancelled/);
  assert.match(syncFn, /inserted/);
  assert.match(syncFn, /_restoreOriginalInputSource/);

  // Watchdog restores on daemon vanishing (crash or unexpected disconnect)
  const vanishStart = extensionSource.indexOf("_onDaemonVanished() {");
  const vanishEnd = extensionSource.indexOf("_disconnectProxySignals()", vanishStart);
  assert.ok(vanishStart >= 0 && vanishEnd > vanishStart);
  const vanishBlock = extensionSource.slice(vanishStart, vanishEnd);
  assert.match(vanishBlock, /this\._restoreOriginalInputSource\(\)/);

  // Restores on extension disable
  const disableStart = extensionSource.indexOf("disable()");
  const disableBlock = extensionSource.slice(disableStart, disableStart + 120);
  assert.match(disableBlock, /this\._restoreOriginalInputSource\(\)/);
});

test("stylesheet styles #narad-stop-btn and #narad-discard-btn with Quiet Precision", () => {
  const stylesheetSource = readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), "../../../extension/stylesheet.css"),
    "utf8",
  );

  assert.match(stylesheetSource, /#narad-stop-btn/);
  assert.match(stylesheetSource, /#narad-discard-btn/);

  // Touch target >= 28px
  assert.match(stylesheetSource, /min-height:\s*28px/);
  assert.match(stylesheetSource, /min-width:\s*28px/);

  // Warm charcoal background
  assert.match(stylesheetSource, /background-color:\s*rgba\(45,\s*43,\s*40/);

  // Saffron hover accent
  assert.match(stylesheetSource, /#narad-stop-btn:hover/);
  assert.match(stylesheetSource, /#narad-discard-btn:hover/);
  assert.match(stylesheetSource, /#e08a1e/);

  // Visible focus state
  assert.match(stylesheetSource, /#narad-stop-btn:focus/);
  assert.match(stylesheetSource, /#narad-discard-btn:focus/);
});

test("preserves existing hotkeys for toggle and cancel", () => {
  assert.match(extensionSource, /Ctrl\+Alt\+Space/);
  assert.match(extensionSource, /_bindEsc/);
  assert.match(extensionSource, /_unbindEsc/);
  assert.match(extensionSource, /_onEscCancel/);
});



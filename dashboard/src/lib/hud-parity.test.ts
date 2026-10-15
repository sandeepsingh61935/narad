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

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

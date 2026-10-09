import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const css = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), "../app.css"),
  "utf8",
);

test("quiet precision light and dark tokens use the spec hex values", () => {
  const tokens = [
    "#fafaf8",
    "#ffffff",
    "#e6e4df",
    "#1c1b1a",
    "#66625d",
    "#252321",
    "#33302d",
    "#ededeb",
    "#9e9a94",
    "#e08a1e",
    "#d9534f",
  ];
  for (const token of tokens) {
    assert.ok(css.includes(token), `missing quiet precision token: ${token}`);
  }
});

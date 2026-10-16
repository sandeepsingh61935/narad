import assert from "node:assert/strict";
import test from "node:test";
import { parseAudioSeconds, validateAudioConfig } from "./audio-settings.ts";

test("parseAudioSeconds parses non-negative integers and allows 0 for off", () => {
  assert.equal(parseAudioSeconds("2"), 2);
  assert.equal(parseAudioSeconds("300"), 300);
  assert.equal(parseAudioSeconds("0"), 0);
  assert.equal(parseAudioSeconds("  15  "), 15);
  assert.equal(parseAudioSeconds(""), 0);
  assert.equal(parseAudioSeconds("-1"), undefined);
  assert.equal(parseAudioSeconds("2.5"), undefined);
  assert.equal(parseAudioSeconds("abc"), undefined);
});

test("validateAudioConfig validates complete audio draft", () => {
  const valid = validateAudioConfig({
    silenceRaw: "2",
    capRaw: "300",
    inputTarget: " alsa_input.pci ",
  });
  assert.equal(valid.ok, true);
  if (valid.ok) {
    assert.equal(valid.silenceSecs, 2);
    assert.equal(valid.capSecs, 300);
    assert.equal(valid.inputTarget, "alsa_input.pci");
  }

  const invalid = validateAudioConfig({
    silenceRaw: "not-a-number",
    capRaw: "300",
    inputTarget: "",
  });
  assert.equal(invalid.ok, false);
  if (!invalid.ok) {
    assert.match(invalid.error, /whole seconds/i);
  }
});

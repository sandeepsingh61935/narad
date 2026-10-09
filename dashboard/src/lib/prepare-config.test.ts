import assert from "node:assert/strict";
import test from "node:test";
import { defaultConfig } from "./config.ts";
import { prepareConfigForSave } from "./prepare-config.ts";

test("a valid edit rounds the threshold and trims hotkey, mode, and model id", () => {
  const input = defaultConfig();
  input.activation.dictate_hotkey = "  Ctrl+Alt+D  ";
  input.activation.hold_tap_threshold_ms = 250.6;
  input.general.active_mode = "  email  ";
  input.models.asr_english = "  parakeet-ctc-110m-int8  ";
  input.audio.sounds_enabled = false;

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.activation.hold_tap_threshold_ms, 251);
  assert.equal(result.config.activation.dictate_hotkey, "Ctrl+Alt+D");
  assert.equal(result.config.general.active_mode, "email");
  assert.equal(result.config.models.asr_english, "parakeet-ctc-110m-int8");
  assert.equal(result.config.audio.sounds_enabled, false);
  assert.equal(result.config.audio.auto_stop_silence_secs, 2);
  assert.equal(result.config.audio.max_duration_secs, 300);
});

test("a blank mode becomes plain", () => {
  const input = defaultConfig();
  input.general.active_mode = "   ";

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.general.active_mode, "plain");
});

test("a zero threshold stays zero", () => {
  const input = defaultConfig();
  input.activation.hold_tap_threshold_ms = 0;

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.activation.hold_tap_threshold_ms, 0);
});

const THRESHOLD_ERROR = "Hold/tap threshold must be a non-negative number.";

test("a negative threshold is rejected and the config is left unchanged", () => {
  const input = defaultConfig();
  input.activation.hold_tap_threshold_ms = -1;
  input.activation.dictate_hotkey = "  Ctrl+Alt+D  ";
  const before = structuredClone(input);

  const result = prepareConfigForSave(input);

  assert.deepEqual(result, { ok: false, error: THRESHOLD_ERROR });
  assert.deepEqual(input, before);
});

test("a non-numeric threshold is rejected", () => {
  for (const hold_tap_threshold_ms of ["nope", "", "  ", Number.NaN, Number.POSITIVE_INFINITY, null]) {
    const input = defaultConfig();
    input.activation.hold_tap_threshold_ms = hold_tap_threshold_ms as number;

    const result = prepareConfigForSave(input);

    assert.deepEqual(result, { ok: false, error: THRESHOLD_ERROR });
  }
});

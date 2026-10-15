import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { defaultConfig } from "./config.ts";
import { prepareConfigForSave } from "./prepare-config.ts";

const here = dirname(fileURLToPath(import.meta.url));

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

test("a settings save keeps the paste-last hotkey", () => {
  const input = defaultConfig();
  assert.equal(input.activation.dictate_hotkey, "Ctrl+Alt+Space");
  assert.equal(input.activation.paste_last_hotkey, "Ctrl+Alt+V");
  input.activation.paste_last_hotkey = "  Ctrl+Alt+V  ";

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.activation.dictate_hotkey, "Ctrl+Alt+Space");
  assert.equal(result.config.activation.paste_last_hotkey, "Ctrl+Alt+V");
  assert.equal(result.config.activation.undo_last_hotkey, "Ctrl+Alt+Z");
  assert.equal(result.config.activation.hold_tap_threshold_ms, 250);
});

test("a settings save keeps the mic target", () => {
  const input = defaultConfig();
  assert.equal(input.audio.input_target, "");
  input.audio.input_target = "  alsa_input.pci-0000_00_1f.3.analog-stereo  ";

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.audio.input_target, "alsa_input.pci-0000_00_1f.3.analog-stereo");
  assert.equal(result.config.audio.sounds_enabled, true);
  assert.equal(result.config.audio.auto_stop_silence_secs, 2);
  assert.equal(result.config.audio.max_duration_secs, 300);
});

test("a settings save keeps noise suppression", () => {
  const input = defaultConfig();
  assert.equal(input.audio.noise_suppression, false);
  input.audio.noise_suppression = true;

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.audio.noise_suppression, true);
  assert.equal(result.config.audio.sounds_enabled, true);
  assert.equal(result.config.audio.input_target, "");
  assert.equal(result.config.audio.auto_stop_silence_secs, 2);
  assert.equal(result.config.audio.max_duration_secs, 300);
});

test("default noise suppression is off and stays off", () => {
  const input = defaultConfig();
  input.audio.noise_suppression = false;

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.audio.noise_suppression, false);
});

test("a blank mic target stays empty", () => {
  const input = defaultConfig();
  input.audio.input_target = "   ";

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.audio.input_target, "");
});

test("default retention is off and both durations are blank", () => {
  const config = defaultConfig();
  assert.equal(config.history.retain_audio, false);
  assert.equal(config.history.text_retention_days, null);
  assert.equal(config.history.audio_retention_days, null);
});

test("a settings save keeps retention keys", () => {
  const input = defaultConfig();
  input.history.retain_audio = true;
  input.history.text_retention_days = 4;
  input.history.audio_retention_days = null;
  input.activation.dictate_hotkey = "  Ctrl+Alt+D  ";

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.activation.dictate_hotkey, "Ctrl+Alt+D");
  assert.equal(result.config.history.retain_audio, true);
  assert.equal(result.config.history.text_retention_days, 4);
  assert.equal(result.config.history.audio_retention_days, null);
});

test("blank retention durations stay blank", () => {
  const input = defaultConfig();
  input.history.retain_audio = false;
  input.history.text_retention_days = "   " as unknown as number;
  input.history.audio_retention_days = "" as unknown as number;

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.equal(result.config.history.retain_audio, false);
  assert.equal(result.config.history.text_retention_days, null);
  assert.equal(result.config.history.audio_retention_days, null);
});

test("a settings save keeps replacements and drops a blank spoken phrase", () => {
  const input = defaultConfig();
  input.replacements = [
    { spoken: "  hi there  ", written: "  hello  " },
    { spoken: "   ", written: "gone" },
    { spoken: "cat", written: "fat cat" },
  ];

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.deepEqual(result.config.replacements, [
    { spoken: "hi there", written: "hello" },
    { spoken: "cat", written: "fat cat" },
  ]);
  assert.equal(result.config.activation.undo_last_hotkey, "Ctrl+Alt+Z");
  assert.deepEqual(result.config.vocabulary, []);
});

test("a settings save keeps vocabulary phrases", () => {
  const input = defaultConfig();
  input.vocabulary = [
    { spoken: "  nar ad ", written: " Narad " },
    { spoken: " ", written: "nope" },
  ];
  input.replacements = [{ spoken: "hi", written: "hello" }];

  const result = prepareConfigForSave(input);

  assert.equal(result.ok, true);
  if (!result.ok) return;
  assert.deepEqual(result.config.vocabulary, [{ spoken: "nar ad", written: "Narad" }]);
  assert.deepEqual(result.config.replacements, [{ spoken: "hi", written: "hello" }]);
});

test("zero audio limits stay off and retention days parse", () => {
  const input = defaultConfig();
  input.audio.auto_stop_silence_secs = 0;
  input.audio.max_duration_secs = 0;
  input.audio.noise_suppression = false;
  input.audio.input_target = "  alsa  ";
  input.history.text_retention_days = "   " as unknown as number;

  const blank = prepareConfigForSave(input);

  assert.equal(blank.ok, true);
  if (!blank.ok) return;
  assert.equal(blank.config.audio.auto_stop_silence_secs, 0);
  assert.equal(blank.config.audio.max_duration_secs, 0);
  assert.equal(blank.config.audio.noise_suppression, false);
  assert.equal(blank.config.audio.input_target, "alsa");
  assert.equal(blank.config.history.text_retention_days, null);

  const days = defaultConfig();
  days.history.text_retention_days = "4" as unknown as number;
  const parsed = prepareConfigForSave(days);
  assert.equal(parsed.ok, true);
  if (!parsed.ok) return;
  assert.equal(parsed.config.history.text_retention_days, 4);

  const zeroDays = defaultConfig();
  zeroDays.history.text_retention_days = "0" as unknown as number;
  zeroDays.history.audio_retention_days = 0;
  const zero = prepareConfigForSave(zeroDays);
  assert.equal(zero.ok, true);
  if (!zero.ok) return;
  assert.equal(zero.config.history.text_retention_days, 0);
  assert.equal(zero.config.history.audio_retention_days, 0);
});

test("an unknown mode is rejected", () => {
  const input = defaultConfig();
  input.general.active_mode = "notes";

  const result = prepareConfigForSave(input);

  assert.deepEqual(result, { ok: false, error: "unknown mode: notes" });
});

test("the modes page lists the four modes and privacy drops the mock copy", () => {
  const modes = readFileSync(join(here, "../routes/settings/modes/+page.svelte"), "utf8");
  const privacy = readFileSync(join(here, "../routes/settings/privacy/+page.svelte"), "utf8");
  for (const mode of ["plain", "email", "bullet", "code"]) {
    assert.ok(modes.includes(`"${mode}"`), `modes page missing ${mode}`);
  }
  for (const source of [modes, privacy]) {
    assert.equal(source.includes("FLAC"), false);
    assert.equal(source.includes("30 days"), false);
  }
  assert.ok(privacy.includes("WAV"));
  assert.ok(privacy.includes("set_paused"));
  assert.ok(privacy.includes("session_status"));
  assert.ok(privacy.includes("retain_audio"));
  assert.ok(privacy.includes("text_retention_days"));
  assert.ok(privacy.includes("audio_retention_days"));

  const audio = readFileSync(join(here, "../routes/settings/audio/+page.svelte"), "utf8");
  assert.ok(audio.includes("input_target"));
  assert.ok(audio.includes("noise_suppression"));
  assert.ok(audio.includes("auto_stop_silence_secs"));
  assert.ok(audio.includes("max_duration_secs"));
  assert.equal(audio.includes("sounds_enabled"), false);

  const dictionary = readFileSync(join(here, "../routes/dictionary/+page.svelte"), "utf8");
  assert.ok(dictionary.includes("vocabulary"));
  assert.ok(dictionary.includes("replacements"));

  const notice = "Saved. The daemon reads this config on its next start.";
  const form = readFileSync(join(here, "settings-form.ts"), "utf8");
  assert.ok(form.includes(notice));
  for (const page of [modes, privacy, audio, dictionary]) {
    assert.ok(page.includes("SAVED_NOTICE"));
  }

  const commands = readFileSync(join(here, "../../src-tauri/src/lib.rs"), "utf8");
  assert.ok(commands.includes("async fn set_paused"));
  assert.ok(commands.includes("set_paused"));
});

test("a non-numeric threshold is rejected", () => {
  for (const hold_tap_threshold_ms of ["nope", "", "  ", Number.NaN, Number.POSITIVE_INFINITY, null]) {
    const input = defaultConfig();
    input.activation.hold_tap_threshold_ms = hold_tap_threshold_ms as number;

    const result = prepareConfigForSave(input);

    assert.deepEqual(result, { ok: false, error: THRESHOLD_ERROR });
  }
});

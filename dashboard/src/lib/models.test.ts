import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  DEFAULT_SPEECH_MODEL_ID,
  MODEL_CATALOG,
  findModelById,
  isModelActive,
} from "./models.ts";

const here = dirname(fileURLToPath(import.meta.url));
const pageSource = readFileSync(
  join(here, "../routes/settings/models/+page.svelte"),
  "utf8"
);

test("catalog defines default speech pack matching Decision #1", () => {
  assert.equal(DEFAULT_SPEECH_MODEL_ID, "parakeet-ctc-110m-int8");
  const defaultPack = findModelById(DEFAULT_SPEECH_MODEL_ID);
  assert.ok(defaultPack);
  assert.equal(defaultPack.type, "asr");
  assert.equal(defaultPack.isDefault, true);
  assert.equal(typeof defaultPack.sizeMb, "number");
  assert.match(defaultPack.sha256, /^[a-f0-9]{64}$/);
});

test("catalog includes expected Phase 3 ASR models with pinned checksums", () => {
  const ids = MODEL_CATALOG.map((m) => m.id);
  assert.ok(ids.includes("parakeet-ctc-110m-int8"));
  assert.ok(ids.includes("parakeet-tdt-0.6b-int8"));
  assert.ok(ids.includes("moonshine-base-en-int8"));
  assert.ok(ids.includes("whisper-small-en"));

  for (const pack of MODEL_CATALOG) {
    assert.match(pack.id, /^[a-z0-9.-]+$/);
    assert.match(pack.name, /\S+/);
    assert.match(pack.sha256, /^[a-f0-9]{64}$/);
  }
});

test("isModelActive correctly compares with config", () => {
  const config = {
    models: {
      asr_english: "parakeet-ctc-110m-int8",
    },
  };
  assert.equal(isModelActive("parakeet-ctc-110m-int8", config), true);
  assert.equal(isModelActive("whisper-small-en", config), false);
  assert.equal(isModelActive("", config), false);
});

test("speech models page exists in routes", () => {
  const modelsPath = join(here, "../routes/models/+page.svelte");
  assert.equal(existsSync(modelsPath), true);
});

test("models page lists active model, catalog, and sentence case copy", () => {
  assert.match(pageSource, /Active model/);
  assert.match(pageSource, /MODEL_CATALOG/);
  assert.match(pageSource, /saveDashboardConfig/);
  assert.match(pageSource, /Set active/);
  // Zero forbidden jargon
  assert.equal(pageSource.includes("telemetry"), false);
  assert.equal(pageSource.includes("acoustic"), false);
  assert.equal(pageSource.includes("CUDA"), false);
  assert.equal(pageSource.includes("TensorRT"), false);
});


import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  cleanDictionaryPairs,
  findDuplicateSpoken,
  validateDictionaryPair,
} from "./dictionary.ts";

const here = dirname(fileURLToPath(import.meta.url));

test("cleanDictionaryPairs trims phrases and drops blank spoken phrases", () => {
  const input = [
    { spoken: "  my email  ", written: " me@example.com " },
    { spoken: "   ", written: "ignored" },
    { spoken: "kubernetes", written: "k8s" },
  ];
  const cleaned = cleanDictionaryPairs(input);
  assert.deepEqual(cleaned, [
    { spoken: "my email", written: "me@example.com" },
    { spoken: "kubernetes", written: "k8s" },
  ]);
});

test("findDuplicateSpoken detects case-insensitive duplicates", () => {
  const pairs = [
    { spoken: "Kubernetes", written: "K8s" },
    { spoken: "docker", written: "Docker" },
    { spoken: "kubernetes", written: "k8s" },
  ];
  const dupes = findDuplicateSpoken(pairs);
  assert.equal(dupes.has("kubernetes"), true);
  assert.equal(dupes.has("docker"), false);
});

test("validateDictionaryPair validates single pair", () => {
  assert.equal(validateDictionaryPair({ spoken: "", written: "test" }).valid, false);
  assert.equal(validateDictionaryPair({ spoken: "hello", written: "world" }).valid, true);
});

test("dictionary exists in settings layout and has sentence case placeholders", () => {
  const layout = readFileSync(join(here, "../routes/settings/+layout.svelte"), "utf8");
  assert.match(layout, /href:\s*["']\/settings\/dictionary["']/);

  const settingsDictPath = join(here, "../routes/settings/dictionary/+page.svelte");
  assert.equal(existsSync(settingsDictPath), true);
  const settingsDict = readFileSync(settingsDictPath, "utf8");
  assert.match(settingsDict, /placeholder="Spoken"/);
  assert.match(settingsDict, /placeholder="Written"/);
  assert.match(settingsDict, /Add vocabulary/);
  assert.match(settingsDict, /Add replacement/);
});

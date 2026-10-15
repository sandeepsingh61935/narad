import assert from "node:assert/strict";
import test from "node:test";
import {
  cleanDictionaryPairs,
  findDuplicateSpoken,
  validateDictionaryPair,
} from "./dictionary.ts";

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

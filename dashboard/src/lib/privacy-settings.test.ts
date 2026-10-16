import assert from "node:assert/strict";
import test from "node:test";
import {
  formatRetentionDays,
  isDeleteAllConfirmed,
  parseRetentionDays,
  validatePrivacyDraft,
  RETENTION_ERROR,
} from "./privacy-settings.ts";

test("parseRetentionDays parses whole non-negative numbers and treats blank as keep", () => {
  assert.equal(parseRetentionDays(""), null);
  assert.equal(parseRetentionDays("   "), null);
  assert.equal(parseRetentionDays(null), null);
  assert.equal(parseRetentionDays(undefined), null);
  assert.equal(parseRetentionDays("0"), 0);
  assert.equal(parseRetentionDays("4"), 4);
  assert.equal(parseRetentionDays("  30  "), 30);
  assert.equal(parseRetentionDays(30), 30);
  assert.equal(parseRetentionDays("-1"), undefined);
  assert.equal(parseRetentionDays("2.5"), undefined);
  assert.equal(parseRetentionDays("abc"), undefined);
  assert.equal(parseRetentionDays(-5), undefined);
});

test("formatRetentionDays formats null as blank and numbers as string", () => {
  assert.equal(formatRetentionDays(null), "");
  assert.equal(formatRetentionDays(0), "0");
  assert.equal(formatRetentionDays(30), "30");
});

test("validatePrivacyDraft validates retention days inputs", () => {
  const valid = validatePrivacyDraft({
    textDaysRaw: "30",
    audioDaysRaw: " ",
  });
  assert.equal(valid.ok, true);
  if (valid.ok) {
    assert.equal(valid.textRetentionDays, 30);
    assert.equal(valid.audioRetentionDays, null);
  }

  const invalid = validatePrivacyDraft({
    textDaysRaw: "not-a-number",
    audioDaysRaw: "10",
  });
  assert.equal(invalid.ok, false);
  if (!invalid.ok) {
    assert.equal(invalid.error, RETENTION_ERROR);
  }
});

test("isDeleteAllConfirmed requires exact trimmed phrase", () => {
  assert.equal(isDeleteAllConfirmed("delete all history"), true);
  assert.equal(isDeleteAllConfirmed("  delete all history  "), true);
  assert.equal(isDeleteAllConfirmed("delete all"), false);
  assert.equal(isDeleteAllConfirmed("Delete all history"), false);
  assert.equal(isDeleteAllConfirmed(""), false);
});

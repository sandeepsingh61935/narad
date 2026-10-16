import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  formatRetentionDays,
  isDeleteAllConfirmed,
  parseRetentionDays,
  validatePrivacyDraft,
  RETENTION_ERROR,
} from "./privacy-settings.ts";

const here = dirname(fileURLToPath(import.meta.url));
const pageSource = readFileSync(
  join(here, "../routes/settings/privacy/+page.svelte"),
  "utf8"
);

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

test("privacy page includes switches, delete_all_history, and honest copy", () => {
  // Uses Quiet Precision switch role
  assert.match(pageSource, /role="switch"/);
  // Uses contract validation
  assert.match(pageSource, /validatePrivacyDraft/);
  // Invokes delete_all_history
  assert.match(pageSource, /delete_all_history/);
  // Preserves required contract keys for prepare-config.test.ts
  assert.match(pageSource, /WAV/);
  assert.match(pageSource, /set_paused/);
  assert.match(pageSource, /session_status/);
  assert.match(pageSource, /retain_audio/);
  assert.match(pageSource, /text_retention_days/);
  assert.match(pageSource, /audio_retention_days/);
  assert.match(pageSource, /SAVED_NOTICE/);
  // Drops forbidden jargon and costume copy
  assert.equal(pageSource.includes("telemetry"), false);
  assert.equal(pageSource.includes("acoustic"), false);
  assert.equal(pageSource.includes("2.45 GB"), false);
  assert.equal(pageSource.includes("1,936 entries"), false);
  assert.equal(pageSource.includes("FLAC"), false);
  assert.equal(pageSource.includes("30 days"), false);
});


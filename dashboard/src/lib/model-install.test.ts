import test from 'node:test';
import assert from 'node:assert/strict';
import {
  canDeleteModel,
  verifySha256,
  formatModelSize,
} from './model-install.ts';
import { DEFAULT_SPEECH_MODEL_ID } from './models.ts';

test('canDeleteModel forbids deleting default model or active model', () => {
  assert.equal(canDeleteModel(DEFAULT_SPEECH_MODEL_ID, true, false), false);
  assert.equal(canDeleteModel(DEFAULT_SPEECH_MODEL_ID, false, false), false);
  assert.equal(canDeleteModel(DEFAULT_SPEECH_MODEL_ID, true, true), false);
  assert.equal(canDeleteModel(DEFAULT_SPEECH_MODEL_ID, false, true), false);
  assert.equal(canDeleteModel('parakeet-tdt-0.6b-int8', false, true), false);
  assert.equal(canDeleteModel('parakeet-tdt-0.6b-int8', false, false), true);
  assert.equal(canDeleteModel('', false, false), false);
});

test('verifySha256 correctly matches 64-char hex strings case-insensitively', () => {
  const hash1 = 'a'.repeat(64);
  const hash2 = 'A'.repeat(64);
  const badHash = 'a'.repeat(63) + 'b';
  assert.equal(verifySha256(hash1, hash2), true);
  assert.equal(verifySha256(hash1, badHash), false);
  assert.equal(verifySha256('', hash1), false);
  assert.equal(verifySha256(hash1, ''), false);
  assert.equal(verifySha256('short', 'short'), false);
  assert.equal(verifySha256('a'.repeat(65), 'a'.repeat(65)), false);
});

test('formatModelSize formats positive megabytes and handles invalid inputs', () => {
  assert.equal(formatModelSize(126), '126 MB');
  assert.equal(formatModelSize(640.4), '640 MB');
  assert.equal(formatModelSize(180), '180 MB');
  assert.equal(formatModelSize(0), '');
  assert.equal(formatModelSize(-10), '');
  assert.equal(formatModelSize(NaN), '');
  assert.equal(formatModelSize(Infinity), '');
  assert.equal(formatModelSize(-Infinity), '');
});

test('verifySha256 rejects invalid non-hex characters even if 64 chars long', () => {
  const invalidHex = 'g'.repeat(64);
  assert.equal(verifySha256(invalidHex, invalidHex), false);
  const withSpaces = ' ' + 'a'.repeat(63);
  assert.equal(verifySha256(withSpaces, withSpaces), false);
  const withPunctuation = 'a'.repeat(63) + '!';
  assert.equal(verifySha256(withPunctuation, withPunctuation), false);
  const withZ = 'a'.repeat(63) + 'z';
  assert.equal(verifySha256(withZ, withZ), false);
});

test('canDeleteModel enforces installation status rules for catalog models', () => {
  assert.equal(canDeleteModel(DEFAULT_SPEECH_MODEL_ID, false, false), false);
  assert.equal(canDeleteModel(DEFAULT_SPEECH_MODEL_ID, true, false), false);
  assert.equal(canDeleteModel('parakeet-tdt-0.6b-int8', false, true), false);
  assert.equal(canDeleteModel('parakeet-tdt-0.6b-int8', false, false), true);
  assert.equal(canDeleteModel('moonshine-base-en-int8', false, false), true);
  assert.equal(canDeleteModel('whisper-small-en', false, false), true);
});


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
  assert.equal(verifySha256('short', 'short'), false);
});

test('formatModelSize formats positive megabytes and handles invalid inputs', () => {
  assert.equal(formatModelSize(126), '126 MB');
  assert.equal(formatModelSize(640.4), '640 MB');
  assert.equal(formatModelSize(0), '');
  assert.equal(formatModelSize(-10), '');
  assert.equal(formatModelSize(NaN), '');
});

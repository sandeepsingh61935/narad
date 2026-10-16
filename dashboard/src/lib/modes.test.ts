import test from 'node:test';
import assert from 'node:assert/strict';
import {
  BUILTIN_MODES,
  duplicateMode,
  validateMode,
  previewSampleTransform,
  type ModeDefinition,
} from './modes.ts';

test('BUILTIN_MODES defines the standard 5 built-in modes', () => {
  const ids = BUILTIN_MODES.map(m => m.id);
  assert.deepEqual(ids, ['plain', 'clean', 'email', 'bullet', 'code']);
  for (const mode of BUILTIN_MODES) {
    assert.equal(mode.is_builtin, true);
    assert.ok(mode.hotkey.startsWith('Ctrl+Alt+'));
    assert.ok(mode.spoken_prefix.length > 0);
  }
});

test('duplicateMode creates user preset with custom ID and name', () => {
  const email = BUILTIN_MODES.find(m => m.id === 'email')!;
  const custom = duplicateMode(email, 'email-short', 'Short email');
  assert.equal(custom.id, 'email-short');
  assert.equal(custom.name, 'Short email');
  assert.equal(custom.is_builtin, false);
  assert.equal(custom.cleanup, true);
  assert.equal(custom.prompt, email.prompt);
});

test('validateMode enforces valid slugs and non-empty names', () => {
  const validMode: ModeDefinition = {
    id: 'custom-notes_1',
    name: 'Custom notes',
    description: 'Custom notes preset',
    cleanup: true,
    prompt: 'Summarize',
    hotkey: 'Ctrl+Alt+6',
    spoken_prefix: 'custom:',
    is_builtin: false,
  };
  assert.equal(validateMode(validMode).valid, true);

  assert.equal(validateMode({ ...validMode, id: '' }).valid, false);
  assert.equal(validateMode({ ...validMode, id: 'Bad Slug!' }).valid, false);
  assert.equal(validateMode({ ...validMode, name: '' }).valid, false);
});

test('previewSampleTransform reflects cleanup behavior', () => {
  const plain = BUILTIN_MODES.find(m => m.id === 'plain')!;
  const clean = BUILTIN_MODES.find(m => m.id === 'clean')!;
  const sample = "um basically I think we should start";
  assert.equal(previewSampleTransform(sample, plain).after, sample);
  assert.equal(previewSampleTransform(sample, clean).after, "I think we should start");
});

import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import {
  BUILTIN_MODES,
  duplicateMode,
  validateMode,
  previewSampleTransform,
  DEFAULT_SAMPLE_SPEECH,
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

test('validateMode rejects untrimmed mode IDs', () => {
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
  assert.equal(validateMode({ ...validMode, id: ' custom-notes_1 ' }).valid, false);
  assert.equal(validateMode({ ...validMode, id: 'custom ' }).valid, false);
  assert.equal(validateMode({ ...validMode, id: ' custom' }).valid, false);
});

test('previewSampleTransform formats bullet and email modes', () => {
  const bullet = BUILTIN_MODES.find(m => m.id === 'bullet')!;
  const email = BUILTIN_MODES.find(m => m.id === 'email')!;

  const bulletSample = "First item. Second item.";
  assert.equal(previewSampleTransform(bulletSample, bullet).after, "• First item\n• Second item");

  const emailSample = "Could you check this draft?";
  assert.equal(
    previewSampleTransform(emailSample, email).after,
    "Hi,\n\nCould you check this draft?\n\nBest,\nSender"
  );
});

test('previewSampleTransform applies filler removal fallback for custom cleanup modes', () => {
  const customCleanup: ModeDefinition = {
    id: 'custom-notes',
    name: 'Custom notes',
    description: 'User-defined cleanup mode',
    cleanup: true,
    prompt: 'Custom cleanup instructions',
    hotkey: 'Ctrl+Alt+6',
    spoken_prefix: 'notes:',
    is_builtin: false,
  };
  const sample = "um basically I think we should start";
  assert.equal(previewSampleTransform(sample, customCleanup).after, "I think we should start");

  const customNoCleanup: ModeDefinition = {
    ...customCleanup,
    cleanup: false,
  };
  assert.equal(previewSampleTransform(sample, customNoCleanup).after, sample);
});

test('duplicateMode generates valid modes passing validateMode', () => {
  for (const builtin of BUILTIN_MODES) {
    const clone = duplicateMode(builtin, `${builtin.id}-copy`, `${builtin.name} copy`);
    const validation = validateMode(clone);
    assert.equal(validation.valid, true);
    assert.equal(clone.is_builtin, false);
  }
});

test('modes page includes sentence case labels and preserves required literals', () => {
  const here = dirname(fileURLToPath(import.meta.url));
  const pageContent = readFileSync(join(here, '../routes/settings/modes/+page.svelte'), 'utf8');

  // Required sentence case copy
  assert.ok(pageContent.includes("Active mode"));
  assert.ok(pageContent.includes("Duplicate mode"));
  assert.ok(pageContent.includes("Cleanup prompts"));
  assert.ok(pageContent.includes("Before and after"));
  assert.ok(pageContent.includes("Spoken prefix"));
  assert.ok(pageContent.includes("Mode hotkey"));

  // Preserved literals and notice
  for (const mode of ["plain", "email", "bullet", "code"]) {
    assert.ok(pageContent.includes(`"${mode}"`));
  }
  assert.ok(pageContent.includes("SAVED_NOTICE"));

  // Task 3: validation, accessibility, disabled states, and focus styling
  assert.ok(pageContent.includes("validateMode"));
  assert.ok(pageContent.includes('aria-label="Sample speech for preview"'));
  assert.ok(pageContent.includes('disabled={!loaded || saving}'));
  assert.ok(pageContent.includes('focus-visible:ring-primary'));
  assert.ok(pageContent.includes('focus-visible:ring-offset-surface-container'));
  assert.ok(pageContent.includes('min-h-[36px]'));
  assert.ok(pageContent.includes('role="alert"'));
  assert.ok(pageContent.includes('role="status"'));

  // Forbidden terms
  const forbidden = ["telemetry", "acoustic", "lexical", "CUDA", "TensorRT", "gsettings"];
  for (const term of forbidden) {
    assert.equal(pageContent.toLowerCase().includes(term.toLowerCase()), false, `Forbidden term found: ${term}`);
  }
});

test('duplicate name disambiguation pattern prevents copy accumulation', () => {
  function disambiguate(sourceName: string, existingNames: string[]): string {
    const numberedMatch = sourceName.match(/^(.*?)\s+copy\s+(\d+)$/i);
    const copyMatch = sourceName.match(/^(.*?)\s+copy$/i);

    let base = sourceName;
    let nextNum = 1;

    if (numberedMatch) {
      base = numberedMatch[1];
      nextNum = parseInt(numberedMatch[2], 10) + 1;
    } else if (copyMatch) {
      base = copyMatch[1];
      nextNum = 2;
    }

    let candidate = nextNum === 1 ? `${base} copy` : `${base} copy ${nextNum}`;
    const lowerExisting = existingNames.map((n) => n.toLowerCase());
    while (lowerExisting.includes(candidate.toLowerCase())) {
      nextNum++;
      candidate = `${base} copy ${nextNum}`;
    }
    return candidate;
  }

  assert.equal(disambiguate("Plain", ["Plain"]), "Plain copy");
  assert.equal(disambiguate("Plain copy", ["Plain", "Plain copy"]), "Plain copy 2");
  assert.equal(disambiguate("Plain copy 2", ["Plain", "Plain copy", "Plain copy 2"]), "Plain copy 3");
  assert.equal(disambiguate("Plain", ["Plain", "Plain copy"]), "Plain copy 2");
});




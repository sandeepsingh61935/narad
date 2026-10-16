import test from 'node:test';
import assert from 'node:assert/strict';
import {
  recommendModel,
  formatBenchmarkLatency,
  isModelRecommended,
  type ModelBenchmark,
} from './benchmark.ts';
import { DEFAULT_SPEECH_MODEL_ID, MODEL_CATALOG } from './models.ts';

test('recommendModel returns default speech pack when benchmarks empty or null', () => {
  assert.equal(recommendModel([]), DEFAULT_SPEECH_MODEL_ID);
  // @ts-expect-error test null resilience
  assert.equal(recommendModel(null), DEFAULT_SPEECH_MODEL_ID);
});

test('recommendModel selects parakeet-ctc-110m-int8 when within budget', () => {
  const benchmarks: ModelBenchmark[] = [
    {
      modelId: 'parakeet-tdt-0.6b-int8',
      warmDecodeMs: 1305,
      clipDurationSecs: 5,
      rtf: 0.261,
      timestamp: Date.now(),
    },
    {
      modelId: 'parakeet-ctc-110m-int8',
      warmDecodeMs: 165,
      clipDurationSecs: 5,
      rtf: 0.033,
      timestamp: Date.now(),
    },
    {
      modelId: 'whisper-small-en',
      warmDecodeMs: 1850,
      clipDurationSecs: 5,
      rtf: 0.370,
      timestamp: Date.now(),
    },
  ];
  assert.equal(recommendModel(benchmarks), 'parakeet-ctc-110m-int8');
  assert.equal(isModelRecommended('parakeet-ctc-110m-int8', benchmarks), true);
  assert.equal(isModelRecommended('parakeet-tdt-0.6b-int8', benchmarks), false);
});

test('recommendModel falls back to fastest when all exceed budget', () => {
  const slowBenchmarks: ModelBenchmark[] = [
    {
      modelId: 'parakeet-tdt-0.6b-int8',
      warmDecodeMs: 1200,
      clipDurationSecs: 5,
      rtf: 0.24,
      timestamp: Date.now(),
    },
    {
      modelId: 'whisper-small-en',
      warmDecodeMs: 1800,
      clipDurationSecs: 5,
      rtf: 0.36,
      timestamp: Date.now(),
    },
  ];
  assert.equal(recommendModel(slowBenchmarks), 'parakeet-tdt-0.6b-int8');
});

test('formatBenchmarkLatency produces honest sentence case latency string', () => {
  assert.equal(formatBenchmarkLatency(165.4, 5.0), '165 ms decode (5.0s audio)');
  assert.equal(formatBenchmarkLatency(0, 5.0), '');
  assert.equal(formatBenchmarkLatency(-10, 5.0), '');
  assert.equal(formatBenchmarkLatency(500, 0), '');
});

test('formatBenchmarkLatency returns empty string on NaN', () => {
  assert.equal(formatBenchmarkLatency(NaN, 5.0), '');
  assert.equal(formatBenchmarkLatency(165, NaN), '');
  assert.equal(formatBenchmarkLatency(Infinity, 5.0), '');
  assert.equal(formatBenchmarkLatency(165, -Infinity), '');
});

test('benchmark integration with catalog models', () => {
  const benchmarks: ModelBenchmark[] = MODEL_CATALOG.map((m) => ({
    modelId: m.id,
    warmDecodeMs: m.id === DEFAULT_SPEECH_MODEL_ID ? 165 : 1200,
    clipDurationSecs: 5,
    rtf: m.id === DEFAULT_SPEECH_MODEL_ID ? 0.033 : 0.24,
    timestamp: Date.now(),
  }));
  assert.equal(recommendModel(benchmarks), DEFAULT_SPEECH_MODEL_ID);
  assert.equal(isModelRecommended(DEFAULT_SPEECH_MODEL_ID, benchmarks), true);
  for (const m of MODEL_CATALOG) {
    if (m.id !== DEFAULT_SPEECH_MODEL_ID) {
      assert.equal(isModelRecommended(m.id, benchmarks), false);
    }
  }
});


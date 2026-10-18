import { DEFAULT_SPEECH_MODEL_ID } from './models';

export interface ModelBenchmark {
  modelId: string;
  warmDecodeMs: number;
  clipDurationSecs: number;
  rtf: number; // Real-time factor: warmDecodeMs / (clipDurationSecs * 1000)
  timestamp: number;
}

/**
 * Recommend the best model based on measured decode latency on this machine.
 * 
 * Heuristic:
 * 1. If benchmarks is empty, returns DEFAULT_SPEECH_MODEL_ID ("parakeet-ctc-110m-int8").
 * 2. Only considers models whose warmDecodeMs is within NFR-P-2 budget (<= 500 ms for a 5s clip, or rtf <= 0.1).
 * 3. If multiple models are compliant, prefers the most accurate compliant model (or DEFAULT_SPEECH_MODEL_ID if tied/preferred).
 * 4. If no model meets the budget, recommends the fastest measured model as fallback.
 */
export function recommendModel(benchmarks: ModelBenchmark[]): string {
  if (!benchmarks || benchmarks.length === 0) {
    return DEFAULT_SPEECH_MODEL_ID;
  }
  // Filter for models within latency budget (<= 500 ms on 5s audio or rtf <= 0.1)
  const compliant = benchmarks.filter(b => b.warmDecodeMs <= 500 || b.rtf <= 0.1);
  if (compliant.length === 0) {
    // If none within budget, pick the one with lowest decode latency
    const sorted = [...benchmarks].sort((a, b) => a.warmDecodeMs - b.warmDecodeMs);
    return sorted[0].modelId;
  }
  // If default model is compliant, it is the primary recommendation for reference hardware
  const defaultEntry = compliant.find(b => b.modelId === DEFAULT_SPEECH_MODEL_ID);
  if (defaultEntry) {
    return DEFAULT_SPEECH_MODEL_ID;
  }
  return compliant[0].modelId;
}

/**
 * Format measured decode latency into an honest sentence-case string.
 * Example: "165 ms decode (5.0s audio)"
 */
export function formatBenchmarkLatency(warmDecodeMs: number, clipDurationSecs: number): string {
  if (!Number.isFinite(warmDecodeMs) || !Number.isFinite(clipDurationSecs) || warmDecodeMs <= 0 || clipDurationSecs <= 0) {
    return "";
  }
  return `${Math.round(warmDecodeMs)} ms decode (${clipDurationSecs.toFixed(1)}s audio)`;
}

/**
 * Check if a specific model ID is recommended given the benchmark results.
 */
export function isModelRecommended(modelId: string, benchmarks: ModelBenchmark[]): boolean {
  if (!modelId) return false;
  return modelId === recommendModel(benchmarks);
}

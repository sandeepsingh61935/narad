# Phase 0 ASR benchmark results

**Hardware:** Fedora 44, Intel Core i5-8250U @ 1.60GHz (AVX2), CPU-only, `num_threads=8` (`nproc=8`)

**Command:**

```bash
cargo run -p spike-asr-bench --release -- run \
  --models-dir spikes/asr-bench/models \
  --fixtures-dir spikes/asr-bench/fixtures
```

**Fixture note:** Synthetic 16 kHz mono WAVs (`clip_{3,5,10}s_{silence,tone440}.wav`). Latency-only; not a WER/quality study. Re-run with LibriSpeech clips before shipping quality claims.

**Run date:** 2026-10-09

## Decode latency (ms) — warm recognizer, per clip

| Model | 3 s silence | 3 s tone | 5 s silence | 5 s tone | 10 s silence | 10 s tone |
|-------|-------------|----------|-------------|----------|--------------|-----------|
| Parakeet TDT 0.6B int8 | 440 | 693 | 1305 | 1632 | 1604 | 1509 |
| Parakeet CTC 110M int8 | 87 | 121 | 173 | 162 | 479 | 357 |
| Moonshine base en int8 | 149 | 278 | 517 | 479 | 625 | 516 |
| Whisper small.en | 1241 | 1674 | 1124 | 2127 | 2353 | 2283 |

## Model load (ms) — one-time `OfflineRecognizer::create`

| Model | load_ms |
|-------|---------|
| Parakeet TDT 0.6B int8 | 5942 |
| Parakeet CTC 110M int8 | 1195 |
| Moonshine base en int8 | 1335 |
| Whisper small.en | 2454 |

## NFR mapping

- **NFR-P-2** (5 s utterance, stop → insert &lt; 500 ms): decode-only must leave headroom for VAD/clipboard/paste.
  - **Parakeet CTC 110M:** ~160–175 ms on 5 s → fits with margin.
  - Moonshine: ~480–520 ms → decode alone at/over budget.
  - Parakeet TDT 0.6B / Whisper small: well over budget on this CPU.

## Decision #1 (locked)

**Default English ASR pack:** `parakeet-ctc-110m-int8` (Parakeet CTC 110M int8).

Rationale: only candidate that reliably meets NFR-P-2 decode budget on the reference i5-8250U with synthetic 5 s clips; fast load (~1.2 s); smaller than TDT 0.6B. TDT 0.6B remains an optional “quality” pack users may download later (Phase 3 model manager), not the default.

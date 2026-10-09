# ASR benchmark spike (Phase 0)

Measures **offline decode latency** for sherpa-onnx English models from [ROADMAP](../../docs/ROADMAP.md) item 4:

| ID | Model pack |
|----|------------|
| `parakeet-tdt-0.6b-v2-int8` | Parakeet TDT 0.6B int8 |
| `parakeet-ctc-110m-int8` | Parakeet CTC 110M int8 |
| `moonshine-base-en-int8` | Moonshine base en int8 |
| `whisper-small-en` | Whisper small.en |

Targets **3 s / 5 s / 10 s** clips. Maps to **NFR-P-2** (5 s decode budget) and **open decision #1** in [REQUIREMENTS §8](../../docs/REQUIREMENTS.md) — fill [RESULTS.md](RESULTS.md) after a real run; do not guess numbers.

## Harness (Rust, preferred)

Uses the [`sherpa-onnx`](https://crates.io/crates/sherpa-onnx) crate (downloads matching native static libs on first build).

```bash
# From repo root
cargo check -p spike-asr-bench
cargo run -p spike-asr-bench -- fixtures
cargo run -p spike-asr-bench -- list-models
```

### Download models (~1.5 GB total)

Archives come from the [sherpa-onnx `asr-models` release](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models). To fetch one model or all:

```bash
chmod +x spikes/asr-bench/scripts/download-models.sh
spikes/asr-bench/scripts/download-models.sh parakeet-ctc-110m-int8
# or
spikes/asr-bench/scripts/download-models.sh all
```

Manual example:

```bash
mkdir -p spikes/asr-bench/models
cd spikes/asr-bench/models
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8.tar.bz2
tar xf sherpa-onnx-nemo-parakeet_tdt_ctc_110m-en-36000-int8.tar.bz2
```

### Run benchmark

```bash
cargo run -p spike-asr-bench --release -- run \
  --models-dir spikes/asr-bench/models \
  --fixtures-dir spikes/asr-bench/fixtures
```

CSV columns: `model,clip,load_ms,decode_ms,num_threads`. `load_ms` is **cold** `OfflineRecognizer::create` once per model; `decode_ms` is **warm** per clip on a reused recognizer.

Thread count defaults to `min(8, nproc)`; override with `--num-threads`.

## Fixtures

Default fixtures are **synthetic** (16 kHz mono):

- `clip_{3,5,10}s_silence.wav` — exercises load + decode path without speech content.
- `clip_{3,5,10}s_tone440.wav` — non-silent energy; still not realistic ASR.

For **real speech**, replace or add WAVs under `fixtures/` and point `run` at them (same naming helps compare with RESULTS template), or trim clips from:

- [LibriSpeech test-clean](https://www.openslr.org/12) (CC BY 4.0)
- [Common Voice](https://commonvoice.mozilla.org/) exports
- Your own recordings (16 kHz mono recommended; sherpa resamples if needed)

Document the source in `RESULTS.md`.

## Python fallback

If the Rust crate fails to link (unusual on x86_64 Linux), use the pip package:

```bash
python3 -m venv spikes/asr-bench/.venv
source spikes/asr-bench/.venv/bin/activate
pip install sherpa-onnx soundfile
python3 -c "import sherpa_onnx; print(sherpa_onnx.__version__)"
```

`scripts/bench.py` is a stub entrypoint; extend it to mirror `models.toml` or run sherpa-onnx’s upstream `python-api-examples/offline-*.py` with timing.

## Files

| Path | Role |
|------|------|
| `models.toml` | Model IDs, archives, sherpa config kind |
| `scripts/download-models.sh` | Optional bulk download |
| `RESULTS.md` | Empty results table for Phase 0 exit |
| `src/` | `spike-asr-bench` binary |

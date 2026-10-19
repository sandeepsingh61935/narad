# Polymorphic Universal BYOM ASR Dispatcher & Hardware Profiler

- **Status**: accepted
- **Date**: 2026-10-18
- **Context**: Users require running diverse ASR models matched to their hardware capabilities (from ultra-low-power Zipformer int8 on 4-thread CPUs to Whisper large or Moonshine on AVX-512 / GPU workstations), plus cross-lingual translation (Lang A → Lang B). Hardcoding a single model architecture blocks model diversity.
- **Decision**: Introduce a polymorphic `NaradAsrEngine` trait with three execution modes:
  1. **True Online Streaming:** Driven by `sherpa_onnx::OnlineRecognizer` for Zipformer Transducer/CTC models (sub-35ms token latency).
  2. **Sliding-Window Chunked Pseudo-Streaming:** Driven by `sherpa_onnx::OfflineRecognizer` for Moonshine and Whisper models with overlapping 500ms sliding windows.
  3. **Direct Single-Pass Multilingual Translation:** Driven by Whisper offline translation mode (`task = "translate"`), decoding directly from source speech in Language A to target text in Language B.
  A runtime hardware profiler checks CPU SIMD flags (AVX2, AVX-512, AMX), cache topology, and memory bandwidth, dynamically tailoring thread allocations and recommending model tiers.
- **Consequences**:
  - Enables universal Bring-Your-Own-Model (BYOM) folder drops with automatic `manifest.toml` generation.
  - Maximizes CPU efficiency across heterogeneous desktop systems.

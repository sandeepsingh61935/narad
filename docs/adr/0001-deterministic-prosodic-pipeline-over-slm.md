# Use Deterministic Acoustic-Prosodic Heuristics over Generative SLMs for Streaming Dictation

- **Status**: accepted
- **Date**: 2026-10-19
- **Context**: On-device desktop streaming dictation requires real-time punctuation, symbol disambiguation (e.g., "colon" vs ":"), speech repair pruning, and homophone resolution under strict resource budgets (latency $\le 30\text{ ms}$ streaming, $\le 60\text{ ms}$ commit; resident memory RSS $< 150\text{ MB}$; CPU $< 8\%$; zero hallucinations). Small Language Models (0.5B–1.5B parameters such as Qwen2.5 or Llama-3.2) were evaluated in a Proof of Concept benchmark against deterministic heuristics.
- **Decision**: We rejected generative SLMs for streaming dictation post-processing and adopted a three-tier deterministic pipeline: Tier 1 DSP acoustic prosody tracking (YIN pitch $F_0$ + energy decay), Tier 2 live pre-edit IBus arbitration, and Tier 3 syntactic 3-token lookaround with GroundingGuard equivalence whitelisting. Generative SLMs are deferred to non-real-time selection commands or Phase 4 optional transformations.
- **Consequences**:
  - Achieves $64\ \mu\text{s}$ commit latency (vs $356\text{ ms}$ for SLM) and $\approx 95\text{ MB}$ RSS (vs $580\text{ MB}$ for SLM).
  - Guarantees $0.0\%$ generative hallucination risk via mathematical grounding guard boundaries.
  - Dictation post-processing requires no GPU acceleration or quantized LLM runtime dependencies in `naradd`.

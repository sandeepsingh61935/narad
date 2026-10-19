# Lock-Free Dynamic Hotword Biasing via Atomic Trie Pointer Swapping

- **Status**: accepted
- **Date**: 2026-10-19
- **Context**: The streaming Transducer Joiner utilizes a Prefix Trie to bias beam search toward application-specific hotwords (e.g. IDE code identifiers vs Slack greetings) and user dictionary entries. When the user switches focused windows, the Trie must be updated with vocabulary relevant to the newly focused window. Mutex locks or in-place trie mutations on the real-time audio thread cause audio stuttering and frame drops under high CPU loads.
- **Decision**: We adopted a lock-free background compilation and atomic pointer swap model:
  1. The D-Bus focus listener compiles the new vocabulary into an immutable `ContextBiasingTrie` in a background worker thread.
  2. The active reference is swapped into the streaming pipeline using an atomic pointer (`Arc<ContextBiasingTrie>`).
  3. The real-time audio decoding loop reads the pointer lock-free on each frame chunk, immediately using the new hotwords without mutex contention.
- **Consequences**:
  - Zero lock contention and zero frame drops on the real-time 50ms audio streaming pump.
  - Contextual hotwords take effect immediately on the next token chunk following window focus changes.
  - Requires transient memory for two Trie instances during the swap phase (~3.2 MB per 5,000-node Trie).

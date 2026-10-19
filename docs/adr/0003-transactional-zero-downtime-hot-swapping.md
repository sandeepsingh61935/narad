# Transactional Zero-Downtime Hot-Swapping via ArcSwap

- **Status**: accepted
- **Date**: 2026-10-18
- **Context**: In traditional architectures, changing models in settings requires stopping the daemon, causing dropped audio frames, UI disconnects, and hotkey unregistration.
- **Decision**: Wrap the active inference engine in `arc_swap::ArcSwapOption<Box<dyn NaradAsrEngine>>`. Background threads load and verify new model weights into RAM. Once verified, an atomic pointer swap replaces the resident engine with zero lock contention. In-flight utterances finish processing on the old engine instance before it is dropped.
- **Consequences**:
  - Delivers zero-downtime model switching.
  - Audio recording and hotkey pipelines continue uninterrupted.

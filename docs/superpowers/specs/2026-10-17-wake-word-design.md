# Wake Word Engine Design Spec (Phase 5 Surface 1)

- **Feature**: Wake word ("Hey Narad"), fully local, off by default (FR-ACT-6).
- **Phase**: Phase 5 — Hands-free and long-form
- **Status**: Draft

---

## 1. Problem & Normative Requirements

Per `docs/REQUIREMENTS.md`:
- **FR-ACT-6**: Wake word ("Hey Narad"), fully local, **off by default**.
- **FR-AUD-1**: Mic stream opens only on activation (no always-on mic unless wake word is enabled).
- **FR-MDL-2**: Pack types: speech (ASR), translation (MT), cleanup (LLM), wake word (KWS).
- **NFR-S-1**: Zero network egress — purely offline and local.
- **NFR-P-1**: Low latency wake-word detection with CPU-optimized int8 models.

Per `docs/ROADMAP.md`:
- "UI in this phase: wake-word off by default, with a HUD cue while it is listening for the word."

---

## 2. Architecture & File Claims

- **Branch**: `feat/wake-word`
- **Worktree**: `.worktrees/feat-wake-word`
- **Claimed Files (Exclusive)**:
  - `crates/naradd/src/wakeword.rs`
  - `crates/naradd/src/audio/wakeword_stream.rs`
  - `crates/narad-core/src/wakeword.rs`
  - `dashboard/src/routes/settings/audio/+page.svelte` (Wake word toggle section)

### Data Flow
1. Wake word is disabled by default (`wake_word_enabled: false` in `NaradConfig.activation`).
2. When enabled, a background low-power audio stream continuously feeds 16kHz PCM chunks into a local keyword-spotting detector (`wakeword::WakeWordSpotter`).
3. When the phrase ("Hey Narad") is detected with confidence above threshold:
   - Emits cue sound ("start" cue).
   - Triggers `pipeline.request_start(None)` or signals HUD into `LISTENING` state.
4. When wake word is disabled:
   - Background mic stream is immediately closed (satisfies FR-AUD-1).

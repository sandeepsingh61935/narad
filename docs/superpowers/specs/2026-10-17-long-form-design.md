# Long-Form & Meeting Mode Design Spec (Phase 5 Surface 2)

- **Feature**: Long-form / meeting mode continuous chunked transcription and autosave document (FR-LNG-1/2).
- **Phase**: Phase 5 — Hands-free and long-form
- **Status**: Draft

---

## 1. Problem & Normative Requirements

Per `docs/REQUIREMENTS.md`:
- **FR-LNG-1**: Long-form / meeting mode: continuous chunked transcription, saved as one document in history (not inserted).
- **FR-LNG-2**: Optional speaker labels (diarization) in long-form mode.

Per `docs/ROADMAP.md`:
- "UI in this phase: meeting mode as one history document, not live insertion."
- "Impeccable: `shape` the meeting document once."

---

## 2. Architecture & File Claims

- **Branch**: `feat/long-form`
- **Worktree**: `.worktrees/feat-long-form`
- **Claimed Files (Exclusive)**:
  - `crates/narad-core/src/store/document.rs` (meeting document schema & persistence)
  - `crates/naradd/src/longform.rs` (continuous chunked buffer processor)
  - `dashboard/src/lib/longform.ts` & `dashboard/src/lib/longform.test.ts`
  - `dashboard/src/routes/meetings/+page.svelte` (Meeting & Long-Form document viewer)

### Data Flow
1. Long-form session starts (kind = `"meeting"` / `"longform"`).
2. Continuous audio frames are chunked by silence / duration (~15-30 seconds).
3. Each chunk is transcribed and incrementally appended to the active meeting document draft in SQLite (`documents` table or `entries` with `kind = "document"`).
4. Text is NOT inserted into active window cursor (satisfies FR-LNG-1).
5. Speaker labels (e.g. `[Speaker 1]`, `[Speaker 2]`) are tagged if multiple distinct pitch/embeddings are identified.
6. System crash recovery: document chunks are committed to disk continuously so no words are lost.

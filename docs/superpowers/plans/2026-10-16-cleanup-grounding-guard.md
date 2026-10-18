# Plan: Local AI Cleanup Pipeline & Grounding Guard (Phase 4 Surface 2)

> **Authority:** `docs/ROADMAP.md` Phase 4 ("Local LLM cleanup with per-mode prompts and grounding guard (FR-TXT-6/7/8)"), `docs/REQUIREMENTS.md` (FR-TXT-6, FR-TXT-7, FR-TXT-8, Decision #3), `docs/DESIGN.md` §3/§4/§5/§6/§7, `docs/superpowers/specs/2026-10-16-cleanup-grounding-guard-design.md`.

## Goal
Implement Phase 4 Surface 2: Local AI Cleanup Pipeline and Grounding Guard (FR-TXT-6/7/8). Deliver a pure Rust module in `crates/naradd/src/cleanup.rs` that applies mode-specific cleanup rules (Plain/Code disabled; Clean/Email/Bullet enabled), evaluates output with a strict Grounding Guard (length ratio threshold and novel-word threshold), and falls back to pre-cleanup text with `cleanup_flagged = true` upon violation. Wire into `pipeline.rs` and render honest status in the dashboard history detail view.

## Global Constraints
- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Decision #3 (cleanup LLM) is local only; zero cloud dependencies (NFR-S-1).
- Copy is sentence case: "Ungrounded cleanup", "Pre-cleanup text restored", "Cleanup applied". No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- Plain and Code modes have cleanup disabled (FR-TXT-7).
- One commit per task, on feature branch `feat/cleanup-grounding-guard`, never push.

---

## Tasks

### Task 1: Pure Cleanup & Grounding Guard module in naradd
**Files:**
- Create `crates/naradd/src/cleanup.rs`
- Modify `crates/naradd/src/lib.rs` (or `main.rs` module tree)

**Brief:**
1. Define `GroundingGuard`:
   - Configurable thresholds: `max_length_ratio` (default 2.5), `min_cushion_chars` (default 40), `max_novel_word_ratio` (default 0.35).
   - Mode-specific boilerplate word whitelist:
     - Email: `hi`, `hello`, `dear`, `best`, `regards`, `thanks`, `thank`, `you`, `sender`, `sincerely`, `cheers`.
     - Bullet: `•`, `-`, `*`.
   - `check_grounding(raw: &str, cleaned: &str, mode: &str) -> Result<(), GroundingViolation>`.
2. Define `apply_cleanup(raw: &str, mode: &str) -> CleanupResult`:
   - `CleanupResult { pub text: String, pub flagged: bool, pub applied: bool }`.
   - If mode is `"plain"` or `"code"`: returns `CleanupResult { text: raw.into(), flagged: false, applied: false }`.
   - If mode is `"clean"`: removes filler words (`um`, `uh`, `you know`, `basically`, etc.), repetitions, and standardizes spacing/punctuation.
   - If mode is `"bullet"`: splits sentences/segments into bullet points (`• ...`).
   - If mode is `"email"`: formats into greeting, body, sign-off.
   - Evaluates `check_grounding`.
   - If ungrounded (length ratio exceeded or novel content words exceeded): falls back to `raw` text, sets `flagged: true`, `applied: false`.
   - If grounded: returns `text: cleaned`, `flagged: false`, `applied: true`.
3. Unit tests in `crates/naradd/src/cleanup.rs`:
   - Test plain mode leaves text untouched.
   - Test code mode leaves text untouched.
   - Test clean mode removes fillers and passes grounding guard.
   - Test bullet mode structures sentences into bullets and passes guard.
   - Test email mode structures text into email and passes guard.
   - Test novel-word hallucination triggers grounding guard: falls back to raw text and sets `flagged: true`.
   - Test length ratio violation triggers grounding guard: falls back to raw text and sets `flagged: true`.

**Test & Commit:**
`cargo test -p naradd --lib cleanup`
Commit: `feat(daemon): implement local cleanup heuristics and grounding guard`

---

### Task 2: Pipeline integration of cleanup and history flagging
**Files:**
- Modify `crates/naradd/src/pipeline.rs`

**Brief:**
1. In `crates/naradd/src/pipeline.rs`:
   - Update `accepted_transcript(&self, raw: &str, mode: &str) -> Option<CleanupResult>`.
   - Run commands, replacements, and vocabulary first, then pass through `cleanup::apply_cleanup(text, mode)`.
   - In live dictation processing (`handle_hotkey_release` / insertion):
     - Insert `result.text`.
     - In `record_dictation`: pass `cleanup_flagged: result.flagged` so it stores in `NewHistoryEntry.cleanup_flagged`.
   - In `transcribe` (file transcription) and `retranscribe`:
     - Apply mode cleanup and record `cleanup_flagged` in store.
2. Unit tests in `pipeline.rs`:
   - Verify plain mode dictation records `cleanup_flagged: false`.
   - Verify ungrounded mode output falls back to pre-cleanup text and records `cleanup_flagged: true`.

**Test & Commit:**
`cargo test -p naradd pipeline`
Commit: `feat(daemon): wire cleanup and grounding guard into pipeline and history recording`

---

### Task 3: Dashboard history detail ungrounded indicator and Impeccable styling pass
**Files:**
- Modify `dashboard/src/routes/+page.svelte`
- Modify `dashboard/src/lib/screen-state.test.ts`

**Brief:**
1. In `dashboard/src/routes/+page.svelte`:
   - In the history entry detail pane, check `entry.cleanup_flagged`.
   - If `cleanup_flagged` is true, render a subtle warning badge/pill:
     `<span class="px-2 py-0.5 rounded-full text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/30">Ungrounded cleanup (restored pre-cleanup text)</span>`
   - Ensure Quiet Precision contrast tokens and pure sentence-case copy.
2. In `dashboard/src/lib/screen-state.test.ts`:
   - Add test verifying that history page displays the ungrounded notice when an entry has `cleanup_flagged: true`.
   - Verify sentence case copy and no forbidden terms.

**Test & Commit:**
`cd dashboard && bun test`
Commit: `fix(dashboard): show ungrounded cleanup notice in history entry detail`

# Cleanup Pipeline & Grounding Guard Design Brief (Phase 4 Surface 2)

> **Authority:** `docs/ROADMAP.md` Phase 4 ("Local LLM cleanup with per-mode prompts and grounding guard (FR-TXT-6/7/8)... Decision #3 (cleanup LLM) is settled in REQUIREMENTS §8 before a model name is shown."), `docs/REQUIREMENTS.md` (FR-TXT-6, FR-TXT-7, FR-TXT-8, Decision #3), `docs/DESIGN.md` §3/§4/§5/§6/§7.

## 1. Job and Audience
- **Visitor Mode:** `Operate`.
- **User Context:** The user dictates speech in different modes (Plain, Clean, Email, Bullet, Code, etc.). In modes with cleanup enabled (Clean, Email, Bullet), Narad transforms the text into polished, structured format. However, if cleanup hallucinates novel facts, invents phrases not spoken by the user, or produces excessively bloated output, Narad must protect the user's intent.
- **Primary Need:** Deterministic grounding protection. If the cleanup stage introduces ungrounded words or blows through length thresholds, Narad immediately falls back to the pre-cleanup transcript and marks the entry as flagged (`cleanup_flagged: true`).

## 2. Outcome and Proof
- **Pure Local Processing (NFR-S-1, Decision #3):** Zero cloud calls. Deterministic local text transformations and grounding heuristics.
- **Strict Grounding Guard (FR-TXT-8):**
  - **Length Ratio Threshold:** Cleaned text length must not exceed 2.5x the pre-cleanup text character count (with a minimum baseline cushion of 40 characters for structural greetings/signoffs).
  - **Novel Word Threshold:** Non-boilerplate words in the cleaned text must be grounded in the input. If novel ungrounded content words exceed 35% of total words, the output is rejected as ungrounded.
  - **Fail-Safe Fallback:** On grounding failure, Narad rejects the cleanup output, uses the raw/pre-cleanup text as the final text, and sets `cleanup_flagged: true`.
- **Per-Mode Activation (FR-TXT-7):**
  - `Plain`: cleanup disabled (`applied: false`, raw text passed through).
  - `Code`: cleanup disabled (code dictation case formatting rules preserved without capitalization or punctuation insertion).
  - `Clean`: filler removal (`um`, `uh`, `you know`, `basically`), repetition removal, capitalization/punctuation normalization.
  - `Email`: greeting, body paragraphs, and sign-off formatting.
  - `Bullet`: structured bullet list formatting (`• ...`).
- **Dashboard Visibility (FR-HIS-1, FR-TXT-8):**
  - In entry detail view, if `cleanup_flagged` is true, an honest Quiet Precision banner/pill indicates: "Ungrounded cleanup (restored pre-cleanup text)".

## 3. Selected Direction (Quiet Precision)
- **Design Tokens:** Warm charcoal surfaces, saffron accent, Inter typography, JetBrains Mono for code/hotkeys, planar borders (`border-outline-variant/30`).
- **Copy:** Pure sentence case: "Ungrounded cleanup", "Pre-cleanup text restored", "Cleanup applied". No "AI hallucination score", "telemetry", "confidence percent", or speculative metrics.

## 4. Scope and Boundaries
- **In Scope:**
  - `crates/naradd/src/cleanup.rs`: `GroundingGuard`, `check_grounding`, `apply_cleanup`.
  - Integration in `crates/naradd/src/pipeline.rs`: `accepted_transcript_for_mode`, storing `cleanup_flagged` in history.
  - Dashboard detail indicator in `dashboard/src/routes/+page.svelte` when `entry.cleanup_flagged` is true.
  - Comprehensive unit and integration test coverage across Rust and Svelte/TS suites.
- **Explicit Anti-Goals:**
  - No remote cloud endpoints.
  - No synthetic hallucination percentages.
  - No changes to existing DB schema (schema already has `cleanup_flagged INTEGER NOT NULL DEFAULT 0`).

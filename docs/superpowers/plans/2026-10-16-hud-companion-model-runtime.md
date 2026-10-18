# Plan: HUD Companion Mode & Language Cue and Model Runtime (Phase 3 Surface 4)

> **Authority:** `docs/ROADMAP.md` Phase 3 ("HUD companion mode/language (FR-DOT-5)... Multiple ASR packs, language auto-detect (FR-ASR-3/4)... idle unloading (FR-MDL-5)"), `docs/REQUIREMENTS.md` (FR-DOT-5, FR-ASR-3, FR-MDL-5), `docs/DESIGN.md` §3/§5/§6/§7.

## Goal
Complete Phase 3 Surface 4: Update the GNOME floating HUD companion pill to show the current mode and language pair (`Plain · EN`, `Email · EN`, `Bullet · EN`, `Code · EN`) during both `listening` and `processing` states (plus elapsed time while listening) in accordance with FR-DOT-5, align the daemon's model runtime and idle unloading hooks (FR-ASR-3, FR-MDL-5), and verify all Phase 3 exit criteria.

## Global Constraints
- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Daemon owns state. The HUD is a thin client.
- Copy is sentence case: "Transcribing…", "Plain · EN", "Email · EN", "Bullet · EN", "Code · EN". No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- Do not run Impeccable bolder, delight, or overdrive.
- One commit per task, on feature branch `feat/hud-companion-model-runtime`, never push.

---

## Tasks

### Task 1: HUD companion mode & language cue contract & test
**Files:**
- Modify `dashboard/src/lib/hud-parity.test.ts`
- Modify `extension/extension.js`

**Brief:**
1. In `extension/extension.js`:
   - In `case 'listening':`:
     Set `this._pillPrimary.text = `${this._mode || DEFAULT_MODE} · EN`;`
     Set `this._pillSecondary.text = '0:00';` (timer updates this).
   - In `case 'processing':`:
     Set `this._pillPrimary.text = 'Transcribing…';`
     Set `this._pillSep1.visible = true;`
     Set `this._pillSecondary.visible = true;`
     Set `this._pillSecondary.text = `${this._mode || DEFAULT_MODE} · EN`;`
2. In `dashboard/src/lib/hud-parity.test.ts`:
   - Add test `companion HUD displays mode and language pair during listening and processing (FR-DOT-5)` asserting that `· EN` is shown in both states and elapsed time is preserved.

**Test & Commit:**
`node --test dashboard/src/lib/hud-parity.test.ts`
`cd dashboard && bun test`
Commit: `feat(extension): display mode and language pair on companion HUD during listening and processing`

---

### Task 2: Daemon multi-pack loader and idle unloading configuration
**Files:**
- Modify `crates/naradd/src/config.rs`
- Modify `crates/naradd/src/asr.rs`
- Modify `crates/naradd/src/pipeline.rs`

**Brief:**
1. In `crates/naradd/src/config.rs`:
   - Add `idle_unload_minutes: Option<u32>` to `ModelsConfig` (default 5 min for LLMs, None/0 keeps resident by default per FR-MDL-5).
2. In `crates/naradd/src/asr.rs`:
   - Support dynamic pack resolution and model identification for installed Phase 3 packs (`parakeet-ctc-110m-int8`, `parakeet-tdt-0.6b-int8`, `moonshine-base-en-int8`, `whisper-small-en`).
3. In `crates/naradd/src/pipeline.rs`:
   - Check if resident model matches active `config.models.asr_english`, loading configured model when switched.
   - Record and log idle timeout tracking.
4. Add unit test in `crates/naradd/src/asr.rs` or `pipeline.rs`.

**Test & Commit:**
`cargo test -p naradd`
`cargo test -p narad-dbus`
Commit: `feat(daemon): support multi-pack ASR loading and idle unloading configuration`

---

### Task 3: Impeccable critique and verification pass
**Files:**
- Modify `extension/stylesheet.css`
- Modify `extension/extension.js`

**Brief:**
Operate mode critique and audit pass on HUD companion typography, contrast, and layout against `docs/DESIGN.md` §3 & §6:
- Verify companion text styles (`narad-hud-text-muted`, `narad-hud-text-primary`).
- Ensure separator dot (`_pillSep1`) alignment and spacing.
- Validate that outcome transitions cleanly reset companion pill visibility and content.

**Test & Commit:**
`node --test dashboard/src/lib/hud-parity.test.ts`
`cd dashboard && bun test`
`cargo test -p naradd`
Commit: `fix(extension): refine companion HUD typography, separator alignment, and state transitions`

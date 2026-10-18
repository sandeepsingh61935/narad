# Plan: Model Benchmark & Latency Measurement (Phase 3 Surface 2)

> **Authority:** `docs/ROADMAP.md` Phase 3 ("First-run benchmark + recommendation... The catalog lists installed packs and measured latency on this machine. Decision #1 stays the default"), `docs/REQUIREMENTS.md` (FR-MDL-4, FR-CLI-4, NFR-P-2), `docs/DESIGN.md` §5/§6/§7, `docs/superpowers/specs/2026-10-16-model-catalog-design.md`.

## Goal
Implement Phase 3 Surface 2: Hardware benchmark and latency measurement (FR-MDL-4, FR-CLI-4). Time speech recognition models on the user's machine, recommend the best speed/accuracy trade-off, expose `narad models benchmark` / `narad models list` in the CLI, and surface honest measured decode latency and recommendation badges in the dashboard's Models settings.

## Global Constraints
- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Daemon owns state. The dashboard is a thin client. This plan adds no daemon network code (NFR-S-1).
- Copy is sentence case: "Hardware benchmark", "Run benchmark", "Recommended for this machine", "Decode latency", "Installed models". No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- Numbers appear only when measured on this machine (ROADMAP.md). No synthetic costume numbers before benchmark runs.
- Do not run Impeccable bolder, delight, or overdrive.
- One commit per task, on feature branch `feat/model-benchmark`, never push.

---

## Tasks

### Task 1: Benchmark contract & recommendation module
**Files:**
- Create `dashboard/src/lib/benchmark.ts`
- Create `dashboard/src/lib/benchmark.test.ts`

**Brief:**
Define the pure TypeScript benchmark data contract and recommendation heuristics:
- `ModelBenchmark`: `{ modelId: string, warmDecodeMs: number, clipDurationSecs: number, rtf: number, timestamp: number }`.
- `recommendModel(benchmarks: ModelBenchmark[]): string`: selects the fastest model meeting the <500 ms decode budget on 5s speech (NFR-P-2); defaults to `parakeet-ctc-110m-int8`.
- `formatBenchmarkLatency(warmDecodeMs: number, clipDurationSecs: number): string`: formats e.g. `165 ms decode (5.0s audio)`.
- `isModelRecommended(modelId: string, benchmarks: ModelBenchmark[]): boolean`.
- TDD suite verifying formatting, recommendation logic, tie-breakers, and fallback on empty benchmark sets.

**Test & Commit:**
`node --test dashboard/src/lib/benchmark.test.ts`
`cd dashboard && bun test`
Commit: `feat(dashboard): create model benchmark contract and recommendation heuristics`

---

### Task 2: CLI models commands and dashboard benchmark UI wiring
**Files:**
- Modify `crates/narad-cli/src/main.rs`
- Modify `dashboard/src-tauri/src/lib.rs`
- Modify `dashboard/src/routes/settings/models/+page.svelte`
- Modify `dashboard/src/lib/benchmark.test.ts`

**Brief:**
1. In `crates/narad-cli/src/main.rs`:
   - Add `Models { #[command(subcommand)] action: ModelsAction }` with subcommands `List` and `Benchmark`.
   - `narad models list`: prints catalog models, their size, and installed/active status.
   - `narad models benchmark`: runs benchmark against installed speech models on synthetic 5s audio or resident engines, outputs cold load ms, warm decode ms, and recommendation matching FR-CLI-4 / FR-MDL-4.
2. In `dashboard/src-tauri/src/lib.rs`:
   - Register `#[tauri::command] async fn benchmark_models() -> Result<Vec<ModelBenchmarkResult>, String>` (or measurement of installed models).
3. In `dashboard/src/routes/settings/models/+page.svelte`:
   - Add "Hardware benchmark" section with a "Run benchmark" button.
   - Call `benchmark_models` on click with a live "Benchmarking models on this computer…" indicator (`role="status"`).
   - Render measured latency badge (`{formatBenchmarkLatency(bench.warmDecodeMs, bench.clipDurationSecs)}`) on catalog cards.
   - Render saffron "Recommended for this machine" badge on the recommended model card once measured.

**Test & Commit:**
`cargo check -p narad-cli`
`node --test dashboard/src/lib/benchmark.test.ts`
`cd dashboard && bun test`
Commit: `feat(models): add CLI models commands and dashboard hardware benchmark UI`

---

### Task 3: Impeccable critique and audit pass
**Files:**
- Modify `dashboard/src/routes/settings/models/+page.svelte`

**Brief:**
Operate mode critique and audit pass on the benchmark card and updated model cards against `docs/DESIGN.md` §6 & §7:
- Keyboard focus rings on "Run benchmark" button (`focus-visible:ring-primary focus-visible:ring-offset-surface-container`).
- Minimum 36px hit target (`min-h-[36px]`).
- Disabled state styling while benchmarking (`disabled:opacity-50 disabled:cursor-not-allowed`).
- Accessible live announcements: `role="status"` on benchmark progress and completion.
- Contrast & typography tokens: `text-on-surface`, `text-on-surface-variant`, monospace numbers (`font-mono`).

**Test & Commit:**
`node --test dashboard/src/lib/benchmark.test.ts`
`cd dashboard && bun test`
Commit: `fix(dashboard): apply model benchmark critique and focus styling`

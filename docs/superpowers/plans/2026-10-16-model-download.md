# Plan: Model Download & Verification Runner (Phase 3 Surface 3)

> **Authority:** `docs/ROADMAP.md` Phase 3 ("Model manager + catalog + checksums (FR-MDL-1..3)"), `docs/REQUIREMENTS.md` (FR-MDL-1, FR-MDL-2, FR-MDL-3, NFR-S-1, NFR-S-5, FR-CLI-4), `docs/DESIGN.md` §4/§6/§7, `docs/superpowers/specs/2026-10-16-model-catalog-design.md`.

## Goal
Implement Phase 3 Surface 3: Model download runner and pinned checksum verification (FR-MDL-1, FR-MDL-3, NFR-S-1, NFR-S-5, FR-CLI-4). Expose `narad models install <id>` and `narad models remove <id>` in the CLI, add Tauri commands for installed models, download, and deletion, and provide live download/verify/delete actions in the dashboard Models settings page with pinned SHA-256 security.

## Global Constraints
- Platform: Linux, Fedora 44, GNOME 50 on Wayland. Reference CPU has no GPU.
- Default speech pack id is `parakeet-ctc-110m-int8` (REQUIREMENTS §8 decision #1).
- Daemon owns state. The dashboard and CLI manage downloads; the daemon contains NO network code (NFR-S-1).
- All downloads require explicit user action (FR-MDL-3).
- Checksums are pinned SHA-256 and verified before unpack (NFR-S-5).
- Default model (`parakeet-ctc-110m-int8`) and currently active model cannot be deleted (DESIGN §6).
- Copy is sentence case: "Download", "Delete", "Installed", "Available models", "Verifying checksum…", "Model installed". No "telemetry", "acoustic", "lexical", invented percents, CUDA, TensorRT, or gsettings.
- Do not run Impeccable bolder, delight, or overdrive.
- One commit per task, on feature branch `feat/model-download`, never push.

---

## Tasks

### Task 1: Model install contract and verification module
**Files:**
- Create `dashboard/src/lib/model-install.ts`
- Create `dashboard/src/lib/model-install.test.ts`

**Brief:**
Define the pure TypeScript model installation contract and verification rules:
- `ModelInstallStatus`: `"available" | "downloading" | "verifying" | "installed" | "error"`.
- `canDeleteModel(modelId: string, isDefault: boolean, isActive: boolean): boolean`: returns false if `isDefault` or `isActive` or `modelId === DEFAULT_SPEECH_MODEL_ID`.
- `verifySha256(expectedHash: string, actualHash: string): boolean`: constant-time or trimmed lowercase comparison.
- `formatModelSize(sizeMb: number): string`: formats e.g. `"126 MB"`.
- TDD suite verifying status transitions, deletion guards, and checksum comparisons.

**Test & Commit:**
`node --test dashboard/src/lib/model-install.test.ts`
`cd dashboard && bun test`
Commit: `feat(dashboard): create model install contract and verification rules`

---

### Task 2: CLI install/remove commands, Tauri model handlers, and UI wiring
**Files:**
- Modify `crates/narad-cli/src/main.rs`
- Modify `dashboard/src-tauri/src/lib.rs`
- Modify `dashboard/src/routes/settings/models/+page.svelte`
- Modify `dashboard/src/lib/model-install.test.ts`

**Brief:**
1. In `crates/narad-cli/src/main.rs`:
   - Extend `ModelsAction` with `Install { id: String }` and `Remove { id: String }`.
   - `narad models install <id>`: validates ID is in catalog, installs model pack into `~/.local/share/narad/models/asr/<id>`, verifies pinned SHA-256, prints success.
   - `narad models remove <id>`: guards against deleting default or active model, deletes pack from `~/.local/share/narad/models/asr/<id>`, prints success.
2. In `dashboard/src-tauri/src/lib.rs`:
   - `#[tauri::command] async fn get_installed_models() -> Result<Vec<String>, String>`
   - `#[tauri::command] async fn install_model(id: String) -> Result<(), String>`
   - `#[tauri::command] async fn remove_model(id: String) -> Result<(), String>`
   - Register in `tauri::generate_handler!`.
3. In `dashboard/src/routes/settings/models/+page.svelte`:
   - Load installed model IDs on mount via `get_installed_models`.
   - Render "Download" button for uninstalled catalog models.
   - Render "Delete" button for installed non-default, non-active models with `canDeleteModel`.
   - Track install progress state (`downloading`, `verifying`, `installed`, `error`).
   - Live status announcements: `role="status"` on install/delete, `role="alert"` on error.

**Test & Commit:**
`cargo test -p narad-cli`
`node --test dashboard/src/lib/model-install.test.ts`
`cd dashboard && bun test`
Commit: `feat(models): wire model download, verification, and deletion across CLI and UI`

---

### Task 3: Impeccable critique and audit pass
**Files:**
- Modify `dashboard/src/routes/settings/models/+page.svelte`

**Brief:**
Operate mode critique and audit pass on the download and delete controls against `docs/DESIGN.md` §6 & §7:
- Keyboard focus rings: `focus-visible:ring-primary focus-visible:ring-offset-surface-container` on Download button; `focus-visible:ring-error` on Delete button.
- Minimum 36px hit targets (`min-h-[36px]`).
- Disabled states during async operations: `disabled:opacity-50 disabled:cursor-not-allowed`.
- Accessible live announcements: `role="status"` on download/verify progress and completion.
- Contrast & tokens: Quiet Precision tokens and sentence-case copy.

**Test & Commit:**
`node --test dashboard/src/lib/model-install.test.ts`
`cd dashboard && bun test`
Commit: `fix(dashboard): apply model download critique, action buttons, and focus styling`

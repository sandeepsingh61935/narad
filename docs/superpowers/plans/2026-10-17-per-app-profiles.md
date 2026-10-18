# Per-App Profiles Implementation Plan (Phase 4 Surface 3)

- **Goal**: Implement Per-App Profiles (FR-CTX-2) enabling automatic mode selection and paste chord overrides based on focused application id / window class.
- **Spec**: `docs/superpowers/specs/2026-10-17-per-app-profiles-design.md`
- **Branch**: `feat/per-app-profiles`
- **Worktree**: `.worktrees/feat-per-app-profiles`

---

## Task 1: Daemon Config & Pipeline Per-App Profiles Mapping

### Description
1. Update `crates/naradd/src/config.rs`:
   - Define `AppProfile` struct with `app_id: String`, `mode: String`, and `paste_with_shift: Option<bool>`.
   - Add `pub profiles: Vec<AppProfile>` to `NaradConfig`.
   - Implement `find_profile(&self, app_id: &str) -> Option<&AppProfile>` with robust normalization and matching.
   - Unit tests for TOML load/save and profile matching in `config.rs`.
2. Update `crates/naradd/src/pipeline.rs`:
   - In `report_focus(app_id)`:
     - When focused application changes, look up matching profile.
     - If matched, set `st.mode = profile.mode.clone()`.
     - If not matched and no manual override within this session, revert to `self.inner.config.general.active_mode`.
   - In `focused_is_terminal(&self)`:
     - If focused app has a profile with `paste_with_shift == Some(true)`, return `true`.
     - If `paste_with_shift == Some(false)`, return `false`.
     - Otherwise fall back to `insert::uses_terminal_paste(app)`.
   - Unit tests for focus reporting triggering mode change and paste shift behavior.

### Deliverables
- `crates/naradd/src/config.rs`
- `crates/naradd/src/pipeline.rs`

---

## Task 2: Dashboard Config, Route & Profiles UI Wiring

### Description
1. Update `dashboard/src-tauri/src/config.rs`:
   - Add `AppProfile` struct and `pub profiles: Vec<AppProfile>` to `DashboardConfig`.
   - Test round-trip serialization with TOML.
2. Update `dashboard/src/lib/config.ts`:
   - Add `AppProfile` type and `profiles: AppProfile[]` to `DashboardConfig` and `defaultConfig()`.
3. Update `dashboard/src/lib/prepare-config.ts`:
   - Validate and clean profiles (trim strings, filter out empty `app_id`, deduplicate by normalized `app_id`).
4. Create `dashboard/src/lib/profiles.ts` and `dashboard/src/lib/profiles.test.ts`:
   - Helper functions for profile matching, suggestions, and validation.
5. Create `dashboard/src/routes/settings/profiles/+page.svelte`:
   - App profiles settings page with list of profiles, mode dropdowns, paste chord toggles, delete action, and add profile form with suggestions.
   - Auto-save via `saveDashboardConfig`.
6. Update `dashboard/src/routes/settings/+layout.svelte`:
   - Link `App profiles` to `/settings/profiles`.

### Deliverables
- `dashboard/src-tauri/src/config.rs`
- `dashboard/src/lib/config.ts`
- `dashboard/src/lib/prepare-config.ts`
- `dashboard/src/lib/profiles.ts`
- `dashboard/src/lib/profiles.test.ts`
- `dashboard/src/routes/settings/profiles/+page.svelte`
- `dashboard/src/routes/settings/+layout.svelte`

---

## Task 3: Impeccable Critique, A11y, Focus Rings, and Verification Pass

### Description
1. Review `dashboard/src/routes/settings/profiles/+page.svelte` against Quiet Precision guidelines:
   - All hit targets min 36px (`min-h-[36px]`).
   - Standard focus rings: `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2`.
   - Proper sentence-case copy across all labels and notices.
   - Accessible form controls: `aria-label`, `aria-describedby`, field labels.
   - Disabled states during auto-save.
2. End-to-end verification across the stack:
   - `cargo test -p naradd`
   - `cargo test -p narad-dbus`
   - `cd dashboard && bun test`
3. Commit and prepare for whole-branch review.

### Deliverables
- `dashboard/src/routes/settings/profiles/+page.svelte`
- Tests and styling polish

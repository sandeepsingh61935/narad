# Model Catalog Design Brief (Phase 3)

> **Authority:** `docs/ROADMAP.md` Phase 3 ("shape the catalog once, against FR-MDL, before implementation. Then the three beats."), `docs/REQUIREMENTS.md` (FR-MDL-1..6, FR-ASR-3/4, NFR-S-1, NFR-S-5), `docs/DESIGN.md` §3/§4/§5/§6/§7.

## 1. Job and Audience
- **Visitor Mode:** `Operate`.
- **User Context:** The user wants to see what speech model is currently active, how fast it performs on their physical hardware, or optionally download a higher-accuracy model (e.g. Parakeet TDT 0.6B) or a lightweight alternative.
- **Primary Need:** Immediate visibility into the active model and measured latency, with clear, unhyped actions to benchmark, download, verify, activate, or delete speech packs.

## 2. Outcome and Proof
- **Visual Truth First:** "What's active and how fast it is on this machine" sits at the top before the catalog (DESIGN §5).
- **Proof over Assertion:** Latency numbers appear only after being measured on this machine (via the benchmark action or pre-measured hardware spike data). No synthetic percents, no fake accuracy claims.
- **Network Honesty:** Model downloads are the only network operation in Narad and require explicit user action (FR-MDL-3, NFR-S-1, DESIGN §4). The UI clearly states: *"Model downloads are the only network operation in Narad. All speech processing runs 100% locally on your device."*

## 3. Selected Direction (Quiet Precision)
- **Design Tokens:** Warm charcoal surfaces (`#1c1b1a`, `bg-surface`, `bg-surface-container-low`, `bg-surface-container-high`), saffron accent (`#e08a1e`), Inter typography, JetBrains Mono for model IDs and latency metrics, libadwaita radii (`rounded-xl`), and planar borders (`border border-outline-variant/30`).
- **Hierarchy:**
  1. **Header & Network Notice:** Title "Models", subtitle, and honest network notice.
  2. **Active Speech Pack & Benchmark Card:**
     - Current active model ID (`parakeet-ctc-110m-int8`).
     - Measured warm decode latency on this machine (e.g. ~165 ms on 5 s speech).
     - "Run benchmark" button to re-measure all installed models on this CPU.
  3. **Installed Models List:**
     - Cards showing pack name, disk size, status badge ("Active" in saffron/primary, or "Installed"), and action buttons ("Set active", "Delete" for non-default).
  4. **Available Catalog:**
     - Cards showing pack name, architecture, download size, license, and "Download" button.
     - Pack cards for Phase 3:
       - `parakeet-ctc-110m-int8`: Fast, low-latency default (126 MB, 87–173 ms decode).
       - `parakeet-tdt-0.6b-int8`: High-accuracy English pack (640 MB, ~1300 ms decode).
       - `moonshine-base-en-int8`: Alternative encoder-decoder pack (180 MB, ~480 ms decode).
       - `whisper-small-en`: Multilingual/Whisper baseline (480 MB, ~1100–2100 ms decode).

## 4. Scope and Boundaries
- **In Scope for Phase 3:**
  - Speech (ASR) packs.
  - Model catalog metadata (id, name, description, size, sha256 checksum, download URL).
  - Pinned SHA-256 checksum verification (NFR-S-5).
  - First-run benchmark integration (FR-MDL-4).
  - Activation: setting active model in `config.toml` (`models.asr_english`).
  - Pure sentence-case copy.
- **Explicit Anti-Goals:**
  - No translation packs until Decision #2 runtime is settled in REQUIREMENTS §8 (ROADMAP.md).
  - No cleanup LLMs until Phase 4 (Decision #3).
  - No network code in the daemon (`naradd`). The dashboard (via Tauri) or CLI manages downloads.
  - No `bolder`, `delight`, or `overdrive`.

## 5. States and Error Handling
- **Active:** Saffron badge ("Active"), checkmark indicator, active model cannot be deleted.
- **Installed:** "Installed" badge, action buttons "Set active" and "Delete".
- **Available:** Action button "Download" with download size badge.
- **Downloading:** Live progress bar with downloaded MB / total MB and "Cancel" button.
- **Verifying:** Spinner with "Verifying checksum…".
- **Failed:** Red alert banner with specific error ("Checksum mismatch" or "Download failed. Check your internet connection.") and "Try again" button.
- **Benchmarking:** Non-blocking status indicator with progress; updates latency badges upon completion.

## 6. Interaction and Accessibility
- **Focus Rings:** `focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container`.
- **Destructive Actions:** "Delete" uses `focus-visible:ring-error`. Default model (`parakeet-ctc-110m-int8`) cannot be deleted.
- **Hit Targets:** Minimum 36px interactive heights (`min-h-[36px]`).
- **Live Announcements:** `role="status"` on progress/benchmarks, `role="alert"` on checksum/network failures.

# Phase 0 hotkey latency results (NFR-P-1)

**Hardware:** Fedora 44 Workstation, GNOME Shell 50.5, Intel Core i5-8250U @ 1.60GHz (4C/8T, AVX2), 8 logical CPUs, CPU-only

**Run date:** 2026-10-09

**Metric (NFR-P-1):** Hotkey → recording started (sound cue) &lt; 80 ms  
Operational definition used here: wall-clock from first `naradd::hotkeys` log line `hotkey press → Start` (portal `Activated` handled) to `naradd::pipeline: listening` (start cue invoked, mic open via `pw-record`, session state = listening). Sound cue is currently log-only (`audio::play_cue`); it runs before mic open inside `begin_listen`.

## Method

1. Live `naradd` (debug build) with GlobalShortcuts bound (`CTRL+ALT+space`).
2. Human presses of the dictate hotkey during an E2E session; timestamps from `/tmp/naradd-e2e.log` (`RUST_LOG=info`).
3. Parse ISO-8601 tracing timestamps (µs resolution) for each press → listening pair. Presses that never reached `listening` (session busy / overlapping insert) excluded.
4. Instrumentation added in `crates/naradd/src/hotkeys.rs`: after `request_start`, a poll task logs `NFR-P-1 hotkey→listening` with `nfr_p1_ms` (for future runs; not required for the numbers below).

## Trials — hotkey Activated → listening

Source: `/tmp/naradd-e2e.log` (2026-10-09T09:36Z).

| Trial | press → mic open (ms) | press → listening (ms) | Timestamp (press) |
|------:|----------------------:|-----------------------:|-------------------|
| 1 | 0.458 | 0.653 | 09:36:09.280117Z |
| 2 | 0.425 | 0.608 | 09:36:10.826059Z |
| 3 | 0.344 | 0.515 | 09:36:11.737973Z |
| 4 | 0.481 | 0.652 | 09:36:22.538140Z |
| 5 | 0.368 | 0.546 | 09:36:23.271982Z |

| Stat | press → listening (ms) |
|------|-----------------------:|
| **n** | 5 |
| **min** | 0.515 |
| **median** | 0.608 |
| **max** | 0.653 |

## Pass / fail

**PASS** vs NFR-P-1 (&lt; 80 ms). Max observed **0.653 ms** (~122× under budget).

## Corroboration (not scored)

D-Bus `Start` → `mic open` → `listening` on the same machine after rebuild (`/tmp/naradd-nfr-p1.log`): mic→listening max **0.360 ms** (n=8). Shares `begin_listen` with the hotkey path; excludes portal Activated delivery.

## Notes / caveats

- Does **not** include keyboard hardware debounce or compositor→portal delivery before the daemon sees `Activated` (unobservable inside `naradd` without portal timestamps).
- Debug build; release would not be slower for this path in a way that threatens 80 ms.
- Start cue is not yet an audible asset (FR-AUD-6 MVP logs only); cue call is still ordered before mic open.
- One press in the E2E log overlapped insert and never logged `listening`; excluded from the table.

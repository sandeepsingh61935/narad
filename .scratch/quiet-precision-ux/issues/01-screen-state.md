# 01: Screen-state model

**What to build:** Given whether the daemon is reachable, whether history is paused, the current mode, the configured English model id, the search query, and the history rows, Narad decides which History screen to show and which sentence the floating HUD should say. The sentences are fixed: "Nothing yet", "No matching transcriptions", "Narad is not running", "History paused", "Ready" plus the model id, "Transcribing…", "Inserted", "Copied to clipboard" with "No text field focused", "Cancelled", and the default microphone error with "Check audio settings".

**Blocked by:** None (can start immediately).

**Status:** resolved

- [x] Empty query and zero rows returns the empty screen titled "Nothing yet".
- [x] A non-empty query and zero rows returns no-matches titled "No matching transcriptions" with clear label "Clear search".
- [x] A failed load returns daemon-unreachable titled "Narad is not running" with recovery "Retry".
- [x] Paused history returns status "History paused". A reachable, unpaused session returns a status containing "Ready" and the model id that was passed in.
- [x] Preview prefers final text, then raw text, then "(empty)". Final/raw detail keeps the existing fallback when the chosen side is blank.
- [x] Insert status `success`, `skipped`, and `failed` map to "Inserted", "Copied", and "Not inserted". Any other token is shown unchanged.
- [x] A Devanagari string and a multiline snippet round-trip unchanged.
- [x] All seven HUD lines match the spec table, including the copied reason and the default error recovery. A caller-supplied error primary replaces the default microphone sentence.
- [x] Tests call the pure module with Node's built-in test runner. No new dependency. No D-Bus, Svelte, or window.

## Answer

Screen-state module is on `feat/quiet-precision-ux` (`d826104`, import fix `a214168` plus the Node resolve hook). 17 tests passed, including this module. `svelte-check` is clean after test files were excluded and the history import stayed extensionless.

# 01: Screen-state model

**What to build:** Given whether the daemon is reachable, whether history is paused, the current mode, the configured English model id, the search query, and the history rows, Narad decides which History screen to show and which sentence the floating HUD should say. The sentences are fixed: "Nothing yet", "No matching transcriptions", "Narad is not running", "History paused", "Ready" plus the model id, "Transcribing…", "Inserted", "Copied to clipboard" with "No text field focused", "Cancelled", and the default microphone error with "Check audio settings".

**Blocked by:** None (can start immediately).

**Status:** claimed

- [ ] Empty query and zero rows returns the empty screen titled "Nothing yet".
- [ ] A non-empty query and zero rows returns no-matches titled "No matching transcriptions" with clear label "Clear search".
- [ ] A failed load returns daemon-unreachable titled "Narad is not running" with recovery "Retry".
- [ ] Paused history returns status "History paused". A reachable, unpaused session returns a status containing "Ready" and the model id that was passed in.
- [ ] Preview prefers final text, then raw text, then "(empty)". Final/raw detail keeps the existing fallback when the chosen side is blank.
- [ ] Insert status `success`, `skipped`, and `failed` map to "Inserted", "Copied", and "Not inserted". Any other token is shown unchanged.
- [ ] A Devanagari string and a multiline snippet round-trip unchanged.
- [ ] All seven HUD lines match the spec table, including the copied reason and the default error recovery. A caller-supplied error primary replaces the default microphone sentence.
- [ ] Tests call the pure module with Node's built-in test runner. No new dependency. No D-Bus, Svelte, or window.

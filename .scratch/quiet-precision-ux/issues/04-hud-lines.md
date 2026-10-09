# 04: HUD lines match the model

**What to build:** The floating HUD says the same seven lines as the screen-state model. Idle has no pill. Listening shows the mode and elapsed time. Processing says "Transcribing…" and does not invent a model id. Inserted says "Inserted". Copied says "Copied to clipboard" and "No text field focused". Cancelled says "Cancelled". The default error says "Microphone unavailable" and "Check audio settings". Reduced motion keeps the listening label and icon and does not bounce the waveform. An existing hide control reads "Hide HUD".

**Blocked by:** 01: Screen-state model

**Status:** resolved

- [x] Each HUD state renders the primary sentence from the shared model. No "Hide Dot", no whisper-base.en, no model id on the processing pill.
- [x] Reduced motion leaves a static listening glyph and the mode label, and does not start the waveform tick.
- [x] No new D-Bus API. The right-click menu is not built. Only an existing hide label is renamed.

## Answer

Fast-forwarded onto `feat/quiet-precision-ux` as `d88d285`. Copied reason is `No text field focused`. The waveform tick runs only when `St.Settings` animations are enabled. There was no Hide Dot label to rename. HUD parity test is included in the 18 passing tests.

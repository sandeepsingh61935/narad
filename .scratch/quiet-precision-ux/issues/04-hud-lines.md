# 04: HUD lines match the model

**What to build:** The floating HUD says the same seven lines as the screen-state model. Idle has no pill. Listening shows the mode and elapsed time. Processing says "Transcribing…" and does not invent a model id. Inserted says "Inserted". Copied says "Copied to clipboard" and "No text field focused". Cancelled says "Cancelled". The default error says "Microphone unavailable" and "Check audio settings". Reduced motion keeps the listening label and icon and does not bounce the waveform. An existing hide control reads "Hide HUD".

**Blocked by:** 01: Screen-state model

**Status:** ready-for-agent

- [ ] Each HUD state renders the primary sentence from the shared model. No "Hide Dot", no whisper-base.en, no model id on the processing pill.
- [ ] Reduced motion leaves a static listening glyph and the mode label, and does not start the waveform tick.
- [ ] No new D-Bus API. The right-click menu is not built. Only an existing hide label is renamed.

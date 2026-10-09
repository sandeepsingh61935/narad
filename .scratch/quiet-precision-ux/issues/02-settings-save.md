# 02: Settings save when the control changes

**What to build:** Changing a setting that already exists (dictate hotkey, hold/tap threshold, active mode, English speech pack, sound cues) saves that change when the control blurs or the choice changes. There is no Save button and no "Unsaved changes" badge. A hold/tap threshold that is not a non-negative number is rejected with "Hold/tap threshold must be a non-negative number." and the last good config stays in effect. The screen does not mention gsettings or GNOME Shell keybindings.

**Blocked by:** None (can start immediately).

**Status:** claimed

- [ ] A valid edit normalizes the threshold to an integer, trims the hotkey, mode, and model id, and is what gets saved.
- [ ] A blank mode becomes "plain".
- [ ] A negative or non-numeric threshold is rejected with the exact sentence above and does not replace the saved config.
- [ ] The Settings page saves from blur or change. It has no Save button and no unsaved badge.
- [ ] The validation is a pure function tested with Node's built-in test runner. No new dependency.

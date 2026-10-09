# 03: History page speaks the model

**What to build:** History shows one search field. "/" focuses it. An empty archive says "Nothing yet" and tells the person to hold the dictate hotkey and speak. A search with no rows says "No matching transcriptions" and offers "Clear search". If the daemon cannot be reached, the page says "Narad is not running" and offers "Retry". Rows lead with the transcript preview; time, app, and mode are secondary. The detail pane shows the text, then Copy, Re-insert, and Delete, then Final/Raw. J/K and the arrow keys move the selection, Enter re-inserts, C copies, Delete deletes, Esc closes the detail. Toasts say "Copied", "Re-inserted", and "Deleted". There is no Undo, no second search, no Record button, and no account avatar.

**Blocked by:** 01: Screen-state model

**Status:** ready-for-agent

- [ ] The page renders only the words returned by the screen-state model for empty, no-matches, daemon-unreachable, status, preview, detail, and insert labels.
- [ ] Keyboard bindings match the spec and do nothing while typing in the search field, except Esc, which still closes the detail.
- [ ] Copy, Re-insert, and Delete call the existing dashboard commands and toast those verbs.
- [ ] Delete removes that one entry and does not offer Undo.
- [ ] Long text is clamped in the row and shown in full in the detail, including Devanagari and line breaks.

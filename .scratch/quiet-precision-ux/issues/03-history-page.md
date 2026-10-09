# 03: History page speaks the model

**What to build:** History shows one search field. "/" focuses it. An empty archive says "Nothing yet" and tells the person to hold the dictate hotkey and speak. A search with no rows says "No matching transcriptions" and offers "Clear search". If the daemon cannot be reached, the page says "Narad is not running" and offers "Retry". Rows lead with the transcript preview; time, app, and mode are secondary. The detail pane shows the text, then Copy, Re-insert, and Delete, then Final/Raw. J/K and the arrow keys move the selection, Enter re-inserts, C copies, Delete deletes, Esc closes the detail. Toasts say "Copied", "Re-inserted", and "Deleted". There is no Undo, no second search, no Record button, and no account avatar.

**Blocked by:** 01: Screen-state model

**Status:** resolved

- [x] The page renders only the words returned by the screen-state model for empty, no-matches, daemon-unreachable, status, preview, detail, and insert labels.
- [x] Keyboard bindings match the spec and do nothing while typing in the search field, except Esc, which still closes the detail.
- [x] Copy, Re-insert, and Delete call the existing dashboard commands and toast those verbs.
- [x] Delete removes that one entry and does not offer Undo.
- [x] Long text is clamped in the row and shown in full in the detail, including Devanagari and line breaks.

## Answer

Merged as `f488722` (`62daa2f` on `feat/qp-03-history-page`). `session_status` only reads existing `Paused` and `Mode`. 18 tests passed and `svelte-check` was clean after the merge.

Status: ready-for-agent

# Quiet Precision — Phase 1 screen states

## Problem Statement

Narad already dictates, pastes, and lists history. The person using it still cannot trust the words on the screen. The Stitch export that owns the visual design is a stack of competing mockups: three History pages, a HUD drawn as a lab sheet, a first-run window that stops after step 1, and copy that talks about PIDs, buffer RAM, hallucination guards, and the wrong speech model. The shipping dashboard is quieter than those mocks, but its empty states, status line, and settings still do not say the same things as the floating HUD. A person cannot tell, from the interface alone, whether Narad heard them, where the text went, or whether the daemon is even running.

## Solution

One screen-state model decides the words for the Phase 1 surfaces that already exist: History (loading, daemon unreachable, empty, no matches, results, selected entry) and the floating HUD outcomes (idle, listening, transcribing, inserted, copied, cancelled, error). The dashboard and the HUD render that model. They do not invent their own labels.

The look stays Quiet Precision: warm charcoal surfaces, saffron accent, Inter for language, JetBrains Mono for keycaps and model ids, planar borders, no glow. The interface speaks in plain sentence case. The default speech pack is named `parakeet-ctc-110m-int8` wherever a model id is shown. Nothing in this spec adds a daemon method, a catalog, or a later-phase page.

## User Stories

1. As a person dictating into a focused app, I want the HUD to show a listening state that is not color alone, so that I know Narad heard the hotkey.
2. As a person dictating, I want the listening pill to show the current mode and the elapsed time, so that I know which mode I am in and that capture is still going.
3. As a person who released the hotkey, I want the pill to say "Transcribing…", so that I know capture stopped and the text is not ready yet.
4. As a person whose words were pasted, I want a brief "Inserted" outcome, so that I know the text went into the focused app.
5. As a person with no text field focused, I want "Copied to clipboard", the reason "No text field focused", and a paste keycap, so that I know where the text went.
6. As a person who pressed Esc while Narad was listening, I want "Cancelled", so that I know nothing was typed and nothing was saved.
7. As a person whose microphone cannot be opened, I want a plain error and one recovery line, so that I know what failed and what to do next.
8. As a person who has finished an outcome, I want the HUD to return to the idle anchor on its own, so that it gets out of the way.
9. As a person looking at the idle anchor, I want no companion pill, so that the HUD stays quiet until I dictate.
10. As a person using the dashboard, I want History to be the archive, so that I can find what Narad already captured.
11. As a person opening History, I want one search field, so that I am not asked to search twice.
12. As a person who presses `/`, I want focus to move to that search field, so that I can search without the mouse.
13. As a person with an empty query and no rows, I want "Nothing yet" and a sentence that tells me to hold the dictate hotkey and speak, so that I know the product works from the hotkey.
14. As a person whose search matches nothing, I want "No matching transcriptions" and a way to clear the search, so that I can get back to the full list in one action.
15. As a person whose search matches, I want the transcript preview to be the largest text in the row, so that I can scan by what I said.
16. As a person scanning a row, I want the time, the target app, and the mode to be secondary, so that they do not compete with the words.
17. As a person who has selected an entry, I want the final text first, so that I see what was inserted.
18. As a person who selects Raw, I want the raw transcript, so that I can see what the speech model returned before any later cleanup.
19. As a person who selects Final again, I want the final text back, so that the toggle is reversible.
20. As a person who presses `C` on a selected entry, I want the visible text copied, so that the key and the button do the same thing.
21. As a person who presses Enter on a selected entry, I want that entry re-inserted into the focused app, so that I can put the text back without the mouse.
22. As a person who presses `J` or `K` (or Arrow Down / Arrow Up), I want the selection to move through the list, so that I can review without the mouse.
23. As a person who presses Esc, I want the detail pane to close, so that I can leave an entry without deleting it.
24. As a person who presses Delete on a selected entry, I want that entry removed, so that a mistake does not stay in the archive.
25. As a person who just deleted an entry, I want a toast that says "Deleted", so that the action is confirmed in the same verb.
26. As a person who copies, I want the toast to say "Copied".
27. As a person who re-inserts, I want the toast to say "Re-inserted".
28. As a person who re-inserts and the daemon refuses, I want the error to say what happened, so that I know the entry is still in the list.
29. As a person whose dashboard cannot reach the daemon, I want "Narad is not running" and a recovery line, so that I do not see an empty archive and think I have no history.
30. As a person who retries after starting the daemon, I want History to load, so that the failure was about reachability and not about my data.
31. As a person reading the status line, I want "Ready" plus the active model id and the current mode, so that I can see what will transcribe the next utterance.
32. As a person reading any model id, I want `parakeet-ctc-110m-int8` when that pack is the configured English pack, so that the screen matches the pack that actually runs.
33. As a person reading an insert outcome on a row, I want "Inserted", "Copied", or "Not inserted" from the stored insert status, so that history agrees with the HUD.
34. As a person who uses a keyboard, I want the same action names on the buttons and the keycaps, so that I can learn the screen once.
35. As a person who uses only the pointer, I want the same Copy, Re-insert, and Delete actions on the detail pane, so that the mouse path matches the keys.
36. As a person looking at a long transcript, I want the row to clamp to a short preview and the detail pane to show the full text, so that a four-minute dictation does not break the list.
37. As a person who dictated one word, I want that word to remain the preview, so that short entries are not replaced by metadata.
38. As a person who dictated Hindi or Hinglish, I want the preview and the detail text to keep the original characters, so that the archive does not Latinize them.
39. As a person who dictated a code snippet, I want the detail text to keep its line breaks, so that the snippet stays readable.
40. As a person with thousands of entries, I want the list to stay a list of previews, so that density does not turn into a second dashboard.
41. As a person in a narrow window, I want the detail pane to sit under the list instead of being clipped, so that every action remains reachable.
42. As a person who prefers a light GNOME theme, I want the same layout with the Quiet Precision light surfaces, so that the archive follows the desktop.
43. As a person who prefers a dark theme, I want the Quiet Precision dark surfaces, so that the archive matches the HUD.
44. As a person who tabs through History, I want a visible focus ring on the search field, the rows, and the actions, so that I can see where I am.
45. As a person who uses reduced motion, I want the HUD listening state to keep its label and icon without a bouncing waveform, so that status does not depend on animation.
46. As a person changing a setting that already exists (dictate hotkey, hold/tap threshold, active mode, English speech pack, sounds), I want the change saved when I leave the control, so that there is no separate Save step to forget.
47. As a person who enters a bad hold/tap threshold, I want the field to explain that it must be a non-negative number, so that the previous good value stays in effect.
48. As a person reading Settings, I want no claim that Narad writes GNOME Shell keybindings, so that I am not sent to the wrong control panel.
49. As a person reading any Phase 1 screen, I want no Record button, so that I do not think dictation starts from the dashboard.
50. As a person reading any Phase 1 screen, I want no account avatar, so that a local tool does not pretend I signed in.
51. As a person reading any Phase 1 screen, I want no buffer size, PID, window id, token count, or "hallucination guard", so that the screen talks about my text and not the machinery.
52. As a person reading the HUD menu label, when that menu exists, I want "Hide HUD", so that the control matches the thing I see.
53. As a person who has paused history, I want the dashboard status to say "History paused" when the daemon reports paused, so that a missing row is explained.
54. As a maintainer, I want the user-facing sentences to live in one model, so that the dashboard and the HUD cannot drift apart.

## Implementation Decisions

- The daemon stays the owner of session state, history, and config. This spec adds no D-Bus method, signal, or property. `StateChanged` remains a single state string. Properties stay `Mode`, `Paused`, and `HudVisible`. History rows stay the existing fields, including `insert_status` values `success`, `failed`, and `skipped`, and `pinned` stays unread by the UI.
- One pure screen-state module is the only new seam. It sits with the existing history view helpers. Callers pass the daemon reachability, the paused flag, the configured mode, the configured English model id, the search query, the load flag, and the history rows. The module returns which History screen to show and the strings on it. It also returns the HUD line for a session state. The Svelte pages and the Shell extension render those strings. They do not compose product copy themselves.
- History screens are exactly: loading, daemon unreachable, empty, no matches, results. "No matches" is only when the query is non-empty and the row list is empty. An empty archive with an empty query is "Nothing yet", not an error.
- Daemon unreachable copy is fixed: title "Narad is not running". Body tells the person the background service is stopped, so dictation and history are unavailable. The recovery action is "Retry". The module does not detect the bus. The dashboard passes reachability from the failure of the existing history load.
- Extension-missing is not detected and not shown. The module may accept a flag for a later caller, but the dashboard does not set it in this spec.
- Row preview is the trimmed final text, falling back to the trimmed raw text, falling back to "(empty)". Detail text follows the existing final/raw rule, including the fallback when the chosen side is blank. Insert status labels: `success` → "Inserted", `skipped` → "Copied", `failed` → "Not inserted". Any other status is shown as the raw token, not a guessed sentence.
- The HUD line table is fixed:
  - idle: no pill
  - listening: mode label, elapsed time, waveform (or a static listening glyph when reduced motion is requested)
  - processing: primary "Transcribing…". No model id on this pill. The daemon does not send the model id with `StateChanged`, and this spec does not add one.
  - inserted: "Inserted"
  - copied: "Copied to clipboard", reason "No text field focused", paste hint `Ctrl+V`
  - cancelled: "Cancelled"
  - error: primary taken from the caller when the caller has a specific failure; otherwise "Microphone unavailable" and recovery "Check audio settings"
- The dashboard status line, when the daemon is reachable and history is not paused, is "Ready · {model id}" plus the mode name. When paused, the status is "History paused". The model id is the configured English pack, not a hard-coded marketing name. The known default is `parakeet-ctc-110m-int8`.
- Keyboard map on History, and only these bindings: `/` focuses search; `J` / Arrow Down and `K` / Arrow Up move the selection; Enter re-inserts; `C` copies the visible detail text; Delete deletes the selected entry; Esc closes the detail. Bindings do nothing while the person is typing in the search field, except Esc, which still closes the detail. `E`, `P`, and `Ctrl+K` are not bound. There is no command palette in this spec.
- Delete calls the existing delete operation for that one id, then removes the row locally. The toast says "Deleted". There is no Undo. The daemon has no restore, and this spec does not add one. There is no confirmation dialog for a single entry.
- Copy uses the visible detail text. The toast says "Copied". Re-insert uses the existing re-insert operation. The toast says "Re-inserted". Buttons use those same verbs: "Copy", "Re-insert", "Delete". Never "Paste again", "Record", or "Inspect".
- Search stays one field on the History page. Submitting it, or pressing Enter inside it, runs the existing query. Clearing it and submitting shows the unfiltered page. Date, mode, app, and language chips are not added. The header does not grow a second search, a global hotkey cluster, a Record button, or an account control.
- Layout: text preview is the primary line of a row; time, app, and mode are one secondary line. The detail pane shows the text, then the three actions, then the Final/Raw control. On a narrow window the detail pane stacks under the list. No lab chrome (buffer RAM, portal session, daemon version, hex ids).
- Settings that already exist save when the edited control blurs or the choice changes, using the existing save operation. There is no Save button and no "Unsaved changes" badge. Invalid threshold copy: "Hold/tap threshold must be a non-negative number." The previous saved config stays if save fails. Do not mention `gsettings` or `org.gnome.shell.keybindings`.
- Visual tokens follow Quiet Precision, both themes. Dark window `#1C1B1A`, dark grouped surface `#252321`, dark border `#33302D`, text `#EDEDEB`, secondary text `#9E9A94`. Light window `#FAFAF8`, light surface `#FFFFFF`, light border `#E6E4DF`, text `#1C1B1A`, secondary text `#66625D`. Accent `#E08A1E`. Destructive actions use the muted crimson `#D9534F` only for Delete. No pure black, no drop shadow on rows, no gradient mesh. Inter for interface text. JetBrains Mono for the model id and keycaps. The theme follows the GNOME preference.
- The Shell extension's existing state switch keeps its structure. Its visible strings for the states above are changed only where they differ from the HUD line table. Reduced motion replaces the waveform tick with the static listening glyph and leaves the label. The right-click menu is not built in this spec. If a hide control already exists, its label is "Hide HUD".
- Do not import the Stitch HTML. Do not add a font download, an icon font, a test framework, or a dependency.

The screen result is a closed set. Shape, settled by this spec:

```ts
type HistoryScreen =
  | { kind: "loading" }
  | { kind: "daemon-unreachable"; title: "Narad is not running"; recovery: "Retry" }
  | { kind: "empty"; title: "Nothing yet" }
  | { kind: "no-matches"; title: "No matching transcriptions"; clear: "Clear search" }
  | { kind: "results"; status: string; rows: RowView[]; detail: DetailView | null };

type HudLine =
  | { state: "idle"; pill: null }
  | { state: "listening"; mode: string; elapsed: string }
  | { state: "processing"; primary: "Transcribing…" }
  | { state: "inserted"; primary: "Inserted" }
  | { state: "copied"; primary: "Copied to clipboard"; reason: "No text field focused" }
  | { state: "cancelled"; primary: "Cancelled" }
  | { state: "error"; primary: string; recovery: string };
```

## Testing Decisions

A good test calls the screen-state module with plain inputs and asserts the screen kind and the user-facing strings. It does not render Svelte, open a window, talk to D-Bus, or check CSS class names.

Test the history module's pure functions. Cases:

- Empty query, zero rows → empty, title "Nothing yet".
- Non-empty query, zero rows → no matches, title "No matching transcriptions".
- Load failure → daemon unreachable, title "Narad is not running", recovery "Retry".
- Paused flag → status "History paused". Reachable and not paused → status contains "Ready" and the model id that was passed in.
- Preview prefers final text, then raw text, then "(empty)". Detail raw/final keeps the existing fallback.
- `success`, `skipped`, and `failed` map to "Inserted", "Copied", and "Not inserted".
- A Devanagari string and a multiline snippet round-trip unchanged.
- HUD lines for all seven session states match the table above, including the copied reason and the default error recovery.
- A supplied error primary is used instead of the default microphone sentence.

Prior art is the Rust unit tests on history mapping and on the `HudVisible` property name: assert the external value, not the call path. The dashboard has no test runner today. Run these tests with Node's built-in test runner. Do not add Vitest, Jest, or a Playwright dependency.

The Shell extension is not unit-tested here. Its strings are reviewed against the same HUD table. No GNOME session test is required for this spec.

## Out of Scope

- New D-Bus methods, properties, or signals. No model id on `StateChanged`. No undelete. No pin setter. No edit-text method.
- Command palette (`Ctrl+K`), date/mode/app/language filters, multi-select, bulk delete, export, tags, and audio playback.
- Undo on delete, dictionary prompt after an edit, and the correction storyboard ("cooper netties" → "Kubernetes").
- First-run steps, model catalog, download, verify, benchmark, translation pairs, cleanup modes, selection command, stats, privacy retention, app profiles, and microphone picker.
- Right-click HUD menu items other than renaming an existing hide control to "Hide HUD". Edge snap, fullscreen hide, and position memory stay as they are.
- Language pair on the listening pill. That waits until the daemon already exposes it without a new API.
- Detecting a missing GNOME extension, drawing that state, or installing the extension from the dashboard.
- Rewriting the Stitch export, generating new mock HTML, or changing `DESIGN.md` except where a fact in this spec contradicts it. If that happens, record `DRIFT:` rather than silently editing the narrative.
- Terminal paste chords other than the `Ctrl+V` hint already shown on the copied pill.
- A Save button's removal is in scope; building the unbuilt Settings sections (Audio, Modes, Dictionary, Privacy, About) is not.

## Further Notes

- Visual authority remains the Quiet Precision tokens in the Stitch export. Behavior authority remains `DESIGN.md`. This spec only binds the Phase 1 screens that already have a daemon or dashboard seam. Later-phase mocks in that export (Models with CUDA and a 0.6B default, Stats streaks, Dictionary triphones, Modes with a Save button, the edge-state lab page, first-run steps 2–4) are not canonical and are not to be implemented from those files.
- Of the three History mocks, the heuristic-revised page is the closest layout: preview first, one detail pane, Final/Raw, Re-insert. Drop its second search field, Record button, avatar, buffer readout, and post-processing essay.
- Phase 1 exit is already met. This spec is a presentation alignment, not a new phase.
- The issue tracker for this repo was not registered under `AGENTS.md` before this spec. The spec lives at `.scratch/quiet-precision-ux/spec.md` under the local-markdown convention, with `Status: ready-for-agent`.

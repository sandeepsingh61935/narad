# Narad — Stitch UX Improvement Prompts

Prompts to improve the UX of the existing Stitch designs. They don't dictate visual style — keep whatever design system the Stitch project has. Based on [DESIGN.md](DESIGN.md).

How to use:
- Start each prompt with: **"Keep the current visual style and design system."**
- One prompt per screen; one change per follow-up.
- Run the review prompts (section 1) on every screen before refining.

---

## 1. Review prompts (run on any screen)

### 1.1 Heuristic review

```text
Keep the current visual style. Act as a senior UX designer reviewing this screen of Narad, an offline voice-to-text desktop utility used mostly via a hotkey and a floating HUD (compact circular anchor plus companion waveform pill while active). Evaluate it against Nielsen's 10 usability heuristics. List the 10 most impactful issues (visibility of status, user control and undo, consistency, error prevention, recognition over recall, efficiency for keyboard users, minimalism, error recovery). Then apply the fixes and show the revised screen. Do not add decoration or new features.
```

### 1.2 First-time vs power user

```text
Keep the current visual style. Review this screen twice: once as a first-time user who just installed Narad, once as a power user with 5,000 transcriptions who never touches the mouse. Fix what confuses the first and what slows down the second, without adding clutter.
```

### 1.3 Stress-test content

```text
Keep the current visual style. Replace the sample data with stress cases: a 4-minute dictation, a one-word entry, Hindi in Devanagari, mixed Hinglish, a code snippet, a very long app name, 0 results, and 5,000 entries. Fix any truncation, overflow, alignment, or scanning problems that appear.
```

### 1.4 Simplify

```text
Keep the current visual style. Remove everything on this screen that a user wouldn't miss. Merge duplicate actions, move rarely used options behind an "Advanced" or overflow menu, and leave one clear primary action. Show before and after.
```

## 2. Flow prompts

Generate each flow as a sequence of frames so the transitions can be judged, not just the screens.

### 2.1 Core dictation loop

```text
Keep the current visual style. Show the core dictation flow as a frame-by-frame storyboard on a desktop with a text editor open: idle HUD anchor → hotkey pressed (saffron listening anchor + companion pill with waveform, mode, language, elapsed time) → key released (processing spinner + Transcribing pill) → text appears at the cursor (brief inserted confirmation) → HUD returns to idle anchor. Make every moment answer: is it hearing me, is it done, where did my text go.
```

### 2.2 Failure and edge paths

```text
Keep the current visual style. Storyboard these alternate endings of the dictation flow: (1) user presses Esc to cancel — show that nothing was typed or saved; (2) no text field focused — text copied to clipboard with a clear message; (3) microphone unavailable — error with a direct fix; (4) nothing heard — gentle "didn't catch that" with no history entry; (5) AI cleanup reverted because it added content — show raw text was inserted and why.
```

### 2.3 Correction loop

```text
Keep the current visual style. Storyboard the correction flow: user notices "cooper netties" was typed instead of "Kubernetes" → opens dashboard (or command palette) → finds the entry → edits the word → Narad suggests "Add 'Kubernetes' to your dictionary?" → accepted → next dictation gets it right. Minimize steps and make the learning visible.
```

### 2.4 Selection command

```text
Keep the current visual style. Storyboard the selection command: user selects a paragraph in an email → presses the selection hotkey → companion HUD near the anchor says it's listening for an instruction with 2–3 examples → user says "make this more formal" → processing → paragraph replaced → toast with Undo. Make it clear what text will be changed before it changes.
```

### 2.5 First run

```text
Keep the current visual style. Redesign onboarding as at most 3 steps plus a "try it" step: allow typing permission (explain the one-time GNOME prompt), choose shortcut (explain hold-to-talk vs tap-to-toggle visually), pick a voice model (quick benchmark on this computer with a recommendation). End with a live "press your shortcut and say hello" moment that confirms success. Every step skippable with safe defaults.
```

### 2.6 Model install

```text
Keep the current visual style. Storyboard installing a translation model: browse → see size, languages, and measured speed on this computer → download with progress and cancel → verification confirmed → set active → companion HUD now shows the language pair while listening. Make it obvious that downloading is the only time Narad uses the internet.
```

## 3. Screen prompts

### 3.1 Floating HUD

```text
Keep the current visual style. Improve the floating HUD UX (Quiet Precision): idle is a compact ~28px circular anchor; listening adds a saffron pulse ring plus a companion frosted pill with live waveform, mode, language pair, and elapsed time; processing shows a spinner on the anchor and a Transcribing pill with model id; inserted/copied/cancelled/error use brief companion toasts/pills. Every state must be distinguishable without color alone. Add a right-click Adwaita popover on the anchor (mode switch, open dashboard, pause history, hide HUD, quit) with keycap accelerators, edge-snapping when dragged, and auto-hide during fullscreen apps. The companion pill must never cover what the user is typing into.
```

### 3.2 History

```text
Keep the current visual style. Improve History for scanning and speed: text preview is the most prominent element; time, app, and mode are secondary. Hover reveals pin, copy, re-insert. Selected entry detail shows final text first, then actions, then a Final/Raw toggle, audio, and metadata. Add keyboard hints (J/K navigate, Enter re-insert, C copy, E edit, / search). Add a multi-select state with bulk delete/export and an Undo toast after delete.
```

### 3.3 Search and filters

```text
Keep the current visual style. Improve search: instant results with matched words highlighted, filters for date, mode, app, language, pinned that show active state clearly and can be cleared in one action, a helpful no-results state that suggests removing filters, and recent searches.
```

### 3.4 Stats

```text
Keep the current visual style. Restructure Stats around one headline insight ("You saved 11h 20m this month"), then a trend over time, then breakdowns by mode and app. Remove any metric that doesn't lead to a decision or a feeling of progress. Add a range switch (day/week/month/all).
```

### 3.5 Models

```text
Keep the current visual style. Restructure Models so the top answers "what's active and how fast is it on my computer", with a benchmark action. Below, a catalog grouped by type (speech, translation, cleanup AI, wake word) with size, languages, measured speed, and a single clear action per model (Set active / Download / Remove). Show download, verifying, failed, and active states.
```

### 3.6 Settings

```text
Keep the current visual style. Reorganize Settings by user intent: Shortcuts, Microphone & sound, Modes, Words (dictionary, replacements, snippets), Apps (per-app modes), Privacy & storage, About. Each page shows essentials first with an "Advanced" section collapsed. Shortcut editing shows "press new shortcut", conflict warnings, and reset to default. Settings save instantly — no Save button.
```

### 3.7 Modes

```text
Keep the current visual style. Improve the Modes editor: show each mode's hotkey and spoken prefix in the list; editing a mode shows a live before/after preview of a sample dictation and the measured extra delay when AI cleanup is on. Make it easy to duplicate a built-in mode instead of editing it.
```

### 3.8 Privacy

```text
Keep the current visual style. Make Privacy reassuring and actionable: a clear statement that everything stays on this computer, toggles for keeping audio and pausing history, auto-delete periods, storage usage per category with clear actions, export options, and "delete all history" protected by typed confirmation.
```

### 3.9 Command palette and app states

```text
Keep the current visual style. Design a Ctrl+K command palette that reaches every action, mode, model, setting, and history search, each item showing its shortcut. Then design app-wide states: daemon stopped (one-click start), GNOME extension missing (floating HUD unavailable, hotkey still works, how to install), microphone missing, permission denied (how to re-grant), AI model unloaded while idle.
```

## 4. Final passes

### 4.1 Accessibility

```text
Keep the current visual style. Audit this screen for accessibility: visible keyboard focus everywhere, no information by color alone, adequate contrast, comfortable click targets, layout that survives larger text and a narrow window, and a reduced-motion alternative for animations. Apply fixes and annotate them.
```

### 4.2 Consistency across screens

```text
Keep the current visual style. Compare History, Stats, Models, and Settings and make them consistent: same placement of page title and primary action, same row patterns, same empty-state pattern, same toast/undo behavior, same keyboard hint style, same terminology (e.g. always "Re-insert", never "Paste again").
```

### 4.3 Responsive window

```text
Keep the current visual style. Show this screen at a large window, a default window, and a narrow window (~800px). At narrow width, collapse the sidebar to icons and turn the detail pane into an overlay sheet. Nothing should be cut off.
```

## 5. Single-change follow-ups (examples)

```text
Make the text preview the most prominent element in each history row; keep everything else unchanged.
```
```text
Replace the delete confirmation dialog with an immediate delete and an Undo toast; keep everything else unchanged.
```
```text
Show the current mode and language pair on the companion HUD pill while listening; keep everything else unchanged.
```
```text
Add keyboard shortcut hints beside the detail pane actions; keep everything else unchanged.
```

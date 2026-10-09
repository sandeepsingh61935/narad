# 05: Quiet Precision on the dashboard

**What to build:** The dashboard follows Quiet Precision in light and dark. Dark window #1C1B1A, grouped surface #252321, border #33302D, text #EDEDEB, secondary text #9E9A94. Light window #FAFAF8, surface #FFFFFF, border #E6E4DF, text #1C1B1A, secondary text #66625D. Accent #E08A1E. Delete uses #D9534F. Inter for interface text, JetBrains Mono for the model id and keycaps. No pure black, no row drop shadows, no gradient mesh. The theme follows the GNOME color scheme. On a narrow window the History detail pane stacks under the list. No lab chrome.

**Blocked by:** 02: Settings save when the control changes; 03: History page speaks the model

**Status:** claimed

- [x] Global tokens match the hex values above for both prefers-color-scheme values.
- [x] History and Settings use those tokens. Delete is the only destructive color.
- [x] Below the narrow breakpoint, the detail pane is under the list and every action remains reachable.
- [x] No Record button, avatar, buffer readout, PID, or gsettings copy is introduced.

## Answer

Fast-forwarded as `16cbebc`, then review fixes `22e6deb` and `1d7e2c4`. Token test locks the spec hex values. Narrow history stacks under 800px. 22 tests passed and `svelte-check` was clean on `feat/quiet-precision-ux` at `1d7e2c4`.

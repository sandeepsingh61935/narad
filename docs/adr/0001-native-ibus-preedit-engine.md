# Native IBus Pre-Edit Engine vs. Simulated Keystroke/Clipboard Paste

- **Status**: accepted
- **Date**: 2026-10-18
- **Context**: Wispr Flow and legacy dictation tools rely on simulated `Ctrl+V` / `Cmd+V` clipboard pastes. This clobbers the user's system clipboard, disrupts web applications (e.g. Monaco editor in VS Code, Slack message boxes, Google Docs), and prevents users from seeing words appear live as they speak.
- **Decision**: Implement a native Linux Input Method Engine (`org.freedesktop.IBus.Engine`). Partial transcription tokens are emitted directly to the active application context via `UpdatePreeditText`, displaying an underlined inline preview at the blinking cursor. Upon utterance end or natural pause, text is atomically locked in via `CommitText`.
- **Fallback**: If an application lacks an active IBus context (e.g. raw VTE terminals or legacy XWayland applications), the daemon gracefully falls back to clipboard paste via the RemoteDesktop portal (`libei`).
- **Consequences**:
  - Eliminates clipboard race conditions and clobbering.
  - Provides immediate visual feedback inline at the cursor.
  - Works identically across Wayland and X11 compositors.
  - Delivers lower latency than simulated keystroke injection.

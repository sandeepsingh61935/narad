# Local Model Context Protocol (MCP) Server

- **Status**: accepted
- **Date**: 2026-10-18
- **Context**: Modern coding environments are driven by autonomous AI agents (Antigravity, Claude, Cursor) that need seamless access to dictation, history, and voice-command operations.
- **Decision**: Embed a native stdio and Unix domain socket Model Context Protocol (MCP) server directly inside `naradd`. Agents can query transcription history, inject domain vocabulary, inspect current engine status, and trigger dictation flows.
- **Consequences**:
  - Positions Narad as the primary voice interface for autonomous AI coding agents.

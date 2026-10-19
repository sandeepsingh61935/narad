# Local Desktop Accessibility (AT-SPI2) & Surrounding-Text Context Harvesting

- **Status**: accepted
- **Date**: 2026-10-18
- **Context**: Wispr Flow achieves high transcription accuracy by harvesting accessibility trees (`ax_context`), screen OCR, and IDE symbol names. Replicating this via cloud transmission would violate Narad's core local-only privacy contract.
- **Decision**: Harvest context 100% locally on-device:
  1. Inspect the focused application's accessibility tree via AT-SPI2 D-Bus (`org.a11y.Bus`) to retrieve input field roles and visible text labels.
  2. Query IBus surrounding-text context via `SetSurroundingText` to detect preceding punctuation, sentence capitalization, and spacing.
  3. Scan active project directories for code identifiers and variable names.
  Harvested tokens are loaded directly into Sherpa-ONNX's in-memory dynamic hotword biasing table (`OnlineRecognizer::create_stream_with_hotwords`). Zero pixels, audio, or text leave the device.
- **Consequences**:
  - Delivers context-aware dictation accuracy matching or exceeding Wispr Flow while maintaining a zero-cloud, 100% private security boundary.

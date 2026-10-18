/** Status of a long-form or meeting document. */
export type DocumentStatus = "active" | "completed";

/** A stored chunk representing an incremental segment of speech. */
export type DocumentChunk = {
  chunk_index: number;
  speaker_label: string | null;
  text: string;
  start_ms: number;
  duration_ms: number;
};

/** Aggregated contiguous chunks from the same speaker. */
export type SpeakerBlock = {
  speaker_label: string | null;
  text: string;
  start_ms: number;
  duration_ms: number;
};

/** Full meeting document with metadata and transcript chunks. */
export type MeetingDocument = {
  id: number;
  title: string;
  created_at: number;
  updated_at: number;
  duration_ms: number;
  status: DocumentStatus;
  chunks: DocumentChunk[];
};

/** Summary of a meeting document without full chunk list. */
export type DocumentSummary = {
  id: number;
  title: string;
  created_at: number;
  updated_at: number;
  duration_ms: number;
  status: DocumentStatus;
  chunk_count: number;
};

/**
 * Format milliseconds into MM:SS or HH:MM:SS.
 */
export function formatMeetingDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms <= 0) {
    return "00:00";
  }
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  const pad = (n: number) => String(n).padStart(2, "0");

  if (hours > 0) {
    return `${pad(hours)}:${pad(minutes)}:${pad(seconds)}`;
  }
  return `${pad(minutes)}:${pad(seconds)}`;
}

/**
 * Format meeting title with sentence case fallback.
 */
export function formatMeetingTitle(title: string | null | undefined): string {
  if (!title) return "Untitled meeting";
  const trimmed = title.trim();
  return trimmed.length > 0 ? trimmed : "Untitled meeting";
}

/**
 * Map status to sentence case badge text ("Draft" or "Completed").
 */
export function formatMeetingStatus(
  status: DocumentStatus | string | null | undefined,
): "Draft" | "Completed" {
  if (!status) return "Draft";
  const normalized = status.trim().toLowerCase();
  if (normalized === "completed") {
    return "Completed";
  }
  return "Draft";
}

/**
 * Format speaker badge text with fallback.
 */
export function formatSpeakerLabel(
  speakerLabel: string | null | undefined,
  fallbackIndex = 1,
): string {
  if (speakerLabel && speakerLabel.trim().length > 0) {
    return speakerLabel.trim();
  }
  return `Speaker ${fallbackIndex}`;
}

/**
 * Format meeting timestamp into a human-readable date and time.
 */
export function formatMeetingDate(timestampMs: number): string {
  const d = new Date(timestampMs);
  if (Number.isNaN(d.getTime())) return "Unknown date";
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * Aggregate contiguous chunks from the same speaker into speaker blocks.
 */
export function aggregateSpeakerBlocks(
  chunks: readonly DocumentChunk[],
): SpeakerBlock[] {
  const blocks: SpeakerBlock[] = [];

  for (const chunk of chunks) {
    const last = blocks[blocks.length - 1];
    if (last && last.speaker_label === chunk.speaker_label) {
      const trimmed = chunk.text.trim();
      if (trimmed.length > 0) {
        if (last.text.length > 0) {
          last.text += " ";
        }
        last.text += trimmed;
      }
      const chunkEnd = chunk.start_ms + chunk.duration_ms;
      const lastSpan = chunkEnd - last.start_ms;
      if (lastSpan > last.duration_ms) {
        last.duration_ms = lastSpan;
      }
      continue;
    }

    blocks.push({
      speaker_label: chunk.speaker_label,
      text: chunk.text.trim(),
      start_ms: chunk.start_ms,
      duration_ms: chunk.duration_ms,
    });
  }

  return blocks;
}

/**
 * Generate formatted Markdown text for export.
 */
export function generateMarkdownExport(doc: {
  title: string;
  created_at?: number;
  duration_ms?: number;
  status?: DocumentStatus | string;
  chunks?: readonly DocumentChunk[];
}): string {
  const title = formatMeetingTitle(doc.title);
  const status = formatMeetingStatus(doc.status);
  const duration = formatMeetingDuration(doc.duration_ms ?? 0);
  const lines: string[] = [
    `# ${title}`,
    "",
    `- Status: ${status}`,
    `- Duration: ${duration}`,
  ];

  if (doc.created_at && Number.isFinite(doc.created_at)) {
    lines.push(`- Date: ${formatMeetingDate(doc.created_at)}`);
  }

  lines.push("", "## Transcript", "");

  const blocks = aggregateSpeakerBlocks(doc.chunks ?? []);
  if (blocks.length === 0) {
    lines.push("*No transcript available.*");
  } else {
    for (let i = 0; i < blocks.length; i++) {
      const block = blocks[i];
      const speaker = formatSpeakerLabel(block.speaker_label, i + 1);
      const offset = formatMeetingDuration(block.start_ms);
      lines.push(`### ${speaker} (${offset})`);
      lines.push("");
      lines.push(block.text || "(empty)");
      lines.push("");
    }
  }

  return lines.join("\n").trim();
}

/**
 * Generate plain text string of the meeting transcript for copying.
 */
export function generatePlainText(doc: {
  title: string;
  created_at?: number;
  duration_ms?: number;
  status?: DocumentStatus | string;
  chunks?: readonly DocumentChunk[];
}): string {
  const title = formatMeetingTitle(doc.title);
  const status = formatMeetingStatus(doc.status);
  const duration = formatMeetingDuration(doc.duration_ms ?? 0);
  const parts: string[] = [
    `${title}`,
    `Status: ${status} | Duration: ${duration}`,
    "",
  ];

  const blocks = aggregateSpeakerBlocks(doc.chunks ?? []);
  if (blocks.length === 0) {
    parts.push("No transcript available.");
  } else {
    for (let i = 0; i < blocks.length; i++) {
      const block = blocks[i];
      const speaker = formatSpeakerLabel(block.speaker_label, i + 1);
      const offset = formatMeetingDuration(block.start_ms);
      parts.push(`${speaker} (${offset}):\n${block.text || "(empty)"}`);
    }
  }

  return parts.join("\n\n").trim();
}

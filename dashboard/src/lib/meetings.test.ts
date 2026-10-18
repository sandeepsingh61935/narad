import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import {
  aggregateSpeakerBlocks,
  formatMeetingDate,
  formatMeetingDuration,
  formatMeetingStatus,
  formatMeetingTitle,
  formatSpeakerLabel,
  generateMarkdownExport,
  generatePlainText,
  type DocumentChunk,
  type MeetingDocument,
} from "./meetings.ts";

const here = dirname(fileURLToPath(import.meta.url));

test("formatMeetingDuration formats milliseconds to MM:SS and HH:MM:SS", () => {
  assert.equal(formatMeetingDuration(0), "00:00");
  assert.equal(formatMeetingDuration(-500), "00:00");
  assert.equal(formatMeetingDuration(Number.NaN), "00:00");
  assert.equal(formatMeetingDuration(5000), "00:05");
  assert.equal(formatMeetingDuration(65000), "01:05");
  assert.equal(formatMeetingDuration(599000), "09:59");
  assert.equal(formatMeetingDuration(3600000), "01:00:00");
  assert.equal(formatMeetingDuration(3665000), "01:01:05");
  assert.equal(formatMeetingDuration(7325000), "02:02:05");
});

test("formatMeetingTitle uses sentence case fallback for empty titles", () => {
  assert.equal(formatMeetingTitle(undefined), "Untitled meeting");
  assert.equal(formatMeetingTitle(null), "Untitled meeting");
  assert.equal(formatMeetingTitle(""), "Untitled meeting");
  assert.equal(formatMeetingTitle("   "), "Untitled meeting");
  assert.equal(formatMeetingTitle("Sprint sync"), "Sprint sync");
  assert.equal(formatMeetingTitle("  Product review  "), "Product review");
});

test("formatMeetingStatus maps status codes to sentence case badges", () => {
  assert.equal(formatMeetingStatus("active"), "Draft");
  assert.equal(formatMeetingStatus("draft"), "Draft");
  assert.equal(formatMeetingStatus("completed"), "Completed");
  assert.equal(formatMeetingStatus("Completed"), "Completed");
  assert.equal(formatMeetingStatus("unknown"), "Draft");
});

test("formatSpeakerLabel assigns speaker badge and fallback indexes", () => {
  assert.equal(formatSpeakerLabel("Alice"), "Alice");
  assert.equal(formatSpeakerLabel(null, 1), "Speaker 1");
  assert.equal(formatSpeakerLabel(undefined, 2), "Speaker 2");
  assert.equal(formatSpeakerLabel(""), "Speaker 1");
});

test("aggregateSpeakerBlocks merges contiguous chunks with identical speaker labels", () => {
  const chunks: DocumentChunk[] = [
    {
      chunk_index: 0,
      speaker_label: "Speaker 1",
      text: "Good morning everyone.",
      start_ms: 0,
      duration_ms: 3000,
    },
    {
      chunk_index: 1,
      speaker_label: "Speaker 1",
      text: "Let's review the roadmap.",
      start_ms: 3000,
      duration_ms: 4000,
    },
    {
      chunk_index: 2,
      speaker_label: "Speaker 2",
      text: "I have the updates ready.",
      start_ms: 7500,
      duration_ms: 2500,
    },
    {
      chunk_index: 3,
      speaker_label: "Speaker 1",
      text: "Great, please go ahead.",
      start_ms: 10500,
      duration_ms: 2000,
    },
  ];

  const blocks = aggregateSpeakerBlocks(chunks);
  assert.equal(blocks.length, 3);

  assert.equal(blocks[0].speaker_label, "Speaker 1");
  assert.equal(blocks[0].text, "Good morning everyone. Let's review the roadmap.");
  assert.equal(blocks[0].start_ms, 0);
  assert.equal(blocks[0].duration_ms, 7000);

  assert.equal(blocks[1].speaker_label, "Speaker 2");
  assert.equal(blocks[1].text, "I have the updates ready.");
  assert.equal(blocks[1].start_ms, 7500);
  assert.equal(blocks[1].duration_ms, 2500);

  assert.equal(blocks[2].speaker_label, "Speaker 1");
  assert.equal(blocks[2].text, "Great, please go ahead.");
  assert.equal(blocks[2].start_ms, 10500);
  assert.equal(blocks[2].duration_ms, 2000);
});

test("generateMarkdownExport formats title, metadata, and speaker blocks", () => {
  const doc: MeetingDocument = {
    id: 1,
    title: "Quarterly planning",
    created_at: 1713430000000,
    updated_at: 1713433600000,
    duration_ms: 65000,
    status: "completed",
    chunks: [
      {
        chunk_index: 0,
        speaker_label: "Speaker 1",
        text: "Welcome to the meeting.",
        start_ms: 0,
        duration_ms: 5000,
      },
      {
        chunk_index: 1,
        speaker_label: "Speaker 2",
        text: "Thanks for organizing.",
        start_ms: 6000,
        duration_ms: 4000,
      },
    ],
  };

  const md = generateMarkdownExport(doc);
  assert.ok(md.includes("# Quarterly planning"));
  assert.ok(md.includes("- Status: Completed"));
  assert.ok(md.includes("- Duration: 01:05"));
  assert.ok(md.includes("## Transcript"));
  assert.ok(md.includes("### Speaker 1 (00:00)"));
  assert.ok(md.includes("Welcome to the meeting."));
  assert.ok(md.includes("### Speaker 2 (00:06)"));
  assert.ok(md.includes("Thanks for organizing."));
});

test("generatePlainText formats transcript for copying", () => {
  const doc: MeetingDocument = {
    id: 1,
    title: "",
    created_at: 1713430000000,
    updated_at: 1713433600000,
    duration_ms: 12000,
    status: "active",
    chunks: [
      {
        chunk_index: 0,
        speaker_label: null,
        text: "Starting transcription.",
        start_ms: 0,
        duration_ms: 4000,
      },
    ],
  };

  const text = generatePlainText(doc);
  assert.ok(text.includes("Untitled meeting"));
  assert.ok(text.includes("Speaker 1 (00:00):"));
  assert.ok(text.includes("Starting transcription."));
});

test("layout links to /meetings with label Meetings", () => {
  const layout = readFileSync(join(here, "../routes/+layout.svelte"), "utf8");
  assert.ok(layout.includes('{ href: "/meetings", label: "Meetings"'));
});

test("meeting page exists and satisfies Impeccable and Quiet Precision rules", () => {
  const pagePath = join(here, "../routes/meetings/+page.svelte");
  assert.equal(existsSync(pagePath), true);
  const content = readFileSync(pagePath, "utf8");

  // Required copy
  assert.ok(content.includes("Meeting notes"));
  assert.ok(content.includes("Export markdown"));
  assert.ok(content.includes("Copy text"));
  assert.ok(content.includes("No meeting documents yet"));
  assert.ok(content.includes("Untitled meeting"));

  // Design tokens
  assert.ok(content.includes("bg-surface"));
  assert.ok(content.includes("bg-surface-container"));
  assert.ok(content.includes("border-outline-variant"));
  assert.ok(content.includes("text-on-surface"));
  assert.ok(content.includes("text-on-surface-variant"));

  // Minimum touch target sizes & focus rings
  assert.ok(content.includes("min-h-[36px]") || content.includes("min-h-[44px]"));
  assert.ok(content.includes("focus-visible:ring-2") || content.includes("focus-visible:outline-none"));

  // Forbidden terms
  assert.equal(content.includes("telemetry"), false);
  assert.equal(content.includes("acoustic"), false);
  assert.equal(content.includes("lexical"), false);
  assert.equal(content.includes("CUDA"), false);
  assert.equal(content.includes("TensorRT"), false);
  assert.equal(content.includes("gsettings"), false);
});

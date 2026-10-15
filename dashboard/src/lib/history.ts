/** Mirrors `narad_dbus::HistoryEntry` (snake_case JSON from Rust). */
export type HistoryEntry = {
  id: number;
  created_at: number;
  duration_ms: number;
  mode: string;
  src_lang: string;
  dst_lang: string;
  asr_model: string;
  mt_model: string | null;
  llm_model: string | null;
  app_id: string | null;
  raw_text: string;
  final_text: string;
  insert_status: string;
  cleanup_flagged: boolean;
  audio_path: string | null;
  pinned: boolean;
  /** `dictation` or `retranscribe`. */
  kind: string;
  /** Tag names in kept order. */
  tags: string[];
};

export type TextView = "final" | "raw";

export function previewText(entry: HistoryEntry): string {
  const text = (entry.final_text || entry.raw_text).trim();
  return text.length > 0 ? text : "(empty)";
}

/** Text for the detail pane given raw↔final toggle (FR-HIS-4). */
export function displayText(entry: HistoryEntry, view: TextView): string {
  const text = (view === "raw" ? entry.raw_text : entry.final_text).trim();
  if (text.length > 0) return text;
  const fallback = (view === "raw" ? entry.final_text : entry.raw_text).trim();
  return fallback.length > 0 ? fallback : "(empty)";
}

export function formatTime(createdAtMs: number): string {
  const d = new Date(createdAtMs);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function formatDuration(ms: number): string {
  if (!Number.isFinite(ms) || ms < 0) return "—";
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toFixed(1)} s`;
}

/** First path from a webview drop. An empty list is nothing. */
export function firstDroppedPath(paths: readonly string[]): string | null {
  if (paths.length === 0) return null;
  return paths[0];
}

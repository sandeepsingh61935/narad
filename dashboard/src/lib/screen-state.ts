import { displayText, type HistoryEntry, type TextView } from "./history";

export type HistoryScreenInput = {
  loading: boolean;
  reachable: boolean;
  paused: boolean;
  mode: string;
  modelId: string;
  query: string;
  /** True when a date, mode, app, or language filter is set. */
  narrowed: boolean;
  rows: HistoryEntry[];
  selectedId: number | null;
  textView: TextView;
};

export type HistoryScreen =
  | { kind: "loading" }
  | {
      kind: "daemon-unreachable";
      title: "Narad is not running";
      body: "The background service is stopped, so dictation and history are unavailable.";
      recovery: "Retry";
    }
  | {
      kind: "empty";
      status: string;
      title: "Nothing yet";
      body: "Hold the dictate hotkey and speak.";
    }
  | {
      kind: "no-matches";
      status: string;
      title: "No matching transcriptions";
      clear: "Clear search";
    }
  | {
      kind: "results";
      status: string;
      rows: RowView[];
      detail: DetailView | null;
    };

export type RowView = {
  id: number;
  preview: string;
  insert: string;
};

export type DetailView = {
  id: number;
  text: string;
  /** True when the row has a retained audio path. */
  canPlay: boolean;
  /** Stored tag names in kept order. Not a list filter. */
  tags: string[];
};

function detailView(input: HistoryScreenInput): DetailView | null {
  if (input.selectedId == null) return null;
  const selected = input.rows.find((row) => row.id === input.selectedId);
  if (selected == null) return null;
  const path = selected.audio_path;
  return {
    id: selected.id,
    text: displayText(selected, input.textView),
    canPlay: path != null && path.length > 0,
    tags: selected.tags,
  };
}

function sessionStatus(input: HistoryScreenInput): string {
  return input.paused ? "History paused" : `Ready · ${input.modelId} · ${input.mode}`;
}

function insertLabel(status: string): string {
  if (status === "success") return "Inserted";
  if (status === "skipped") return "Copied";
  if (status === "failed") return "Not inserted";
  return status;
}

/** Trimmed final, then trimmed raw. A whitespace-only final still falls through. */
function rowPreview(entry: HistoryEntry): string {
  const finalText = entry.final_text.trim();
  if (finalText.length > 0) return finalText;
  const rawText = entry.raw_text.trim();
  if (rawText.length > 0) return rawText;
  return "(empty)";
}

export type HudLine =
  | { state: "idle"; pill: null }
  | { state: "listening"; mode: string; elapsed: string }
  | { state: "processing"; primary: "Transcribing…" }
  | { state: "inserted"; primary: "Inserted" }
  | { state: "copied"; primary: "Copied to clipboard"; reason: "No text field focused" }
  | { state: "cancelled"; primary: "Cancelled" }
  | { state: "error"; primary: string; recovery: string }
  | { state: "undone"; primary: "Undone"; deleted: string };

export type HudInput = {
  state: HudLine["state"];
  mode?: string;
  elapsed?: string;
  errorPrimary?: string | null;
  /** Text removed by undo. Only the undone line reads this. */
  deleted?: string | null;
};

export function hudLine(input: HudInput): HudLine {
  switch (input.state) {
    case "idle":
      return { state: "idle", pill: null };
    case "listening":
      return { state: "listening", mode: input.mode ?? "", elapsed: input.elapsed ?? "" };
    case "processing":
      return { state: "processing", primary: "Transcribing…" };
    case "inserted":
      return { state: "inserted", primary: "Inserted" };
    case "copied":
      return {
        state: "copied",
        primary: "Copied to clipboard",
        reason: "No text field focused",
      };
    case "cancelled":
      return { state: "cancelled", primary: "Cancelled" };
    case "error":
      return {
        state: "error",
        primary:
          input.errorPrimary != null && input.errorPrimary.length > 0
            ? input.errorPrimary
            : "Microphone unavailable",
        recovery: "Check audio settings",
      };
    case "undone":
      return {
        state: "undone",
        primary: "Undone",
        deleted: input.deleted ?? "",
      };
  }
}

export function historyScreen(input: HistoryScreenInput): HistoryScreen {
  if (input.loading) return { kind: "loading" };
  if (!input.reachable) {
    return {
      kind: "daemon-unreachable",
      title: "Narad is not running",
      body: "The background service is stopped, so dictation and history are unavailable.",
      recovery: "Retry",
    };
  }
  if (input.rows.length > 0) {
    return {
      kind: "results",
      status: sessionStatus(input),
      rows: input.rows.map((row) => ({
        id: row.id,
        preview: rowPreview(row),
        insert: insertLabel(row.insert_status),
      })),
      detail: detailView(input),
    };
  }
  if (input.query.length > 0 || input.narrowed) {
    return {
      kind: "no-matches",
      status: sessionStatus(input),
      title: "No matching transcriptions",
      clear: "Clear search",
    };
  }
  return {
    kind: "empty",
    status: sessionStatus(input),
    title: "Nothing yet",
    body: "Hold the dictate hotkey and speak.",
  };
}

import assert from "node:assert/strict";
import { test } from "node:test";
import type { HistoryEntry } from "./history.ts";
import { historyScreen, hudLine, type HistoryScreenInput } from "./screen-state.ts";

function entry(overrides: Partial<HistoryEntry> = {}): HistoryEntry {
  return {
    id: 1,
    created_at: 1_700_000_000_000,
    duration_ms: 1200,
    mode: "plain",
    src_lang: "en",
    dst_lang: "en",
    asr_model: "parakeet-ctc-110m-int8",
    mt_model: null,
    llm_model: null,
    app_id: "org.gnome.TextEditor",
    raw_text: "raw words",
    final_text: "final words",
    insert_status: "success",
    cleanup_flagged: false,
    audio_path: null,
    pinned: false,
    ...overrides,
  };
}

function input(overrides: Partial<HistoryScreenInput> = {}): HistoryScreenInput {
  return {
    loading: false,
    reachable: true,
    paused: false,
    mode: "plain",
    modelId: "parakeet-ctc-110m-int8",
    query: "",
    rows: [],
    selectedId: null,
    textView: "final",
    ...overrides,
  };
}

test("empty query and zero rows shows Nothing yet", () => {
  const screen = historyScreen(input());
  assert.deepEqual(screen, {
    kind: "empty",
    status: "Ready · parakeet-ctc-110m-int8 · plain",
    title: "Nothing yet",
    body: "Hold the dictate hotkey and speak.",
  });
});

test("a reachable empty archive still names Ready and the model id", () => {
  const screen = historyScreen(input({ mode: "command", modelId: "custom-pack" }));
  assert.equal(screen.kind, "empty");
  if (screen.kind !== "empty") return;
  assert.equal(screen.status, "Ready · custom-pack · command");
});

test("paused empty archive says History paused", () => {
  const screen = historyScreen(input({ paused: true }));
  assert.equal(screen.kind, "empty");
  if (screen.kind !== "empty") return;
  assert.equal(screen.status, "History paused");
});

test("a search with no rows offers Clear search", () => {
  const screen = historyScreen(input({ query: "kubernetes" }));
  assert.deepEqual(screen, {
    kind: "no-matches",
    status: "Ready · parakeet-ctc-110m-int8 · plain",
    title: "No matching transcriptions",
    clear: "Clear search",
  });
});

test("a search with no rows keeps History paused", () => {
  const screen = historyScreen(
    input({ query: "kubernetes", paused: true, mode: "command", modelId: "custom-pack" }),
  );
  assert.equal(screen.kind, "no-matches");
  if (screen.kind !== "no-matches") return;
  assert.equal(screen.status, "History paused");
});

test("history still loading stays on the loading screen", () => {
  const screen = historyScreen(input({ loading: true, reachable: false, query: "notes" }));
  assert.deepEqual(screen, { kind: "loading" });
});

test("a failed load says Narad is not running", () => {
  const screen = historyScreen(input({ reachable: false, query: "notes" }));
  assert.deepEqual(screen, {
    kind: "daemon-unreachable",
    title: "Narad is not running",
    body: "The background service is stopped, so dictation and history are unavailable.",
    recovery: "Retry",
  });
});

test("paused history says History paused", () => {
  const screen = historyScreen(input({ paused: true, rows: [entry()] }));
  assert.equal(screen.kind, "results");
  if (screen.kind !== "results") return;
  assert.equal(screen.status, "History paused");
});

test("a reachable unpaused session names Ready and the model id", () => {
  const screen = historyScreen(
    input({ rows: [entry()], mode: "command", modelId: "custom-pack" }),
  );
  assert.equal(screen.kind, "results");
  if (screen.kind !== "results") return;
  assert.equal(screen.status, "Ready · custom-pack · command");
});

test("detail keeps the final/raw fallback when the chosen side is blank", () => {
  const rows = [
    entry({ id: 1, final_text: "kept final", raw_text: "" }),
    entry({ id: 2, final_text: "", raw_text: "kept raw" }),
    entry({ id: 3, final_text: "   ", raw_text: "   " }),
    entry({ id: 4, final_text: "final side", raw_text: "raw side" }),
  ];
  const detailFor = (selectedId: number | null, textView: "final" | "raw") => {
    const screen = historyScreen(input({ rows, selectedId, textView }));
    assert.equal(screen.kind, "results");
    if (screen.kind !== "results") return null;
    return screen.detail;
  };
  assert.deepEqual(detailFor(1, "raw"), { id: 1, text: "kept final" });
  assert.deepEqual(detailFor(2, "final"), { id: 2, text: "kept raw" });
  assert.deepEqual(detailFor(3, "final"), { id: 3, text: "(empty)" });
  assert.deepEqual(detailFor(4, "raw"), { id: 4, text: "raw side" });
  assert.equal(detailFor(null, "final"), null);
  assert.equal(detailFor(99, "final"), null);
});

test("insert status maps to Inserted, Copied, and Not inserted", () => {
  const screen = historyScreen(
    input({
      rows: [
        entry({ id: 1, insert_status: "success" }),
        entry({ id: 2, insert_status: "skipped" }),
        entry({ id: 3, insert_status: "failed" }),
        entry({ id: 4, insert_status: "queued" }),
      ],
    }),
  );
  assert.equal(screen.kind, "results");
  if (screen.kind !== "results") return;
  assert.deepEqual(
    screen.rows.map((row) => row.insert),
    ["Inserted", "Copied", "Not inserted", "queued"],
  );
});

test("a supplied error primary replaces the microphone sentence", () => {
  assert.deepEqual(hudLine({ state: "error", errorPrimary: "Portal denied" }), {
    state: "error",
    primary: "Portal denied",
    recovery: "Check audio settings",
  });
});

test("HUD lines match the seven session states", () => {
  assert.deepEqual(hudLine({ state: "idle" }), { state: "idle", pill: null });
  assert.deepEqual(hudLine({ state: "listening", mode: "plain", elapsed: "1:05" }), {
    state: "listening",
    mode: "plain",
    elapsed: "1:05",
  });
  assert.deepEqual(hudLine({ state: "processing" }), {
    state: "processing",
    primary: "Transcribing…",
  });
  assert.deepEqual(hudLine({ state: "inserted" }), {
    state: "inserted",
    primary: "Inserted",
  });
  assert.deepEqual(hudLine({ state: "copied" }), {
    state: "copied",
    primary: "Copied to clipboard",
    reason: "No text field focused",
  });
  assert.deepEqual(hudLine({ state: "cancelled" }), {
    state: "cancelled",
    primary: "Cancelled",
  });
  assert.deepEqual(hudLine({ state: "error" }), {
    state: "error",
    primary: "Microphone unavailable",
    recovery: "Check audio settings",
  });
});

test("Devanagari and multiline text round-trip unchanged", () => {
  const devanagari = "नमस्ते दुनिया";
  const snippet = "if (ok) {\n  return 1;\n}";
  const screen = historyScreen(
    input({
      textView: "final",
      selectedId: 2,
      rows: [
        entry({ id: 1, final_text: devanagari, raw_text: "latin" }),
        entry({ id: 2, final_text: snippet, raw_text: "flat" }),
      ],
    }),
  );
  assert.equal(screen.kind, "results");
  if (screen.kind !== "results") return;
  assert.deepEqual(
    screen.rows.map((row) => row.preview),
    [devanagari, snippet],
  );
  assert.deepEqual(screen.detail, { id: 2, text: snippet });
});

test("row preview prefers final text, then raw text, then (empty)", () => {
  const screen = historyScreen(
    input({
      rows: [
        entry({ id: 1, final_text: "  said this  ", raw_text: "heard this" }),
        entry({ id: 2, final_text: "", raw_text: "  heard only  " }),
        entry({ id: 3, final_text: "", raw_text: "" }),
        entry({ id: 4, final_text: "   ", raw_text: "fallback raw" }),
      ],
    }),
  );
  assert.equal(screen.kind, "results");
  if (screen.kind !== "results") return;
  assert.deepEqual(
    screen.rows.map((row) => row.preview),
    ["said this", "heard only", "(empty)", "fallback raw"],
  );
});

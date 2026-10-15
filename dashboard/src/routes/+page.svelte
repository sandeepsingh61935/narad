<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { ACTIVE_MODES, type DashboardConfig } from "$lib/config";
  import {
    firstDroppedPath,
    formatDuration,
    formatTime,
    type HistoryEntry,
    type TextView,
  } from "$lib/history";
  import { deleteAllConfirmed, exportIds, historyScreen, type RowView } from "$lib/screen-state";
  import {
    filterCacheKey,
    isFilterNarrowed,
    prepareHistoryFilter,
  } from "$lib/history-search";

  const PAGE_SIZE = 50;
  const SEARCH_ID = "history-search";
  const TAGS_ID = "entry-tags";
  const DELETE_ALL_ID = "delete-all-history";
  const EXPORT_FILES = {
    json: "history.json",
    markdown: "history.md",
    txt: "history.txt",
  } as const;
  const RECOVERY_COMMAND = "systemctl --user restart narad.service";

  const segmentOn =
    "px-3 py-1 rounded font-body-default font-medium bg-surface-container-high text-on-surface shadow-xs transition-all";
  const segmentOff =
    "px-3 py-1 rounded font-body-default text-on-surface-variant hover:text-on-surface transition-all";
  const rowOn =
    "group relative rounded-xl p-3 bg-surface-container border border-primary-container/60 shadow-sm transition-all cursor-pointer ring-1 ring-primary-container/40 text-left w-full";
  const rowOff =
    "group relative rounded-xl p-3 bg-surface-container/60 hover:bg-surface-container border border-outline-variant/30 hover:border-outline-variant/70 transition-all cursor-pointer text-left w-full";

  type SessionStatus = {
    paused: boolean;
    mode: string;
  };

  type Group = {
    label: string;
    rows: RowView[];
  };

  let query = $state("");
  let appliedQuery = $state("");
  let fromDay = $state("");
  let toDay = $state("");
  let modeFilter = $state("");
  let appFilter = $state("");
  let langFilter = $state("");
  let appliedNarrowed = $state(false);
  let entries = $state<HistoryEntry[]>([]);
  let loading = $state(true);
  let reachable = $state(true);
  let paused = $state(false);
  let mode = $state("plain");
  let modelId = $state("");

  let selectedId = $state<number | null>(null);
  let textView = $state<TextView>("final");
  let actionBusy = $state(false);
  let retranscribeMode = $state("plain");
  let actionMessage = $state<string | null>(null);
  let actionError = $state<string | null>(null);
  let daemonBannerOpen = $state(true);
  let tagDraft = $state("");
  let tagDraftId = $state<number | null>(null);
  let tagCommitBusy = false;
  let checkedIds = $state<number[]>([]);
  let appliedKey = $state("");
  let deleteAllOpen = $state(false);
  let deleteAllPhrase = $state("");

  const screen = $derived(
    historyScreen({
      loading,
      reachable,
      paused,
      mode,
      modelId,
      query: appliedQuery,
      narrowed: appliedNarrowed,
      rows: entries,
      selectedId,
      textView,
    }),
  );

  const detail = $derived(screen.kind === "results" ? screen.detail : null);
  const selectedRow = $derived(
    screen.kind === "results" ? screen.rows.find((row) => row.id === selectedId) : undefined,
  );
  const selectedEntry = $derived(detail == null ? undefined : entryById(detail.id));
  const groups = $derived.by(() => groupRows(screen.kind === "results" ? screen.rows : []));

  $effect(() => {
    const id = detail?.id ?? null;
    const tags = detail?.tags ?? [];
    if (id !== tagDraftId) {
      tagDraftId = id;
      tagDraft = tags.join(", ");
    }
  });
  const statusLabel = $derived(
    screen.kind === "loading" || screen.kind === "daemon-unreachable" ? null : screen.status,
  );

  function entryById(id: number): HistoryEntry | undefined {
    return entries.find((entry) => entry.id === id);
  }

  function wordsIn(text: string): number {
    const trimmed = text.trim();
    if (trimmed.length === 0 || trimmed === "(empty)") return 0;
    return trimmed.split(/\s+/).length;
  }

  function appLabel(appId: string | null | undefined): string {
    if (appId == null || appId.trim() === "") return "Unknown";
    return appId;
  }

  function groupLabel(createdAtMs: number): string {
    const stamp = new Date(createdAtMs);
    if (Number.isNaN(stamp.getTime())) return "Unknown date";
    const startOf = (day: Date) => new Date(day.getFullYear(), day.getMonth(), day.getDate()).getTime();
    const days = Math.round((startOf(new Date()) - startOf(stamp)) / 86_400_000);
    if (days <= 0) return "Today";
    if (days === 1) return "Yesterday";
    if (days < 7) return "Earlier this week";
    return stamp.toLocaleDateString(undefined, { month: "short", day: "numeric" });
  }

  function groupRows(rows: RowView[]): Group[] {
    const order: Group[] = [];
    for (const row of rows) {
      const entry = entryById(row.id);
      const label = entry ? groupLabel(entry.created_at) : "Unknown date";
      const existing = order.find((group) => group.label === label);
      if (existing) existing.rows.push(row);
      else order.push({ label, rows: [row] });
    }
    return order;
  }

  function captureLabel(count: number): string {
    return count === 1 ? "1 capture" : `${count} captures`;
  }

  function speakingWpm(entry: HistoryEntry, text: string): string | null {
    const words = wordsIn(text);
    if (words === 0 || entry.duration_ms < 1000) return null;
    return `${Math.round(words / (entry.duration_ms / 60_000))} WPM`;
  }

  function filterArgs() {
    return {
      from_day: fromDay.trim(),
      to_day: toDay.trim(),
      mode: modeFilter.trim(),
      app_id: appFilter.trim(),
      src_lang: langFilter.trim(),
    };
  }

  async function loadHistory() {
    loading = true;
    actionMessage = null;
    actionError = null;
    const prepared = prepareHistoryFilter({
      query,
      from_day: fromDay,
      to_day: toDay,
      mode: modeFilter,
      app_id: appFilter,
      src_lang: langFilter,
    });
    if (!prepared.ok) {
      actionError = prepared.error;
      loading = false;
      return;
    }
    const filters = prepared.filter;
    const narrowed = isFilterNarrowed(filters);
    const nextKey = filterCacheKey(filters);
    if (nextKey !== appliedKey) {
      checkedIds = [];
      appliedKey = nextKey;
    }
    if (prepared.isReversed) {
      entries = [];
      appliedQuery = filters.query;
      appliedNarrowed = true;
      reachable = true;
      loading = false;
      return;
    }
    const listed = invoke<HistoryEntry[]>("list_history", {
      offset: 0,
      limit: PAGE_SIZE,
      ...filters,
    });
    const configured = invoke<DashboardConfig>("load_config").then(
      (config) => config,
      () => null,
    );
    try {
      entries = await listed;
      reachable = true;
      appliedQuery = filters.query;
      appliedNarrowed = narrowed;
      if (selectedId != null && !entries.some((entry) => entry.id === selectedId)) {
        selectedId = null;
        textView = "final";
      }
      const config = await configured;
      if (config) {
        modelId = config.models.asr_english;
        mode = config.general.active_mode;
      }
      try {
        const status = await invoke<SessionStatus>("session_status");
        paused = status.paused;
        mode = status.mode;
      } catch {
        // A failed status read is not evidence the session is unpaused.
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (message.includes("invalid date")) {
        actionError = message;
        reachable = true;
      } else {
        entries = [];
        selectedId = null;
        textView = "final";
        reachable = false;
        daemonBannerOpen = true;
      }
      await configured;
    } finally {
      loading = false;
    }
  }

  function selectEntry(id: number) {
    if (selectedId === id) return;
    selectedId = id;
    textView = "final";
    actionMessage = null;
    actionError = null;
  }

  function clearSelection() {
    selectedId = null;
    textView = "final";
  }

  function focusSearch() {
    const field = document.getElementById(SEARCH_ID);
    if (field instanceof HTMLInputElement) field.focus();
  }

  function focusRow(id: number) {
    const row = document.getElementById(`history-row-${id}`);
    if (row instanceof HTMLElement) row.focus();
  }

  async function copySelected() {
    if (detail == null || actionBusy) return;
    const text = detail.text;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      if (!navigator.clipboard?.writeText) {
        throw new Error("Clipboard API unavailable in this environment.");
      }
      await navigator.clipboard.writeText(text);
      actionMessage = "Copied";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function reinsertSelected() {
    if (detail == null || actionBusy) return;
    const id = detail.id;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      await invoke("reinsert_entry", { id });
      actionMessage = "Re-inserted";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function playSelected() {
    if (detail == null || !detail.canPlay || actionBusy) return;
    const id = detail.id;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      const bytes = await invoke<number[]>("read_audio", { id });
      const blob = new Blob([new Uint8Array(bytes)], { type: "audio/wav" });
      const url = URL.createObjectURL(blob);
      const audio = new Audio(url);
      audio.onended = () => URL.revokeObjectURL(url);
      await audio.play();
      actionMessage = "Playing";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function transcribeDropped(path: string) {
    if (actionBusy) return;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      const newId = await invoke<number>("transcribe_file", {
        path,
        mode: "",
        dstLang: "",
      });
      await loadHistory();
      selectedId = newId;
      textView = "final";
      actionMessage = "Transcribed";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function retranscribeSelected() {
    if (detail == null || actionBusy) return;
    const id = detail.id;
    const mode = retranscribeMode;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      const newId = await invoke<number>("retranscribe", {
        id,
        mode,
        modelId,
      });
      await loadHistory();
      selectedId = newId;
      textView = "final";
      actionMessage = "Retranscribed";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function deleteSelected() {
    if (detail == null || actionBusy) return;
    const id = detail.id;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      await invoke("delete_entries", { ids: [id] });
      entries = entries.filter((entry) => entry.id !== id);
      selectedId = null;
      textView = "final";
      actionMessage = "Deleted";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function copyRecoveryCommand() {
    actionMessage = null;
    actionError = null;
    try {
      if (!navigator.clipboard?.writeText) {
        throw new Error("Clipboard API unavailable in this environment.");
      }
      await navigator.clipboard.writeText(RECOVERY_COMMAND);
      actionMessage = "Copied";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    }
  }

  function onSearchSubmit(event: Event) {
    event.preventDefault();
    void loadHistory();
  }

  function clearSearch() {
    query = "";
    fromDay = "";
    toDay = "";
    modeFilter = "";
    appFilter = "";
    langFilter = "";
    void loadHistory();
  }

  function onFilterCommit() {
    void loadHistory();
  }

  function searchFor(text: string) {
    query = text;
    void loadHistory();
  }

  function modelInStatus(
    status: string,
  ): { before: string; model: string; after: string } | null {
    if (modelId === "") return null;
    const marker = ` · ${modelId} · `;
    const at = status.indexOf(marker);
    if (at < 0) return null;
    return {
      before: status.slice(0, at + 3),
      model: modelId,
      after: status.slice(at + 3 + modelId.length),
    };
  }

  function moveSelection(delta: number) {
    if (entries.length === 0) return;
    const idx =
      selectedId == null
        ? delta > 0
          ? -1
          : 0
        : entries.findIndex((entry) => entry.id === selectedId);
    const next = Math.max(0, Math.min(entries.length - 1, idx + delta));
    const entry = entries[next];
    selectEntry(entry.id);
    focusRow(entry.id);
  }

  function typingInSearch(event: KeyboardEvent): boolean {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return false;
    return target.id === SEARCH_ID || target.id === TAGS_ID || target.id === DELETE_ALL_ID;
  }

  function toggleChecked(id: number) {
    if (checkedIds.includes(id)) {
      checkedIds = checkedIds.filter((existing) => existing !== id);
    } else {
      checkedIds = [...checkedIds, id];
    }
  }

  function downloadDocument(filename: string, text: string) {
    const blob = new Blob([text], { type: "text/plain;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = filename;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
  }

  async function exportHistory(format: keyof typeof EXPORT_FILES) {
    if (actionBusy) return;
    const ids = exportIds(
      checkedIds,
      entries.map((entry) => entry.id),
    );
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      const text = await invoke<string>("export_history", { ids, format });
      downloadDocument(EXPORT_FILES[format], text);
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function deleteChecked() {
    const ids = exportIds(
      checkedIds,
      entries.map((entry) => entry.id),
    );
    if (ids.length === 0 || actionBusy) return;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      await invoke("delete_entries", { ids });
      const gone = new Set(ids);
      entries = entries.filter((entry) => !gone.has(entry.id));
      checkedIds = checkedIds.filter((id) => !gone.has(id));
      if (selectedId != null && gone.has(selectedId)) {
        selectedId = null;
        textView = "final";
      }
      actionMessage = "Deleted";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function submitDeleteAll(event: Event) {
    event.preventDefault();
    if (!deleteAllConfirmed(deleteAllPhrase)) {
      actionMessage = null;
      return;
    }
    if (actionBusy) return;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      await invoke("delete_all_history");
      entries = [];
      checkedIds = [];
      selectedId = null;
      textView = "final";
      deleteAllOpen = false;
      deleteAllPhrase = "";
      actionMessage = "Deleted";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function togglePin(entry: HistoryEntry) {
    const pinned = !entry.pinned;
    actionError = null;
    try {
      await invoke("set_pinned", { id: entry.id, pinned });
      entries = entries.map((row) => (row.id === entry.id ? { ...row, pinned } : row));
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    }
  }

  async function commitTags() {
    if (detail == null || actionBusy || tagCommitBusy) return;
    const id = detail.id;
    const names = tagDraft.length === 0 ? [] : tagDraft.split(",");
    tagCommitBusy = true;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      await invoke("set_tags", { id, names });
      const listed = await invoke<HistoryEntry[]>("list_history", {
        query: query.trim(),
        offset: 0,
        limit: PAGE_SIZE,
        ...filterArgs(),
      });
      entries = listed;
      const updated = listed.find((row) => row.id === id);
      tagDraft = updated == null ? "" : updated.tags.join(", ");
      actionMessage = "Tags saved";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      tagCommitBusy = false;
      actionBusy = false;
    }
  }

  function onKeyDown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      if (selectedId != null) {
        event.preventDefault();
        clearSelection();
      }
      return;
    }

    if (event.isComposing || typingInSearch(event)) return;
    if (event.ctrlKey || event.metaKey || event.altKey) return;

    if (event.key === "/") {
      event.preventDefault();
      focusSearch();
      return;
    }

    if (loading || !reachable || entries.length === 0) return;

    if (event.key === "ArrowDown" || event.key === "j" || event.key === "J") {
      event.preventDefault();
      moveSelection(1);
    } else if (event.key === "ArrowUp" || event.key === "k" || event.key === "K") {
      event.preventDefault();
      moveSelection(-1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      void reinsertSelected();
    } else if (event.key === "c" || event.key === "C") {
      event.preventDefault();
      void copySelected();
    } else if (event.key === "Delete") {
      event.preventDefault();
      void deleteSelected();
    }
  }

  onMount(() => {
    void loadHistory();
    if (!isTauri()) return;
    const unlisten = getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type !== "drop") return;
      const path = firstDroppedPath(event.payload.paths);
      if (path == null) return;
      void transcribeDropped(path);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  });
</script>

<svelte:window onkeydown={onKeyDown} />

{#snippet searchField()}
  <form
    class="relative flex-1 flex items-center bg-surface-container rounded-lg px-space-sm py-1.5 border border-outline-variant/40 focus-within:border-primary-container transition-colors"
    onsubmit={onSearchSubmit}
  >
    <label class="sr-only" for={SEARCH_ID}>Search history</label>
    <span class="material-symbols-outlined text-on-surface-variant text-[18px] mr-space-xs">search</span>
    <input
      id={SEARCH_ID}
      class="w-full bg-transparent border-none outline-none font-body-default text-body-default text-on-surface placeholder:text-on-surface-variant/60"
      type="search"
      placeholder="Search transcripts"
      bind:value={query}
      autocomplete="off"
      spellcheck="false"
    />
    <div class="flex items-center gap-1 shrink-0 ml-1">
      <kbd class="px-1.5 py-0.5 rounded bg-surface-container-high border border-outline-variant/40 font-keycap text-keycap text-on-surface-variant">/</kbd>
    </div>
  </form>
{/snippet}

{#snippet filterBar()}
  <div class="p-space-md flex flex-col gap-space-sm border-b border-outline-variant/20 bg-surface-container-low/80 backdrop-blur-sm z-10">
    <div class="flex items-center justify-between gap-space-sm">
      {@render searchField()}
      <button
        class="p-2 rounded-lg bg-surface-container hover:bg-surface-container-high text-on-surface-variant hover:text-on-surface border border-outline-variant/30 transition-colors flex items-center justify-center"
        title="Refresh"
        type="button"
        onclick={() => loadHistory()}
      >
        <span class="material-symbols-outlined text-[18px]">tune</span>
      </button>
    </div>
    <div class="flex items-center gap-space-xs overflow-x-auto pb-0.5 text-on-surface-variant font-body-muted text-body-muted">
      <input
        class="px-2.5 py-1 rounded bg-surface-container-high text-on-surface border border-outline-variant/40 shrink-0"
        type="date"
        aria-label="From day"
        bind:value={fromDay}
        onchange={onFilterCommit}
      />
      <input
        class="px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/30 shrink-0"
        type="date"
        aria-label="To day"
        bind:value={toDay}
        onchange={onFilterCommit}
      />
      <select
        class="w-28 px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/30 shrink-0 font-body-muted text-body-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
        aria-label="Filter by mode"
        bind:value={modeFilter}
        onchange={onFilterCommit}
      >
        <option value="">All modes</option>
        {#each ACTIVE_MODES as m}
          <option value={m}>{m.charAt(0).toUpperCase() + m.slice(1)}</option>
        {/each}
      </select>
      <input
        class="w-40 px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/30 shrink-0"
        type="text"
        aria-label="App id"
        placeholder="App id"
        bind:value={appFilter}
        onchange={onFilterCommit}
      />
      <input
        class="w-24 px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/30 shrink-0"
        type="text"
        aria-label="Source language"
        placeholder="Language"
        bind:value={langFilter}
        onchange={onFilterCommit}
      />
    </div>
    {#if reachable && !loading}
      {@render historyActions()}
    {/if}
  </div>
{/snippet}

{#snippet historyActions()}
  <div class="flex flex-wrap items-center gap-1.5">
    <button
      class="px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/40 font-body-muted text-body-muted hover:bg-surface-container-high disabled:opacity-50"
      type="button"
      disabled={actionBusy}
      onclick={() => exportHistory("json")}
    >Export JSON</button>
    <button
      class="px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/40 font-body-muted text-body-muted hover:bg-surface-container-high disabled:opacity-50"
      type="button"
      disabled={actionBusy}
      onclick={() => exportHistory("markdown")}
    >Export Markdown</button>
    <button
      class="px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/40 font-body-muted text-body-muted hover:bg-surface-container-high disabled:opacity-50"
      type="button"
      disabled={actionBusy}
      onclick={() => exportHistory("txt")}
    >Export TXT</button>
    {#if checkedIds.length > 0}
      <button
        class="px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/40 font-body-muted text-body-muted hover:text-error disabled:opacity-50"
        type="button"
        disabled={actionBusy}
        onclick={() => deleteChecked()}
      >Delete selected</button>
    {/if}
    {#if deleteAllOpen}
      <form class="flex items-center gap-1.5" onsubmit={submitDeleteAll}>
        <label class="sr-only" for={DELETE_ALL_ID}>Type delete all history</label>
        <input
          id={DELETE_ALL_ID}
          class="w-44 px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/40"
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="delete all history"
          bind:value={deleteAllPhrase}
        />
        <button
          class="px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/40 font-body-muted text-body-muted hover:text-error disabled:opacity-50"
          type="submit"
          disabled={actionBusy}
        >Delete all</button>
      </form>
    {:else}
      <button
        class="px-2.5 py-1 rounded bg-surface-container text-on-surface border border-outline-variant/40 font-body-muted text-body-muted hover:text-error disabled:opacity-50"
        type="button"
        disabled={actionBusy}
        onclick={() => (deleteAllOpen = true)}
      >Delete all</button>
    {/if}
  </div>
{/snippet}

<div class="flex flex-col w-full h-[calc(100vh-3.5rem)] min-h-0 overflow-hidden bg-surface text-on-surface font-body-default text-body-default antialiased">
  {#if actionError}
    <p class="px-space-md py-2 bg-error-container/40 border-b border-error/20 text-error font-body-muted text-body-muted" role="alert">
      {actionError}
    </p>
  {/if}

  {#if screen.kind === "empty" || screen.kind === "no-matches" || screen.kind === "daemon-unreachable"}
    <div class="flex-1 min-h-0 overflow-y-auto">
      {#if screen.kind !== "no-matches"}
        {@render filterBar()}
      {/if}
      <div class="w-full max-w-7xl mx-auto p-space-md lg:p-space-xl flex flex-col gap-space-xl">
        {#if screen.kind === "empty"}
          <section class="flex flex-col rounded-xl bg-surface-container-low border border-outline-variant overflow-hidden">
            <div class="py-16 px-space-md flex flex-col items-center justify-center text-center max-w-xl mx-auto w-full">
              <div class="relative w-24 h-24 rounded-full bg-surface-container flex items-center justify-center border border-outline-variant mb-space-lg">
                <div class="absolute inset-1 rounded-full border border-dashed border-outline-variant opacity-40"></div>
                <span class="material-symbols-outlined text-[44px] text-outline">mic_off</span>
              </div>
              <h2 class="font-headline-lg text-headline-lg text-on-surface tracking-tight mb-space-xs">{screen.title}</h2>
              <p class="font-body-default text-body-default text-on-surface-variant max-w-md leading-relaxed mb-space-sm">{screen.body}</p>
              <p class="font-body-default text-body-default text-on-surface-variant max-w-md leading-relaxed mb-space-lg">
                Press
                <kbd class="inline-flex items-center px-1.5 py-0.5 bg-surface-container-high border border-outline rounded font-keycap text-keycap text-on-surface shadow-sm">Ctrl</kbd>
                +
                <kbd class="inline-flex items-center px-1.5 py-0.5 bg-surface-container-high border border-outline rounded font-keycap text-keycap text-on-surface shadow-sm">Alt</kbd>
                +
                <kbd class="inline-flex items-center px-1.5 py-0.5 bg-surface-container-high border border-outline rounded font-keycap text-keycap text-on-surface shadow-sm">Space</kbd>
                and start talking anywhere in GNOME. Your transcribed text will be saved here privately.
              </p>
              <div class="flex flex-wrap items-center justify-center gap-space-sm mb-space-xl">
                <a
                  class="flex items-center gap-space-xs bg-surface-container hover:bg-surface-container-high text-on-surface px-space-md py-2 rounded-lg font-headline-md text-headline-md border border-outline-variant transition-colors"
                  href="/settings"
                >
                  <span class="material-symbols-outlined text-[18px]">tune</span>
                  <span>Configure Shortcut</span>
                </a>
              </div>
              <div class="flex items-center gap-space-xs px-space-md py-1.5 rounded-full bg-surface-container border border-outline-variant text-on-surface-variant font-label-code text-label-code">
                <span class="material-symbols-outlined text-[14px] text-emerald-400">lock</span>
                <span>100% On-device · Zero network egress guaranteed</span>
              </div>
            </div>
          </section>
        {:else if screen.kind === "no-matches"}
          <section class="flex flex-col rounded-xl bg-surface-container-low border border-outline-variant overflow-hidden">
            <div class="p-space-md bg-surface-container/60 border-b border-outline-variant flex flex-col md:flex-row items-center gap-space-md">
              <form
                class="flex items-center bg-surface-container px-3 py-1.5 rounded-lg border border-primary-container gap-2 w-full md:w-80"
                onsubmit={onSearchSubmit}
              >
                <span class="material-symbols-outlined text-primary-container text-[18px]">search</span>
                <label class="sr-only" for={SEARCH_ID}>Search history</label>
                <input
                  id={SEARCH_ID}
                  class="font-body-default text-body-default text-on-surface truncate flex-1 bg-transparent border-none outline-none placeholder:text-on-surface-variant/60 min-w-0"
                  type="search"
                  placeholder="Search transcripts..."
                  bind:value={query}
                  autocomplete="off"
                  spellcheck="false"
                />
                <button
                  aria-label="Clear search"
                  class="w-4 h-4 rounded-full bg-surface-container-high flex items-center justify-center text-on-surface-variant hover:text-on-surface"
                  type="button"
                  onclick={clearSearch}
                >
                  <span class="material-symbols-outlined text-[12px]">close</span>
                </button>
              </form>
              <div class="flex items-center gap-1.5 flex-wrap w-full md:w-auto">
                <span class="px-2.5 py-1 rounded bg-surface-container border border-outline-variant font-label-code text-label-code text-on-surface flex items-center gap-1">
                  <span>Mode: All</span>
                  <span class="material-symbols-outlined text-[14px] text-outline">expand_more</span>
                </span>
                <span class="font-body-muted text-body-muted text-on-surface-variant ml-auto md:ml-2">0 matches</span>
              </div>
              {#if reachable && !loading}
                {@render historyActions()}
              {/if}
            </div>
            <div class="py-14 px-space-md flex flex-col items-center justify-center text-center max-w-lg mx-auto w-full">
              <div class="w-20 h-20 rounded-full bg-surface-container flex items-center justify-center border border-outline-variant mb-space-md">
                <span class="material-symbols-outlined text-[36px] text-outline">search_off</span>
              </div>
              <h3 class="font-headline-lg text-headline-lg text-on-surface tracking-tight mb-space-xs">{screen.title}</h3>
              <p class="font-body-default text-body-default text-on-surface-variant max-w-md leading-relaxed mb-space-lg">
                No recordings or transcriptions matched
                <span class="font-label-code text-label-code text-primary bg-surface-container px-1 py-0.5 rounded border border-outline-variant">"{appliedQuery}"</span>.
                Try checking for typos or resetting filter boundaries.
              </p>
              <button
                class="flex items-center gap-space-xs bg-surface-container hover:bg-surface-container-high text-on-surface px-space-md py-1.5 rounded-lg font-headline-md text-headline-md border border-outline-variant transition-colors mb-space-lg"
                type="button"
                onclick={clearSearch}
              >
                <span class="material-symbols-outlined text-[18px]">filter_alt_off</span>
                <span>{screen.clear}</span>
              </button>
              <div class="flex flex-col items-center gap-space-xs">
                <span class="font-body-muted text-body-muted text-on-surface-variant">Try searching for recent topics:</span>
                <div class="flex flex-wrap gap-1.5 justify-center">
                  {#each ["email draft", "code comment", "Hindi", "git commit"] as topic (topic)}
                    <button
                      class="px-2.5 py-1 rounded-full bg-surface-container border border-outline-variant text-on-surface font-label-code text-label-code hover:bg-surface-container-high transition-colors"
                      type="button"
                      onclick={() => searchFor(topic)}
                    >
                      "{topic}"
                    </button>
                  {/each}
                </div>
              </div>
            </div>
          </section>
        {:else}
          <section class="flex flex-col rounded-xl bg-surface-container-low border border-outline-variant overflow-hidden">
            {#if daemonBannerOpen}
              <div class="px-space-md py-2.5 bg-error-container/40 border-b border-error/20 flex items-center justify-between gap-space-sm">
                <div class="flex items-center gap-space-sm">
                  <span class="material-symbols-outlined text-[18px] text-error shrink-0">error</span>
                  <span class="font-body-muted text-body-muted text-on-error-container">
                    {screen.body}
                    (<code class="font-label-code text-[11px] bg-black/30 px-1 py-0.5 rounded">narad.service</code>)
                  </span>
                </div>
                <button
                  aria-label="Dismiss banner"
                  class="text-on-error-container hover:text-white transition-colors"
                  type="button"
                  onclick={() => (daemonBannerOpen = false)}
                >
                  <span class="material-symbols-outlined text-[16px]">close</span>
                </button>
              </div>
            {/if}
            <div class="py-12 px-space-md flex flex-col items-center justify-center text-center max-w-xl mx-auto w-full">
              <div class="relative w-20 h-20 rounded-2xl bg-surface-container border border-error/30 flex items-center justify-center mb-space-md">
                <span class="material-symbols-outlined text-[40px] text-error">settings_suggest</span>
                <div class="absolute -bottom-1 -right-1 w-6 h-6 rounded-full bg-error-container border border-error flex items-center justify-center">
                  <span class="material-symbols-outlined text-[14px] text-on-error">priority_high</span>
                </div>
              </div>
              <h3 class="font-headline-lg text-headline-lg text-on-surface tracking-tight mb-space-xs">{screen.title}</h3>
              <p class="font-body-default text-body-default text-on-surface-variant max-w-md leading-relaxed mb-space-lg">{screen.body}</p>
              <div class="w-full bg-surface-container-lowest p-space-sm rounded-lg border border-outline-variant mb-space-lg text-left">
                <div class="flex items-center justify-between text-on-surface-variant font-label-code text-[11px] mb-1.5">
                  <span>Terminal Recovery:</span>
                  <button class="text-primary font-body-muted hover:underline" type="button" onclick={() => copyRecoveryCommand()}>Copy</button>
                </div>
                <div class="flex items-center gap-2 font-label-code text-label-code text-on-surface overflow-x-auto">
                  <span class="text-outline select-none">$</span>
                  <code>{RECOVERY_COMMAND}</code>
                </div>
              </div>
              <div class="flex flex-wrap items-center justify-center gap-space-sm mb-space-lg">
                <button
                  class="flex items-center gap-space-xs bg-primary-container hover:bg-primary text-on-primary-container px-space-md py-2 rounded-lg font-headline-md text-headline-md transition-all shadow-sm"
                  type="button"
                  onclick={() => loadHistory()}
                >
                  <span class="material-symbols-outlined text-[18px]">play_circle</span>
                  <span>{screen.recovery}</span>
                </button>
              </div>
              <div class="w-full bg-surface-container rounded-lg p-2.5 border border-outline-variant flex flex-wrap items-center justify-between text-left gap-2 text-on-surface-variant font-label-code text-label-code">
                <div class="flex items-center gap-1.5">
                  <span class="w-2 h-2 rounded-full bg-error"></span>
                  <span>Bus: dev.narad.Daemon [Unreachable]</span>
                </div>
                <div class="flex items-center gap-3">
                  <span>PID: —</span>
                </div>
              </div>
            </div>
          </section>
        {/if}
      </div>
    </div>
  {:else}
    <div class="flex flex-1 h-full min-h-0 w-full">
      <section class="flex flex-col w-full lg:w-[480px] xl:w-[520px] shrink-0 bg-surface-container-low border-r border-outline-variant/30 select-none">
        {@render filterBar()}
        <div
          class="flex-1 overflow-y-auto px-space-md py-space-sm flex flex-col gap-space-sm select-text"
          role="listbox"
          aria-label="History entries"
        >
          {#if screen.kind === "loading"}
            <p class="px-2 py-6 font-body-muted text-body-muted text-on-surface-variant">Loading…</p>
          {:else}
            {#each groups as group (group.label)}
              <div class="flex flex-col gap-1.5 pt-1">
                <div class="px-2 py-1 flex items-center justify-between text-on-surface-variant/70 select-none">
                  <span class="font-label-code text-label-code uppercase tracking-wider text-[11px] font-semibold text-outline">{group.label}</span>
                  <span class="font-label-code text-label-code text-[11px]">{captureLabel(group.rows.length)}</span>
                </div>
                {#each group.rows as row (row.id)}
                  {@const entry = entryById(row.id)}
                  {#if entry}
                    <div class="flex items-start gap-2">
                      <input
                        class="mt-3 shrink-0"
                        type="checkbox"
                        aria-label="Select entry"
                        checked={checkedIds.includes(row.id)}
                        onclick={(event) => event.stopPropagation()}
                        onchange={() => toggleChecked(row.id)}
                      />
                      <button
                        type="button"
                        id="history-row-{row.id}"
                        class={`${selectedId === row.id ? rowOn : rowOff} !w-auto flex-1 min-w-0`}
                        role="option"
                        aria-selected={selectedId === row.id}
                        onclick={() => selectEntry(row.id)}
                      >
                      <div class="flex items-start justify-between gap-2 mb-1.5">
                        <div class="flex items-center gap-2 min-w-0">
                          <div class="w-6 h-6 rounded flex items-center justify-center bg-surface-container-high text-primary shrink-0 border border-outline-variant/40">
                            <span class="material-symbols-outlined text-[14px]">mic</span>
                          </div>
                          <div class="flex items-center gap-1.5 min-w-0">
                            <span class="font-headline-md text-[13px] text-on-surface font-semibold truncate">{appLabel(entry.app_id)}</span>
                            {#if entry.src_lang}
                              <span class="text-on-surface-variant text-[12px] truncate">{entry.src_lang}</span>
                            {/if}
                          </div>
                        </div>
                        <div class="flex items-center gap-1.5 shrink-0">
                          <span class="px-1.5 py-0.5 rounded bg-surface-container-high border border-outline-variant/40 font-keycap text-keycap text-on-surface-variant">{entry.mode}</span>
                          <time class="font-label-code text-label-code text-on-surface-variant text-[11px]" datetime={new Date(entry.created_at).toISOString()}>
                            {formatTime(entry.created_at)}
                          </time>
                          <span
                            role="button"
                            tabindex="-1"
                            class={entry.pinned ? "text-primary" : "text-on-surface-variant/40"}
                            title={entry.pinned ? "Pinned" : "Not pinned"}
                            aria-pressed={entry.pinned}
                            onclick={(event) => {
                              event.stopPropagation();
                              event.preventDefault();
                              void togglePin(entry);
                            }}
                            onkeydown={(event) => {
                              if (event.key !== "Enter" && event.key !== " ") return;
                              event.stopPropagation();
                              event.preventDefault();
                              void togglePin(entry);
                            }}
                          >
                            <span
                              class="material-symbols-outlined text-[16px]"
                              style={entry.pinned ? "font-variation-settings: 'FILL' 1;" : undefined}
                            >star</span>
                          </span>
                        </div>
                      </div>
                      <p class="font-body-default text-[13px] leading-relaxed text-on-surface line-clamp-2 select-text">{row.preview}</p>
                      <div class="mt-2.5 flex items-center justify-between text-on-surface-variant font-label-code text-[11px]">
                        <div class="flex items-center gap-2">
                          <span class="inline-flex items-center gap-1 {selectedId === row.id ? 'text-primary' : ''}">
                            <span class="material-symbols-outlined text-[14px]">graphic_eq</span>
                            {formatDuration(entry.duration_ms)}
                          </span>
                          <span>·</span>
                          <span>{wordsIn(row.preview)} words</span>
                        </div>
                        <span class="text-[10px] text-on-surface-variant/80">{row.insert}</span>
                      </div>
                      </button>
                    </div>
                  {/if}
                {/each}
              </div>
            {/each}
          {/if}
        </div>
      </section>

      <main class="flex-1 min-w-0 flex flex-col h-full bg-surface overflow-y-auto">
        {#if detail && selectedEntry}
          <header class="p-space-md border-b border-outline-variant/30 bg-surface flex flex-wrap items-center justify-between gap-space-md sticky top-0 z-20">
            <div class="flex items-center gap-3 min-w-0">
              <div class="w-9 h-9 rounded-lg flex items-center justify-center bg-surface-container-high text-primary border border-outline-variant/40">
                <span class="material-symbols-outlined text-[20px]">mic</span>
              </div>
              <div class="flex flex-col min-w-0">
                <span class="font-headline-md text-[13px] text-on-surface font-semibold truncate">{appLabel(selectedEntry.app_id)}</span>
                <div class="flex items-center gap-1.5 flex-wrap font-label-code text-label-code text-[11px] text-on-surface-variant">
                  <span class="material-symbols-outlined text-[13px] text-primary">schedule</span>
                  <span>{formatTime(selectedEntry.created_at)}</span>
                  <span class="text-outline">·</span>
                  <span>{selectedEntry.mode}</span>
                  <span class="text-outline">·</span>
                  <span>{formatDuration(selectedEntry.duration_ms)}</span>
                </div>
              </div>
            </div>
            <div class="flex items-center gap-space-xs shrink-0">
              <button
                class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-surface-container hover:bg-surface-container-high text-on-surface border border-outline-variant/40 font-headline-md text-[13px] transition-colors disabled:opacity-50"
                id="copy-btn"
                title="Copy transcript to clipboard"
                type="button"
                disabled={actionBusy}
                onclick={() => copySelected()}
              >
                <span class="material-symbols-outlined text-[16px] text-primary">content_copy</span>
                <span>Copy</span>
                <kbd class="ml-1 px-1 py-0.2 bg-surface-container-highest rounded text-[10px] font-keycap text-on-surface-variant">C</kbd>
              </button>
              <button
                class="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary-container text-on-primary-container hover:bg-primary font-headline-md text-[13px] font-semibold transition-colors shadow-xs disabled:opacity-50"
                id="reinsert-btn"
                title="Re-insert text at cursor in target window"
                type="button"
                disabled={actionBusy}
                onclick={() => reinsertSelected()}
              >
                <span class="material-symbols-outlined text-[16px]">input</span>
                <span>Re-insert</span>
                <kbd class="ml-1 px-1 py-0.2 bg-primary/20 text-on-primary-container rounded text-[10px] font-keycap font-semibold">Enter</kbd>
              </button>
              {#if detail.canPlay}
                <button
                  class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-surface-container hover:bg-surface-container-high text-on-surface border border-outline-variant/40 font-headline-md text-[13px] transition-colors disabled:opacity-50"
                  id="play-btn"
                  title="Play retained audio"
                  type="button"
                  disabled={actionBusy}
                  onclick={() => playSelected()}
                >
                  <span class="material-symbols-outlined text-[16px] text-primary">play_arrow</span>
                  <span>Play</span>
                </button>
              {/if}
              <label class="sr-only" for="retranscribe-mode">Retranscribe mode</label>
              <select
                id="retranscribe-mode"
                class="px-2 py-1.5 rounded-lg bg-surface-container text-on-surface border border-outline-variant/40 font-headline-md text-[13px] disabled:opacity-50"
                bind:value={retranscribeMode}
                disabled={actionBusy}
              >
                {#each ACTIVE_MODES as item (item)}
                  <option value={item}>{item}</option>
                {/each}
              </select>
              <button
                class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-surface-container hover:bg-surface-container-high text-on-surface border border-outline-variant/40 font-headline-md text-[13px] transition-colors disabled:opacity-50"
                id="retranscribe-btn"
                title="Retranscribe retained audio with the configured model"
                type="button"
                disabled={actionBusy}
                onclick={() => retranscribeSelected()}
              >
                <span class="material-symbols-outlined text-[16px] text-primary">replay</span>
                <span>Retranscribe</span>
              </button>
              <div class="h-4 w-px bg-outline-variant/40 mx-0.5"></div>
              <button
                class="flex items-center gap-1 px-2 py-1.5 rounded-lg bg-surface-container hover:bg-error-container/30 text-on-surface-variant hover:text-error border border-outline-variant/40 transition-colors text-[13px] disabled:opacity-50"
                id="delete-transcript-btn"
                title="Delete record"
                type="button"
                disabled={actionBusy}
                onclick={() => deleteSelected()}
              >
                <span class="material-symbols-outlined text-[16px]">delete</span>
                <kbd class="px-1 py-0.2 bg-surface-container-highest rounded text-[10px] font-keycap">Del</kbd>
              </button>
            </div>
          </header>
          <div class="p-space-xl flex flex-col gap-space-lg max-w-4xl mx-auto w-full">
            {#if actionMessage}
              <div class="flex items-center justify-between px-3.5 py-2 rounded-lg bg-surface-container-high border border-outline-variant/40 text-on-surface text-[12px] font-label-code">
                <div class="flex items-center gap-2">
                  <span class="material-symbols-outlined text-[16px] text-primary">history</span>
                  <span>Last action: {actionMessage}</span>
                </div>
              </div>
            {/if}
            <div class="flex flex-col rounded-xl bg-surface-container border border-outline-variant/40 overflow-hidden">
              <div class="px-space-md py-2 bg-surface-container-high/60 border-b border-outline-variant/30 flex items-center justify-between">
                <div class="inline-flex p-0.5 rounded-lg bg-surface-container-lowest border border-outline-variant/30 text-xs" role="group" aria-label="Final or raw text">
                  <button
                    class={textView === "final" ? segmentOn : segmentOff}
                    type="button"
                    aria-pressed={textView === "final"}
                    onclick={() => (textView = "final")}
                  >Final (Cleaned)</button>
                  <button
                    class={textView === "raw" ? segmentOn : segmentOff}
                    type="button"
                    aria-pressed={textView === "raw"}
                    onclick={() => (textView = "raw")}
                  >Raw ASR Frames</button>
                </div>
                <div class="flex items-center gap-2 font-label-code text-label-code text-on-surface-variant text-[11px]">
                  <span>{selectedEntry.mode}</span>
                  <span>·</span>
                  <span>{selectedEntry.src_lang}</span>
                </div>
              </div>
              <div class="p-space-lg bg-surface-container-low min-h-[140px] flex items-center">
                <p class="font-body-default text-[17px] leading-relaxed text-on-surface font-normal select-text tracking-tight whitespace-pre-wrap break-words" id="transcript-display">
                  {detail.text}
                </p>
              </div>
              <div class="px-space-md py-2 bg-surface-container-high/30 border-t border-outline-variant/20 flex items-center justify-between text-on-surface-variant font-label-code text-[11px]">
                <span>{detail.text.length} characters · {wordsIn(detail.text)} words</span>
              </div>
            </div>
            <div class="flex flex-col rounded-xl bg-surface-container border border-outline-variant/30 overflow-hidden divide-y divide-outline-variant/20">
              <div class="px-space-md py-2.5 flex items-center justify-between text-on-surface gap-space-md">
                <div class="flex items-center gap-2">
                  <span class="material-symbols-outlined text-[16px] text-on-surface-variant">memory</span>
                  <span class="font-body-default text-[13px]">Acoustic Model</span>
                </div>
                <span class="font-label-code text-label-code text-primary bg-primary-container/15 px-2 py-0.5 rounded border border-primary-container/30">{selectedEntry.asr_model}</span>
              </div>
              <div class="px-space-md py-2.5 flex items-center justify-between text-on-surface gap-space-md">
                <div class="flex items-center gap-2">
                  <span class="material-symbols-outlined text-[16px] text-on-surface-variant">translate</span>
                  <span class="font-body-default text-[13px]">Language</span>
                </div>
                <div class="flex items-center gap-1.5 font-label-code text-label-code text-on-surface">
                  <span class="px-1.5 py-0.2 rounded bg-surface-container-high border border-outline-variant/30 text-[11px]">{selectedEntry.src_lang}</span>
                  {#if selectedEntry.dst_lang && selectedEntry.dst_lang !== selectedEntry.src_lang}
                    <span class="text-outline">•</span>
                    <span class="px-1.5 py-0.2 rounded bg-surface-container-high border border-outline-variant/30 text-[11px]">{selectedEntry.dst_lang}</span>
                  {/if}
                </div>
              </div>
              <div class="px-space-md py-2.5 flex items-center justify-between text-on-surface gap-space-md">
                <div class="flex items-center gap-2">
                  <span class="material-symbols-outlined text-[16px] text-on-surface-variant">speed</span>
                  <span class="font-body-default text-[13px]">Speaking Velocity &amp; Timing</span>
                </div>
                <div class="flex items-center gap-2 font-label-code text-label-code text-on-surface">
                  <span>{wordsIn(detail.text)} words</span>
                  <span class="text-outline">·</span>
                  <span>{formatDuration(selectedEntry.duration_ms)}</span>
                  {#if speakingWpm(selectedEntry, detail.text)}
                    <span class="text-outline">·</span>
                    <span class="text-primary font-semibold">{speakingWpm(selectedEntry, detail.text)}</span>
                  {/if}
                </div>
              </div>
              <div class="px-space-md py-2.5 flex items-center justify-between text-on-surface gap-space-md">
                <label class="flex items-center gap-2 shrink-0" for={TAGS_ID}>
                  <span class="material-symbols-outlined text-[16px] text-on-surface-variant">label</span>
                  <span class="font-body-default text-[13px]">Tags</span>
                </label>
                <input
                  id={TAGS_ID}
                  class="min-w-0 flex-1 max-w-md bg-surface-container-low border border-outline-variant/40 rounded px-2 py-1 font-body-default text-[13px] text-on-surface"
                  type="text"
                  placeholder="comma-separated tags"
                  bind:value={tagDraft}
                  onchange={() => void commitTags()}
                  onkeydown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      event.stopPropagation();
                      void commitTags();
                    }
                  }}
                />
              </div>
              <div class="px-space-md py-2.5 flex items-center justify-between text-on-surface gap-space-md">
                <div class="flex items-center gap-2">
                  <span class="material-symbols-outlined text-[16px] text-on-surface-variant">near_me</span>
                  <span class="font-body-default text-[13px]">Insertion</span>
                </div>
                <span class="font-label-code text-label-code text-on-surface">{selectedRow?.insert ?? selectedEntry.insert_status}</span>
              </div>
            </div>
          </div>
        {:else}
          <div class="flex-1 flex flex-col items-center justify-center text-center p-space-xl">
            <div class="w-16 h-16 rounded-full bg-surface-container flex items-center justify-center border border-outline-variant mb-space-md">
              <span class="material-symbols-outlined text-[32px] text-outline">history</span>
            </div>
            <p class="font-headline-md text-headline-md text-on-surface">Select a transcription</p>
            <p class="font-body-muted text-body-muted text-on-surface-variant mt-space-xs">j / k to move. Enter re-inserts the open entry.</p>
          </div>
        {/if}
      </main>
    </div>
  {/if}

  <footer class="h-8 bg-surface-container-low border-t border-outline-variant/30 px-space-md flex items-center justify-between text-on-surface-variant font-label-code text-[11px] shrink-0 select-none">
    <div class="flex items-center gap-3 overflow-x-auto">
      <span class="font-semibold text-outline uppercase tracking-wider text-[10px]">Shortcuts</span>
      <span class="flex items-center gap-1"><kbd class="px-1 rounded bg-surface-container-high border border-outline-variant/40 font-keycap text-[10px]">j/k</kbd> Navigate</span>
      <span class="flex items-center gap-1"><kbd class="px-1 rounded bg-surface-container-high border border-outline-variant/40 font-keycap text-[10px]">Enter</kbd> Re-insert</span>
      <span class="flex items-center gap-1"><kbd class="px-1 rounded bg-surface-container-high border border-outline-variant/40 font-keycap text-[10px]">C</kbd> Copy</span>
      <span class="flex items-center gap-1"><kbd class="px-1 rounded bg-surface-container-high border border-outline-variant/40 font-keycap text-[10px]">Del</kbd> Delete</span>
      <span class="flex items-center gap-1"><kbd class="px-1 rounded bg-surface-container-high border border-outline-variant/40 font-keycap text-[10px]">/</kbd> Search</span>
      <span class="flex items-center gap-1"><kbd class="px-1 rounded bg-surface-container-high border border-outline-variant/40 font-keycap text-[10px]">Esc</kbd> Close</span>
    </div>
    <div class="hidden md:flex items-center gap-2 min-w-0">
      {#if statusLabel}
        {@const marked = modelInStatus(statusLabel)}
        <span class="truncate">
          {#if marked}
            {marked.before}<span class="font-label-code">{marked.model}</span>{marked.after}
          {:else}
            {statusLabel}
          {/if}
        </span>
      {/if}
    </div>
  </footer>
</div>

{#if actionMessage && !actionError}
  <p
    class="fixed z-50 left-1/2 bottom-10 -translate-x-1/2 m-0 px-3 py-1.5 rounded-lg border border-outline-variant bg-surface-container-high text-emerald-400 font-body-muted text-body-muted"
    role="status"
  >{actionMessage}</p>
{/if}

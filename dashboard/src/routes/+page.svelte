<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { DashboardConfig } from "$lib/config";
  import { formatTime, type HistoryEntry, type TextView } from "$lib/history";
  import { historyScreen } from "$lib/screen-state";

  const PAGE_SIZE = 50;
  const SEARCH_ID = "history-search";

  type SessionStatus = {
    paused: boolean;
    mode: string;
  };

  let query = $state("");
  let appliedQuery = $state("");
  let entries = $state<HistoryEntry[]>([]);
  let loading = $state(true);
  let reachable = $state(true);
  let paused = $state(false);
  let mode = $state("plain");
  let modelId = $state("");

  let selectedId = $state<number | null>(null);
  let textView = $state<TextView>("final");
  let actionBusy = $state(false);
  let actionMessage = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  const screen = $derived(
    historyScreen({
      loading,
      reachable,
      paused,
      mode,
      modelId,
      query: appliedQuery,
      rows: entries,
      selectedId,
      textView,
    }),
  );

  const detail = $derived(screen.kind === "results" ? screen.detail : null);

  function entryById(id: number): HistoryEntry | undefined {
    return entries.find((entry) => entry.id === id);
  }

  async function loadHistory() {
    loading = true;
    actionMessage = null;
    actionError = null;
    const trimmed = query.trim();
    const listed = invoke<HistoryEntry[]>("list_history", {
      query: trimmed,
      offset: 0,
      limit: PAGE_SIZE,
    });
    const configured = invoke<DashboardConfig>("load_config").then(
      (config) => config,
      () => null,
    );
    try {
      entries = await listed;
      reachable = true;
      appliedQuery = trimmed;
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
    } catch {
      entries = [];
      selectedId = null;
      textView = "final";
      reachable = false;
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

  function onSearchSubmit(event: Event) {
    event.preventDefault();
    void loadHistory();
  }

  function clearSearch() {
    query = "";
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
    return target instanceof HTMLElement && target.id === SEARCH_ID;
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
  });
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="shell">
  <header class="header">
    <h1>History</h1>
    <form class="search" onsubmit={onSearchSubmit}>
      <label class="sr-only" for={SEARCH_ID}>Search history</label>
      <input
        id={SEARCH_ID}
        type="search"
        placeholder="Search…"
        bind:value={query}
        autocomplete="off"
        spellcheck="false"
      />
    </form>
  </header>

  <main class="main">
    {#if actionError}
      <p class="action-feedback error" role="alert">{actionError}</p>
    {/if}

    {#if screen.kind === "loading"}
      <p class="status">Loading…</p>
    {:else if screen.kind === "daemon-unreachable"}
      <div class="empty">
        <p class="empty-title">{screen.title}</p>
        <p class="empty-body">{screen.body}</p>
        <button type="button" onclick={() => loadHistory()}>{screen.recovery}</button>
      </div>
    {:else}
      {@const marked = modelInStatus(screen.status)}
      <p class="status">
        {#if marked}
          {marked.before}<span class="model-id">{marked.model}</span>{marked.after}
        {:else}
          {screen.status}
        {/if}
      </p>
      {#if screen.kind === "empty"}
        <div class="empty">
          <p class="empty-title">{screen.title}</p>
          <p class="empty-body">{screen.body}</p>
        </div>
      {:else if screen.kind === "no-matches"}
        <div class="empty">
          <p class="empty-title">{screen.title}</p>
          <button type="button" onclick={clearSearch}>{screen.clear}</button>
        </div>
      {:else}
      <div class="workspace" class:with-detail={detail != null}>
        <ul class="list" role="listbox" aria-label="History entries">
          {#each screen.rows as row (row.id)}
            {@const entry = entryById(row.id)}
            <li>
              <button
                type="button"
                id="history-row-{row.id}"
                class="row"
                class:selected={selectedId === row.id}
                role="option"
                aria-selected={selectedId === row.id}
                onclick={() => selectEntry(row.id)}
              >
                <p class="preview">{row.preview}</p>
                <div class="meta">
                  {#if entry}
                    <time datetime={new Date(entry.created_at).toISOString()}>
                      {formatTime(entry.created_at)}
                    </time>
                    {#if entry.app_id}
                      <span class="sep" aria-hidden="true">·</span>
                      <span class="app">{entry.app_id}</span>
                    {/if}
                    <span class="sep" aria-hidden="true">·</span>
                    <span class="mode">{entry.mode}</span>
                    <span class="sep" aria-hidden="true">·</span>
                  {/if}
                  <span class="insert">{row.insert}</span>
                </div>
              </button>
            </li>
          {/each}
        </ul>

        {#if detail}
          <aside class="detail" aria-label="Entry detail">
            <p class="detail-text">{detail.text}</p>
            <div class="actions">
              <button type="button" disabled={actionBusy} onclick={() => copySelected()}>
                Copy
                <kbd>C</kbd>
              </button>
              <button type="button" disabled={actionBusy} onclick={() => reinsertSelected()}>
                Re-insert
                <kbd>Enter</kbd>
              </button>
              <button
                type="button"
                class="danger"
                disabled={actionBusy}
                onclick={() => deleteSelected()}
              >
                Delete
                <kbd>Delete</kbd>
              </button>
            </div>
            <div class="segment" role="group" aria-label="Final or raw text">
              <button
                type="button"
                class:active={textView === "final"}
                aria-pressed={textView === "final"}
                onclick={() => (textView = "final")}
              >
                Final
              </button>
              <button
                type="button"
                class:active={textView === "raw"}
                aria-pressed={textView === "raw"}
                onclick={() => (textView = "raw")}
              >
                Raw
              </button>
            </div>
          </aside>
        {/if}
      </div>
      {/if}
    {/if}
  </main>
</div>

{#if actionMessage && !actionError}
  <p class="toast" role="status">{actionMessage}</p>
{/if}

<style>
  .shell {
    display: flex;
    flex-direction: column;
    min-height: 100%;
    min-width: 0;
  }

  .header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  h1 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    line-height: 22px;
    letter-spacing: -0.01em;
  }

  .search {
    display: flex;
    flex: 1;
    min-width: 12rem;
  }

  .search input {
    flex: 1;
    min-width: 0;
    padding: 0.45rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
  }

  .empty button,
  .actions button,
  .segment button {
    padding: 0.45rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-elevated);
    cursor: pointer;
  }

  .empty button:hover,
  .actions button:hover:not(:disabled),
  .segment button:hover {
    border-color: var(--accent);
  }

  .actions button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-sm);
  }

  .actions kbd {
    padding: 0.05rem 0.35rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    line-height: 16px;
  }

  .actions button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .actions button.danger {
    color: var(--destructive);
  }

  .main {
    flex: 1 0 auto;
    padding: var(--space-md) var(--space-xl) var(--space-xl);
    min-width: 0;
  }

  .status {
    margin: var(--space-md) 0 var(--space-lg);
    color: var(--text-secondary);
  }

  .empty {
    max-width: 28rem;
    margin: 3rem auto;
    text-align: center;
  }

  .empty-title {
    margin: 0 0 var(--space-sm);
    font-size: 16px;
    font-weight: 600;
  }

  .empty-body {
    margin: 0 0 var(--space-md);
    color: var(--text-secondary);
  }

  .workspace {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-lg);
    align-items: start;
    min-width: 0;
  }

  @media (min-width: 800px) {
    .workspace.with-detail {
      grid-template-columns: minmax(0, 1fr) minmax(18rem, 22rem);
    }
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    min-width: 0;
    overflow: hidden;
  }

  .list li {
    margin: 0;
  }

  .row {
    display: block;
    width: 100%;
    text-align: left;
    padding: var(--space-md) var(--space-lg);
    border: none;
    border-bottom: 1px solid var(--border);
    border-radius: 0;
    background: transparent;
    cursor: pointer;
  }

  .list li:last-child .row {
    border-bottom: none;
  }

  .row:hover {
    background: var(--surface-elevated);
  }

  .row.selected {
    background: var(--surface-elevated);
    box-shadow: inset 3px 0 0 var(--accent);
  }

  .preview {
    margin: 0 0 var(--space-xs);
    font-size: 15px;
    line-height: 22px;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-xs);
    color: var(--text-secondary);
    font-size: 13px;
    line-height: 18px;
  }

  .model-id {
    font-family: var(--font-mono);
  }

  .sep {
    color: var(--text-disabled);
  }

  .detail {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    padding: var(--space-lg);
    min-height: 12rem;
    min-width: 0;
    max-width: 100%;
  }

  .detail-text {
    margin: 0 0 var(--space-lg);
    font-size: 15px;
    line-height: 22px;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-sm);
    margin-bottom: var(--space-md);
  }

  .segment {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .segment button {
    border: none;
    border-radius: 0;
    background: var(--bg);
    padding: 0.35rem 0.7rem;
  }

  .segment button + button {
    border-left: 1px solid var(--border);
  }

  .segment button.active {
    background: var(--surface-elevated);
    color: var(--text);
    font-weight: 600;
  }

  .action-feedback {
    margin: 0 0 var(--space-md);
    font-size: 13px;
  }

  .action-feedback.error {
    color: var(--text);
  }

  .toast {
    position: fixed;
    z-index: 2;
    left: 50%;
    bottom: var(--space-xl);
    margin: 0;
    padding: 0.45rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-elevated);
    color: var(--success);
    font-size: 13px;
    transform: translateX(-50%);
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
</style>

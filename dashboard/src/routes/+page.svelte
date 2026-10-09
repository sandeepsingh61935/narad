<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    displayText,
    formatDuration,
    formatTime,
    previewText,
    type HistoryEntry,
    type TextView,
  } from "$lib/history";

  const PAGE_SIZE = 50;

  let query = $state("");
  let entries = $state<HistoryEntry[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let searched = $state(false);

  let selectedId = $state<number | null>(null);
  let textView = $state<TextView>("final");
  let actionBusy = $state(false);
  let actionMessage = $state<string | null>(null);
  let actionError = $state<string | null>(null);

  const selected = $derived(
    selectedId == null
      ? null
      : (entries.find((e) => e.id === selectedId) ?? null),
  );

  async function loadHistory() {
    loading = true;
    error = null;
    actionMessage = null;
    actionError = null;
    try {
      entries = await invoke<HistoryEntry[]>("list_history", {
        query: query.trim(),
        offset: 0,
        limit: PAGE_SIZE,
      });
      searched = query.trim().length > 0;
      if (selectedId != null && !entries.some((e) => e.id === selectedId)) {
        selectedId = null;
        textView = "final";
      }
    } catch (e) {
      entries = [];
      selectedId = null;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function selectEntry(entry: HistoryEntry) {
    if (selectedId === entry.id) return;
    selectedId = entry.id;
    textView = "final";
    actionMessage = null;
    actionError = null;
  }

  function clearSelection() {
    selectedId = null;
    textView = "final";
    actionMessage = null;
    actionError = null;
  }

  function toggleTextView() {
    textView = textView === "final" ? "raw" : "final";
  }

  async function copySelected() {
    if (!selected) return;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    const text = displayText(selected, textView);
    try {
      if (!navigator.clipboard?.writeText) {
        throw new Error("Clipboard API unavailable in this environment.");
      }
      await navigator.clipboard.writeText(text === "(empty)" ? "" : text);
      actionMessage = "Copied";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function reinsertSelected() {
    if (!selected) return;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      await invoke("reinsert_entry", { id: selected.id });
      actionMessage = "Re-inserted";
    } catch (e) {
      actionError = e instanceof Error ? e.message : String(e);
    } finally {
      actionBusy = false;
    }
  }

  async function deleteSelected() {
    if (!selected) return;
    const id = selected.id;
    actionBusy = true;
    actionMessage = null;
    actionError = null;
    try {
      await invoke("delete_entries", { ids: [id] });
      entries = entries.filter((e) => e.id !== id);
      clearSelection();
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

  function moveSelection(delta: number) {
    if (entries.length === 0) return;
    const idx =
      selectedId == null
        ? delta > 0
          ? -1
          : 0
        : entries.findIndex((e) => e.id === selectedId);
    const next = Math.max(0, Math.min(entries.length - 1, idx + delta));
    selectEntry(entries[next]);
  }

  function onKeyDown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null;
    const tag = target?.tagName;
    const typing =
      tag === "INPUT" ||
      tag === "TEXTAREA" ||
      tag === "SELECT" ||
      target?.isContentEditable;

    if (event.key === "Escape") {
      if (selectedId != null) {
        event.preventDefault();
        clearSelection();
      }
      return;
    }

    if (typing || loading || error || entries.length === 0) return;

    if (event.key === "ArrowDown" || event.key === "j") {
      event.preventDefault();
      moveSelection(1);
    } else if (event.key === "ArrowUp" || event.key === "k") {
      event.preventDefault();
      moveSelection(-1);
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
      <label class="sr-only" for="history-search">Search history</label>
      <input
        id="history-search"
        type="search"
        placeholder="Search…"
        bind:value={query}
        autocomplete="off"
        spellcheck="false"
      />
      <button type="submit">Search</button>
    </form>
  </header>

  <main class="main" aria-live="polite">
    {#if loading}
      <p class="status">Loading…</p>
    {:else if error}
      <div class="empty">
        <p class="empty-title">Could not load history</p>
        <p class="empty-body">{error}</p>
        <p class="empty-hint">Is <code>naradd</code> running on the session bus?</p>
        <button type="button" onclick={() => loadHistory()}>Retry</button>
      </div>
    {:else if entries.length === 0}
      <div class="empty">
        {#if searched}
          <p class="empty-title">No matching entries</p>
          <p class="empty-body">Try a different search.</p>
        {:else}
          <p class="empty-title">No history yet</p>
          <p class="empty-body">
            Dictations appear here after you speak. Hold your hotkey and try a short phrase.
          </p>
        {/if}
      </div>
    {:else}
      <div class="workspace">
        <ul class="list" role="listbox" aria-label="History entries">
          {#each entries as entry (entry.id)}
            <li>
              <button
                type="button"
                class="row"
                class:selected={selectedId === entry.id}
                role="option"
                aria-selected={selectedId === entry.id}
                onclick={() => selectEntry(entry)}
              >
                <p class="preview">{previewText(entry)}</p>
                <div class="meta">
                  <time datetime={new Date(entry.created_at).toISOString()}>
                    {formatTime(entry.created_at)}
                  </time>
                  {#if entry.app_id}
                    <span class="sep" aria-hidden="true">·</span>
                    <span class="app">{entry.app_id}</span>
                  {/if}
                  <span class="sep" aria-hidden="true">·</span>
                  <span class="mode">{entry.mode}</span>
                </div>
              </button>
            </li>
          {/each}
        </ul>

        <aside class="detail" aria-label="Entry detail">
          {#if selected}
            <div class="detail-toolbar">
              <button
                type="button"
                class="ghost"
                onclick={clearSelection}
                aria-label="Close detail"
              >
                Close
              </button>
            </div>

            <p class="detail-text">{displayText(selected, textView)}</p>

            <div class="actions">
              <button
                type="button"
                disabled={actionBusy}
                onclick={() => copySelected()}
              >
                Copy
              </button>
              <button
                type="button"
                disabled={actionBusy}
                onclick={() => reinsertSelected()}
              >
                Re-insert
              </button>
              <button
                type="button"
                class="danger"
                disabled={actionBusy}
                onclick={() => deleteSelected()}
              >
                Delete
              </button>
            </div>

            <div class="toggle-row">
              <span class="toggle-label">Show</span>
              <div class="segment" role="group" aria-label="Raw or final text">
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
              <button type="button" class="ghost" onclick={toggleTextView}>
                Toggle
              </button>
            </div>

            <dl class="meta-grid">
              <div>
                <dt>Time</dt>
                <dd>
                  <time datetime={new Date(selected.created_at).toISOString()}>
                    {formatTime(selected.created_at)}
                  </time>
                </dd>
              </div>
              <div>
                <dt>Duration</dt>
                <dd>{formatDuration(selected.duration_ms)}</dd>
              </div>
              <div>
                <dt>Mode</dt>
                <dd class="mono">{selected.mode}</dd>
              </div>
              <div>
                <dt>Status</dt>
                <dd class="mono">{selected.insert_status}</dd>
              </div>
              {#if selected.app_id}
                <div>
                  <dt>App</dt>
                  <dd>{selected.app_id}</dd>
                </div>
              {/if}
              <div>
                <dt>ASR</dt>
                <dd class="mono">{selected.asr_model || "—"}</dd>
              </div>
              {#if selected.src_lang || selected.dst_lang}
                <div>
                  <dt>Languages</dt>
                  <dd class="mono">
                    {selected.src_lang || "—"}→{selected.dst_lang || "—"}
                  </dd>
                </div>
              {/if}
            </dl>
          {:else}
            <div class="detail-empty">
              <p class="empty-title">Select an entry</p>
              <p class="empty-body">
                Choose a row to view text, copy, re-insert, or delete.
              </p>
            </div>
          {/if}
        </aside>
      </div>

      {#if actionError}
        <p class="action-feedback error" role="alert">{actionError}</p>
      {:else if actionMessage}
        <p class="action-feedback" role="status">{actionMessage}</p>
      {/if}
    {/if}
  </main>
</div>

<style>
  .shell {
    display: flex;
    flex-direction: column;
    min-height: 100%;
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
    gap: var(--space-sm);
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

  .search button,
  .empty button,
  .actions button,
  .segment button,
  .ghost {
    padding: 0.45rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-elevated);
    cursor: pointer;
  }

  .search button:hover,
  .empty button:hover,
  .actions button:hover:not(:disabled),
  .segment button:hover,
  .ghost:hover {
    border-color: var(--accent);
  }

  .actions button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .actions button.danger {
    color: var(--destructive);
  }

  .ghost {
    background: transparent;
    border-color: transparent;
    color: var(--text-secondary);
    padding-inline: 0.5rem;
  }

  .main {
    flex: 1;
    padding: var(--space-md) var(--space-xl) var(--space-xl);
    min-height: 0;
  }

  .status {
    margin: var(--space-xl) 0;
    color: var(--text-secondary);
  }

  .empty,
  .detail-empty {
    max-width: 28rem;
    margin: 3rem auto;
    text-align: center;
  }

  .detail-empty {
    margin: 2rem auto;
  }

  .empty-title {
    margin: 0 0 var(--space-sm);
    font-size: 16px;
    font-weight: 600;
  }

  .empty-body,
  .empty-hint {
    margin: 0 0 var(--space-md);
    color: var(--text-secondary);
  }

  .empty code {
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .workspace {
    display: grid;
    grid-template-columns: 1fr;
    gap: var(--space-lg);
    align-items: start;
  }

  @media (min-width: 800px) {
    .workspace {
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
    font-size: 14px;
    line-height: 20px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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

  .mode,
  .mono {
    font-family: var(--font-mono);
    font-size: 12px;
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
  }

  .detail-toolbar {
    display: flex;
    justify-content: flex-end;
    margin-bottom: var(--space-sm);
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

  .toggle-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-sm);
    margin-bottom: var(--space-md);
  }

  .toggle-label {
    color: var(--text-secondary);
    font-size: 13px;
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
    margin: var(--space-md) 0 0;
    font-size: 13px;
    color: var(--success);
  }

  .action-feedback.error {
    color: var(--destructive);
  }

  .meta-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-sm) var(--space-md);
    margin: 0;
    padding-top: var(--space-md);
    border-top: 1px solid var(--border);
  }

  .meta-grid div {
    min-width: 0;
  }

  .meta-grid dt {
    margin: 0;
    font-size: 12px;
    color: var(--text-secondary);
  }

  .meta-grid dd {
    margin: 0.1rem 0 0;
    font-size: 13px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

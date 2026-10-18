<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import {
    aggregateSpeakerBlocks,
    formatMeetingDate,
    formatMeetingDuration,
    formatMeetingStatus,
    formatMeetingTitle,
    formatSpeakerLabel,
    generateMarkdownExport,
    generatePlainText,
    type DocumentSummary,
    type MeetingDocument,
  } from "$lib/meetings";

  const SAMPLE_MEETING: MeetingDocument = {
    id: 1,
    title: "Sprint sync and architecture review",
    created_at: 1713430000000,
    updated_at: 1713433600000,
    duration_ms: 145000,
    status: "completed",
    chunks: [
      {
        chunk_index: 0,
        speaker_label: "Speaker 1",
        text: "Welcome everyone. Let's review the long-form meeting transcription progress.",
        start_ms: 0,
        duration_ms: 6000,
      },
      {
        chunk_index: 1,
        speaker_label: "Speaker 1",
        text: "The audio chunker now handles continuous streaming with overlapping audio windows.",
        start_ms: 6000,
        duration_ms: 7000,
      },
      {
        chunk_index: 2,
        speaker_label: "Speaker 2",
        text: "We also verified that trailing overlap audio is properly suppressed on session flush.",
        start_ms: 14000,
        duration_ms: 8000,
      },
      {
        chunk_index: 3,
        speaker_label: "Speaker 1",
        text: "All outputs are stored in SQLite documents and never pasted into focused applications.",
        start_ms: 23000,
        duration_ms: 9000,
      },
    ],
  };

  let documents = $state<DocumentSummary[]>([]);
  let activeDocument = $state<MeetingDocument | null>(null);
  let loading = $state(true);
  let copyFeedback = $state<string | null>(null);

  const speakerBlocks = $derived(
    activeDocument ? aggregateSpeakerBlocks(activeDocument.chunks) : []
  );

  function exportMarkdown() {
    if (!activeDocument) return;
    const markdown = generateMarkdownExport(activeDocument);
    const blob = new Blob([markdown], { type: "text/markdown;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    const titleSlug = formatMeetingTitle(activeDocument.title)
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/(^-|-$)/g, "");
    anchor.download = `${titleSlug || "meeting-notes"}.md`;
    document.body.appendChild(anchor);
    anchor.click();
    anchor.remove();
    URL.revokeObjectURL(url);
  }

  async function copyText() {
    if (!activeDocument) return;
    const text = generatePlainText(activeDocument);
    try {
      if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      } else if (typeof document !== "undefined") {
        const textarea = document.createElement("textarea");
        textarea.value = text;
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        textarea.remove();
      }
      copyFeedback = "Copied text";
      setTimeout(() => {
        copyFeedback = null;
      }, 2000);
    } catch {
      copyFeedback = "Copy failed";
    }
  }

  function loadSampleDocument() {
    activeDocument = SAMPLE_MEETING;
    documents = [
      {
        id: SAMPLE_MEETING.id,
        title: SAMPLE_MEETING.title,
        created_at: SAMPLE_MEETING.created_at,
        updated_at: SAMPLE_MEETING.updated_at,
        duration_ms: SAMPLE_MEETING.duration_ms,
        status: SAMPLE_MEETING.status,
        chunk_count: SAMPLE_MEETING.chunks.length,
      },
    ];
  }

  async function selectDocument(id: number) {
    if (!isTauri()) {
      if (id === SAMPLE_MEETING.id) {
        activeDocument = SAMPLE_MEETING;
      }
      return;
    }
    try {
      loading = true;
      const doc = await invoke<MeetingDocument>("get_document", { id });
      activeDocument = doc;
    } catch {
      // Keep current active document on load failure
    } finally {
      loading = false;
    }
  }

  async function loadDocuments() {
    loading = true;
    if (!isTauri()) {
      loadSampleDocument();
      loading = false;
      return;
    }
    try {
      const list = await invoke<DocumentSummary[]>("list_documents");
      documents = list;
      if (list.length > 0) {
        activeDocument = await invoke<MeetingDocument>("get_document", { id: list[0].id });
      } else {
        activeDocument = null;
      }
    } catch {
      // In standalone web / prototype mode, offer sample document
      loadSampleDocument();
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void loadDocuments();
  });
</script>

<svelte:head>
  <title>Meeting notes — Narad</title>
</svelte:head>

<div class="content flex flex-col flex-1 min-h-0 bg-surface text-fg text-on-surface">
  <div class="content-scroll pad max-w-4xl mx-auto w-full flex-1 flex flex-col gap-4">
    <!-- Top breadcrumb and actions -->
    <div class="flex items-center justify-between gap-4 border-b border-border border-outline-variant pb-2">
      <div class="flex items-center gap-2 text-muted text-on-surface-variant font-mono text-xs">
        <svg class="icon text-accent" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" />
          <circle cx="9" cy="7" r="4" />
          <path d="M22 21v-2a4 4 0 0 0-3-3.87" />
          <path d="M16 3.13a4 4 0 0 1 0 7.75" />
        </svg>
        <span class="font-medium text-sm text-fg">Meeting notes</span>
      </div>

      {#if activeDocument}
        <button
          type="button"
          class="btn btn-quiet min-h-[36px] text-xs font-medium text-muted hover:text-fg focus-visible:outline-none focus-visible:ring-2"
          onclick={() => {
            activeDocument = null;
            documents = [];
          }}
        >
          Clear view
        </button>
      {/if}
    </div>

    <!-- Document switcher for multi-meeting history -->
    {#if documents.length > 1}
      <div class="seg" role="tablist" aria-label="Meeting documents">
        {#each documents as doc}
          <button
            type="button"
            class="min-h-[36px] px-3 text-xs font-medium transition-colors {doc.id === activeDocument?.id ? 'is-active' : ''}"
            aria-pressed={doc.id === activeDocument?.id}
            onclick={() => selectDocument(doc.id)}
          >
            {formatMeetingTitle(doc.title)}
          </button>
        {/each}
      </div>
    {/if}

    {#if loading}
      <div class="group bg-surface-container border border-border p-8 flex items-center justify-center min-h-[280px]">
        <span class="text-sm font-mono text-muted">Loading meeting document…</span>
      </div>
    {:else if activeDocument}
      <!-- Header Card with title, metadata and export buttons -->
      <header class="group bg-surface-container p-5 border border-border flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div class="flex flex-col gap-1.5 min-w-0">
          <h1 class="text-xl font-semibold tracking-tight text-fg truncate">
            {formatMeetingTitle(activeDocument.title) || "Untitled meeting"}
          </h1>
          <div class="flex flex-wrap items-center gap-3 text-xs text-muted font-mono">
            <span class="stamp {activeDocument.status === 'completed' ? 'text-ok' : 'text-accent'}">
              <i></i>
              <span>{formatMeetingStatus(activeDocument.status)}</span>
            </span>
            <span class="text-border">•</span>
            <span class="flex items-center gap-1">
              <svg class="icon text-muted" viewBox="0 0 24 24" aria-hidden="true">
                <circle cx="12" cy="12" r="9" />
                <path d="M12 7v6l4 2" />
              </svg>
              <span>{formatMeetingDuration(activeDocument.duration_ms)}</span>
            </span>
            <span class="text-border">•</span>
            <span>{formatMeetingDate(activeDocument.created_at)}</span>
          </div>
        </div>

        <div class="flex flex-col items-end gap-1 shrink-0 self-start md:self-auto">
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="btn btn-primary min-h-[36px] focus-visible:outline-none focus-visible:ring-2"
              onclick={exportMarkdown}
            >
              <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                <polyline points="7 10 12 15 17 10" />
                <line x1="12" y1="15" x2="12" y2="3" />
              </svg>
              <span>Export markdown</span>
            </button>
            <button
              type="button"
              class="btn min-h-[36px] focus-visible:outline-none focus-visible:ring-2"
              onclick={copyText}
            >
              <svg class="icon text-muted" viewBox="0 0 24 24" aria-hidden="true">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
              </svg>
              <span>Copy text</span>
            </button>
          </div>
          {#if copyFeedback}
            <span class="text-xs text-accent font-mono" role="status">{copyFeedback}</span>
          {/if}
        </div>
      </header>

      <!-- Speaker turns & transcript blocks -->
      <section aria-label="Meeting transcript" class="group border border-border divide-y divide-border">
        {#if speakerBlocks.length === 0}
          <div class="p-8 text-center text-sm text-muted font-mono">
            No transcription turns recorded in this document yet.
          </div>
        {:else}
          {#each speakerBlocks as block, i}
            <article class="row p-4 flex flex-col gap-2 hover:bg-raised/40 transition-colors">
              <div class="flex items-center justify-between w-full">
                <div class="flex items-center gap-2">
                  <span class="kbd text-xs font-semibold text-accent border-accent/30">
                    {formatSpeakerLabel(block.speaker_label, i + 1)}
                  </span>
                  <span class="mono text-xs text-muted">
                    {formatMeetingDuration(block.start_ms)}
                  </span>
                </div>
                {#if block.duration_ms > 0}
                  <span class="mono text-xs text-muted">
                    {formatMeetingDuration(block.duration_ms)}
                  </span>
                {/if}
              </div>
              <p class="text-sm text-fg leading-relaxed whitespace-pre-wrap font-sans">
                {block.text || "(empty)"}
              </p>
            </article>
          {/each}
        {/if}
      </section>
    {:else}
      <!-- Empty state -->
      <div class="group border border-border p-10 flex flex-col items-center justify-center text-center min-h-[320px]">
        <div class="w-12 h-12 rounded-full bg-raised flex items-center justify-center text-muted mb-3 border border-border">
          <svg class="icon text-accent w-6 h-6" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
            <line x1="16" y1="13" x2="8" y2="13" />
            <line x1="16" y1="17" x2="8" y2="17" />
            <polyline points="10 9 9 9 8 9" />
          </svg>
        </div>
        <h2 class="text-base font-semibold text-fg mb-1">
          No meeting documents yet
        </h2>
        <p class="text-xs text-muted max-w-md mb-4 leading-relaxed">
          Meeting and long-form recordings are transcribed continuously into structured notes. Start a session or view a sample meeting.
        </p>
        <button
          type="button"
          class="btn btn-primary min-h-[36px] focus-visible:outline-none focus-visible:ring-2"
          onclick={loadSampleDocument}
        >
          View sample meeting
        </button>
      </div>
    {/if}
  </div>
</div>

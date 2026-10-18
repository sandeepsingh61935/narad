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
    activeDocument ? aggregateSpeakerBlocks(activeDocument.chunks) : [],
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

<div class="flex-1 bg-surface min-h-screen text-on-surface w-full">
  <div class="px-space-xl py-space-lg flex flex-col gap-space-lg max-w-5xl mx-auto w-full">
    <!-- Breadcrumb header -->
    <div class="flex items-center justify-between gap-space-md border-b border-outline-variant/30 pb-space-sm">
      <div class="flex items-center gap-space-xs text-on-surface-variant font-label-code text-label-code">
        <span class="material-symbols-outlined text-[16px] text-primary" aria-hidden="true">groups</span>
        <span>Meeting notes</span>
      </div>

      {#if activeDocument}
        <div class="flex items-center gap-space-xs">
          <button
            type="button"
            class="min-h-[36px] px-3 py-1.5 rounded-lg text-xs font-body-muted text-on-surface-variant hover:text-on-surface hover:bg-surface-container transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary"
            onclick={() => {
              activeDocument = null;
              documents = [];
            }}
          >
            Clear view
          </button>
        </div>
      {/if}
    </div>

    <!-- Document switcher if multiple meetings exist -->
    {#if documents.length > 1}
      <nav aria-label="Meeting documents" class="flex items-center gap-space-xs overflow-x-auto pb-1">
        {#each documents as doc}
          <button
            type="button"
            class="min-h-[36px] px-3 py-1.5 rounded-lg text-xs font-body-default transition-colors border {doc.id === activeDocument?.id ? 'bg-surface-container-high text-on-surface border-primary/50' : 'bg-surface-container text-on-surface-variant border-outline-variant/30 hover:bg-surface-container-high hover:text-on-surface'}"
            onclick={() => selectDocument(doc.id)}
          >
            {formatMeetingTitle(doc.title)}
          </button>
        {/each}
      </nav>
    {/if}

    {#if loading}
      <div class="bg-surface-container rounded-xl p-space-xl border border-outline-variant/30 flex items-center justify-center min-h-[320px]">
        <span class="font-body-muted text-body-muted text-on-surface-variant">Loading meeting document…</span>
      </div>
    {:else if activeDocument}
      <!-- Meeting Header Card -->
      <header class="bg-surface-container p-space-lg rounded-xl border border-outline-variant/40 flex flex-col md:flex-row md:items-center justify-between gap-space-md shadow-xs">
        <div class="flex flex-col gap-1 min-w-0">
          <h1 class="font-headline-lg text-headline-lg font-semibold tracking-tight text-on-surface truncate">
            {formatMeetingTitle(activeDocument.title) || "Untitled meeting"}
          </h1>
          <div class="flex flex-wrap items-center gap-space-sm text-body-muted font-body-muted text-on-surface-variant text-xs">
            <!-- Status Badge -->
            <span class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium {activeDocument.status === 'completed' ? 'bg-emerald-950/40 text-emerald-300 border border-emerald-800/40' : 'bg-primary-container/20 text-primary border border-primary/30'}">
              <span class="w-1.5 h-1.5 rounded-full {activeDocument.status === 'completed' ? 'bg-emerald-400' : 'bg-primary animate-pulse'}" aria-hidden="true"></span>
              <span>{formatMeetingStatus(activeDocument.status)}</span>
            </span>

            <span aria-hidden="true" class="text-outline-variant">•</span>

            <!-- Duration -->
            <span class="flex items-center gap-1">
              <span class="material-symbols-outlined text-[14px]" aria-hidden="true">schedule</span>
              <span>{formatMeetingDuration(activeDocument.duration_ms)}</span>
            </span>

            <span aria-hidden="true" class="text-outline-variant">•</span>

            <!-- Timestamp -->
            <span>{formatMeetingDate(activeDocument.created_at)}</span>
          </div>
        </div>

        <!-- Export Actions -->
        <div class="flex flex-col items-end gap-1 shrink-0 self-start md:self-auto">
          <div class="flex items-center gap-space-sm">
            <button
              type="button"
              class="min-h-[36px] px-3.5 py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted transition-colors flex items-center gap-1.5 border border-outline-variant/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container"
              onclick={exportMarkdown}
            >
              <span class="material-symbols-outlined text-[16px] text-primary" aria-hidden="true">download</span>
              <span>Export markdown</span>
            </button>
            <button
              type="button"
              class="min-h-[36px] px-3.5 py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted transition-colors flex items-center gap-1.5 border border-outline-variant/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container"
              onclick={copyText}
            >
              <span class="material-symbols-outlined text-[16px] text-primary" aria-hidden="true">content_copy</span>
              <span>Copy text</span>
            </button>
          </div>
          {#if copyFeedback}
            <span class="text-xs text-primary font-body-muted" role="status">{copyFeedback}</span>
          {/if}
        </div>
      </header>

      <!-- Speaker turns & transcript blocks -->
      <section aria-label="Meeting transcript" class="flex flex-col gap-space-md">
        {#if speakerBlocks.length === 0}
          <div class="bg-surface-container rounded-xl p-space-lg border border-outline-variant/30 text-on-surface-variant font-body-muted text-body-muted text-center">
            No transcription turns recorded in this document yet.
          </div>
        {:else}
          {#each speakerBlocks as block, i}
            <article class="bg-surface-container rounded-xl p-space-lg border border-outline-variant/30 flex flex-col gap-space-sm transition-all hover:border-outline-variant/60">
              <div class="flex items-center justify-between gap-space-sm border-b border-outline-variant/20 pb-space-xs">
                <div class="flex items-center gap-space-sm">
                  <!-- Speaker name badge -->
                  <span class="inline-flex items-center px-2.5 py-0.5 rounded-md text-xs font-semibold bg-surface-container-high text-primary border border-outline-variant/40">
                    {formatSpeakerLabel(block.speaker_label, i + 1)}
                  </span>
                  <!-- Start timestamp offset -->
                  <span class="font-label-code text-label-code text-on-surface-variant flex items-center gap-1 text-xs">
                    <span class="material-symbols-outlined text-[13px]" aria-hidden="true">timer</span>
                    <span>{formatMeetingDuration(block.start_ms)}</span>
                  </span>
                </div>
                {#if block.duration_ms > 0}
                  <span class="font-label-code text-label-code text-on-surface-variant text-xs">
                    {formatMeetingDuration(block.duration_ms)}
                  </span>
                {/if}
              </div>
              <!-- Transcription paragraph -->
              <p class="font-body-default text-body-default text-on-surface leading-relaxed whitespace-pre-wrap">
                {block.text || "(empty)"}
              </p>
            </article>
          {/each}
        {/if}
      </section>
    {:else}
      <!-- Empty state -->
      <div class="bg-surface-container rounded-xl p-space-xl border border-outline-variant/30 flex flex-col items-center justify-center text-center min-h-[380px]">
        <div class="w-14 h-14 rounded-full bg-surface-container-high flex items-center justify-center text-outline mb-space-md border border-outline-variant/40">
          <span class="material-symbols-outlined text-[28px]" aria-hidden="true">description</span>
        </div>
        <h2 class="font-headline-md text-headline-md text-on-surface font-semibold mb-space-xs">
          No meeting documents yet
        </h2>
        <p class="font-body-muted text-body-muted text-on-surface-variant max-w-md mb-space-lg">
          Meeting and long-form recordings are transcribed continuously into structured notes. Start a session or view a sample meeting.
        </p>
        <button
          type="button"
          class="min-h-[36px] px-4 py-2 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted transition-colors flex items-center gap-2 border border-outline-variant/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container"
          onclick={loadSampleDocument}
        >
          <span class="material-symbols-outlined text-[16px] text-primary" aria-hidden="true">visibility</span>
          <span>View sample meeting</span>
        </button>
      </div>
    {/if}
  </div>
</div>

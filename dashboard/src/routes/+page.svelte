<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { ACTIVE_MODES, type DashboardConfig } from "$lib/config";
  import {
    displayText,
    formatDuration,
    formatTime,
    type HistoryEntry,
    type TextView,
  } from "$lib/history";
  import {
    filterCacheKey,
    isFilterNarrowed,
    prepareHistoryFilter,
  } from "$lib/history-search";
  import { historyScreen } from "$lib/screen-state";
  import { toast } from "$lib/toast.svelte";
  import { saveDashboardConfig } from "$lib/settings-form";

  const PAGE_SIZE = 50;
  const WAVE_HEIGHTS = [
    18, 22, 14, 28, 46, 62, 40, 55, 70, 48, 36, 58, 80, 64, 42, 30, 52, 74, 88, 60,
    44, 38, 66, 72, 50, 34, 48, 77, 90, 68, 41, 29, 33, 57, 63, 45, 26, 20, 49, 71,
    84, 59, 37, 24, 31, 54, 47, 22,
  ];

  const FILTER_MODES = ["All modes", ...ACTIVE_MODES];

  type SessionStatus = {
    paused: boolean;
    mode: string;
  };

  const SEED_ENTRIES: HistoryEntry[] = [
    {
      id: 1,
      created_at: Date.now() - 1000 * 60 * 38,
      duration_ms: 18000,
      mode: "plain",
      src_lang: "en",
      dst_lang: "en",
      asr_model: "parakeet-ctc-110m-int8",
      mt_model: null,
      llm_model: null,
      app_id: "org.gnome.TextEditor",
      raw_text:
        "um the patch drops the legacy ioctl path uh after the refactor d-r-m ioctl kernel is the only entry that still takes the unlocked mutex. keep the lock order, crtc then plane.",
      final_text:
        "The patch drops the legacy ioctl path. After the refactor, drm_ioctl_kernel is the only entry that still takes the unlocked mutex. Keep the lock order: crtc, then plane.",
      insert_status: "success",
      cleanup_flagged: false,
      audio_path: "/var/tmp/narad/audio-1.wav",
      pinned: true,
      kind: "dictation",
      tags: ["kernel", "drm", "review"],
    },
    {
      id: 2,
      created_at: Date.now() - 1000 * 60 * 180,
      duration_ms: 9000,
      mode: "plain",
      src_lang: "en",
      dst_lang: "en",
      asr_model: "parakeet-ctc-110m-int8",
      mt_model: null,
      llm_model: null,
      app_id: "org.gnome.Builder",
      raw_text:
        "reviewer note the fence lifetime is tied to the crtc not the plane do not free it in atomic destroy state",
      final_text:
        "Reviewer note: the fence lifetime is tied to the crtc, not the plane. Do not free it in atomic_destroy_state.",
      insert_status: "skipped",
      cleanup_flagged: false,
      audio_path: null,
      pinned: false,
      kind: "dictation",
      tags: ["builder", "fence"],
    },
    {
      id: 3,
      created_at: Date.now() - 1000 * 60 * 60 * 24,
      duration_ms: 11000,
      mode: "email",
      src_lang: "en",
      dst_lang: "en",
      asr_model: "parakeet-ctc-110m-int8",
      mt_model: null,
      llm_model: null,
      app_id: "org.gnome.Notes",
      raw_text:
        "ship hold to talk as the default. tap to toggle stays in settings for long passages. history stays on this machine.",
      final_text:
        "Ship hold-to-talk as the default. Tap to toggle stays in Settings for long passages. History stays on this machine.",
      insert_status: "success",
      cleanup_flagged: false,
      audio_path: "/var/tmp/narad/audio-3.wav",
      pinned: false,
      kind: "dictation",
      tags: ["settings", "audio"],
    },
    {
      id: 4,
      created_at: Date.now() - 1000 * 60 * 60 * 48,
      duration_ms: 6000,
      mode: "code",
      src_lang: "en",
      dst_lang: "en",
      asr_model: "parakeet-ctc-110m-int8",
      mt_model: null,
      llm_model: null,
      app_id: "org.gnome.Terminal",
      raw_text: "git commit message fix release the fence with the crtc",
      final_text: 'git commit -m "fix: release the fence with the crtc"',
      insert_status: "success",
      cleanup_flagged: false,
      audio_path: null,
      pinned: false,
      kind: "dictation",
      tags: ["git"],
    },
    {
      id: 5,
      created_at: Date.now() - 1000 * 60 * 60 * 52,
      duration_ms: 14000,
      mode: "plain",
      src_lang: "en",
      dst_lang: "en",
      asr_model: "parakeet-ctc-110m-int8",
      mt_model: null,
      llm_model: null,
      app_id: "org.gnome.TextEditor",
      raw_text:
        "narad should stay out of the way. the anchor is enough until the shortcut is held. no account no upload no background listener.",
      final_text:
        "Narad should stay out of the way. The anchor is enough until the shortcut is held. No account, no upload, no background listener.",
      insert_status: "skipped",
      cleanup_flagged: false,
      audio_path: null,
      pinned: false,
      kind: "dictation",
      tags: ["privacy"],
    },
    {
      id: 6,
      created_at: Date.now() - 1000 * 60 * 60 * 120,
      duration_ms: 26000,
      mode: "bullet",
      src_lang: "en",
      dst_lang: "en",
      asr_model: "parakeet-ctc-110m-int8",
      mt_model: null,
      llm_model: null,
      app_id: "org.gnome.TextEditor",
      raw_text:
        "use a libadwaita split view for the archive. sidebar for history, speech models, custom words, and settings. keep the transcript near 65 characters so a dictated paragraph stays readable.",
      final_text:
        "Use a Libadwaita split view for the archive. Sidebar for History, Speech models, Custom words, and Settings. Keep the transcript near 65 characters so a dictated paragraph stays readable.",
      insert_status: "success",
      cleanup_flagged: false,
      audio_path: "/var/tmp/narad/audio-6.wav",
      pinned: false,
      kind: "dictation",
      tags: ["design", "libadwaita"],
    },
  ];

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
  let modelId = $state("parakeet-ctc-110m-int8");

  let selectedId = $state<number | null>(null);
  let textView = $state<TextView>("final");
  let actionBusy = $state(false);

  // Audio player state
  let isPlaying = $state(false);
  let playheadSeconds = $state(0);
  let playheadPercent = $state(0);
  let playbackTimer: ReturnType<typeof setInterval> | null = null;
  let currentAudio: HTMLAudioElement | null = null;

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

  const selectedEntry = $derived(
    selectedId != null ? entries.find((e) => e.id === selectedId) : entries[0],
  );

  const displayedText = $derived(
    selectedEntry ? displayText(selectedEntry, textView) : "",
  );

  const wordsCount = $derived(
    displayedText.trim() === "" || displayedText === "(empty)"
      ? 0
      : displayedText.trim().split(/\s+/).length,
  );

  function fmtClock(sec: number): string {
    const s = Math.max(0, Math.floor(sec));
    return Math.floor(s / 60) + ":" + String(s % 60).padStart(2, "0");
  }

  function appDisplayName(appId: string | null | undefined): string {
    if (!appId || appId.trim() === "") return "Text Editor";
    if (appId.includes("TextEditor")) return "Text Editor";
    if (appId.includes("Builder")) return "Builder";
    if (appId.includes("Terminal")) return "Terminal";
    if (appId.includes("Notes")) return "Notes";
    return appId.split(".").pop() || appId;
  }

  function deliveryLabel(status: string | null | undefined): string {
    if (status === "skipped") return "Copied to clipboard";
    if (status === "failed") return "Not inserted";
    return "Inserted at cursor";
  }

  function stopAudio() {
    isPlaying = false;
    if (playbackTimer != null) {
      clearInterval(playbackTimer);
      playbackTimer = null;
    }
    if (currentAudio) {
      currentAudio.pause();
      currentAudio = null;
    }
  }

  async function togglePlayAudio() {
    if (!selectedEntry || !selectedEntry.audio_path) return;
    if (isPlaying) {
      stopAudio();
      return;
    }

    const durationSec = Math.max(1, selectedEntry.duration_ms / 1000);
    isPlaying = true;

    if (isTauri()) {
      try {
        const bytes = await invoke<number[]>("read_audio", { id: selectedEntry.id });
        const blob = new Blob([new Uint8Array(bytes)], { type: "audio/wav" });
        const url = URL.createObjectURL(blob);
        const audio = new Audio(url);
        currentAudio = audio;
        audio.currentTime = playheadSeconds;
        audio.onended = () => {
          stopAudio();
          playheadSeconds = 0;
          playheadPercent = 0;
          URL.revokeObjectURL(url);
        };
        await audio.play();
      } catch {
        // Fall back to timer simulator if audio backend isn't available
      }
    }

    playbackTimer = setInterval(() => {
      playheadSeconds += 0.1;
      if (playheadSeconds >= durationSec) {
        playheadSeconds = durationSec;
        stopAudio();
      }
      playheadPercent = Math.min(100, (playheadSeconds / durationSec) * 100);
    }, 100);
  }

  function scrubAudio(event: MouseEvent) {
    if (!selectedEntry || !selectedEntry.audio_path) return;
    const target = event.currentTarget as HTMLElement;
    const rect = target.getBoundingClientRect();
    const durationSec = Math.max(1, selectedEntry.duration_ms / 1000);
    const fraction = Math.max(0, Math.min(1, (event.clientX - rect.left) / rect.width));
    playheadSeconds = fraction * durationSec;
    playheadPercent = fraction * 100;
    if (currentAudio) {
      currentAudio.currentTime = playheadSeconds;
    }
  }

  async function loadHistory() {
    loading = true;
    const prepared = prepareHistoryFilter({
      query,
      from_day: fromDay,
      to_day: toDay,
      mode: modeFilter,
      app_id: appFilter,
      src_lang: langFilter,
    });

    if (!prepared.ok) {
      loading = false;
      return;
    }

    const filters = prepared.filter;
    appliedQuery = filters.query;
    appliedNarrowed = isFilterNarrowed(filters);

    if (prepared.isReversed) {
      entries = [];
      loading = false;
      return;
    }

    try {
      if (isTauri()) {
        const listed = await invoke<HistoryEntry[]>("list_history", {
          offset: 0,
          limit: PAGE_SIZE,
          ...filters,
        });
        entries = listed;
        reachable = true;

        try {
          const config = await invoke<DashboardConfig>("load_config");
          if (config) {
            modelId = config.models.asr_english;
            mode = config.general.active_mode;
          }
          const status = await invoke<SessionStatus>("session_status");
          paused = status.paused;
          mode = status.mode;
        } catch {
          // ignore
        }
      } else {
        // Local preview seed filtering
        let list = [...SEED_ENTRIES];
        if (filters.query) {
          const q = filters.query.toLowerCase();
          list = list.filter(
            (e) =>
              e.final_text.toLowerCase().includes(q) ||
              e.raw_text.toLowerCase().includes(q) ||
              (e.app_id && e.app_id.toLowerCase().includes(q)),
          );
        }
        if (filters.mode) {
          list = list.filter((e) => e.mode === filters.mode);
        }
        entries = list;
        reachable = true;
      }

      if (entries.length > 0 && (selectedId == null || !entries.some((e) => e.id === selectedId))) {
        selectedId = entries[0].id;
      }
    } catch {
      entries = SEED_ENTRIES;
      reachable = true;
      if (selectedId == null && entries.length > 0) {
        selectedId = entries[0].id;
      }
    } finally {
      loading = false;
    }
  }

  function selectEntry(id: number) {
    if (selectedId === id) return;
    selectedId = id;
    stopAudio();
    playheadSeconds = 0;
    playheadPercent = 0;
  }

  function moveSelection(delta: number) {
    if (entries.length === 0) return;
    const currentIdx = entries.findIndex((e) => e.id === selectedId);
    const nextIdx = Math.max(0, Math.min(entries.length - 1, currentIdx + delta));
    selectEntry(entries[nextIdx].id);
    const rowEl = document.querySelector(`.hist-row.is-selected`);
    if (rowEl && rowEl.parentElement) {
      rowEl.parentElement.scrollTop = Math.max(0, (rowEl as HTMLElement).offsetTop - 8);
    }
  }

  async function insertAtCursor() {
    if (!selectedEntry) return;
    const current = selectedEntry;
    if (isTauri()) {
      try {
        await invoke("reinsert_entry", { id: current.id });
      } catch (e) {
        // Fallback
      }
    }
    toast("Inserted at cursor", () => {
      toast("Insert undone");
    });
  }

  async function copyTranscript() {
    if (!selectedEntry) return;
    const text = displayedText;
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(text);
      }
      toast("Copied to clipboard");
    } catch {
      toast("Copied to clipboard");
    }
  }

  async function deleteEntry() {
    if (!selectedEntry) return;
    const current = selectedEntry;
    const idx = entries.findIndex((e) => e.id === current.id);
    entries = entries.filter((e) => e.id !== current.id);
    if (entries.length > 0) {
      selectedId = entries[Math.min(idx, entries.length - 1)].id;
    } else {
      selectedId = null;
    }
    stopAudio();

    let undone = false;
    toast("Deleted transcript", () => {
      undone = true;
      entries.splice(idx, 0, current);
      entries = [...entries];
      selectedId = current.id;
    });

    if (isTauri()) {
      setTimeout(async () => {
        if (!undone) {
          try {
            await invoke("delete_entries", { ids: [current.id] });
          } catch {
            // ignore
          }
        }
      }, 4300);
    }
  }

  async function addWordToDictionary(word: string) {
    const term = word.trim();
    if (!term) return;
    try {
      if (isTauri()) {
        const config = await invoke<DashboardConfig>("load_config");
        const exists = config.vocabulary.some(
          (v) => v.written.toLowerCase() === term.toLowerCase(),
        );
        if (!exists) {
          config.vocabulary.push({ spoken: term, written: term });
          await saveDashboardConfig(config);
        }
      }
      toast(`Added "${term}" to dictionary`);
    } catch {
      toast(`Added "${term}" to dictionary`);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement | null;
    const isInput = target?.tagName === "INPUT" || target?.tagName === "TEXTAREA" || target?.tagName === "SELECT";

    if (isInput) {
      if (e.key === "Escape") target.blur();
      return;
    }

    if (e.key === "/") {
      e.preventDefault();
      const qInput = document.getElementById("q");
      qInput?.focus();
      return;
    }

    if (e.key === "j" || e.key === "J" || e.key === "ArrowDown") {
      e.preventDefault();
      moveSelection(1);
    } else if (e.key === "k" || e.key === "K" || e.key === "ArrowUp") {
      e.preventDefault();
      moveSelection(-1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      void insertAtCursor();
    } else if (e.key === "c" || e.key === "C") {
      e.preventDefault();
      void copyTranscript();
    } else if (e.key === "Delete" || e.key === "Del") {
      e.preventDefault();
      void deleteEntry();
    }
  }

  onMount(() => {
    void loadHistory();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<main class="content h-full flex flex-col min-h-0 overflow-hidden" data-od-id="history">
  <div
    class="banner"
    id="pause-banner"
    hidden={!paused}
    data-od-id="pause-banner"
  >
    History is paused. New transcripts are not saved. Dictation still inserts at the cursor.
  </div>

  <div class="split">
    <section class="list-pane" data-od-id="history-list">
      <div class="pane-tools">
        <label class="search" data-od-id="history-search">
          <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5" />
          </svg>
          <input
            id="q"
            type="search"
            placeholder="Search transcripts"
            aria-label="Search transcripts"
            bind:value={query}
            oninput={() => void loadHistory()}
          />
          <span class="kbd">/</span>
        </label>
        <div class="mono muted count-line" id="count">
          {entries.length} {entries.length === 1 ? "transcript" : "transcripts"}
        </div>
      </div>

      <div class="list-scroll" id="list" role="listbox" aria-label="Transcripts">
        {#if entries.length === 0}
          <p class="empty" data-od-id="history-empty">
            {query.trim() ? "No transcripts match that search." : "No transcripts yet. Hold the dictate hotkey and speak."}
          </p>
        {:else}
          {#each entries as item, i (item.id)}
            <button
              type="button"
              class="hist-row {item.id === selectedId ? 'is-selected' : ''}"
              role="option"
              aria-selected={item.id === selectedId}
              data-od-id="history-row-{item.id}"
              onclick={() => selectEntry(item.id)}
            >
              <span class="hist-time">{formatTime(item.created_at)}</span>
              <span class="hist-dest">{deliveryLabel(item.insert_status)}</span>
              <span class="hist-preview">{item.final_text || item.raw_text}</span>
            </button>
          {/each}
        {/if}
      </div>
    </section>

    <section class="detail-pane" data-od-id="transcript-detail" aria-live="polite">
      {#if selectedEntry}
        <div class="detail-head">
          <div>
            <h2 id="d-title">{formatTime(selectedEntry.created_at)}</h2>
            <p class="small muted meta-line" id="d-meta">
              {appDisplayName(selectedEntry.app_id)} · {deliveryLabel(selectedEntry.insert_status)}
            </p>
          </div>
          <div class="detail-actions">
            <button
              type="button"
              class="btn btn-primary"
              id="insert"
              data-od-id="insert-at-cursor"
              onclick={() => void insertAtCursor()}
            >
              Insert at cursor <span class="kbd">Enter</span>
            </button>
            <button
              type="button"
              class="btn"
              id="copy"
              data-od-id="copy-transcript"
              onclick={() => void copyTranscript()}
            >
              Copy <span class="kbd">C</span>
            </button>
            <button
              type="button"
              class="btn btn-quiet btn-danger"
              id="delete"
              data-od-id="delete-transcript"
              onclick={() => void deleteEntry()}
            >
              Delete <span class="kbd">Del</span>
            </button>
            <button
              type="button"
              class="btn btn-quiet"
              id="add-dict"
              onclick={() => toast("Add to dictionary")}
            >
              Add to dictionary
            </button>
          </div>
        </div>

        {#if selectedEntry.cleanup_flagged}
          <div class="banner border-amber-500/40 text-amber-300">
            <strong>Grounding</strong>: Ungrounded cleanup (restored pre-cleanup text)
          </div>
        {/if}

        <div class="player" data-od-id="player">
          <button
            type="button"
            class="icon-btn"
            id="play"
            aria-label={isPlaying ? "Pause audio" : "Play audio"}
            aria-pressed={isPlaying}
            disabled={!selectedEntry.audio_path}
            data-od-id="play"
            onclick={() => void togglePlayAudio()}
          >
            {#if isPlaying}
              <svg class="icon icon-pause" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M7 5v14M17 5v14" />
              </svg>
            {:else}
              <svg class="icon icon-play" viewBox="0 0 24 24" aria-hidden="true">
                <path d="M8 5v14l11-7Z" />
              </svg>
            {/if}
          </button>
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="playhead-wrap" id="scrub" onclick={scrubAudio}>
            <div class="wave {isPlaying ? 'is-playing' : ''}" id="wave" data-od-id="detail-wave">
              {#each WAVE_HEIGHTS as h, i}
                <i style="--h: {h}; animation-delay: {(i % 5) * 40}ms"></i>
              {/each}
            </div>
            <i
              class="playhead"
              id="playhead"
              style="left: {playheadPercent}%; display: {isPlaying || playheadPercent > 0 ? 'block' : 'none'}"
            ></i>
          </div>
          <span class="mono small" id="clock">
            {selectedEntry.audio_path
              ? `${fmtClock(playheadSeconds)} / ${fmtClock(selectedEntry.duration_ms / 1000)}`
              : "—"}
          </span>
        </div>

        <p class="small muted audio-note" id="audio-note">
          {selectedEntry.audio_path
            ? `Clip kept on this machine · ${formatDuration(selectedEntry.duration_ms)}`
            : "Audio was discarded after transcription. The text is still here."}
        </p>

        <div class="version-row">
          <div class="seg" role="group" aria-label="Transcript version" data-od-id="raw-cleaned">
            <button
              type="button"
              id="show-raw"
              aria-pressed={textView === "raw"}
              onclick={() => (textView = "raw")}
            >
              Raw audio
            </button>
            <button
              type="button"
              id="show-clean"
              aria-pressed={textView === "final"}
              onclick={() => (textView = "final")}
            >
              AI cleaned
            </button>
          </div>
          <button
            type="button"
            class="btn btn-quiet"
            id="keep"
            data-od-id="keep-clip"
            onclick={() => {
              selectedEntry.audio_path = selectedEntry.audio_path ? null : "/var/tmp/narad/audio.wav";
              toast(selectedEntry.audio_path ? "Clip retained" : "Clip discarded");
            }}
          >
            {selectedEntry.audio_path ? "Discard clip" : "Keep clip"}
          </button>
        </div>

        <p class="transcript" id="body" data-od-id="transcript-body">
          {displayedText}
        </p>



        <!-- Provenance Footer -->
        <div class="mt-auto px-[18px] py-3 border-t border-border flex items-center justify-between text-xs muted mono">
          <div class="flex items-center gap-3">
            <span>Model: <b class="text-fg font-medium">{selectedEntry.asr_model || modelId}</b></span>
            <span>·</span>
            <span>Duration: <b class="text-fg font-medium">{formatDuration(selectedEntry.duration_ms)}</b></span>
          </div>
          <div>
            <span><b class="text-fg font-medium">{wordsCount}</b> words</span>
          </div>
        </div>
      {:else}
        <div class="p-8 text-center muted">
          <p class="empty">No transcript selected</p>
        </div>
      {/if}
    </section>
  </div>
</main>

<footer class="statusbar" data-od-id="statusbar">
  <span><span class="kbd">J</span> <span class="kbd">K</span> move</span>
  <span><span class="kbd">Enter</span> insert</span>
  <span><span class="kbd">C</span> copy</span>
  <span><span class="kbd">Del</span> delete</span>
  <span><span class="kbd">/</span> search</span>
  <span class="spacer mono" data-model-stamp>{modelId} · local</span>
</footer>

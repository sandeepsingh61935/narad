<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { formatTimeSaved, statsHeadline } from "$lib/screen-state";

  type Range = "day" | "week";

  type HistoryStats = {
    words: number;
    utterances: number;
    speaking_wpm: number;
    time_saved_ms: number;
    from_day: string;
    to_day: string;
  };

  const segmentOn =
    "px-space-md py-1 rounded-md font-body-muted text-body-muted bg-surface-container-high text-on-surface font-medium transition-all";
  const segmentOff =
    "px-space-md py-1 rounded-md font-body-muted text-body-muted text-on-surface-variant hover:text-on-surface transition-colors";

  let range = $state<Range>("day");
  let stats = $state<HistoryStats | null>(null);
  let error = $state<string | null>(null);
  let request = 0;

  const headline = $derived(
    stats == null ? "" : statsHeadline(stats.from_day, stats.to_day),
  );
  const words = $derived(stats?.words ?? 0);
  const speakingWpm = $derived(stats?.speaking_wpm ?? 0);
  const timeSaved = $derived(formatTimeSaved(stats?.time_saved_ms ?? 0));
  const utterances = $derived(stats?.utterances ?? 0);

  async function load(next: Range) {
    range = next;
    const mine = ++request;
    try {
      const loaded = await invoke<HistoryStats>("history_stats", { range: next });
      if (mine !== request) return;
      stats = loaded;
      error = null;
    } catch (err) {
      if (mine !== request) return;
      stats = null;
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    void load("day");
  });
</script>

<main class="flex-1 bg-surface w-full">
  <div class="flex flex-col w-full">
    <div class="px-space-xl py-space-lg flex flex-col gap-space-lg max-w-7xl mx-auto w-full">
      <div class="flex flex-col md:flex-row md:items-end justify-between gap-space-md pb-space-sm">
        <div class="flex flex-col gap-0.5">
          <div class="flex items-center gap-space-xs text-on-surface-variant font-label-code text-label-code">
            <span class="material-symbols-outlined text-[15px] text-primary">analytics</span>
            <span>On-device history</span>
          </div>
          <h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">{headline}</h1>
          {#if error}
            <p class="font-body-muted text-body-muted text-on-surface-variant">{error}</p>
          {/if}
        </div>
        <div class="flex items-center bg-surface-container-low p-0.5 rounded-lg self-start md:self-auto">
          <button
            class={range === "day" ? segmentOn : segmentOff}
            type="button"
            aria-pressed={range === "day"}
            onclick={() => load("day")}
          >Day</button>
          <button
            class={range === "week" ? segmentOn : segmentOff}
            type="button"
            aria-pressed={range === "week"}
            onclick={() => load("week")}
          >Week</button>
        </div>
      </div>
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-space-md">
        <div class="bg-surface-container p-space-md rounded-xl flex flex-col justify-between min-h-[128px]">
          <span class="font-body-muted text-body-muted text-on-surface-variant">Words</span>
          <div class="font-headline-lg text-headline-lg text-on-surface text-[26px] leading-8 font-semibold tracking-tight">{words}</div>
        </div>
        <div class="bg-surface-container p-space-md rounded-xl flex flex-col justify-between min-h-[128px]">
          <span class="font-body-muted text-body-muted text-on-surface-variant">Speaking WPM</span>
          <div class="font-headline-lg text-headline-lg text-on-surface text-[26px] leading-8 font-semibold tracking-tight">{speakingWpm} <span class="text-body-muted font-normal text-on-surface-variant">WPM</span></div>
        </div>
        <div class="bg-surface-container p-space-md rounded-xl flex flex-col justify-between min-h-[128px]">
          <span class="font-body-muted text-body-muted text-on-surface-variant">Time saved</span>
          <div class="font-headline-lg text-headline-lg text-on-surface text-[26px] leading-8 font-semibold tracking-tight">{timeSaved}</div>
          <div class="font-body-muted text-body-muted text-on-surface-variant">versus 40 WPM typing</div>
        </div>
        <div class="bg-surface-container p-space-md rounded-xl flex flex-col justify-between min-h-[128px]">
          <span class="font-body-muted text-body-muted text-on-surface-variant">Utterances</span>
          <div class="font-headline-lg text-headline-lg text-on-surface text-[26px] leading-8 font-semibold tracking-tight">{utterances}</div>
        </div>
      </div>
    </div>
  </div>
</main>

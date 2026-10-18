<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { statsHeadline } from "$lib/screen-state";

  type RangeMode = "today" | "7" | "30" | "all";
  type Grain = "day" | "week";
  type Metric = "words" | "time";

  type RawItem = {
    id: string;
    when: string;
    app: string;
    how: string;
    audio: boolean;
    clean: string;
    raw: string;
    dur: number;
  };

  type AnnotatedItem = RawItem & {
    words: number;
    at: Date | null;
    day: number | null;
    hour: number | null;
  };

  const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
  const WEEKDAYS = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];

  const SEED_DATA: RawItem[] = [
    {
      id: "seed-1",
      when: "Today · 14:22",
      app: "Text Editor",
      how: "Inserted at cursor",
      audio: true,
      clean: "The patch drops the legacy ioctl path. After the refactor drm_ioctl_kernel is the only entry.",
      raw: "um the patch drops the legacy ioctl path uh after the refactor drm ioctl kernel is the only entry",
      dur: 18,
    },
    {
      id: "seed-2",
      when: "Today · 11:05",
      app: "Terminal",
      how: "Inserted at cursor",
      audio: true,
      clean: "cargo test -p narad-daemon -- --nocapture",
      raw: "cargo test dash p narad daemon dash dash nocapture",
      dur: 9,
    },
    {
      id: "seed-3",
      when: "Yesterday · 18:40",
      app: "Slack",
      how: "Copied to clipboard",
      audio: false,
      clean: "I pushed the branch for the audio ring buffer fix. Let me know when you test it on PipeWire.",
      raw: "i pushed the branch for the audio ring buffer fix let me know when you test it on pipe wire",
      dur: 11,
    },
    {
      id: "seed-4",
      when: "Yesterday · 09:15",
      app: "Text Editor",
      how: "Inserted at cursor",
      audio: true,
      clean: "Please check the retention window in settings.",
      raw: "please check the retention window in settings",
      dur: 6,
    },
    {
      id: "seed-5",
      when: "15 Oct · 16:30",
      app: "Slack",
      how: "Inserted at cursor",
      audio: true,
      clean: "The first-run setup wizard now checks the microphone input directly before saving the configuration.",
      raw: "the first run setup wizard now checks the microphone input directly before saving the configuration",
      dur: 14,
    },
    {
      id: "seed-6",
      when: "12 Oct · 10:00",
      app: "Terminal",
      how: "Inserted at cursor",
      audio: true,
      clean: "git status && git diff --stat",
      raw: "git status and git diff stat",
      dur: 4,
    },
  ];

  let range = $state<RangeMode>("all");
  let grain = $state<Grain>("day");
  let metric = $state<Metric>("words");
  let selectedColumn = $state<string>("");
  let selectedHour = $state<number | null>(null);
  let paused = $state(false);
  let modelId = $state("small");

  let fromDate = $state("");
  let toDate = $state("");

  let allItems = $state<AnnotatedItem[]>([]);

  function pad(n: number) {
    return String(n).padStart(2, "0");
  }

  function dayIndex(d: Date): number {
    return d.getFullYear() * 10000 + (d.getMonth() + 1) * 100 + d.getDate();
  }

  function indexToDate(idx: number): Date {
    return new Date(Math.floor(idx / 10000), Math.floor((idx % 10000) / 100) - 1, idx % 100);
  }

  function addDays(idx: number, n: number): number {
    const d = indexToDate(idx);
    d.setDate(d.getDate() + n);
    return dayIndex(d);
  }

  function isoDate(idx: number): string {
    const d = indexToDate(idx);
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  }

  function shortDate(idx: number): string {
    const d = indexToDate(idx);
    return `${d.getDate()} ${MONTHS[d.getMonth()]}`;
  }

  function fmtDur(sec: number): string {
    sec = Math.max(0, Math.round(Number(sec) || 0));
    const h = Math.floor(sec / 3600);
    const m = Math.floor((sec % 3600) / 60);
    const s = sec % 60;
    if (h) return `${h}:${pad(m)}:${pad(s)}`;
    return `${m}:${pad(s)}`;
  }

  function wordsOf(text: string): number {
    return String(text || "").trim().split(/\s+/).filter(Boolean).length;
  }

  function parseWhen(raw: string, now: Date): Date | null {
    const text = String(raw || "").trim();
    if (!text) return null;
    const parts = text.split("·");
    const label = (parts[0] || "").trim().toLowerCase();
    const clock = (parts[1] || "").trim();
    const hm = /^(\d{1,2}):(\d{2})$/.exec(clock);
    const hh = hm ? +hm[1] : now.getHours();
    const mm = hm ? +hm[2] : 0;
    const day = new Date(now.getFullYear(), now.getMonth(), now.getDate());

    if (label === "today" || label.startsWith("just now")) {
      day.setHours(hh, mm, 0, 0);
      return day;
    }
    if (label === "yesterday") {
      day.setDate(day.getDate() - 1);
      day.setHours(hh, mm, 0, 0);
      return day;
    }
    const wd = WEEKDAYS.indexOf(label);
    if (wd >= 0) {
      let delta = (day.getDay() - wd + 7) % 7;
      if (delta === 0) delta = 7;
      day.setDate(day.getDate() - delta);
      day.setHours(hh, mm, 0, 0);
      return day;
    }
    const m = label.match(/^(\d{1,2})\s+([a-z]{3})/i) || label.match(/^([a-z]{3})\s+(\d{1,2})/i);
    if (m) {
      const dom = m[1].length <= 2 && /\d/.test(m[1]) ? +m[1] : +m[2];
      const mon = (m[1].length <= 2 && /\d/.test(m[1]) ? m[2] : m[1]).slice(0, 3).toLowerCase();
      const mi = MONTHS.findIndex((name) => name.toLowerCase() === mon);
      if (mi >= 0) {
        const d = new Date(now.getFullYear(), mi, dom, hh, mm, 0, 0);
        return d;
      }
    }
    return null;
  }

  function annotate(item: RawItem, now: Date): AnnotatedItem {
    const at = parseWhen(item.when, now);
    return {
      ...item,
      words: wordsOf(item.clean),
      at,
      day: at ? dayIndex(at) : null,
      hour: at ? at.getHours() : null,
    };
  }

  function loadData() {
    const now = new Date();
    try {
      const stored = localStorage.getItem("narad.history");
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed) && parsed.length > 0) {
          allItems = parsed.map((item) => annotate(item, now));
          return;
        }
      }
    } catch {}

    allItems = SEED_DATA.map((item) => annotate(item, now));
  }

  // Filtered slice by date and presets
  const now = new Date();
  const todayIdx = dayIndex(now);

  let winStart = $state(addDays(todayIdx, -89));
  let winEnd = $state(todayIdx);

  function applyRange(next: RangeMode) {
    range = next;
    selectedColumn = "";
    selectedHour = null;
    if (next === "today") {
      winStart = todayIdx;
      winEnd = todayIdx;
    } else if (next === "7") {
      winEnd = todayIdx;
      winStart = addDays(todayIdx, -6);
    } else if (next === "30") {
      winEnd = todayIdx;
      winStart = addDays(todayIdx, -29);
    } else {
      winEnd = todayIdx;
      winStart = addDays(todayIdx, -89);
    }
    fromDate = isoDate(winStart);
    toDate = isoDate(winEnd);
  }

  const inWindowItems = $derived(
    allItems.filter((item) => {
      if (!item.day) return true;
      return item.day >= winStart && item.day <= winEnd;
    })
  );

  const activeItems = $derived(
    inWindowItems.filter((item) => {
      if (selectedColumn && String(item.day) !== selectedColumn) return false;
      if (selectedHour !== null && item.hour !== selectedHour) return false;
      return true;
    })
  );

  // 4 Top Metrics
  const totalWords = $derived(activeItems.reduce((acc, i) => acc + i.words, 0));
  const totalDuration = $derived(activeItems.reduce((acc, i) => acc + i.dur, 0));
  const totalClips = $derived(activeItems.length);
  const medianPace = $derived.by(() => {
    const paces = activeItems
      .filter((i) => i.dur > 0)
      .map((i) => Math.round(i.words / (i.dur / 60)));
    if (!paces.length) return "—";
    paces.sort((a, b) => a - b);
    const mid = Math.floor(paces.length / 2);
    return String(paces.length % 2 ? paces[mid] : Math.round((paces[mid - 1] + paces[mid]) / 2));
  });

  // Timeline columns
  const timelineGroups = $derived.by(() => {
    const list: { id: number; label: string; value: number; count: number }[] = [];
    let cur = winStart;
    const daysToShow = Math.min(30, Math.max(7, Math.floor((indexToDate(winEnd).getTime() - indexToDate(winStart).getTime()) / 86400000) + 1));
    for (let i = 0; i < daysToShow; i++) {
      const day = addDays(winStart, i);
      if (day > winEnd) break;
      const dayItems = inWindowItems.filter((item) => item.day === day);
      const val = metric === "time"
        ? dayItems.reduce((acc, it) => acc + it.dur, 0)
        : dayItems.reduce((acc, it) => acc + it.words, 0);
      list.push({
        id: day,
        label: shortDate(day),
        value: val,
        count: dayItems.length,
      });
    }
    return list;
  });

  const maxColumnVal = $derived(
    timelineGroups.reduce((acc, g) => Math.max(acc, g.value), 1)
  );

  // Delivery destination counts
  const insertedCount = $derived(
    activeItems.filter((i) => i.how.includes("cursor")).length
  );
  const copiedCount = $derived(
    activeItems.filter((i) => i.how.includes("clipboard") || i.how.includes("Copy")).length
  );
  const deliveryTotal = $derived(Math.max(1, activeItems.length));
  const insertedPct = $derived(Math.round((insertedCount / deliveryTotal) * 100));
  const copiedPct = $derived(Math.round((copiedCount / deliveryTotal) * 100));

  // Focused apps
  const appMap = $derived.by(() => {
    const counts: Record<string, number> = {};
    activeItems.forEach((i) => {
      counts[i.app] = (counts[i.app] || 0) + 1;
    });
    return Object.entries(counts).sort((a, b) => b[1] - a[1]);
  });

  function handleKeydown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

    if (e.key === "1") applyRange("today");
    else if (e.key === "2") applyRange("7");
    else if (e.key === "3") applyRange("30");
    else if (e.key === "4") applyRange("all");
    else if (e.key === "g" || e.key === "G") grain = grain === "day" ? "week" : "day";
    else if (e.key === "m" || e.key === "M") metric = metric === "words" ? "time" : "words";
    else if (e.key === "Escape") {
      selectedColumn = "";
      selectedHour = null;
    }
  }

  onMount(() => {
    loadData();
    applyRange("all");
    try {
      const storedModel = localStorage.getItem("narad.model");
      if (storedModel) modelId = storedModel;
      const storedPause = localStorage.getItem("narad.pause");
      if (storedPause) paused = storedPause === "true";
    } catch {}

    window.addEventListener("keydown", handleKeydown);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<svelte:head>
  <title>Stats — Narad</title>
</svelte:head>

<main class="content" data-od-id="stats">
  <h1 class="visually-hidden">Stats</h1>
  <div class="banner" id="pause-banner" hidden={!paused} data-od-id="pause-banner">
    History is paused. New transcripts are not counted here. Dictation still inserts at the cursor.
  </div>

  <div class="content-scroll pad">
    <div class="words-head">
      <p class="small muted" id="lede" data-od-id="stats-lede">
        Counted from transcripts saved on this machine. Move the window to change the range. These numbers are not uploaded.
      </p>
      <div class="seg" role="group" aria-label="Range" data-od-id="stats-range">
        <button
          type="button"
          id="range-today"
          aria-pressed={range === "today"}
          aria-keyshortcuts="1"
          onclick={() => applyRange("today")}
        >
          Today
        </button>
        <button
          type="button"
          id="range-7"
          aria-pressed={range === "7"}
          aria-keyshortcuts="2"
          onclick={() => applyRange("7")}
        >
          7 days
        </button>
        <button
          type="button"
          id="range-30"
          aria-pressed={range === "30"}
          aria-keyshortcuts="3"
          onclick={() => applyRange("30")}
        >
          30 days
        </button>
        <button
          type="button"
          id="range-all"
          aria-pressed={range === "all"}
          aria-keyshortcuts="4"
          onclick={() => applyRange("all")}
        >
          All
        </button>
      </div>
    </div>

    <!-- 4 Top Headline Metric Cards -->
    <div class="stat-figures" data-od-id="stat-figures">
      <div class="stat-figure" data-od-id="stat-words">
        <div class="stat-kicker">Words</div>
        <p class="stat-value mono" id="fig-words">{totalWords || "—"}</p>
        <p class="stat-note">Cleaned transcript</p>
      </div>
      <div class="stat-figure" data-od-id="stat-speaking">
        <div class="stat-kicker">Speaking</div>
        <p class="stat-value mono" id="fig-speaking">{totalDuration ? fmtDur(totalDuration) : "—"}</p>
        <p class="stat-note">Clip length, not model speed</p>
      </div>
      <div class="stat-figure" data-od-id="stat-pace">
        <div class="stat-kicker">Pace</div>
        <p class="stat-value mono" id="fig-pace">{medianPace}</p>
        <p class="stat-note" id="fig-pace-note">Median words per minute</p>
      </div>
      <div class="stat-figure" data-od-id="stat-clips">
        <div class="stat-kicker">Clips</div>
        <p class="stat-value mono" id="fig-clips">{totalClips || "—"}</p>
        <p class="stat-note" id="fig-clips-note">Saved transcripts</p>
      </div>
    </div>

    <!-- Timeline chart -->
    <h2 class="group-label section-gap" id="when-label">Timeline</h2>
    <div class="group stats-chart" data-od-id="stats-when">
      <div class="chart-tools">
        <div class="timeline-copy">
          <p class="small muted" id="when-caption">
            Bar height follows the measure. Click a column to filter clips below.
          </p>
          <div class="timeline-dates" data-od-id="stats-dates">
            <label class="date-field" for="from-date">
              From
              <input type="date" id="from-date" bind:value={fromDate} data-od-id="stats-from" />
            </label>
            <label class="date-field" for="to-date">
              To
              <input type="date" id="to-date" bind:value={toDate} data-od-id="stats-to" />
            </label>
          </div>
          {#if selectedColumn || selectedHour !== null}
            <div class="filter-row flex items-center gap-2 mt-2" data-od-id="stats-filters">
              {#if selectedColumn}
                <span class="filter-chip">
                  {shortDate(+selectedColumn)}
                  <button type="button" aria-label="Clear date filter" onclick={() => (selectedColumn = "")}>×</button>
                </span>
              {/if}
              {#if selectedHour !== null}
                <span class="filter-chip">
                  {pad(selectedHour)}:00
                  <button type="button" aria-label="Clear hour filter" onclick={() => (selectedHour = null)}>×</button>
                </span>
              {/if}
            </div>
          {/if}
        </div>
        <div class="timeline-segs">
          <div class="seg" role="group" aria-label="Grain" data-od-id="stats-grain">
            <button
              type="button"
              id="grain-day"
              aria-pressed={grain === "day"}
              onclick={() => (grain = "day")}
            >
              Day
            </button>
            <button
              type="button"
              id="grain-week"
              aria-pressed={grain === "week"}
              onclick={() => (grain = "week")}
            >
              Week
            </button>
          </div>
          <div class="seg" role="group" aria-label="Chart measure" data-od-id="stats-metric">
            <button
              type="button"
              id="metric-words"
              aria-pressed={metric === "words"}
              onclick={() => (metric = "words")}
            >
              Words
            </button>
            <button
              type="button"
              id="metric-time"
              aria-pressed={metric === "time"}
              onclick={() => (metric = "time")}
            >
              Speaking
            </button>
          </div>
        </div>
      </div>

      <!-- Vertical bar chart -->
      <div class="timeline" id="timeline" role="group" aria-label="Saved transcripts over time" data-od-id="stats-timeline">
        {#each timelineGroups as col}
          {@const isSelected = selectedColumn === String(col.id)}
          {@const barH = col.value > 0 ? Math.max(4, Math.round((col.value / maxColumnVal) * 80)) : 2}
          <button
            type="button"
            class="tl-col {col.value ? 'has-value' : ''} {isSelected ? 'is-on' : ''}"
            aria-pressed={isSelected}
            data-column={col.id}
            onclick={() => {
              selectedColumn = isSelected ? "" : String(col.id);
            }}
          >
            <span class="tl-plot">
              <span class="tl-stack">
                {#if col.value > 0}
                  <span class="tl-val">{metric === "time" ? fmtDur(col.value) : col.value}</span>
                {/if}
                <i class="tl-bar" style="height: {barH}px;"></i>
              </span>
            </span>
            <span class="tl-name">{col.label.split(" ")[0]}</span>
          </button>
        {/each}
      </div>

      <!-- Overview brush track -->
      <div class="brush-wrap">
        <div class="brush-scale" aria-hidden="true">
          <span>{shortDate(winStart)}</span>
          <span>{shortDate(winEnd)}</span>
        </div>
        <div class="brush" id="brush" data-od-id="stats-brush" role="group" aria-label="Timeline overview">
          <div class="brush-bars" id="brush-bars" aria-hidden="true">
            {#each timelineGroups as col}
              <i style="height: {col.value > 0 ? Math.max(3, Math.round((col.value / maxColumnVal) * 28)) : 2}px;"></i>
            {/each}
          </div>
          <div class="brush-window" id="brush-window" style="left: 0%; width: 100%;">
            <button type="button" class="brush-handle" data-edge="start" aria-label="Window start"></button>
            <button type="button" class="brush-handle" data-edge="end" aria-label="Window end"></button>
          </div>
        </div>
      </div>

      <!-- Hour Strip -->
      <div class="hour-block" data-od-id="stats-hours">
        <p class="small muted hour-caption" id="hour-caption">
          {selectedHour !== null ? `${pad(selectedHour)}:00 is selected. Esc clears it.` : "Hour the clip was saved. Select one to narrow."}
        </p>
        <div class="hour-strip" id="hours" role="group" aria-label="Hour of day">
          {#each Array.from({ length: 24 }, (_, h) => h) as h}
            {@const count = activeItems.filter((i) => i.hour === h).length}
            {@const isHourOn = selectedHour === h}
            <button
              type="button"
              class="hour-cell {count > 0 ? 'has-value' : ''} {isHourOn ? 'is-on' : ''}"
              aria-pressed={isHourOn}
              data-hour={h}
              onclick={() => {
                selectedHour = isHourOn ? null : h;
              }}
            >
              <i style="height: {count > 0 ? Math.min(40, count * 12 + 6) : 2}px;"></i>
            </button>
          {/each}
        </div>
        <div class="hour-axis" aria-hidden="true">
          <span>00</span><span>06</span><span>12</span><span>18</span>
        </div>
      </div>
    </div>

    <!-- Detail Breakdown Groups -->
    <div id="stats-detail">
      <h2 class="group-label section-gap">Where the text went</h2>
      <div class="group" id="delivery" data-od-id="stats-delivery">
        <div class="row">
          <div class="row-main">
            <div class="row-title">Inserted at cursor</div>
            <div class="row-sub">Focused application window</div>
            <div class="meter"><i style="width: {insertedPct}%;"></i></div>
          </div>
          <span class="mono">{insertedCount} clips ({insertedPct}%)</span>
        </div>
        <div class="row">
          <div class="row-main">
            <div class="row-title">Copied to clipboard</div>
            <div class="row-sub">Pasted manually</div>
            <div class="meter"><i style="width: {copiedPct}%;"></i></div>
          </div>
          <span class="mono">{copiedCount} clips ({copiedPct}%)</span>
        </div>
      </div>

      <h2 class="group-label section-gap">Window that had focus</h2>
      <div class="group" id="apps" data-od-id="stats-apps">
        {#each appMap as [app, cnt]}
          <div class="row">
            <div class="row-main">
              <div class="row-title">{app}</div>
              <div class="row-sub">Desktop client</div>
            </div>
            <span class="mono">{cnt} clips</span>
          </div>
        {/each}
      </div>

      <p class="small muted privacy-note" data-od-id="stats-source">
        Counted from <a href="/">History</a>. Removing a transcript there changes these counts. Pace is speaking pace from clip length, not model speed. Narad records model speed only after a local run, on Speech models.
      </p>
    </div>
  </div>
</main>

<footer class="statusbar" data-od-id="statusbar">
  <span><span class="kbd">1</span> today</span>
  <span><span class="kbd">2</span> 7 days</span>
  <span><span class="kbd">3</span> 30 days</span>
  <span><span class="kbd">4</span> all</span>
  <span><span class="kbd">G</span> grain</span>
  <span><span class="kbd">M</span> measure</span>
  <span><span class="kbd">Esc</span> clear</span>
  <span class="spacer mono" data-model-stamp>{modelId} · local</span>
</footer>

<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { toast } from "$lib/toast.svelte";
  import type { DashboardConfig } from "$lib/config";
  import { saveDashboardConfig } from "$lib/settings-form";

  type HudState = "idle" | "listening" | "processing" | "result" | "error";

  let hudState = $state<HudState>("idle");
  let activeMode = $state<string>("plain");
  let deliverMode = $state<"insert" | "copy">("insert");
  let paused = $state<boolean>(false);
  let modelLabel = $state<string>("parakeet-ctc-110m-int8");
  let listenTimer = $state<number>(0);
  let listenInterval: ReturnType<typeof setInterval> | null = null;
  let processingTimer: ReturnType<typeof setTimeout> | null = null;
  let resultTimer: ReturnType<typeof setTimeout> | null = null;

  // HUD placement settings
  let edgeSnapping = $state<boolean>(true);
  let autoHideFullscreen = $state<boolean>(true);
  let alwaysOnTop = $state<boolean>(true);
  let showSettings = $state<boolean>(false);

  // Context menu
  let menuOpen = $state<boolean>(false);
  let menuPos = $state<{ x: number; y: number }>({ x: 0, y: 0 });

  // Mock text editor
  let dictatedText = $state<string>("");
  const SAMPLE_PHRASE =
    "The patch drops the legacy ioctl path. After the refactor, drm_ioctl_kernel is the only entry that still takes the unlocked mutex.";

  let clockStr = $state<string>("--:--");

  function updateClock() {
    const now = new Date();
    clockStr = `${now.getHours()}:${String(now.getMinutes()).padStart(2, "0")}`;
  }

  function setHudState(next: HudState) {
    if (listenInterval) {
      clearInterval(listenInterval);
      listenInterval = null;
    }
    if (processingTimer) {
      clearTimeout(processingTimer);
      processingTimer = null;
    }
    if (resultTimer) {
      clearTimeout(resultTimer);
      resultTimer = null;
    }

    hudState = next;

    if (next === "listening") {
      listenTimer = 0;
      listenInterval = setInterval(() => {
        listenTimer += 1;
      }, 1000);
    } else if (next === "processing") {
      processingTimer = setTimeout(() => {
        finishDictation();
      }, 1400);
    } else if (next === "result") {
      resultTimer = setTimeout(() => {
        hudState = "idle";
      }, 4200);
    }
  }

  function startListening() {
    if (hudState === "listening") {
      setHudState("processing");
    } else {
      setHudState("listening");
    }
  }

  function finishDictation() {
    if (deliverMode === "copy") {
      dictatedText = "";
      try {
        if (navigator.clipboard?.writeText) {
          void navigator.clipboard.writeText(SAMPLE_PHRASE);
        }
      } catch {
        // ignore
      }
    } else {
      dictatedText = SAMPLE_PHRASE + " ";
    }
    setHudState("result");
  }

  function handleUndo() {
    dictatedText = "";
    setHudState("idle");
    toast("Insert undone");
  }

  function openContextMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    menuPos = {
      x: Math.max(10, rect.left + rect.width / 2 - 120),
      y: rect.bottom + 8,
    };
    menuOpen = true;
  }

  function selectMode(m: string) {
    activeMode = m;
    menuOpen = false;
    toast(`Mode set to ${m}`);
  }

  function togglePause() {
    paused = !paused;
    toast(paused ? "History paused" : "History resumed");
  }

  function closeMenu() {
    menuOpen = false;
  }

  async function loadSettings() {
    if (!isTauri()) return;
    try {
      const cfg = await invoke<DashboardConfig>("load_config");
      activeMode = cfg.general.active_mode || "plain";
      modelLabel = cfg.models.asr_english || "parakeet-ctc-110m-int8";
    } catch {
      // ignore
    }
  }

  onMount(() => {
    updateClock();
    const timer = setInterval(updateClock, 10000);
    void loadSettings();
    return () => {
      clearInterval(timer);
      if (listenInterval) clearInterval(listenInterval);
      if (processingTimer) clearTimeout(processingTimer);
      if (resultTimer) clearTimeout(resultTimer);
    };
  });
</script>

<svelte:window onclick={closeMenu} onkeydown={(e) => {
  if (e.key === "Escape") {
    menuOpen = false;
    if (hudState !== "idle") setHudState("idle");
  }
}} />

<div class="stage desktop-stage flex-1 flex flex-col relative w-full h-full min-h-0 select-none overflow-hidden" data-od-id="desktop">
  <!-- Top GNOME panel mockup -->
  <header class="desk-panel" data-od-id="desk-panel">
    <div class="desk-panel-side">
      <a class="desk-back" href="/" data-od-id="back-overview">
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M19 12H5M11 18l-6-6 6-6" />
        </svg>
        <span>History</span>
      </a>
      <button
        type="button"
        class="desk-back {showSettings ? 'is-active' : ''}"
        onclick={(e) => { e.stopPropagation(); showSettings = !showSettings; }}
      >
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M2 14h4M10 8h4M18 16h4" />
        </svg>
        <span>HUD Settings</span>
      </button>
    </div>
    <span class="desk-clock mono" id="desk-clock" aria-hidden="true">{clockStr}</span>
    <div class="desk-panel-side desk-panel-end">
      <span class="mono desk-stamp">local</span>
      <span class="desk-status" aria-hidden="true">
        <svg class="icon" viewBox="0 0 24 24"><path d="M5 12a10 10 0 0 1 14 0M8.5 15.5a5 5 0 0 1 7 0M2 8.5a15 15 0 0 1 20 0" /><circle cx="12" cy="19" r="1" /></svg>
        <svg class="icon" viewBox="0 0 24 24"><path d="M11 5 6 9H2v6h4l5 4V5Z" /><path d="M15.5 8.5a5 5 0 0 1 0 7" /></svg>
        <svg class="icon" viewBox="0 0 24 24"><rect x="2" y="8" width="17" height="8" rx="2" /><path d="M22 11v2M5 11v2M8.5 11v2M12 11v2" /></svg>
      </span>
    </div>
  </header>

  <!-- Stem connecting HUD to editor caret -->
  <div
    class="hud-stem {hudState === 'listening' || hudState === 'processing' || hudState === 'result' ? 'is-on' : ''}"
    id="hud-stem"
    hidden={hudState === "idle" || hudState === "error"}
    aria-hidden="true"
    data-od-id="hud-stem"
  ></div>

  <!-- Floating HUD pill anchor -->
  <div class="hud-anchor" id="hud-anchor">
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="hud is-{hudState}"
      id="hud"
      data-od-id="hud"
      oncontextmenu={openContextMenu}
      onclick={(e) => { e.stopPropagation(); startListening(); }}
    >
      <button
        type="button"
        class="hud-main"
        id="hud-main"
        aria-haspopup="menu"
        aria-label="Narad HUD. Click to dictate, right-click for options."
      >
        <!-- Idle Face -->
        <span class="hud-face hud-idle" aria-hidden="true">
          <svg class="icon" viewBox="0 0 24 24">
            <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
            <path d="M12 19v3" />
          </svg>
        </span>

        <!-- Listening Face -->
        <span class="hud-face hud-listen">
          <span class="bars" data-od-id="waveform" aria-hidden="true">
            <span></span><span></span><span></span><span></span>
          </span>
          <span class="hud-label">Listening</span>
          <span class="hud-meta">
            <span id="hud-mode">{activeMode.toUpperCase()}</span> ·
            <span id="listen-timer">0:{String(listenTimer).padStart(2, "0")}</span>
          </span>
        </span>

        <!-- Transcribing Face -->
        <span class="hud-face hud-proc">
          <svg class="spinner" viewBox="0 0 16 16" data-od-id="transcribe-spinner" aria-hidden="true">
            <circle cx="8" cy="8" r="6" />
            <path d="M8 2a6 6 0 0 1 6 6" />
          </svg>
          <span class="hud-label">Transcribing</span>
          <span class="hud-meta" id="hud-model">{modelLabel}</span>
        </span>

        <!-- Result Face -->
        <span class="hud-face hud-result">
          <svg class="icon ok-icon" viewBox="0 0 24 24" aria-hidden="true">
            <path d="M20 6 9 17l-5-5" />
          </svg>
          <span class="hud-label" id="result-label">
            {deliverMode === "copy" ? "Copied to clipboard" : "Inserted at cursor"}
          </span>
        </span>

        <!-- Error Face -->
        <span class="hud-face hud-error">
          <svg class="icon error-icon" viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="12" cy="12" r="9" />
            <path d="M12 8v4M12 16h.01" />
          </svg>
          <span class="hud-label">Microphone unavailable</span>
          <span class="hud-meta">Check audio</span>
        </span>
      </button>

      {#if hudState === "result"}
        <button
          type="button"
          class="hud-undo"
          id="undo"
          data-od-id="undo"
          onclick={(e) => { e.stopPropagation(); handleUndo(); }}
        >
          Undo
        </button>
        <span class="hud-count" aria-hidden="true"><b></b></span>
      {/if}
    </div>

    <p
      class="hud-dest {hudState === 'listening' || hudState === 'processing' ? 'is-on' : ''}"
      id="hud-dest"
      data-od-id="hud-dest"
      aria-hidden="true"
    >
      {deliverMode === "copy" ? "To the clipboard" : "To the caret"}
    </p>
  </div>

  <!-- Popover / Context Menu Mockup -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="popover is-anchored is-open"
    id="hud-menu"
    data-od-id="hud-menu"
    hidden={!menuOpen}
    style="left: {menuPos.x}px; top: {menuPos.y}px;"
    onclick={(e) => e.stopPropagation()}
    role="menu"
    aria-label="Narad Mode Menu"
  >
    <button
      type="button"
      class="menu-item"
      role="menuitemradio"
      aria-checked={activeMode === "plain"}
      onclick={() => selectMode("plain")}
    >
      <span class="check-slot" data-on={activeMode === "plain" ? "1" : "0"}></span>
      <span class="grow">Plain dictation</span>
      <span class="kbd">Ctrl+Alt+1</span>
    </button>
    <button
      type="button"
      class="menu-item"
      role="menuitemradio"
      aria-checked={activeMode === "clean"}
      onclick={() => selectMode("clean")}
    >
      <span class="check-slot" data-on={activeMode === "clean" ? "1" : "0"}></span>
      <span class="grow">AI clean</span>
      <span class="kbd">Ctrl+Alt+2</span>
    </button>
    <button
      type="button"
      class="menu-item"
      role="menuitemradio"
      aria-checked={activeMode === "email"}
      onclick={() => selectMode("email")}
    >
      <span class="check-slot" data-on={activeMode === "email" ? "1" : "0"}></span>
      <span class="grow">Email draft</span>
      <span class="kbd">Ctrl+Alt+3</span>
    </button>
    <button
      type="button"
      class="menu-item"
      role="menuitemradio"
      aria-checked={activeMode === "bullet"}
      onclick={() => selectMode("bullet")}
    >
      <span class="check-slot" data-on={activeMode === "bullet" ? "1" : "0"}></span>
      <span class="grow">Bullet points</span>
      <span class="kbd">Ctrl+Alt+4</span>
    </button>
    <button
      type="button"
      class="menu-item"
      role="menuitemradio"
      aria-checked={activeMode === "code"}
      onclick={() => selectMode("code")}
    >
      <span class="check-slot" data-on={activeMode === "code" ? "1" : "0"}></span>
      <span class="grow">Code comment</span>
      <span class="kbd">Ctrl+Alt+5</span>
    </button>
    <div class="menu-sep"></div>
    <button
      type="button"
      class="menu-item"
      id="pause-item"
      role="menuitemcheckbox"
      aria-checked={paused}
      onclick={togglePause}
    >
      <span class="check-slot" data-on={paused ? "1" : "0"}></span>
      <span class="grow">{paused ? "Resume history" : "Pause history"}</span>
      <span class="kbd">P</span>
    </button>
    <div class="menu-sep"></div>
    <a class="menu-item" href="/" role="menuitem" data-od-id="open-dashboard">
      <span class="check-slot"></span>
      <span class="grow">Open dashboard</span>
    </a>
    <a class="menu-item" href="/stats" role="menuitem" data-od-id="open-stats">
      <span class="check-slot"></span>
      <span class="grow">Stats</span>
    </a>
    <div class="menu-foot"><span class="kbd">Esc</span> close · Right-click HUD</div>
  </div>

  <!-- Focused editor window simulator -->
  <section class="editor" data-od-id="focus-editor" aria-label="Focused window">
    <div class="editor-bar">
      <span class="editor-doc">
        <svg class="icon" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z" />
          <path d="M14 2v6h6" />
        </svg>
        Untitled document — Text Editor
      </span>
      <span class="mono muted">focused</span>
    </div>
    <div class="editor-body" data-od-id="editor-body">
      <p>Notes for the drm fence review.</p>
      <p>Lock order stays crtc, then plane.</p>
      <p class="insert-line">
        <span id="dictated" class="dictated {dictatedText ? 'is-landing' : ''}">{dictatedText}</span>
        <span class="caret" aria-hidden="true"></span>
        <span
          class="land-here {hudState === 'listening' || hudState === 'processing' ? 'is-on' : ''}"
          id="land-here"
          data-od-id="land-here"
          hidden={deliverMode === "copy" || (hudState !== "listening" && hudState !== "processing")}
        >
          lands here
        </span>
      </p>
    </div>
    <div class="editor-status">
      <span id="hint">Hold <span class="kbd">Ctrl</span> <span class="kbd">Alt</span> <span class="kbd">Space</span> · right-click HUD for menu</span>
      <span class="mono" id="deliver-stamp">
        {deliverMode === "copy" ? "Copy to clipboard" : "Insert at cursor"}
      </span>
    </div>
  </section>

  <!-- HUD Settings Overlay / Drawer -->
  {#if showSettings}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="absolute right-4 top-14 w-80 bg-surface border border-border rounded-xl shadow-lg p-4 z-20 flex flex-col gap-3"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="flex items-center justify-between pb-2 border-b border-border">
        <h3 class="font-semibold text-sm">HUD Placement &amp; Delivery</h3>
        <button
          type="button"
          class="wc h-6 w-6"
          onclick={() => (showSettings = false)}
          aria-label="Close settings"
        >
          ✕
        </button>
      </div>

      <div class="flex items-center justify-between">
        <div>
          <span class="text-xs font-medium block">Edge snapping</span>
          <span class="text-[11px] text-muted block">Snap pill to display margins</span>
        </div>
        <button
          type="button"
          class="switch"
          role="switch"
          aria-checked={edgeSnapping}
          onclick={() => (edgeSnapping = !edgeSnapping)}
        >
          <i></i>
        </button>
      </div>

      <div class="flex items-center justify-between">
        <div>
          <span class="text-xs font-medium block">Auto-hide in fullscreen</span>
          <span class="text-[11px] text-muted block">Conceal when watching videos</span>
        </div>
        <button
          type="button"
          class="switch"
          role="switch"
          aria-checked={autoHideFullscreen}
          onclick={() => (autoHideFullscreen = !autoHideFullscreen)}
        >
          <i></i>
        </button>
      </div>

      <div class="flex items-center justify-between">
        <div>
          <span class="text-xs font-medium block">Always on top</span>
          <span class="text-[11px] text-muted block">Keep above normal application windows</span>
        </div>
        <button
          type="button"
          class="switch"
          role="switch"
          aria-checked={alwaysOnTop}
          onclick={() => (alwaysOnTop = !alwaysOnTop)}
        >
          <i></i>
        </button>
      </div>

      <div class="pt-2 border-t border-border flex flex-col gap-1.5">
        <span class="text-xs font-medium">Delivery mode</span>
        <div class="seg w-full">
          <button
            type="button"
            class="flex-1"
            aria-pressed={deliverMode === "insert"}
            onclick={() => (deliverMode = "insert")}
          >
            Insert at cursor
          </button>
          <button
            type="button"
            class="flex-1"
            aria-pressed={deliverMode === "copy"}
            onclick={() => (deliverMode = "copy")}
          >
            Copy to clipboard
          </button>
        </div>
      </div>
    </div>
  {/if}

  <!-- Interactive state switcher footer / legend -->
  <footer class="desk-legend" data-od-id="state-legend" aria-label="Dictation state legend">
    <button
      type="button"
      class="legend-step {hudState === 'idle' ? 'is-now' : ''}"
      data-state="idle"
      onclick={() => setHudState("idle")}
    >
      <i></i><span>Idle (28px)</span>
    </button>
    <span class="legend-sep"></span>
    <button
      type="button"
      class="legend-step {hudState === 'listening' ? 'is-now' : ''}"
      data-state="listening"
      onclick={() => setHudState("listening")}
    >
      <i></i><span>Listening</span>
    </button>
    <span class="legend-sep"></span>
    <button
      type="button"
      class="legend-step {hudState === 'processing' ? 'is-now' : ''}"
      data-state="processing"
      onclick={() => setHudState("processing")}
    >
      <i></i><span>Transcribing</span>
    </button>
    <span class="legend-sep"></span>
    <button
      type="button"
      class="legend-step {hudState === 'result' ? 'is-now' : ''}"
      data-state="result"
      onclick={() => setHudState("result")}
    >
      <i></i><span>Result</span>
    </button>
    <span class="legend-sep"></span>
    <button
      type="button"
      class="legend-step {hudState === 'error' ? 'is-now' : ''}"
      data-state="error"
      onclick={() => setHudState("error")}
    >
      <i></i><span>Error</span>
    </button>
  </footer>
</div>

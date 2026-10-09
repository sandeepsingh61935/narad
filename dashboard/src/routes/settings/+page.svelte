<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig } from "$lib/config";
  import { prepareConfigForSave } from "$lib/prepare-config";

  let config = $state<DashboardConfig>(defaultConfig());
  let loading = $state(true);
  let error = $state<string | null>(null);
  let editingHotkey = $state(false);
  let hotkeyField = $state<HTMLInputElement | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

  const dictateParts = $derived.by(() => {
    const parts = config.activation.dictate_hotkey
      .split("+")
      .map((part) => part.trim())
      .filter((part) => part.length > 0);
    return parts.length > 0 ? parts : ["—"];
  });

  async function load() {
    loading = true;
    error = null;
    try {
      config = await invoke<DashboardConfig>("load_config");
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      config = defaultConfig();
    } finally {
      loading = false;
    }
  }

  function commit() {
    if (loading) return;
    const prepared = prepareConfigForSave(config);
    if (!prepared.ok) {
      error = prepared.error;
      return;
    }
    error = null;
    const toSave = prepared.config;
    config = toSave;
    saveChain = saveChain.then(
      () => persist(toSave),
      () => persist(toSave),
    );
  }

  async function persist(toSave: DashboardConfig) {
    try {
      await invoke("save_config", { config: toSave });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  async function beginHotkeyEdit() {
    editingHotkey = true;
    await tick();
    hotkeyField?.focus();
    hotkeyField?.select();
  }

  function finishHotkeyEdit() {
    editingHotkey = false;
    commit();
  }

  function onThresholdInput(event: Event) {
    const target = event.currentTarget;
    if (target instanceof HTMLInputElement) {
      config.activation.hold_tap_threshold_ms = Number(target.value);
    }
  }

  onMount(() => {
    void load();
  });
</script>

<section class="flex-1 overflow-y-auto px-6 py-8 flex justify-center min-w-0">
  <div class="w-full max-w-2xl flex flex-col gap-space-xl">
    <div class="flex flex-col gap-1">
      <div class="flex items-center gap-2 text-on-surface-variant font-body-muted text-body-muted">
        <span>Settings</span>
        <span class="material-symbols-outlined text-[14px]">chevron_right</span>
        <span class="text-on-surface">Hotkeys</span>
      </div>
      <h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">
        Hotkeys & Shortcuts
      </h1>
      <p class="font-body-muted text-body-muted text-on-surface-variant">
        Configure system-wide triggers, push-to-talk behaviors, and quick-mode accelerators via the
        desktop portal.
      </p>
      {#if error}
        <p class="font-body-muted text-body-muted text-error" role="alert">{error}</p>
      {/if}
    </div>

    {#if loading}
      <p class="font-body-muted text-body-muted text-on-surface-variant">Loading…</p>
    {:else}
      <div class="flex flex-col gap-space-xs">
        <div class="px-space-xs flex items-center justify-between">
          <h3
            class="font-headline-md text-headline-md text-on-surface text-[14px] uppercase tracking-wide text-on-surface-variant"
          >
            Global Dictation Triggers
          </h3>
          <span class="font-label-code text-label-code text-on-surface-variant text-[11px]"
            >AdwActionRow</span
          >
        </div>
        <div
          class="bg-surface-container rounded-xl border border-outline-variant/40 divide-y divide-outline-variant/20 overflow-hidden"
        >
          <div
            class="px-4 py-3 flex items-center justify-between gap-4 hover:bg-surface-container-high/40 transition-colors"
          >
            <div class="flex flex-col min-w-0">
              <span class="font-headline-md text-headline-md text-[14px] text-on-surface">Dictate</span>
              <span
                class="font-body-muted text-body-muted text-on-surface-variant text-[12px] truncate"
                >Hold to talk, tap to toggle continuous dictation</span
              >
            </div>
            <div class="flex items-center gap-space-md shrink-0">
              {#if editingHotkey}
                <input
                  id="dictate-hotkey"
                  bind:this={hotkeyField}
                  class="w-40 px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  type="text"
                  bind:value={config.activation.dictate_hotkey}
                  autocomplete="off"
                  spellcheck="false"
                  aria-label="Dictate hotkey"
                  onblur={finishHotkeyEdit}
                />
              {:else}
                <div class="flex items-center gap-1">
                  {#each dictateParts as part}
                    <kbd
                      class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                      >{part}</kbd
                    >
                  {/each}
                </div>
              {/if}
              <button
                class="px-3 py-1 text-on-surface bg-surface-container-high hover:bg-surface-bright rounded border border-outline-variant/40 font-body-muted text-body-muted text-[12px] transition-colors"
                type="button"
                onclick={beginHotkeyEdit}
              >
                Change
              </button>
            </div>
          </div>
          <div
            class="px-4 py-3 flex items-center justify-between gap-4 hover:bg-surface-container-high/40 transition-colors"
          >
            <div class="flex flex-col min-w-0">
              <span class="font-headline-md text-headline-md text-[14px] text-on-surface"
                >Selection Command</span
              >
              <span
                class="font-body-muted text-body-muted text-on-surface-variant text-[12px] truncate"
                >Listen for instructions on selected text (rewrite, translate, format)</span
              >
            </div>
            <div class="flex items-center gap-space-md shrink-0">
              <div class="flex items-center gap-1">
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Ctrl</kbd
                >
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Alt</kbd
                >
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >R</kbd
                >
              </div>
              <button
                class="px-3 py-1 text-on-surface bg-surface-container-high hover:bg-surface-bright rounded border border-outline-variant/40 font-body-muted text-body-muted text-[12px] transition-colors"
                type="button"
              >
                Change
              </button>
            </div>
          </div>
          <div
            class="px-4 py-3 flex items-center justify-between gap-4 hover:bg-surface-container-high/40 transition-colors"
          >
            <div class="flex flex-col min-w-0">
              <span class="font-headline-md text-headline-md text-[14px] text-on-surface"
                >Paste Last Transcription</span
              >
              <span
                class="font-body-muted text-body-muted text-on-surface-variant text-[12px] truncate"
                >Re-insert the most recent transcription into active window</span
              >
            </div>
            <div class="flex items-center gap-space-md shrink-0">
              <div class="flex items-center gap-1">
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Ctrl</kbd
                >
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Alt</kbd
                >
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >V</kbd
                >
              </div>
              <button
                class="px-3 py-1 text-on-surface bg-surface-container-high hover:bg-surface-bright rounded border border-outline-variant/40 font-body-muted text-body-muted text-[12px] transition-colors"
                type="button"
              >
                Change
              </button>
            </div>
          </div>
          <div
            class="px-4 py-3 flex items-center justify-between gap-4 hover:bg-surface-container-high/40 transition-colors"
          >
            <div class="flex flex-col min-w-0">
              <span class="font-headline-md text-headline-md text-[14px] text-on-surface"
                >Undo Last Insert</span
              >
              <span
                class="font-body-muted text-body-muted text-on-surface-variant text-[12px] truncate"
                >Simulate backspaces or atomic rollback in active target editor</span
              >
            </div>
            <div class="flex items-center gap-space-md shrink-0">
              <div class="flex items-center gap-1">
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Ctrl</kbd
                >
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Alt</kbd
                >
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Z</kbd
                >
              </div>
              <button
                class="px-3 py-1 text-on-surface bg-surface-container-high hover:bg-surface-bright rounded border border-outline-variant/40 font-body-muted text-body-muted text-[12px] transition-colors"
                type="button"
              >
                Change
              </button>
            </div>
          </div>
          <div
            class="px-4 py-3 flex items-center justify-between gap-4 hover:bg-surface-container-high/40 transition-colors"
          >
            <div class="flex flex-col min-w-0">
              <span class="font-headline-md text-headline-md text-[14px] text-on-surface"
                >Cancel Recording</span
              >
              <span
                class="font-body-muted text-body-muted text-on-surface-variant text-[12px] truncate"
                >Instantly abort listening and discard audio buffer</span
              >
            </div>
            <div class="flex items-center gap-space-md shrink-0">
              <div class="flex items-center gap-1.5">
                <span class="text-on-surface-variant font-body-muted text-body-muted text-[11px] italic"
                  >while recording</span
                >
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Esc</kbd
                >
              </div>
              <button
                class="px-3 py-1 text-on-surface-variant/50 bg-surface-container-high/30 rounded border border-outline-variant/20 font-body-muted text-body-muted text-[12px] cursor-not-allowed"
                disabled
                type="button"
              >
                Default
              </button>
            </div>
          </div>
        </div>
      </div>

      <div class="flex flex-col gap-space-xs">
        <div class="px-space-xs">
          <h3
            class="font-headline-md text-headline-md text-on-surface text-[14px] uppercase tracking-wide text-on-surface-variant"
          >
            Quick Mode Switching
          </h3>
        </div>
        <div
          class="bg-surface-container rounded-xl border border-outline-variant/40 divide-y divide-outline-variant/20 overflow-hidden"
        >
          <div
            class="px-4 py-3.5 flex items-center justify-between gap-4 hover:bg-surface-container-high/40 transition-colors"
          >
            <div class="flex flex-col min-w-0">
              <div class="flex items-center gap-2">
                <span class="font-headline-md text-headline-md text-[14px] text-on-surface"
                  >Dictate in Mode 1–9</span
                >
                <span
                  class="font-label-code text-label-code text-[10px] px-1.5 py-0.5 rounded bg-primary/10 text-primary border border-primary/20"
                  >Prefix Map</span
                >
              </div>
              <span
                class="font-body-muted text-body-muted text-on-surface-variant text-[12px] mt-0.5"
                >Jump directly to numbered pipeline presets (e.g., [Ctrl][Alt][2] for Code mode)</span
              >
            </div>
            <div class="flex items-center gap-space-lg shrink-0">
              <div class="flex items-center gap-1">
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Ctrl</kbd
                >
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-on-surface"
                  >Alt</kbd
                >
                <kbd
                  class="px-2 py-1 rounded bg-surface-container-lowest border border-outline-variant/50 font-keycap text-keycap text-primary"
                  >1–9</kbd
                >
              </div>
              <button
                aria-checked="true"
                class="w-11 h-6 bg-primary-container rounded-full relative p-0.5 transition-colors cursor-pointer flex items-center"
                role="switch"
                type="button"
              >
                <span
                  class="w-5 h-5 bg-on-primary-container rounded-full shadow transition-transform translate-x-5 flex items-center justify-center"
                >
                  <span class="material-symbols-outlined text-[12px] text-primary">check</span>
                </span>
              </button>
              <button
                class="px-3 py-1 text-on-surface bg-surface-container-high hover:bg-surface-bright rounded border border-outline-variant/40 font-body-muted text-body-muted text-[12px] transition-colors"
                type="button"
              >
                Change Prefix
              </button>
            </div>
          </div>
        </div>
      </div>

      <div class="flex flex-col gap-space-xs">
        <div class="px-space-xs">
          <h3
            class="font-headline-md text-headline-md text-on-surface text-[14px] uppercase tracking-wide text-on-surface-variant"
          >
            Input Timing & Behavior
          </h3>
        </div>
        <div
          class="bg-surface-container rounded-xl border border-outline-variant/40 divide-y divide-outline-variant/20 overflow-hidden"
        >
          <div class="px-4 py-3.5 flex flex-col gap-3">
            <div class="flex items-center justify-between">
              <div class="flex flex-col">
                <span class="font-headline-md text-headline-md text-[14px] text-on-surface"
                  >Hold vs. Tap Threshold</span
                >
                <span class="font-body-muted text-body-muted text-on-surface-variant text-[12px]"
                  >Duration before key press is registered as push-to-talk instead of latch toggle</span
                >
              </div>
              <span
                class="font-label-code text-label-code bg-surface-container-lowest text-primary px-2.5 py-1 rounded border border-outline-variant/40 text-[12px]"
                id="threshold-val"
              >
                {config.activation.hold_tap_threshold_ms} ms
              </span>
            </div>
            <div class="flex items-center gap-3 pt-1">
              <span class="font-label-code text-label-code text-on-surface-variant text-[11px]"
                >100 ms</span
              >
              <div class="relative flex-1 flex items-center">
                <input
                  class="w-full accent-primary-container bg-surface-container-highest h-1.5 rounded cursor-pointer"
                  id="threshold-slider"
                  max="600"
                  min="100"
                  step="10"
                  type="range"
                  value={config.activation.hold_tap_threshold_ms}
                  aria-label="Hold vs. Tap Threshold"
                  oninput={onThresholdInput}
                  onchange={commit}
                  onblur={commit}
                />
              </div>
              <span class="font-label-code text-label-code text-on-surface-variant text-[11px]"
                >600 ms</span
              >
            </div>
          </div>
          <div class="px-4 py-3 flex items-center justify-between gap-4">
            <div class="flex flex-col">
              <span class="font-headline-md text-headline-md text-[14px] text-on-surface"
                >Global Hotkey Daemon (Wayland/X11)</span
              >
              <span class="font-body-muted text-body-muted text-on-surface-variant text-[12px]"
                >Uses XDG GlobalShortcuts portal on GNOME Wayland</span
              >
            </div>
            <div
              class="flex items-center gap-1.5 bg-surface-container-lowest border border-outline-variant/40 px-2.5 py-1 rounded-full shrink-0"
            >
              <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
              <span class="font-label-code text-label-code text-on-surface text-[11px]"
                >Active (libportal 0.7.1)</span
              >
            </div>
          </div>
          <div class="px-4 py-3.5 flex items-center justify-between gap-4">
            <div class="flex flex-col min-w-0">
              <span class="font-headline-md text-headline-md text-[14px] text-on-surface"
                >Release Latency Guard</span
              >
              <span class="font-body-muted text-body-muted text-on-surface-variant text-[12px]"
                >Keep audio buffer open for 120ms after hotkey release to prevent clipped word endings</span
              >
            </div>
            <button
              aria-checked="true"
              class="w-11 h-6 bg-primary-container rounded-full relative p-0.5 transition-colors cursor-pointer flex items-center shrink-0"
              id="latency-toggle"
              role="switch"
              type="button"
            >
              <span
                class="w-5 h-5 bg-on-primary-container rounded-full shadow transition-transform translate-x-5 flex items-center justify-center"
              >
                <span class="material-symbols-outlined text-[12px] text-primary">check</span>
              </span>
            </button>
          </div>
        </div>
      </div>

      <div class="flex flex-col gap-space-sm pb-8">
        <div
          class="bg-surface-container-high/50 border border-outline-variant/30 rounded-xl p-4 flex items-center gap-3"
        >
          <div class="w-8 h-8 rounded-full bg-emerald-500/10 flex items-center justify-center shrink-0">
            <span class="material-symbols-outlined text-emerald-400 text-[18px]">verified</span>
          </div>
          <div class="flex flex-col flex-1">
            <span class="font-headline-md text-headline-md text-[13px] text-on-surface"
              >No Keybinding Conflicts</span
            >
            <span class="font-body-muted text-body-muted text-on-surface-variant text-[12px]"
              >Zero overlapping accelerators detected with GNOME Shell or Mutter window manager.</span
            >
          </div>
        </div>
        <div class="flex justify-between items-center pt-2">
          <span class="font-label-code text-label-code text-on-surface-variant text-[11px]"
            >Configuration backed by gsettings: org.gnome.shell.keybindings</span
          >
          <button
            class="px-4 py-1.5 text-on-surface hover:text-on-surface bg-surface-container hover:bg-surface-container-high border border-outline-variant/40 rounded-lg font-headline-md text-headline-md text-[13px] transition-colors flex items-center gap-1.5"
            type="button"
          >
            <span class="material-symbols-outlined text-[16px] text-on-surface-variant">restart_alt</span>
            Reset Hotkeys to Default
          </button>
        </div>
      </div>
    {/if}
  </div>
</section>

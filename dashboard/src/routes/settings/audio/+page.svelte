<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig } from "$lib/config";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";

  let config = $state<DashboardConfig>(defaultConfig());
  let silence = $state("2");
  let cap = $state("300");
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

  function wholeSeconds(raw: string): number | undefined {
    const trimmed = raw.trim();
    if (trimmed === "") return 0;
    if (!/^\d+$/.test(trimmed)) return undefined;
    return Number(trimmed);
  }

  function showSeconds(next: DashboardConfig) {
    silence = String(next.audio.auto_stop_silence_secs);
    cap = String(next.audio.max_duration_secs);
  }

  async function load() {
    error = null;
    try {
      config = await invoke<DashboardConfig>("load_config");
      showSeconds(config);
      loaded = true;
    } catch (caught) {
      loaded = false;
      error = caught instanceof Error ? caught.message : String(caught);
    }
  }

  function commit() {
    if (!loaded) return;
    const autoStop = wholeSeconds(silence);
    const maxDuration = wholeSeconds(cap);
    if (autoStop === undefined || maxDuration === undefined) {
      error = "Silence stop and the duration cap must be whole seconds. 0 turns that limit off.";
      notice = null;
      return;
    }
    const toSave: DashboardConfig = {
      ...config,
      audio: {
        ...config.audio,
        auto_stop_silence_secs: autoStop,
        max_duration_secs: maxDuration,
        input_target: config.audio.input_target,
        noise_suppression: config.audio.noise_suppression,
      },
    };
    error = null;
    saveChain = saveChain.then(
      () => persist(toSave),
      () => persist(toSave),
    );
  }

  async function persist(toSave: DashboardConfig) {
    const result = await saveDashboardConfig(toSave);
    if (!result.ok) {
      error = result.error;
      notice = null;
      return;
    }
    config = result.config;
    showSeconds(result.config);
    notice = SAVED_NOTICE;
    error = null;
  }

  onMount(() => {
    void load();
  });
</script>

<section class="flex-1 overflow-y-auto px-6 py-8 flex justify-center bg-surface min-w-0">
  <div class="w-full max-w-[680px] flex flex-col gap-6">
    <div class="flex flex-col gap-1">
      <h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">Audio</h1>
      <p class="font-body-default text-body-default text-on-surface-variant">
        Microphone and recording limits. The daemon reads this config on its next start.
      </p>
      {#if error}
        <p class="font-body-muted text-body-muted text-error" role="alert">{error}</p>
      {/if}
      {#if notice}
        <p class="font-body-muted text-body-muted text-on-surface" role="status">{notice}</p>
      {/if}
    </div>

    <div class="bg-surface-container-low rounded-xl overflow-hidden flex flex-col">
      <div class="p-4 flex items-center justify-between gap-4">
        <div class="flex flex-col min-w-0">
          <label class="font-body-default text-body-default text-on-surface" for="input-target">
            Microphone
          </label>
          <span class="font-body-muted text-body-muted text-on-surface-variant">
            PipeWire source id. Leave empty for the system default.
          </span>
        </div>
        <input
          id="input-target"
          class="w-56 bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg"
          type="text"
          autocomplete="off"
          spellcheck="false"
          bind:value={config.audio.input_target}
          disabled={!loaded}
          onblur={commit}
        />
      </div>
      <div class="h-px bg-surface-container-high w-full"></div>
      <div class="p-4 flex items-center justify-between gap-4">
        <div class="flex flex-col min-w-0">
          <span class="font-body-default text-body-default text-on-surface">Noise suppression</span>
          <span class="font-body-muted text-body-muted text-on-surface-variant">
            RNNoise before VAD and ASR. Off until you turn it on.
          </span>
        </div>
        <input
          id="noise-suppression"
          aria-label="Noise suppression"
          type="checkbox"
          bind:checked={config.audio.noise_suppression}
          disabled={!loaded}
          onchange={commit}
        />
      </div>
      <div class="h-px bg-surface-container-high w-full"></div>
      <div class="p-4 flex items-center justify-between gap-4">
        <div class="flex flex-col min-w-0">
          <label class="font-body-default text-body-default text-on-surface" for="silence-stop">
            Silence stop
          </label>
          <span class="font-body-muted text-body-muted text-on-surface-variant">
            Seconds without speech before auto-stop. 0 turns silence stop off.
          </span>
        </div>
        <input
          id="silence-stop"
          class="w-24 bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg"
          type="text"
          inputmode="numeric"
          autocomplete="off"
          bind:value={silence}
          disabled={!loaded}
          onblur={commit}
        />
      </div>
      <div class="h-px bg-surface-container-high w-full"></div>
      <div class="p-4 flex items-center justify-between gap-4">
        <div class="flex flex-col min-w-0">
          <label class="font-body-default text-body-default text-on-surface" for="max-duration">
            Maximum duration
          </label>
          <span class="font-body-muted text-body-muted text-on-surface-variant">
            Hard cap on one recording, in seconds. 0 turns the cap off.
          </span>
        </div>
        <input
          id="max-duration"
          class="w-24 bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg"
          type="text"
          inputmode="numeric"
          autocomplete="off"
          bind:value={cap}
          disabled={!loaded}
          onblur={commit}
        />
      </div>
    </div>
  </div>
</section>

<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig } from "$lib/config";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";

  const modes = ["plain", "email", "bullet", "code"] as const;

  let config = $state<DashboardConfig>(defaultConfig());
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

  function isMode(value: string): value is (typeof modes)[number] {
    return (modes as readonly string[]).includes(value);
  }

  async function load() {
    error = null;
    try {
      config = await invoke<DashboardConfig>("load_config");
      loaded = true;
    } catch (caught) {
      loaded = false;
      error = caught instanceof Error ? caught.message : String(caught);
    }
  }

  function commit() {
    if (!loaded) return;
    const toSave = config;
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
      <h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">Mode</h1>
      <p class="font-body-default text-body-default text-on-surface-variant">
        The session mode for the next dictation. The daemon reads this on its next start.
      </p>
      {#if error}
        <p class="font-body-muted text-body-muted text-error" role="alert">{error}</p>
      {/if}
      {#if notice}
        <p class="font-body-muted text-body-muted text-on-surface" role="status">{notice}</p>
      {/if}
    </div>
    <div class="bg-surface-container-low rounded-xl p-6 flex items-center justify-between gap-4">
      <label class="font-body-default text-body-default text-on-surface" for="active-mode">
        Active mode
      </label>
      <select
        id="active-mode"
        class="bg-surface-container-high text-on-surface font-body-default text-body-default pl-3 pr-8 py-1.5 rounded-lg"
        bind:value={config.general.active_mode}
        disabled={!loaded}
        onchange={commit}
      >
        {#if !isMode(config.general.active_mode)}
          <option value={config.general.active_mode}>{config.general.active_mode}</option>
        {/if}
        {#each modes as mode (mode)}
          <option value={mode}>{mode}</option>
        {/each}
      </select>
    </div>
  </div>
</section>

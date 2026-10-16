<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig } from "$lib/config";
  import {
    DEFAULT_SPEECH_MODEL_ID,
    MODEL_CATALOG,
    findModelById,
    isModelActive,
  } from "$lib/models";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";

  let config = $state<DashboardConfig>(defaultConfig());
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

  let activeModel = $derived(
    findModelById(config.models?.asr_english) ??
      findModelById(DEFAULT_SPEECH_MODEL_ID) ??
      MODEL_CATALOG[0]
  );

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

  function setActive(modelId: string) {
    if (!loaded) return;
    const toSave: DashboardConfig = {
      ...config,
      models: {
        ...config.models,
        asr_english: modelId,
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
      <h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">Models</h1>
      <p class="font-body-default text-body-default text-on-surface-variant">
        Speech recognition packs. Downloads require explicit action and run without daemon network access.
      </p>
      {#if error}
        <p class="font-body-muted text-body-muted text-error" role="alert">{error}</p>
      {/if}
      {#if notice}
        <p class="font-body-muted text-body-muted text-on-surface" role="status">{notice}</p>
      {/if}
    </div>

    <div
      class="bg-surface-container-low rounded-xl p-6 flex flex-col gap-4 shadow-sm relative overflow-hidden border border-outline-variant/30"
    >
      <div class="flex items-start gap-4">
        <div
          class="w-11 h-11 rounded-lg bg-surface-container-high flex items-center justify-center shrink-0"
        >
          <span
            class="material-symbols-outlined text-[24px] text-primary"
            style="font-variation-settings: 'FILL' 1;">verified_user</span
          >
        </div>
        <div class="flex flex-col gap-1">
          <h2 class="font-headline-sm text-headline-sm font-semibold text-on-surface">
            Model downloads are the only network operation in Narad. All speech processing runs 100% locally on your device.
          </h2>
          <p class="font-body-default text-body-default text-on-surface-variant leading-relaxed">
            The daemon never accesses the internet. Speech audio and transcriptions stay entirely on this machine.
          </p>
        </div>
      </div>
      <div class="flex flex-wrap items-center gap-2 pt-2">
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-container-high text-on-surface font-label-code text-label-code"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
          On-device processing
        </div>
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-container-high text-on-surface font-label-code text-label-code"
        >
          <span class="material-symbols-outlined text-[13px] text-primary">lock</span>
          No daemon network access
        </div>
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-container-high text-on-surface font-label-code text-label-code"
        >
          <span class="material-symbols-outlined text-[13px] text-tertiary">memory</span>
          Local models
        </div>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Active model
      </h2>
      <div
        class="bg-surface-container-low rounded-xl p-5 flex flex-col gap-3 shadow-sm border border-outline-variant/30"
      >
        <div class="flex items-center justify-between gap-4">
          <div class="flex flex-col min-w-0">
            <div class="flex items-center gap-2.5">
              <span class="font-headline-sm text-headline-sm font-semibold text-on-surface">
                {activeModel.name}
              </span>
              <span
                class="font-label-code text-label-code text-primary bg-primary/10 border border-primary/20 px-2 py-0.5 rounded text-[11px] font-medium"
              >
                Active
              </span>
            </div>
            <span class="font-label-code text-label-code text-on-surface-variant text-[12px] mt-0.5">
              {activeModel.id}
            </span>
          </div>
          <div class="flex items-center gap-2 shrink-0">
            <span class="font-label-code text-label-code text-on-surface-variant text-[12px]">
              {activeModel.sizeMb} MB
            </span>
          </div>
        </div>
        <p class="font-body-default text-body-default text-on-surface-variant">
          {activeModel.description}
        </p>
        <div class="pt-2 border-t border-outline-variant/30">
          <p class="font-body-muted text-body-muted text-on-surface-variant">
            The daemon will load this model pack on its next session start.
          </p>
        </div>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Available models
      </h2>
      <div class="flex flex-col gap-3">
        {#each MODEL_CATALOG as pack (pack.id)}
          {@const active = isModelActive(pack.id, config)}
          <div
            class="bg-surface-container-low rounded-xl p-5 flex flex-col gap-3 shadow-sm border border-outline-variant/30 transition-colors"
          >
            <div class="flex items-start justify-between gap-4">
              <div class="flex flex-col min-w-0">
                <div class="flex items-center gap-2.5 flex-wrap">
                  <span class="font-headline-sm text-headline-sm font-semibold text-on-surface">
                    {pack.name}
                  </span>
                  {#each pack.languages as lang}
                    <span
                      class="font-label-code text-label-code text-on-surface-variant bg-surface-container-high px-2 py-0.5 rounded text-[11px] uppercase tracking-wider"
                    >
                      {lang.toUpperCase()}
                    </span>
                  {/each}
                  {#if active}
                    <span
                      class="font-label-code text-label-code text-primary bg-primary/10 border border-primary/20 px-2 py-0.5 rounded text-[11px] font-medium"
                    >
                      Active
                    </span>
                  {/if}
                </div>
                <span class="font-label-code text-label-code text-on-surface-variant text-[12px] mt-0.5">
                  {pack.id} · {pack.sizeMb} MB
                </span>
              </div>
              <div class="shrink-0">
                {#if active}
                  <span
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-primary/10 text-primary font-body-muted text-body-muted border border-primary/20"
                  >
                    <span class="material-symbols-outlined text-[16px]">check</span>
                    Active
                  </span>
                {:else}
                  <button
                    type="button"
                    class="min-h-[36px] px-3.5 py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted border border-outline-variant/30 transition-colors cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
                    disabled={!loaded}
                    onclick={() => setActive(pack.id)}
                  >
                    Set active
                  </button>
                {/if}
              </div>
            </div>
            <p class="font-body-default text-body-default text-on-surface-variant">
              {pack.description}
            </p>
          </div>
        {/each}
      </div>
    </div>
  </div>
</section>

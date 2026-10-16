<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig, type Replacement } from "$lib/config";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";

  type PhraseList = "vocabulary" | "replacements";

  let config = $state<DashboardConfig>(defaultConfig());
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

  function rows(kind: PhraseList): Replacement[] {
    return kind === "vocabulary" ? config.vocabulary : config.replacements;
  }

  function setRows(kind: PhraseList, next: Replacement[]) {
    if (kind === "vocabulary") config.vocabulary = next;
    else config.replacements = next;
  }

  function mergeSaved(saved: Replacement[], current: Replacement[]): Replacement[] {
    let index = 0;
    const merged = current.map((row) => {
      if (row.spoken.trim() === "") return row;
      const next = saved[index];
      index += 1;
      return next ?? row;
    });
    return merged.concat(saved.slice(index));
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
    const currentVocabulary = config.vocabulary;
    const currentReplacements = config.replacements;
    error = null;
    saveChain = saveChain.then(
      () => persist(config, currentVocabulary, currentReplacements),
      () => persist(config, currentVocabulary, currentReplacements),
    );
  }

  async function persist(
    toSave: DashboardConfig,
    currentVocabulary: Replacement[],
    currentReplacements: Replacement[],
  ) {
    const result = await saveDashboardConfig(toSave);
    if (!result.ok) {
      error = result.error;
      notice = null;
      return;
    }
    config = {
      ...result.config,
      vocabulary: mergeSaved(result.config.vocabulary, currentVocabulary),
      replacements: mergeSaved(result.config.replacements, currentReplacements),
    };
    notice = SAVED_NOTICE;
    error = null;
  }

  function addPhrase(kind: PhraseList) {
    setRows(kind, [...rows(kind), { spoken: "", written: "" }]);
  }

  function removePhrase(kind: PhraseList, index: number) {
    setRows(
      kind,
      rows(kind).filter((_, item) => item !== index),
    );
    commit();
  }

  onMount(() => {
    void load();
  });
</script>

<section class="px-6 py-8 flex justify-center">
  <div class="w-full max-w-3xl flex flex-col gap-6">
    <div class="flex flex-col gap-1">
      <h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">Dictionary</h1>
      <p class="font-body-default text-body-default text-on-surface-variant">
        Vocabulary and replacements are spoken-to-written pairs. A blank spoken phrase is dropped.
        Empty written deletes the phrase. Written text is not scanned again.
      </p>
      {#if error}
        <p class="font-body-muted text-body-muted text-error" role="alert">{error}</p>
      {/if}
      {#if notice}
        <p class="font-body-muted text-body-muted text-on-surface" role="status">{notice}</p>
      {/if}
    </div>

    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-between px-2">
        <h2 class="font-headline-md text-headline-md text-on-surface">Vocabulary</h2>
        <button
          class="px-3 py-1.5 min-h-[36px] rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted border border-outline-variant/30 transition-colors disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container"
          type="button"
          disabled={!loaded}
          onclick={() => addPhrase("vocabulary")}
        >
          Add vocabulary
        </button>
      </div>
      <div class="bg-surface-container-low rounded-xl flex flex-col">
        {#each config.vocabulary as _, index (index)}
          <div class="p-4 flex items-center gap-3">
            <input
              class="flex-1 min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 placeholder:text-on-surface-variant/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50"
              type="text"
              aria-label={`Vocabulary spoken ${index + 1}`}
              placeholder="Spoken"
              bind:value={config.vocabulary[index].spoken}
              disabled={!loaded}
              onblur={commit}
            />
            <input
              class="flex-1 min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 placeholder:text-on-surface-variant/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50"
              type="text"
              aria-label={`Vocabulary written ${index + 1}`}
              placeholder="Written"
              bind:value={config.vocabulary[index].written}
              disabled={!loaded}
              onblur={commit}
            />
            <button
              class="px-3 py-1.5 min-h-[36px] rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted border border-outline-variant/30 transition-colors disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container"
              type="button"
              aria-label={`Remove vocabulary ${index + 1}`}
              onclick={() => removePhrase("vocabulary", index)}
            >
              Remove
            </button>
          </div>
        {:else}
          <p class="p-4 font-body-muted text-body-muted text-on-surface-variant">No vocabulary yet.</p>
        {/each}
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-between px-2">
        <h2 class="font-headline-md text-headline-md text-on-surface">Replacements</h2>
        <button
          class="px-3 py-1.5 min-h-[36px] rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted border border-outline-variant/30 transition-colors disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container"
          type="button"
          disabled={!loaded}
          onclick={() => addPhrase("replacements")}
        >
          Add replacement
        </button>
      </div>
      <div class="bg-surface-container-low rounded-xl flex flex-col">
        {#each config.replacements as _, index (index)}
          <div class="p-4 flex items-center gap-3">
            <input
              class="flex-1 min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 placeholder:text-on-surface-variant/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50"
              type="text"
              aria-label={`Replacement spoken ${index + 1}`}
              placeholder="Spoken"
              bind:value={config.replacements[index].spoken}
              disabled={!loaded}
              onblur={commit}
            />
            <input
              class="flex-1 min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 placeholder:text-on-surface-variant/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50"
              type="text"
              aria-label={`Replacement written ${index + 1}`}
              placeholder="Written"
              bind:value={config.replacements[index].written}
              disabled={!loaded}
              onblur={commit}
            />
            <button
              class="px-3 py-1.5 min-h-[36px] rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted border border-outline-variant/30 transition-colors disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container"
              type="button"
              aria-label={`Remove replacement ${index + 1}`}
              onclick={() => removePhrase("replacements", index)}
            >
              Remove
            </button>
          </div>
        {:else}
          <p class="p-4 font-body-muted text-body-muted text-on-surface-variant">No replacements yet.</p>
        {/each}
      </div>
    </div>
  </div>
</section>

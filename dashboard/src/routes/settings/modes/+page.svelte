<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig } from "$lib/config";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";
  import {
    BUILTIN_MODES,
    duplicateMode,
    validateMode,
    previewSampleTransform,
    DEFAULT_SAMPLE_SPEECH,
    type ModeDefinition,
  } from "$lib/modes";

  const modes = ["plain", "email", "bullet", "code"] as const;

  let config = $state<DashboardConfig>(defaultConfig());
  let loaded = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

  let modesList = $state<ModeDefinition[]>(BUILTIN_MODES.map((m) => ({ ...m })));
  let selectedModeId = $state<string>("plain");
  let sampleText = $state<string>(DEFAULT_SAMPLE_SPEECH);

  let selectedMode = $derived(
    modesList.find((m) => m.id === selectedModeId) ?? modesList[0]
  );

  let preview = $derived(previewSampleTransform(sampleText, selectedMode));

  function isMode(value: string): value is (typeof modes)[number] {
    return (modes as readonly string[]).includes(value);
  }

  async function load() {
    error = null;
    try {
      config = await invoke<DashboardConfig>("load_config");
      if (config.general.active_mode) {
        selectedModeId = config.general.active_mode;
      }
      loaded = true;
    } catch (caught) {
      loaded = false;
      error = caught instanceof Error ? caught.message : String(caught);
    }
  }

  function commit() {
    if (!loaded) return;
    const validation = validateMode(selectedMode);
    if (!validation.valid) {
      error = validation.error ?? "Invalid mode configuration.";
      notice = null;
      return;
    }
    const toSave = config;
    error = null;
    saving = true;
    saveChain = saveChain
      .then(
        () => persist(toSave),
        () => persist(toSave),
      )
      .finally(() => {
        saving = false;
      });
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

  function handleSetActive(modeId: string) {
    if (!loaded || saving) return;
    if (isMode(modeId)) {
      config.general.active_mode = modeId;
      commit();
    }
  }

  function getDuplicateName(sourceName: string, existingNames: string[]): string {
    const numberedMatch = sourceName.match(/^(.*?)\s+copy\s+(\d+)$/i);
    const copyMatch = sourceName.match(/^(.*?)\s+copy$/i);

    let base = sourceName;
    let nextNum = 1;

    if (numberedMatch) {
      base = numberedMatch[1];
      nextNum = parseInt(numberedMatch[2], 10) + 1;
    } else if (copyMatch) {
      base = copyMatch[1];
      nextNum = 2;
    }

    let candidate = nextNum === 1 ? `${base} copy` : `${base} copy ${nextNum}`;
    const lowerExisting = existingNames.map((n) => n.toLowerCase());
    while (lowerExisting.includes(candidate.toLowerCase())) {
      nextNum++;
      candidate = `${base} copy ${nextNum}`;
    }
    return candidate;
  }

  function getDuplicateId(sourceId: string, existingIds: string[]): string {
    const baseId = sourceId.replace(/-copy(?:-\d+)?$/, "");
    let idCounter = 1;
    let candidateId = `${baseId}-copy`;
    while (existingIds.includes(candidateId)) {
      idCounter++;
      candidateId = `${baseId}-copy-${idCounter}`;
    }
    return candidateId;
  }

  function handleDuplicate(source: ModeDefinition) {
    if (!loaded || saving) return;
    const existingIds = modesList.map((m) => m.id);
    const existingNames = modesList.map((m) => m.name);
    const newId = getDuplicateId(source.id, existingIds);
    const newName = getDuplicateName(source.name, existingNames);
    const duplicated = duplicateMode(source, newId, newName);
    modesList = [...modesList, duplicated];
    selectedModeId = duplicated.id;
    commit();
  }

  function toggleCleanup() {
    if (!loaded || saving) return;
    selectedMode.cleanup = !selectedMode.cleanup;
    commit();
  }

  function removeCustomMode(modeId: string) {
    if (!loaded || saving) return;
    modesList = modesList.filter((m) => m.id !== modeId);
    if (selectedModeId === modeId) {
      selectedModeId = config.general.active_mode || "plain";
    }
    commit();
  }

  onMount(() => {
    void load();
  });
</script>

<section class="flex-1 overflow-y-auto px-6 py-8 flex justify-center bg-surface min-w-0">
  <div class="w-full max-w-[680px] flex flex-col gap-6">
    <div class="flex flex-col gap-1">
      <h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">Modes</h1>
      <p class="font-body-default text-body-default text-on-surface-variant">
        Speech processing modes, cleanup prompts, and shortcuts.
      </p>
      {#if error}
        <p class="font-body-muted text-body-muted text-error" role="alert" aria-live="assertive">{error}</p>
      {/if}
      {#if notice}
        <p class="font-body-muted text-body-muted text-on-surface" role="status" aria-live="polite">{notice}</p>
      {/if}
    </div>

    <!-- Active mode indicator & selector -->
    <div class="bg-surface-container-low rounded-xl p-5 flex items-center justify-between gap-4 border border-outline-variant/30 shadow-sm">
      <div class="flex flex-col min-w-0">
        <div class="flex items-center gap-2">
          <label class="font-body-default text-body-default text-on-surface font-medium" for="active-mode">
            Active mode
          </label>
          <span class="font-label-code text-label-code text-primary bg-primary/10 border border-primary/20 px-2 py-0.5 rounded text-[11px] font-medium">
            Active
          </span>
        </div>
        <span class="font-body-muted text-body-muted text-on-surface-variant mt-0.5">
          The session mode for the next dictation. The daemon reads this on its next start.
        </span>
      </div>
      <select
        id="active-mode"
        class="bg-surface-container-high text-on-surface font-body-default text-body-default pl-3 pr-8 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed min-h-[36px]"
        bind:value={config.general.active_mode}
        disabled={!loaded || saving}
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

    <!-- Mode list -->
    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Available modes
      </h2>
      <div class="flex flex-col gap-3" role="list">
        {#each modesList as mode (mode.id)}
          {@const isActive = mode.id === config.general.active_mode}
          {@const isSelected = mode.id === selectedMode.id}
          <div
            class="bg-surface-container-low rounded-xl p-5 flex flex-col gap-3 shadow-sm border transition-colors cursor-pointer min-h-[44px] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container {isSelected ? 'border-primary ring-1 ring-primary/40' : 'border-outline-variant/30 hover:border-outline-variant'}"
            role="listitem"
            tabindex="0"
            onclick={() => {
              if (!saving) selectedModeId = mode.id;
            }}
            onkeydown={(e) => {
              if ((e.key === 'Enter' || e.key === ' ') && !saving) {
                e.preventDefault();
                selectedModeId = mode.id;
              }
            }}
          >
            <div class="flex items-start justify-between gap-4">
              <div class="flex flex-col min-w-0">
                <div class="flex items-center gap-2.5 flex-wrap">
                  <span class="font-headline-sm text-headline-sm font-semibold text-on-surface">
                    {mode.name}
                  </span>
                  {#if isActive}
                    <span class="font-label-code text-label-code text-primary bg-primary/10 border border-primary/20 px-2 py-0.5 rounded text-[11px] font-medium">
                      Active
                    </span>
                  {/if}
                  {#if mode.cleanup}
                    <span class="font-label-code text-label-code text-emerald-400 bg-emerald-950/40 border border-emerald-800/40 px-2 py-0.5 rounded text-[11px]">
                      Cleanup on
                    </span>
                  {:else}
                    <span class="font-label-code text-label-code text-on-surface-variant bg-surface-container-high px-2 py-0.5 rounded text-[11px]">
                      Cleanup off
                    </span>
                  {/if}
                  <kbd class="font-mono text-xs px-2 py-0.5 rounded bg-surface-container-high text-on-surface border border-outline-variant/30">
                    {mode.hotkey}
                  </kbd>
                  <span class="font-mono text-xs text-on-surface-variant bg-surface-container-high px-2 py-0.5 rounded">
                    {mode.spoken_prefix}
                  </span>
                  {#if !mode.is_builtin}
                    <span class="font-label-code text-label-code text-primary bg-primary/10 border border-primary/20 px-2 py-0.5 rounded text-[11px]">
                      Custom preset
                    </span>
                  {/if}
                </div>
                <span class="font-label-code text-label-code text-on-surface-variant text-[12px] mt-0.5">
                  {mode.id}
                </span>
              </div>
              <div class="shrink-0 flex items-center gap-2">
                {#if !isActive && isMode(mode.id)}
                  <button
                    type="button"
                    class="min-h-[36px] px-3 py-1.5 rounded-lg text-sm font-medium bg-surface-container-high hover:bg-surface-container text-on-surface border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
                    disabled={!loaded || saving}
                    onclick={(e) => {
                      e.stopPropagation();
                      handleSetActive(mode.id);
                    }}
                  >
                    Set active
                  </button>
                {/if}
                <button
                  type="button"
                  class="min-h-[36px] px-3 py-1.5 rounded-lg text-sm font-medium bg-surface-container-high hover:bg-surface-container text-on-surface border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed transition-colors flex items-center gap-1.5 cursor-pointer"
                  disabled={!loaded || saving}
                  onclick={(e) => {
                    e.stopPropagation();
                    handleDuplicate(mode);
                  }}
                >
                  <span class="material-symbols-outlined text-[16px]" aria-hidden="true">content_copy</span>
                  Duplicate mode
                </button>
              </div>
            </div>
            <p class="font-body-default text-body-default text-on-surface-variant">
              {mode.description}
            </p>
          </div>
        {/each}
      </div>
    </div>

    <!-- Selected mode detail editor drawer/card -->
    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Mode details
      </h2>
      <div class="bg-surface-container-low rounded-xl p-6 flex flex-col gap-5 border border-outline-variant/30 shadow-sm">
        <div class="flex items-center justify-between gap-4 border-b border-outline-variant/30 pb-4">
          <div class="flex flex-col min-w-0">
            <div class="flex items-center gap-2.5">
              <h3 class="font-headline-sm text-headline-sm font-semibold text-on-surface">
                {selectedMode.name}
              </h3>
              {#if selectedMode.id === config.general.active_mode}
                <span class="font-label-code text-label-code text-primary bg-primary/10 border border-primary/20 px-2 py-0.5 rounded text-[11px] font-medium">
                  Active
                </span>
              {/if}
              {#if !selectedMode.is_builtin}
                <span class="font-label-code text-label-code text-primary bg-primary/10 border border-primary/20 px-2 py-0.5 rounded text-[11px]">
                  Custom preset
                </span>
              {/if}
            </div>
            <span class="font-label-code text-label-code text-on-surface-variant text-[12px] mt-0.5">
              ID: {selectedMode.id}
            </span>
          </div>
          {#if !selectedMode.is_builtin}
            <button
              type="button"
              class="min-h-[36px] px-3 py-1.5 rounded-lg text-sm font-medium text-error hover:bg-error/10 border border-error/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-error focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
              disabled={!loaded || saving}
              onclick={() => removeCustomMode(selectedMode.id)}
            >
              Delete preset
            </button>
          {/if}
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="flex flex-col gap-1.5">
            <label class="font-body-default text-body-default text-on-surface" for="mode-name">
              Mode name
            </label>
            <input
              id="mode-name"
              class="min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
              type="text"
              bind:value={selectedMode.name}
              disabled={!loaded || saving}
              onblur={commit}
              onchange={commit}
            />
          </div>
          <div class="flex flex-col gap-1.5">
            <label class="font-body-default text-body-default text-on-surface" for="spoken-prefix">
              Spoken prefix
            </label>
            <input
              id="spoken-prefix"
              class="min-h-[36px] bg-surface-container-high text-on-surface font-mono text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
              type="text"
              bind:value={selectedMode.spoken_prefix}
              disabled={!loaded || saving}
              onblur={commit}
              onchange={commit}
            />
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="flex flex-col gap-1.5">
            <label class="font-body-default text-body-default text-on-surface" for="mode-hotkey">
              Mode hotkey
            </label>
            <input
              id="mode-hotkey"
              class="min-h-[36px] bg-surface-container-high text-on-surface font-mono text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
              type="text"
              bind:value={selectedMode.hotkey}
              disabled={!loaded || saving}
              onblur={commit}
              onchange={commit}
            />
          </div>
          <div class="flex flex-col gap-1.5">
            <label class="font-body-default text-body-default text-on-surface" for="mode-description">
              Description
            </label>
            <input
              id="mode-description"
              class="min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
              type="text"
              bind:value={selectedMode.description}
              disabled={!loaded || saving}
              onblur={commit}
              onchange={commit}
            />
          </div>
        </div>

        <div class="flex items-center justify-between gap-4 pt-2 border-t border-outline-variant/30">
          <div class="flex flex-col min-w-0">
            <span class="font-body-default text-body-default text-on-surface">Cleanup prompts</span>
            <span class="font-body-muted text-body-muted text-on-surface-variant">
              Enable local AI cleanup and formatting for this mode.
            </span>
          </div>
          <button
            id="cleanup-toggle"
            aria-checked={selectedMode.cleanup}
            aria-label="Cleanup prompts"
            class="w-11 h-6 {selectedMode.cleanup ? 'bg-primary-container' : 'bg-surface-container-highest'} rounded-full relative p-0.5 transition-colors cursor-pointer flex items-center shrink-0 before:absolute before:-inset-2 before:content-[''] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
            role="switch"
            type="button"
            disabled={!loaded || saving}
            onclick={toggleCleanup}
          >
            <span
              class="w-5 h-5 {selectedMode.cleanup ? 'bg-on-primary-container translate-x-5' : 'bg-outline translate-x-0'} rounded-full shadow transition-transform flex items-center justify-center"
            >
              {#if selectedMode.cleanup}
                <span class="material-symbols-outlined text-[12px] text-primary" aria-hidden="true">check</span>
              {/if}
            </span>
          </button>
        </div>

        <div class="flex flex-col gap-1.5">
          <label class="font-body-default text-body-default text-on-surface" for="cleanup-prompt">
            Cleanup prompt
          </label>
          <textarea
            id="cleanup-prompt"
            rows="3"
            class="w-full min-h-[80px] bg-surface-container-high text-on-surface font-mono text-sm px-3 py-2 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed resize-y"
            bind:value={selectedMode.prompt}
            disabled={!loaded || saving || !selectedMode.cleanup}
            onblur={commit}
            onchange={commit}
            placeholder="Cleanup prompt instructions..."
          ></textarea>
        </div>

        <!-- Before and after preview -->
        <div class="flex flex-col gap-3 pt-3 border-t border-outline-variant/30">
          <div class="flex flex-col gap-1">
            <h4 class="font-headline-sm text-headline-sm font-semibold text-on-surface">
              Before and after
            </h4>
            <p class="font-body-muted text-body-muted text-on-surface-variant">
              Live sample speech transformation preview.
            </p>
          </div>
          <div class="flex flex-col gap-1.5">
            <label class="font-label-code text-label-code text-on-surface-variant text-xs uppercase tracking-wider" for="sample-input">
              Sample speech
            </label>
            <input
              id="sample-input"
              aria-label="Sample speech for preview"
              class="min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-sm px-3 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
              type="text"
              bind:value={sampleText}
              disabled={!loaded || saving}
            />
          </div>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <div class="bg-surface-container-high/60 rounded-lg p-3.5 flex flex-col gap-1.5 border border-outline-variant/20">
              <span class="font-label-code text-label-code text-on-surface-variant text-[11px] uppercase tracking-wider">
                Before
              </span>
              <p class="font-mono text-xs text-on-surface whitespace-pre-wrap leading-relaxed">
                {preview.before}
              </p>
            </div>
            <div class="bg-surface-container-high/60 rounded-lg p-3.5 flex flex-col gap-1.5 border border-primary/25 bg-primary/5" aria-live="polite">
              <span class="font-label-code text-label-code text-primary text-[11px] uppercase tracking-wider font-semibold">
                After
              </span>
              <p class="font-mono text-xs text-on-surface whitespace-pre-wrap leading-relaxed">
                {preview.after}
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</section>

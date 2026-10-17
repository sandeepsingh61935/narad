<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig, type AppProfile, ACTIVE_MODES } from "$lib/config";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";
  import {
    POPULAR_APP_SUGGESTIONS,
    validateProfile,
    type PopularAppSuggestion,
  } from "$lib/profiles";

  const modes = ACTIVE_MODES;

  let config = $state<DashboardConfig>(defaultConfig());
  let loaded = $state(false);
  let saving = $state(false);
  let activeSaves = 0;
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

  // Add profile card form state
  let newAppId = $state("");
  let newMode = $state<string>("plain");
  let newPasteWithShift = $state(false);

  let addValidation = $derived(
    validateProfile(
      {
        app_id: newAppId,
        mode: newMode,
        paste_with_shift: newPasteWithShift,
      },
      config.profiles
    )
  );

  let canAdd = $derived(addValidation.valid);

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
    activeSaves++;
    saving = true;
    saveChain = saveChain
      .then(
        () => persist(toSave),
        () => persist(toSave)
      )
      .finally(() => {
        activeSaves--;
        saving = activeSaves > 0;
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

  function handleAddProfile() {
    if (!loaded || saving || !canAdd) return;
    const profile: AppProfile = {
      app_id: newAppId.trim(),
      mode: newMode.trim() || "plain",
    };
    if (newPasteWithShift) {
      profile.paste_with_shift = true;
    }
    config.profiles = [...config.profiles, profile];
    newAppId = "";
    newMode = "plain";
    newPasteWithShift = false;
    commit();
  }

  function removeProfile(appId: string) {
    if (!loaded || saving) return;
    config.profiles = config.profiles.filter(
      (p) => p.app_id.trim().toLowerCase() !== appId.trim().toLowerCase()
    );
    commit();
  }

  function applySuggestion(suggestion: PopularAppSuggestion) {
    newAppId = suggestion.app_id;
    newMode = suggestion.mode;
    newPasteWithShift = suggestion.paste_with_shift;
  }

  onMount(() => {
    void load();
  });
</script>

<section class="flex-1 overflow-y-auto px-6 py-8 flex justify-center bg-surface min-w-0">
  <div class="w-full max-w-[680px] flex flex-col gap-6">
    <div class="flex flex-col gap-1">
      <h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">App profiles</h1>
      <p class="font-body-default text-body-default text-on-surface-variant">
        Configure default dictation mode and paste shortcut for specific applications.
      </p>
      <div class="min-h-[1.25rem]">
        {#if error}
          <p class="font-body-muted text-body-muted text-error" role="alert" aria-live="assertive">{error}</p>
        {:else if notice}
          <p class="font-body-muted text-body-muted text-on-surface" role="status" aria-live="polite">{notice}</p>
        {/if}
      </div>
    </div>

    <!-- Configured profiles list -->
    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Configured profiles
      </h2>
      {#if config.profiles.length === 0}
        <div class="bg-surface-container-low rounded-xl p-6 flex flex-col gap-2 border border-outline-variant/30 text-center items-center justify-center">
          <p class="font-body-default text-body-default text-on-surface-variant max-w-[480px]">
            No per-app profiles configured yet. Add an application profile below to automatically set the mode when focused.
          </p>
        </div>
      {:else}
        <div class="flex flex-col gap-3" role="list">
          {#each config.profiles as profile (profile.app_id)}
            <div
              class="bg-surface-container-low rounded-xl p-4 flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4 border border-outline-variant/30 shadow-sm"
              role="listitem"
            >
              <div class="flex items-center gap-3 min-w-0">
                <span class="font-mono text-sm font-semibold text-on-surface bg-surface-container-high px-3 py-1.5 rounded-lg border border-outline-variant/30 truncate">
                  {profile.app_id}
                </span>
              </div>
              <div class="flex items-center gap-3 sm:gap-4 flex-wrap w-full sm:w-auto justify-end">
                <div class="flex items-center gap-2">
                  <label class="sr-only" for="mode-{profile.app_id}">Default mode for {profile.app_id}</label>
                  <select
                    id="mode-{profile.app_id}"
                    class="bg-surface-container-high text-on-surface font-body-default text-sm pl-3 pr-8 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface disabled:opacity-50 disabled:cursor-not-allowed min-h-[36px]"
                    bind:value={profile.mode}
                    onchange={commit}
                    disabled={!loaded}
                  >
                    {#each modes as mode}
                      <option value={mode}>{mode}</option>
                    {/each}
                  </select>
                </div>
                <label class="flex items-center gap-2 text-sm text-on-surface cursor-pointer select-none min-h-[36px]">
                  <input
                    type="checkbox"
                    aria-label="Ctrl+Shift+V paste for {profile.app_id}"
                    class="w-4 h-4 rounded border-outline-variant bg-surface-container-high text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface disabled:opacity-50"
                    checked={profile.paste_with_shift === true}
                    onchange={(e) => {
                      profile.paste_with_shift = (e.currentTarget as HTMLInputElement).checked;
                      commit();
                    }}
                    disabled={!loaded}
                  />
                  <span>Ctrl+Shift+V paste</span>
                </label>
                <button
                  type="button"
                  class="min-h-[36px] min-w-[36px] px-3 py-1.5 rounded-lg text-sm font-medium text-error hover:bg-error/10 border border-error/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-error focus-visible:ring-offset-2 focus-visible:ring-offset-surface disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
                  aria-label="Remove profile for {profile.app_id}"
                  onclick={() => removeProfile(profile.app_id)}
                  disabled={!loaded || saving}
                >
                  Remove
                </button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Add application profile card -->
    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Add application profile
      </h2>
      <div class="bg-surface-container-low rounded-xl p-5 flex flex-col gap-4 border border-outline-variant/30 shadow-sm">
        <!-- Quick-add suggestion chips -->
        <div class="flex flex-col gap-1.5">
          <span class="font-body-muted text-xs text-on-surface-variant">Popular applications:</span>
          <div class="flex items-center gap-2 flex-wrap">
            {#each POPULAR_APP_SUGGESTIONS as suggestion}
              <button
                type="button"
                aria-label="Use suggestion for {suggestion.name}"
                class="min-h-[36px] px-3 py-1.5 rounded-lg text-xs font-medium bg-surface-container-high hover:bg-surface-container text-on-surface border border-outline-variant/30 transition-colors cursor-pointer disabled:opacity-50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface"
                onclick={() => applySuggestion(suggestion)}
                disabled={!loaded}
              >
                + {suggestion.name}
              </button>
            {/each}
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div class="flex flex-col gap-1.5">
            <label class="font-body-default text-body-default text-on-surface font-medium" for="new-app-id">
              Application ID
            </label>
            <input
              id="new-app-id"
              class="min-h-[36px] bg-surface-container-high text-on-surface font-mono text-sm px-3 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface disabled:opacity-50 disabled:cursor-not-allowed placeholder:font-sans placeholder:text-on-surface-variant/60"
              type="text"
              placeholder="e.g. code, slack, org.gnome.Terminal"
              bind:value={newAppId}
              disabled={!loaded}
              aria-invalid={newAppId.trim().length > 0 && !addValidation.valid}
              aria-describedby={newAppId.trim().length > 0 && !addValidation.valid && addValidation.error ? "app-id-error" : undefined}
            />
            {#if newAppId.trim().length > 0 && !addValidation.valid && addValidation.error}
              <span id="app-id-error" class="font-body-muted text-xs text-error" role="alert">{addValidation.error}</span>
            {/if}
          </div>

          <div class="flex flex-col gap-1.5">
            <label class="font-body-default text-body-default text-on-surface font-medium" for="new-mode">
              Default mode
            </label>
            <select
              id="new-mode"
              class="min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-sm pl-3 pr-8 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface disabled:opacity-50 disabled:cursor-not-allowed"
              bind:value={newMode}
              disabled={!loaded}
            >
              {#each modes as mode}
                <option value={mode}>{mode}</option>
              {/each}
            </select>
          </div>
        </div>

        <div class="flex items-center justify-between gap-4 pt-2 border-t border-outline-variant/30 flex-wrap">
          <label for="new-paste-with-shift" class="flex items-center gap-2 text-sm text-on-surface cursor-pointer select-none min-h-[36px]">
            <input
              type="checkbox"
              id="new-paste-with-shift"
              class="w-4 h-4 rounded border-outline-variant bg-surface-container-high text-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface disabled:opacity-50"
              bind:checked={newPasteWithShift}
              disabled={!loaded}
            />
            <span>Ctrl+Shift+V paste</span>
          </label>

          <button
            type="button"
            class="min-h-[36px] px-4 py-2 rounded-lg text-sm font-medium bg-primary text-on-primary hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface disabled:opacity-50 disabled:cursor-not-allowed transition-colors cursor-pointer"
            disabled={!loaded || saving || !canAdd}
            onclick={handleAddProfile}
          >
            Add profile
          </button>
        </div>
      </div>
    </div>
  </div>
</section>

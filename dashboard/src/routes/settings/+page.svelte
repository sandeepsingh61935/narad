<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    ACTIVE_MODES,
    defaultConfig,
    type DashboardConfig,
  } from "$lib/config";
  import { prepareConfigForSave } from "$lib/prepare-config";

  let config = $state<DashboardConfig>(defaultConfig());
  let loading = $state(true);
  let error = $state<string | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

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

  function onSubmit(event: Event) {
    event.preventDefault();
    const active = document.activeElement;
    if (active instanceof HTMLElement) active.blur();
  }

  onMount(() => {
    void load();
  });
</script>

<div class="page">
  <header class="header">
    <h1>Settings</h1>
  </header>

  <main class="main">
    {#if loading}
      <p class="status">Loading…</p>
    {:else}
      <form class="form" onsubmit={onSubmit}>
        <fieldset class="group">
          <legend>Activation</legend>

          <label class="field" for="dictate-hotkey">
            <span class="label">Dictate hotkey</span>
            <input
              id="dictate-hotkey"
              type="text"
              bind:value={config.activation.dictate_hotkey}
              autocomplete="off"
              spellcheck="false"
              onblur={commit}
            />
          </label>

          <label class="field" for="hold-tap">
            <span class="label">Hold/tap threshold (ms)</span>
            <input
              id="hold-tap"
              type="number"
              min="0"
              step="1"
              bind:value={config.activation.hold_tap_threshold_ms}
              onblur={commit}
            />
          </label>

          <p class="note" role="note">
            Restart naradd / systemd for hotkey changes to apply.
          </p>
        </fieldset>

        <fieldset class="group">
          <legend>Audio</legend>

          <label class="field check" for="sounds-enabled">
            <input
              id="sounds-enabled"
              type="checkbox"
              bind:checked={config.audio.sounds_enabled}
              onchange={(event) => {
                const target = event.currentTarget;
                if (target instanceof HTMLInputElement) {
                  config.audio.sounds_enabled = target.checked;
                }
                commit();
              }}
            />
            <span class="label">Sound cues enabled</span>
          </label>
        </fieldset>

        <fieldset class="group">
          <legend>General</legend>

          <label class="field" for="active-mode">
            <span class="label">Active mode</span>
            <select
              id="active-mode"
              bind:value={config.general.active_mode}
              onchange={(event) => {
                const target = event.currentTarget;
                if (target instanceof HTMLSelectElement) {
                  config.general.active_mode = target.value;
                }
                commit();
              }}
            >
              {#each ACTIVE_MODES as mode}
                <option value={mode}>{mode}</option>
              {/each}
              {#if !ACTIVE_MODES.includes(
                config.general.active_mode as (typeof ACTIVE_MODES)[number],
              )}
                <option value={config.general.active_mode}>
                  {config.general.active_mode}
                </option>
              {/if}
            </select>
          </label>
        </fieldset>

        <fieldset class="group">
          <legend>Models</legend>

          <label class="field" for="asr-english">
            <span class="label">ASR English pack</span>
            <input
              id="asr-english"
              type="text"
              class="mono"
              bind:value={config.models.asr_english}
              autocomplete="off"
              spellcheck="false"
              onblur={commit}
            />
          </label>
        </fieldset>

        <div class="actions">
          <button type="button" class="ghost" onclick={() => load()}>
            Reload
          </button>
        </div>

        {#if error}
          <p class="feedback error" role="alert">{error}</p>
        {/if}
      </form>
    {/if}
  </main>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    min-height: 100%;
  }

  .header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-md);
    padding: var(--space-lg) var(--space-xl);
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  h1 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    line-height: 22px;
    letter-spacing: -0.01em;
  }

  .main {
    flex: 1;
    padding: var(--space-lg) var(--space-xl) var(--space-xl);
    max-width: 36rem;
  }

  .status {
    margin: var(--space-xl) 0;
    color: var(--text-secondary);
  }

  .form {
    display: flex;
    flex-direction: column;
    gap: var(--space-lg);
  }

  .group {
    margin: 0;
    padding: var(--space-lg);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .group legend {
    padding: 0 var(--space-xs);
    font-size: 13px;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-xs);
    margin-top: var(--space-md);
  }

  .field:first-of-type {
    margin-top: var(--space-sm);
  }

  .field.check {
    flex-direction: row;
    align-items: center;
    gap: var(--space-sm);
  }

  .label {
    font-size: 13px;
    color: var(--text);
  }

  .field input[type="text"],
  .field input[type="number"],
  .field select {
    padding: 0.45rem 0.7rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
  }

  .field input.mono {
    font-family: var(--font-mono);
    font-size: 13px;
  }

  .note {
    margin: var(--space-md) 0 0;
    font-size: 12px;
    line-height: 18px;
    color: var(--text-secondary);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-sm);
  }

  .actions button {
    padding: 0.45rem 0.85rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface-elevated);
    cursor: pointer;
  }

  .actions button:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .ghost {
    background: transparent !important;
    border-color: transparent !important;
    color: var(--text-secondary);
  }

  .feedback.error {
    margin: 0;
    font-size: 13px;
    color: var(--destructive);
  }
</style>

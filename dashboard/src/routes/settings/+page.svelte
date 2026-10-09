<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import {
    ACTIVE_MODES,
    defaultConfig,
    type DashboardConfig,
  } from "$lib/config";

  let config = $state<DashboardConfig>(defaultConfig());
  let loading = $state(true);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let saveMessage = $state<string | null>(null);
  let dirty = $state(false);

  async function load() {
    loading = true;
    error = null;
    saveMessage = null;
    try {
      config = await invoke<DashboardConfig>("load_config");
      dirty = false;
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      config = defaultConfig();
    } finally {
      loading = false;
    }
  }

  async function save() {
    saving = true;
    error = null;
    saveMessage = null;
    try {
      const threshold = Number(config.activation.hold_tap_threshold_ms);
      if (!Number.isFinite(threshold) || threshold < 0) {
        throw new Error("Hold/tap threshold must be a non-negative number.");
      }
      config.activation.hold_tap_threshold_ms = Math.round(threshold);
      config.activation.dictate_hotkey = config.activation.dictate_hotkey.trim();
      config.general.active_mode = config.general.active_mode.trim() || "plain";
      config.models.asr_english = config.models.asr_english.trim();
      await invoke("save_config", { config });
      dirty = false;
      saveMessage = "Saved";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      saving = false;
    }
  }

  function markDirty() {
    dirty = true;
    saveMessage = null;
  }

  function onSubmit(event: Event) {
    event.preventDefault();
    void save();
  }

  onMount(() => {
    void load();
  });
</script>

<div class="page">
  <header class="header">
    <h1>Settings</h1>
    {#if dirty}
      <span class="dirty" role="status">Unsaved changes</span>
    {/if}
  </header>

  <main class="main">
    {#if loading}
      <p class="status">Loading…</p>
    {:else}
      <form class="form" onsubmit={onSubmit}>
        <fieldset class="group" oninput={markDirty}>
          <legend>Activation</legend>

          <label class="field" for="dictate-hotkey">
            <span class="label">Dictate hotkey</span>
            <input
              id="dictate-hotkey"
              type="text"
              bind:value={config.activation.dictate_hotkey}
              autocomplete="off"
              spellcheck="false"
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
            />
          </label>

          <p class="note" role="note">
            Restart naradd / systemd for hotkey changes to apply.
          </p>
        </fieldset>

        <fieldset class="group" oninput={markDirty}>
          <legend>Audio</legend>

          <label class="field check" for="sounds-enabled">
            <input
              id="sounds-enabled"
              type="checkbox"
              bind:checked={config.audio.sounds_enabled}
              onchange={markDirty}
            />
            <span class="label">Sound cues enabled</span>
          </label>
        </fieldset>

        <fieldset class="group" oninput={markDirty}>
          <legend>General</legend>

          <label class="field" for="active-mode">
            <span class="label">Active mode</span>
            <select
              id="active-mode"
              bind:value={config.general.active_mode}
              onchange={markDirty}
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

        <fieldset class="group" oninput={markDirty}>
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
            />
          </label>
        </fieldset>

        <div class="actions">
          <button type="submit" disabled={saving || !dirty}>
            {saving ? "Saving…" : "Save"}
          </button>
          <button
            type="button"
            class="ghost"
            disabled={saving || loading}
            onclick={() => load()}
          >
            Reload
          </button>
        </div>

        {#if error}
          <p class="feedback error" role="alert">{error}</p>
        {:else if saveMessage}
          <p class="feedback" role="status">{saveMessage}</p>
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

  .dirty {
    font-size: 12px;
    color: var(--accent);
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

  .actions button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  .actions button[type="submit"]:not(:disabled) {
    background: var(--accent);
    border-color: var(--accent);
    color: #1c1b1a;
    font-weight: 600;
  }

  .ghost {
    background: transparent !important;
    border-color: transparent !important;
    color: var(--text-secondary);
  }

  .feedback {
    margin: 0;
    font-size: 13px;
    color: var(--success);
  }

  .feedback.error {
    color: var(--destructive);
  }
</style>

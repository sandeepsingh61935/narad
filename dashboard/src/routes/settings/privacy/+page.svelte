<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig } from "$lib/config";
  import {
    formatRetentionDays,
    isDeleteAllConfirmed,
    validatePrivacyDraft,
  } from "$lib/privacy-settings";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";

  let config = $state<DashboardConfig>(defaultConfig());
  let textDays = $state("");
  let audioDays = $state("");
  let paused = $state<boolean | null>(null);
  let pauseBusy = $state(false);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

  let showClearDialog = $state(false);
  let clearConfirm = $state("");
  let clearBusy = $state(false);

  function showDays(next: DashboardConfig) {
    textDays = formatRetentionDays(next.history.text_retention_days);
    audioDays = formatRetentionDays(next.history.audio_retention_days);
  }

  async function load() {
    error = null;
    try {
      config = await invoke<DashboardConfig>("load_config");
      showDays(config);
      loaded = true;
    } catch (caught) {
      loaded = false;
      error = caught instanceof Error ? caught.message : String(caught);
    }
    try {
      const status = await invoke<{ paused: boolean }>("session_status");
      paused = status.paused;
    } catch (caught) {
      paused = null;
      const message = caught instanceof Error ? caught.message : String(caught);
      error = error == null ? message : `${error} ${message}`;
    }
  }

  function commit() {
    if (!loaded) return;
    const validated = validatePrivacyDraft({
      textDaysRaw: textDays,
      audioDaysRaw: audioDays,
    });
    if (!validated.ok) {
      error = validated.error;
      notice = null;
      return;
    }
    const toSave: DashboardConfig = {
      ...config,
      history: {
        retain_audio: config.history.retain_audio,
        text_retention_days: validated.textRetentionDays,
        audio_retention_days: validated.audioRetentionDays,
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
    showDays(result.config);
    notice = SAVED_NOTICE;
    error = null;
  }

  async function setPaused(next: boolean) {
    if (pauseBusy || paused == null) return;
    const previous = paused;
    paused = next;
    pauseBusy = true;
    try {
      await invoke("set_paused", { paused: next });
    } catch (caught) {
      paused = previous;
      error = caught instanceof Error ? caught.message : String(caught);
    } finally {
      pauseBusy = false;
    }
  }

  async function handleDeleteAll() {
    if (!isDeleteAllConfirmed(clearConfirm) || clearBusy) return;
    clearBusy = true;
    error = null;
    try {
      await invoke("delete_all_history");
      showClearDialog = false;
      clearConfirm = "";
      notice = "All history cleared.";
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
    } finally {
      clearBusy = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape" && showClearDialog && !clearBusy) {
      showClearDialog = false;
      clearConfirm = "";
    }
  }}
/>

<section class="flex-1 overflow-y-auto px-6 py-8 flex justify-center bg-surface min-w-0">
  <div class="w-full max-w-[680px] flex flex-col gap-6">
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
          <h1 class="font-headline-lg text-headline-lg text-on-surface tracking-tight">
            Everything stays on this computer.
          </h1>
          <p class="font-body-default text-body-default text-on-surface-variant leading-relaxed">
            Narad processes all speech locally on your device. Audio never leaves this computer, and models run without network access.
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
          No network access
        </div>
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-container-high text-on-surface font-label-code text-label-code"
        >
          <span class="material-symbols-outlined text-[13px] text-tertiary">folder</span>
          Standard local storage
        </div>
      </div>
    </div>

    {#if error}
      <p class="font-body-muted text-body-muted text-error" role="alert">{error}</p>
    {/if}
    {#if notice}
      <p class="font-body-muted text-body-muted text-on-surface" role="status">{notice}</p>
    {/if}

    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Capture & Storage
      </h2>
      <div class="bg-surface-container-low rounded-xl overflow-hidden shadow-sm flex flex-col border border-outline-variant/30">
        <div class="p-4 flex items-center justify-between gap-4">
          <div class="flex flex-col min-w-0">
            <span class="font-body-default text-body-default font-medium text-on-surface">
              Keep audio recordings
            </span>
            <span class="font-body-muted text-body-muted text-on-surface-variant">
              Store WAV audio clips alongside transcripts. Off discards audio after transcription.
            </span>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={config.history.retain_audio}
            aria-label="Keep audio recordings"
            disabled={!loaded}
            onclick={() => {
              config.history.retain_audio = !config.history.retain_audio;
              commit();
            }}
            class="w-11 h-6 rounded-full transition-colors relative flex items-center p-0.5 shrink-0 cursor-pointer border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed {config.history.retain_audio ? 'bg-primary' : 'bg-surface-container-highest'}"
          >
            <span
              class="w-5 h-5 rounded-full bg-surface-bright shadow-sm transform transition-transform {config.history.retain_audio ? 'translate-x-5' : 'translate-x-0'}"
            ></span>
          </button>
        </div>
        <div class="h-px bg-surface-container-high w-full"></div>
        <div class="p-4 flex items-center justify-between gap-4">
          <div class="flex flex-col min-w-0">
            <span class="font-body-default text-body-default font-medium text-on-surface">
              Pause history
            </span>
            <span class="font-body-muted text-body-muted text-on-surface-variant">
              Dictation still pastes, but no history row is stored. This applies immediately and is
              not saved in the config file.
            </span>
          </div>
          <button
            type="button"
            role="switch"
            aria-checked={paused === true}
            aria-label="Pause history"
            disabled={paused == null || pauseBusy}
            onclick={() => void setPaused(!paused)}
            class="w-11 h-6 rounded-full transition-colors relative flex items-center p-0.5 shrink-0 cursor-pointer border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed {paused ? 'bg-primary' : 'bg-surface-container-highest'}"
          >
            <span
              class="w-5 h-5 rounded-full bg-surface-bright shadow-sm transform transition-transform {paused ? 'translate-x-5' : 'translate-x-0'}"
            ></span>
          </button>
        </div>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Automatic Retention
      </h2>
      <div class="bg-surface-container-low rounded-xl overflow-hidden shadow-sm flex flex-col border border-outline-variant/30">
        <div class="p-4 flex items-center justify-between gap-4">
          <div class="flex flex-col min-w-0">
            <label class="font-body-default text-body-default font-medium text-on-surface" for="text-retention-days">
              Auto-delete text transcripts
            </label>
            <span class="font-body-muted text-body-muted text-on-surface-variant">
              Whole days before text rows are deleted. Blank keeps text. 0 is kept as 0.
            </span>
          </div>
          <input
            id="text-retention-days"
            class="w-24 min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
            type="text"
            inputmode="numeric"
            placeholder="keep"
            autocomplete="off"
            bind:value={textDays}
            disabled={!loaded}
            onblur={commit}
          />
        </div>
        <div class="h-px bg-surface-container-high w-full"></div>
        <div class="p-4 flex items-center justify-between gap-4">
          <div class="flex flex-col min-w-0">
            <label class="font-body-default text-body-default font-medium text-on-surface" for="audio-retention-days">
              Auto-delete retained audio
            </label>
            <span class="font-body-muted text-body-muted text-on-surface-variant">
              Whole days before WAV files are removed. Blank keeps audio.
            </span>
          </div>
          <input
            id="audio-retention-days"
            class="w-24 min-h-[36px] bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
            type="text"
            inputmode="numeric"
            placeholder="keep"
            autocomplete="off"
            bind:value={audioDays}
            disabled={!loaded}
            onblur={commit}
          />
        </div>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Clear history
      </h2>
      <div class="bg-surface-container-low rounded-xl overflow-hidden shadow-sm p-4 flex items-center justify-between gap-4 border border-outline-variant/30">
        <div class="flex flex-col min-w-0">
          <span class="font-body-default text-body-default font-medium text-on-surface">
            Permanently delete all records
          </span>
          <span class="font-body-muted text-body-muted text-on-surface-variant">
            Deletes all transcription text entries and retained audio files from disk.
          </span>
        </div>
        <button
          type="button"
          disabled={!loaded}
          class="min-h-[36px] px-3.5 py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-error font-body-muted text-body-muted transition-colors flex items-center gap-1.5 shrink-0 border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-error focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
          onclick={() => {
            clearConfirm = "";
            showClearDialog = true;
          }}
        >
          <span class="material-symbols-outlined text-[16px]">delete_outline</span>
          <span>Clear history…</span>
        </button>
      </div>
    </div>
  </div>
</section>

{#if showClearDialog}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-sm"
    role="presentation"
    onclick={(event) => {
      if (!clearBusy && event.target === event.currentTarget) {
        showClearDialog = false;
        clearConfirm = "";
      }
    }}
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-labelledby="clear-dialog-title"
      class="w-full max-w-md bg-surface-container-low border border-outline-variant/30 rounded-xl p-6 shadow-xl flex flex-col gap-4 text-on-surface"
    >
      <div class="flex flex-col gap-1.5">
        <h3
          id="clear-dialog-title"
          class="font-headline-sm text-headline-sm font-semibold text-on-surface"
        >
          Clear all history
        </h3>
        <p class="font-body-default text-body-default text-on-surface-variant leading-relaxed">
          This will permanently delete all transcription entries and stored audio files. To confirm, type delete all history below.
        </p>
      </div>

      <div class="flex flex-col gap-2">
        <input
          type="text"
          autocomplete="off"
          spellcheck="false"
          placeholder="delete all history"
          bind:value={clearConfirm}
          disabled={clearBusy}
          onkeydown={(event) => {
            if (event.key === "Enter" && isDeleteAllConfirmed(clearConfirm) && !clearBusy) {
              event.preventDefault();
              void handleDeleteAll();
            }
          }}
          class="w-full min-h-[36px] px-3 py-2 rounded-lg bg-surface-container-high text-on-surface border border-outline-variant/30 font-body-default text-body-default placeholder:text-on-surface-variant/50 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
        />
      </div>

      <div class="flex items-center justify-end gap-3 pt-2">
        <button
          type="button"
          disabled={clearBusy}
          class="min-h-[36px] px-4 py-2 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-default text-body-default transition-colors border border-outline-variant/30 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container disabled:opacity-50 disabled:cursor-not-allowed"
          onclick={() => {
            if (!clearBusy) {
              showClearDialog = false;
              clearConfirm = "";
            }
          }}
        >
          Cancel
        </button>
        <button
          type="button"
          disabled={!isDeleteAllConfirmed(clearConfirm) || clearBusy}
          class="min-h-[36px] px-4 py-2 rounded-lg bg-error hover:bg-error/90 text-on-error font-body-default text-body-default transition-colors disabled:opacity-40 disabled:cursor-not-allowed flex items-center gap-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-error focus-visible:ring-offset-1 focus-visible:ring-offset-surface-container"
          onclick={handleDeleteAll}
        >
          {#if clearBusy}
            <span class="material-symbols-outlined text-[16px] animate-spin">progress_activity</span>
          {/if}
          <span>Delete all history</span>
        </button>
      </div>
    </div>
  </div>
{/if}

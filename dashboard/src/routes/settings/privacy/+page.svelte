<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig } from "$lib/config";
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

  function dayField(value: number | null): string {
    return value == null ? "" : String(value);
  }

  function parseDays(raw: string): number | null | undefined {
    const trimmed = raw.trim();
    if (trimmed === "") return null;
    if (!/^\d+$/.test(trimmed)) return undefined;
    return Number(trimmed);
  }

  function showDays(next: DashboardConfig) {
    textDays = dayField(next.history.text_retention_days);
    audioDays = dayField(next.history.audio_retention_days);
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
    const text = parseDays(textDays);
    const audio = parseDays(audioDays);
    if (text === undefined || audio === undefined) {
      error = "Retention must be a whole number of days, or blank to keep.";
      notice = null;
      return;
    }
    const toSave: DashboardConfig = {
      ...config,
      history: {
        retain_audio: config.history.retain_audio,
        text_retention_days: text,
        audio_retention_days: audio,
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

  onMount(() => {
    void load();
  });
</script>

<section class="flex-1 overflow-y-auto px-6 py-8 flex justify-center bg-surface min-w-0">
  <div class="w-full max-w-[680px] flex flex-col gap-6">
    <div
      class="bg-surface-container-low rounded-xl p-6 flex flex-col gap-4 shadow-sm relative overflow-hidden"
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
            Narad processes all audio streams strictly in local device RAM using on-device acoustic
            models. Zero telemetry, zero cloud backends, zero network egress.
          </p>
        </div>
      </div>
      <div class="flex flex-wrap items-center gap-2 pt-2">
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-container-high text-on-surface font-label-code text-label-code"
        >
          <span class="w-1.5 h-1.5 rounded-full bg-emerald-400"></span>
          Ethernet & Wi-Fi sockets unbound
        </div>
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-container-high text-on-surface font-label-code text-label-code"
        >
          <span class="material-symbols-outlined text-[13px] text-primary">lock</span>
          Sandboxed local daemon
        </div>
        <div
          class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface-container-high text-on-surface font-label-code text-label-code"
        >
          <span class="material-symbols-outlined text-[13px] text-tertiary">memory</span>
          RAM-only buffer scratchpad
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
      <div class="bg-surface-container-low rounded-xl overflow-hidden shadow-sm flex flex-col">
        <div class="p-4 flex items-center justify-between gap-4">
          <div class="flex flex-col min-w-0">
            <span class="font-body-default text-body-default font-medium text-on-surface">
              Keep audio recordings
            </span>
            <span class="font-body-muted text-body-muted text-on-surface-variant">
              Store WAV audio clips alongside transcripts. Off discards audio after transcription.
            </span>
          </div>
          <input
            id="toggle-keep-audio"
            aria-label="Keep audio recordings"
            type="checkbox"
            bind:checked={config.history.retain_audio}
            disabled={!loaded}
            onchange={commit}
          />
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
          <input
            id="toggle-pause-history"
            aria-label="Pause history"
            type="checkbox"
            checked={paused === true}
            disabled={paused == null || pauseBusy}
            onchange={(event) => {
              const target = event.currentTarget;
              if (target instanceof HTMLInputElement) void setPaused(target.checked);
            }}
          />
        </div>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Automatic Retention
      </h2>
      <div class="bg-surface-container-low rounded-xl overflow-hidden shadow-sm flex flex-col">
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
            class="w-24 bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg"
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
            class="w-24 bg-surface-container-high text-on-surface font-body-default text-body-default px-3 py-1.5 rounded-lg"
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
      <div class="flex items-center justify-between px-2">
        <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider">
          Local Storage Breakdown
        </h2>
        <span class="font-label-code text-label-code text-on-surface">Total: 2.45 GB</span>
      </div>
      <div class="bg-surface-container-low rounded-xl p-5 shadow-sm flex flex-col gap-4">
        <div class="flex flex-col gap-1.5">
          <div class="h-3 w-full bg-surface-container-highest rounded-full overflow-hidden flex gap-0.5">
            <div class="h-full bg-primary-container rounded-l-full" style="width: 4%;" title="Transcripts: 12 MB"></div>
            <div class="h-full bg-tertiary-container" style="width: 14%;" title="Cached audio: 340 MB"></div>
            <div class="h-full bg-outline rounded-r-full" style="width: 82%;" title="AI Models: 2.1 GB"></div>
          </div>
          <div
            class="flex items-center justify-between text-on-surface-variant font-body-muted text-body-muted pt-1"
          >
            <span class="flex items-center gap-1 font-label-code text-label-code">
              <span class="material-symbols-outlined text-[14px]">hard_drive</span>
              /home mount
            </span>
            <span class="font-label-code text-label-code text-on-surface">482 GB available</span>
          </div>
        </div>
        <div class="flex flex-col gap-2 pt-2">
          <div
            class="flex items-center justify-between p-3 rounded-lg bg-surface-container hover:bg-surface-container-high transition-colors"
          >
            <div class="flex items-center gap-3">
              <span class="w-3 h-3 rounded-sm bg-primary-container shrink-0"></span>
              <div class="flex flex-col">
                <span class="font-body-default text-body-default font-medium text-on-surface"
                  >Transcription History</span
                >
                <span
                  class="font-body-muted text-body-muted text-on-surface-variant font-label-code text-label-code"
                  >12 MB · 1,936 entries</span
                >
              </div>
            </div>
            <button
              class="px-3 py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted transition-colors flex items-center gap-1.5"
              type="button"
            >
              <span class="material-symbols-outlined text-[16px] text-error">delete_outline</span>
              <span>Clear history…</span>
            </button>
          </div>
          <div
            class="flex items-center justify-between p-3 rounded-lg bg-surface-container hover:bg-surface-container-high transition-colors"
          >
            <div class="flex items-center gap-3">
              <span class="w-3 h-3 rounded-sm bg-tertiary-container shrink-0"></span>
              <div class="flex flex-col">
                <span class="font-body-default text-body-default font-medium text-on-surface"
                  >Cached audio</span
                >
                <span
                  class="font-body-muted text-body-muted text-on-surface-variant font-label-code text-label-code"
                  >340 MB · 42 recordings</span
                >
              </div>
            </div>
            <button
              class="px-3 py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-muted text-body-muted transition-colors flex items-center gap-1.5"
              type="button"
            >
              <span class="material-symbols-outlined text-[16px] text-on-surface-variant"
                >cleaning_services</span
              >
              <span>Purge audio…</span>
            </button>
          </div>
          <div
            class="flex items-center justify-between p-3 rounded-lg bg-surface-container hover:bg-surface-container-high transition-colors"
          >
            <div class="flex items-center gap-3">
              <span class="w-3 h-3 rounded-sm bg-outline shrink-0"></span>
              <div class="flex flex-col">
                <span class="font-body-default text-body-default font-medium text-on-surface"
                  >Downloaded Models</span
                >
                <span
                  class="font-body-muted text-body-muted text-on-surface-variant font-label-code text-label-code"
                  >2.1 GB · 3 models installed</span
                >
              </div>
            </div>
            <a
              class="px-3 py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-primary font-body-muted text-body-muted transition-colors flex items-center gap-1"
              href="#manage-models"
              onclick={(event) => event.preventDefault()}
            >
              <span>Manage models</span>
              <span class="material-symbols-outlined text-[16px]">arrow_forward</span>
            </a>
          </div>
        </div>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-between px-2">
        <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider">
          Data Portability
        </h2>
      </div>
      <div class="bg-surface-container-low rounded-xl p-5 shadow-sm flex flex-col gap-4">
        <div class="flex flex-col gap-1">
          <span class="font-body-default text-body-default text-on-surface font-medium">Export History</span>
          <p class="font-body-muted text-body-muted text-on-surface-variant">
            Export your complete transcription history and metadata to local open formats.
          </p>
        </div>
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <button
            class="flex flex-col items-start gap-2 p-3.5 rounded-lg bg-surface-container hover:bg-surface-container-high transition-colors text-left group"
            type="button"
          >
            <div class="flex items-center justify-between w-full">
              <span class="material-symbols-outlined text-[20px] text-primary">description</span>
              <span class="font-keycap text-keycap px-1.5 py-0.5 rounded bg-surface-container-highest text-on-surface"
                >.MD</span
              >
            </div>
            <div>
              <div
                class="font-body-default text-body-default font-medium text-on-surface group-hover:text-primary transition-colors"
              >
                Markdown
              </div>
              <div class="font-body-muted text-body-muted text-on-surface-variant text-[12px] leading-tight mt-0.5">
                Timestamps, apps, and formatted tags
              </div>
            </div>
          </button>
          <button
            class="flex flex-col items-start gap-2 p-3.5 rounded-lg bg-surface-container hover:bg-surface-container-high transition-colors text-left group"
            type="button"
          >
            <div class="flex items-center justify-between w-full">
              <span class="material-symbols-outlined text-[20px] text-tertiary">data_object</span>
              <span class="font-keycap text-keycap px-1.5 py-0.5 rounded bg-surface-container-highest text-on-surface"
                >.JSON</span
              >
            </div>
            <div>
              <div
                class="font-body-default text-body-default font-medium text-on-surface group-hover:text-tertiary transition-colors"
              >
                Raw JSON
              </div>
              <div class="font-body-muted text-body-muted text-on-surface-variant text-[12px] leading-tight mt-0.5">
                Full acoustic metadata & confidence
              </div>
            </div>
          </button>
          <button
            class="flex flex-col items-start gap-2 p-3.5 rounded-lg bg-surface-container hover:bg-surface-container-high transition-colors text-left group"
            type="button"
          >
            <div class="flex items-center justify-between w-full">
              <span class="material-symbols-outlined text-[20px] text-on-surface-variant">text_snippet</span>
              <span class="font-keycap text-keycap px-1.5 py-0.5 rounded bg-surface-container-highest text-on-surface"
                >.TXT</span
              >
            </div>
            <div>
              <div
                class="font-body-default text-body-default font-medium text-on-surface group-hover:text-on-surface transition-colors"
              >
                Plain Text
              </div>
              <div class="font-body-muted text-body-muted text-on-surface-variant text-[12px] leading-tight mt-0.5">
                Clean sequential dictation stream
              </div>
            </div>
          </button>
        </div>
        <div
          class="flex flex-wrap items-center justify-between gap-2 pt-2 text-on-surface-variant font-label-code text-label-code text-[11px]"
        >
          <span class="flex items-center gap-1.5">
            <span class="material-symbols-outlined text-[14px]">folder_open</span>
            ~/.local/share/narad/
          </span>
          <span>Last exported: Never</span>
        </div>
      </div>
    </div>
  </div>
</section>

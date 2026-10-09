<script lang="ts">
  let keepAudio = $state(false);
  let pauseHistory = $state(false);
  let textRetention = $state("30d");
  let audioRetention = $state("7d");
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

    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Capture & Storage
      </h2>
      <div class="bg-surface-container-low rounded-xl overflow-hidden shadow-sm flex flex-col">
        <div class="p-4 flex items-center justify-between gap-4 hover:bg-surface-container transition-colors">
          <div class="flex items-center gap-3.5 min-w-0">
            <span class="material-symbols-outlined text-[20px] text-on-surface-variant shrink-0"
              >mic_none</span
            >
            <div class="flex flex-col min-w-0">
              <span class="font-body-default text-body-default font-medium text-on-surface truncate"
                >Keep audio recordings</span
              >
              <span class="font-body-muted text-body-muted text-on-surface-variant"
                >Store raw FLAC audio clips alongside transcripts. Disabling discards audio immediately
                after transcription.</span
              >
            </div>
          </div>
          <label class="relative inline-flex items-center cursor-pointer shrink-0 ml-3">
            <input class="sr-only peer" id="toggle-keep-audio" type="checkbox" bind:checked={keepAudio} />
            <div
              class="w-11 h-6 bg-surface-container-highest peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-surface peer-checked:bg-primary-container after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-on-background after:rounded-full after:h-5 after:w-5 after:transition-all"
            ></div>
          </label>
        </div>
        <div class="h-px bg-surface-container-high w-full"></div>
        <div class="p-4 flex items-center justify-between gap-4 hover:bg-surface-container transition-colors">
          <div class="flex items-center gap-3.5 min-w-0">
            <span class="material-symbols-outlined text-[20px] text-on-surface-variant shrink-0"
              >history_toggle_off</span
            >
            <div class="flex flex-col min-w-0">
              <span class="font-body-default text-body-default font-medium text-on-surface truncate"
                >Pause history</span
              >
              <span class="font-body-muted text-body-muted text-on-surface-variant"
                >Perform real-time dictation and paste into target apps without storing any record in
                History.</span
              >
            </div>
          </div>
          <label class="relative inline-flex items-center cursor-pointer shrink-0 ml-3">
            <input
              class="sr-only peer"
              id="toggle-pause-history"
              type="checkbox"
              bind:checked={pauseHistory}
            />
            <div
              class="w-11 h-6 bg-surface-container-highest peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-surface peer-checked:bg-primary-container after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-on-background after:rounded-full after:h-5 after:w-5 after:transition-all"
            ></div>
          </label>
        </div>
      </div>
    </div>

    <div class="flex flex-col gap-2">
      <h2 class="font-label-code text-label-code text-on-surface-variant uppercase tracking-wider px-2">
        Automatic Retention
      </h2>
      <div class="bg-surface-container-low rounded-xl overflow-hidden shadow-sm flex flex-col">
        <div class="p-4 flex items-center justify-between gap-4 hover:bg-surface-container transition-colors">
          <div class="flex items-center gap-3.5 min-w-0">
            <span class="material-symbols-outlined text-[20px] text-on-surface-variant shrink-0"
              >auto_delete</span
            >
            <div class="flex flex-col min-w-0">
              <span class="font-body-default text-body-default font-medium text-on-surface truncate"
                >Auto-delete text transcripts</span
              >
              <span class="font-body-muted text-body-muted text-on-surface-variant"
                >Prune transcription text entries older than the selected retention period.</span
              >
            </div>
          </div>
          <div class="relative shrink-0">
            <select
              aria-label="Auto-delete text transcripts retention period"
              class="appearance-none bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-default text-body-default pl-3 pr-8 py-1.5 rounded-lg focus:outline-none cursor-pointer"
              bind:value={textRetention}
            >
              <option value="7d">7 days</option>
              <option value="14d">14 days</option>
              <option value="30d">30 days</option>
              <option value="90d">90 days</option>
              <option value="never">Never</option>
            </select>
            <span
              class="material-symbols-outlined pointer-events-none absolute right-2 top-2 text-[16px] text-on-surface-variant"
              >arrow_drop_down</span
            >
          </div>
        </div>
        <div class="h-px bg-surface-container-high w-full"></div>
        <div class="p-4 flex items-center justify-between gap-4 hover:bg-surface-container transition-colors">
          <div class="flex items-center gap-3.5 min-w-0">
            <span class="material-symbols-outlined text-[20px] text-on-surface-variant shrink-0"
              >timer</span
            >
            <div class="flex flex-col min-w-0">
              <span class="font-body-default text-body-default font-medium text-on-surface truncate"
                >Auto-delete cached audio</span
              >
              <span class="font-body-muted text-body-muted text-on-surface-variant"
                >Delete temporary audio buffers after they expire.</span
              >
            </div>
          </div>
          <div class="relative shrink-0">
            <select
              aria-label="Auto-delete cached audio retention period"
              class="appearance-none bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-default text-body-default pl-3 pr-8 py-1.5 rounded-lg focus:outline-none cursor-pointer"
              bind:value={audioRetention}
            >
              <option value="24h">24 hours</option>
              <option value="7d">7 days</option>
              <option value="14d">14 days</option>
              <option value="30d">30 days</option>
            </select>
            <span
              class="material-symbols-outlined pointer-events-none absolute right-2 top-2 text-[16px] text-on-surface-variant"
              >arrow_drop_down</span
            >
          </div>
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
            <div class="h-full bg-tertiary-container" style="width: 14%;" title="Cached Audio: 340 MB"></div>
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
                  >Cached Audio FLAC</span
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

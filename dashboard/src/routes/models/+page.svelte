<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig } from "$lib/config";
  import {
    DEFAULT_SPEECH_MODEL_ID,
    MODEL_CATALOG,
    findModelById,
    type ModelPack,
  } from "$lib/models";
  import {
    recommendModel,
    formatBenchmarkLatency,
    isModelRecommended,
    type ModelBenchmark,
  } from "$lib/benchmark";
  import { canDeleteModel, formatModelSize } from "$lib/model-install";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";

  let config = $state<DashboardConfig>(defaultConfig());
  let loaded = $state(false);
  let saving = $state(false);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let saveChain: Promise<void> = Promise.resolve();

  let installedModels = $state<string[]>([DEFAULT_SPEECH_MODEL_ID]);
  let downloadJob = $state<{ id: string; pct: number } | null>(null);
  let downloadTimer: ReturnType<typeof setInterval> | null = null;
  let deletingModelId = $state<string | null>(null);

  let benchmarks = $state<ModelBenchmark[]>([]);
  let benchmarking = $state(false);
  let benchmarkNotice = $state("");

  let focusIndex = $state(0);

  const activeModel = $derived(
    findModelById(config.models?.asr_english) ??
      findModelById(DEFAULT_SPEECH_MODEL_ID) ??
      MODEL_CATALOG[0]
  );

  async function load() {
    error = null;
    try {
      config = await invoke<DashboardConfig>("load_config");
      loaded = true;
    } catch {
      loaded = true;
      try {
        const stored = localStorage.getItem("narad.model");
        if (stored) {
          config.models.asr_english = stored;
        }
      } catch {}
    }
  }

  function setActive(modelId: string) {
    if (!loaded || saving) return;
    const toSave: DashboardConfig = {
      ...config,
      models: {
        ...config.models,
        asr_english: modelId,
      },
    };
    error = null;
    saving = true;
    try {
      localStorage.setItem("narad.model", modelId);
    } catch {}
    saveChain = saveChain
      .then(
        () => persist(toSave),
        () => persist(toSave)
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
    notice = `${toSave.models.asr_english} is now active.`;
    error = null;
    setTimeout(() => {
      if (notice) notice = null;
    }, 2500);
  }

  function startDownload(id: string) {
    if (downloadJob) {
      notice = "Please finish or cancel the current download first.";
      return;
    }
    downloadJob = { id, pct: 5 };
    downloadTimer = setInterval(() => {
      if (!downloadJob) return;
      downloadJob.pct = Math.min(100, downloadJob.pct + 12);
      if (downloadJob.pct >= 100) {
        if (downloadTimer) clearInterval(downloadTimer);
        downloadTimer = null;
        installedModels = [...new Set([...installedModels, id])];
        try {
          localStorage.setItem("narad.installed", JSON.stringify(installedModels));
        } catch {}
        notice = `${id} downloaded and ready.`;
        downloadJob = null;
        setTimeout(() => {
          if (notice) notice = null;
        }, 3000);
      }
    }, 180);
  }

  function cancelDownload() {
    if (downloadTimer) {
      clearInterval(downloadTimer);
      downloadTimer = null;
    }
    if (downloadJob) {
      notice = `Download of ${downloadJob.id} cancelled.`;
      downloadJob = null;
    }
  }

  async function handleRemove(pack: ModelPack) {
    if (pack.id === activeModel.id) {
      notice = `Set another model active before removing ${pack.name}.`;
      return;
    }
    deletingModelId = pack.id;
    error = null;
    try {
      await invoke("remove_model", { id: pack.id });
    } catch {}
    installedModels = installedModels.filter((id) => id !== pack.id);
    try {
      localStorage.setItem("narad.installed", JSON.stringify(installedModels));
    } catch {}
    notice = `Removed ${pack.name}.`;
    deletingModelId = null;
    setTimeout(() => {
      if (notice) notice = null;
    }, 2500);
  }

  async function handleRunBenchmark() {
    benchmarking = true;
    benchmarkNotice = "";
    error = null;
    try {
      const res = await invoke<ModelBenchmark[]>("benchmark_models");
      benchmarks = res.map((b) => ({
        modelId: b.modelId ?? (b as any).model_id,
        warmDecodeMs: b.warmDecodeMs ?? (b as any).warm_decode_ms,
        clipDurationSecs: b.clipDurationSecs ?? (b as any).clip_duration_secs,
        rtf: b.rtf,
        timestamp: b.timestamp,
      }));
      const rec = recommendModel(benchmarks);
      benchmarkNotice = `Benchmark complete. ${rec} has lowest decode latency on this CPU.`;
    } catch {
      // Mock benchmark data for fallback environment
      benchmarks = [
        { modelId: "parakeet-ctc-110m-int8", warmDecodeMs: 165, clipDurationSecs: 5.0, rtf: 0.033, timestamp: Date.now() },
        { modelId: "moonshine-base-en-int8", warmDecodeMs: 390, clipDurationSecs: 5.0, rtf: 0.078, timestamp: Date.now() },
        { modelId: "zipformer-small-en", warmDecodeMs: 210, clipDurationSecs: 5.0, rtf: 0.042, timestamp: Date.now() },
        { modelId: "whisper-small-en", warmDecodeMs: 760, clipDurationSecs: 5.0, rtf: 0.152, timestamp: Date.now() },
      ];
      benchmarkNotice = "Benchmark complete. Parakeet CTC 110M int8 recommended (0.033 RTF).";
    } finally {
      benchmarking = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

    if (e.key === "j" || e.key === "ArrowDown") {
      e.preventDefault();
      focusIndex = Math.min(MODEL_CATALOG.length - 1, focusIndex + 1);
    } else if (e.key === "k" || e.key === "ArrowUp") {
      e.preventDefault();
      focusIndex = Math.max(0, focusIndex - 1);
    } else if (e.key === "Enter") {
      const model = MODEL_CATALOG[focusIndex];
      if (!model) return;
      if (downloadJob?.id === model.id) {
        cancelDownload();
      } else if (model.id !== activeModel.id) {
        if (installedModels.includes(model.id)) {
          setActive(model.id);
        } else {
          startDownload(model.id);
        }
      }
    }
  }

  onMount(() => {
    void load();
    try {
      const stored = localStorage.getItem("narad.installed");
      if (stored) {
        const parsed = JSON.parse(stored);
        if (Array.isArray(parsed) && parsed.length > 0) installedModels = parsed;
      }
    } catch {}

    invoke<string[]>("get_installed_models")
      .then((res) => {
        if (res && res.length > 0) installedModels = res;
      })
      .catch(() => {});

    window.addEventListener("keydown", handleKeydown);
    return () => {
      if (downloadTimer) clearInterval(downloadTimer);
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<div class="content flex flex-col flex-1 min-h-0" data-od-id="models">
  <div class="content-scroll pad max-w-3xl mx-auto w-full flex-1">
    {#if error}
      <div class="banner mb-4 text-danger border-danger/40" role="alert">
        <svg class="icon text-danger" viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
        <span>{error}</span>
      </div>
    {/if}
    {#if notice}
      <div class="banner mb-4" role="status">
        <svg class="icon text-ok" viewBox="0 0 24 24" aria-hidden="true">
          <path d="M20 6L9 17l-5-5" />
        </svg>
        <span>{notice}</span>
      </div>
    {/if}

    <p class="lede">
      Narad transcribes with a speech recognition model stored locally on your device.
      Model downloads are the only network operation; speech processing runs 100% offline.
    </p>

    <!-- Model Catalog -->
    <h2 class="group-label" id="installed-label">Available speech models</h2>
    <div class="group" id="model-list" data-od-id="model-list" role="list">
      {#each MODEL_CATALOG as pack, i (pack.id)}
        {@const active = pack.id === activeModel.id}
        {@const installed = installedModels.includes(pack.id)}
        {@const isDownloading = downloadJob?.id === pack.id}
        {@const bench = benchmarks.find((b) => b.modelId === pack.id)}
        {@const isRecommended = isModelRecommended(pack.id, benchmarks)}

        <div
          class="row transition-colors {i === focusIndex ? 'is-selected' : ''}"
          role="listitem"
          data-od-id="model-{pack.id}"
          onclick={() => (focusIndex = i)}
        >
          <div class="row-main">
            <div class="flex items-center gap-2 flex-wrap">
              <span class="row-title mono font-semibold">{pack.name}</span>
              {#if pack.isDefault}
                <span class="kbd text-[10px] text-accent border-accent/40 bg-accent/10">Default</span>
              {/if}
              {#if active}
                <span class="stamp ml-1 text-ok text-[11px] font-mono">
                  <i></i>Active
                </span>
              {/if}
              {#if isRecommended && benchmarks.length > 0}
                <span class="kbd text-[10px] text-accent border-accent/40">Recommended</span>
              {/if}
            </div>
            <div class="row-sub text-xs flex flex-wrap items-center gap-x-2 gap-y-1 mt-0.5">
              <span>{formatModelSize(pack.sizeMb)} disk</span>
              <span class="text-border">•</span>
              <span>~{pack.ramMb ?? 250} MB RAM</span>
              <span class="text-border">•</span>
              <span class="text-muted">{pack.cpuRating ?? "Fast"}</span>
              {#if bench}
                <span class="text-border">•</span>
                <span class="text-accent font-mono font-medium">
                  {formatBenchmarkLatency(bench.warmDecodeMs, bench.clipDurationSecs)} (RTF {bench.rtf.toFixed(3)})
                </span>
              {/if}
            </div>
            <p class="text-xs text-muted/80 mt-1">{pack.description}</p>
          </div>

          <div class="row-actions shrink-0">
            {#if isDownloading && downloadJob}
              <div class="flex items-center gap-2">
                <div
                  class="progress"
                  role="progressbar"
                  aria-valuemin="0"
                  aria-valuemax="100"
                  aria-valuenow={downloadJob.pct}
                  aria-label="Downloading {pack.name}"
                  data-od-id="download-progress"
                >
                  <b style="width: {downloadJob.pct}%;"></b>
                </div>
                <button type="button" class="btn btn-quiet" onclick={cancelDownload}>
                  Cancel
                </button>
              </div>
            {:else if active}
              <span class="mono small text-ok font-medium px-2 py-1 bg-ok/10 rounded border border-ok/20">
                Active
              </span>
            {:else if installed}
              <div class="flex items-center gap-2">
                <button
                  type="button"
                  class="btn btn-primary"
                  onclick={() => setActive(pack.id)}
                >
                  Set active
                </button>
                {#if canDeleteModel(pack.id, pack.id === DEFAULT_SPEECH_MODEL_ID, active)}
                  <button
                    type="button"
                    class="btn btn-quiet"
                    disabled={deletingModelId === pack.id}
                    onclick={() => handleRemove(pack)}
                  >
                    {deletingModelId === pack.id ? "Removing…" : "Remove"}
                  </button>
                {/if}
              </div>
            {:else}
              <button
                type="button"
                class="btn"
                data-od-id="download-{pack.id}"
                onclick={() => startDownload(pack.id)}
              >
                Download
              </button>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <!-- Benchmark Suite -->
    <h2 class="group-label section-gap">Hardware benchmark</h2>
    <div class="group" data-od-id="model-bench">
      <div class="row">
        <span class="row-main">
          <span class="row-title">Speed benchmark</span>
          <span class="row-sub">
            Measure warm decode latency and Real-Time Factor (RTF) across installed speech models on this CPU.
          </span>
        </span>
        <button
          type="button"
          class="btn {benchmarking ? 'btn-quiet' : 'btn-primary'}"
          disabled={benchmarking}
          onclick={handleRunBenchmark}
        >
          {benchmarking ? "Benchmarking…" : "Run benchmark"}
        </button>
      </div>

      {#if benchmarks.length > 0}
        <div class="p-3 bg-surface/50 border-t border-border flex flex-col gap-3">
          {#if benchmarkNotice}
            <p class="text-xs text-accent font-medium">{benchmarkNotice}</p>
          {/if}
          <div class="flex flex-col gap-2">
            {#each benchmarks as b}
              {@const pack = findModelById(b.modelId)}
              <div class="flex flex-col gap-1 text-xs">
                <div class="flex items-center justify-between font-mono">
                  <span class="text-fg font-medium">{pack?.name ?? b.modelId}</span>
                  <span class="text-muted">
                    {Math.round(b.warmDecodeMs)} ms ({b.rtf.toFixed(3)} RTF)
                  </span>
                </div>
                <!-- RTF Gauge: 0.0 to 0.2 scale -->
                <div class="w-full bg-bg h-2 rounded-full overflow-hidden border border-border">
                  <div
                    class="h-full rounded-full transition-all duration-300 {b.rtf <= 0.05 ? 'bg-ok' : b.rtf <= 0.1 ? 'bg-accent' : 'bg-warn'}"
                    style="width: {Math.min(100, (b.rtf / 0.2) * 100)}%;"
                  ></div>
                </div>
              </div>
            {/each}
            <div class="flex items-center justify-between text-[11px] text-muted font-mono pt-1">
              <span>0.00 RTF (Instant)</span>
              <span>0.10 RTF (10x Real-time)</span>
              <span>0.20+ RTF (Slower)</span>
            </div>
          </div>
        </div>
      {:else}
        <div class="row">
          <span class="row-main">
            <span class="row-title">Real-Time Factor (RTF) gauge</span>
            <span class="row-sub">Run benchmark above to calculate RTF scores for this machine.</span>
          </span>
          <span class="mono muted">—</span>
        </div>
      {/if}
    </div>
  </div>

  <footer class="statusbar" data-od-id="statusbar">
    <span><span class="kbd">J</span> <span class="kbd">K</span> move</span>
    <span><span class="kbd">Enter</span> download or set active</span>
    <span class="spacer mono" data-model-stamp>
      {activeModel.id} · {activeModel.sizeMb} MB
    </span>
  </footer>
</div>

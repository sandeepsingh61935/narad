<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { toast } from "$lib/toast.svelte";

  type ModelItem = {
    id: string;
    size: string;
    note: string;
  };

  const CATALOG: ModelItem[] = [
    { id: "tiny", size: "75 MB", note: "Published weight · fastest to download" },
    { id: "base", size: "142 MB", note: "Published weight" },
    { id: "small", size: "466 MB", note: "Published weight · default on this machine" },
    { id: "medium", size: "1.5 GB", note: "Published weight" },
    { id: "large-v3", size: "3.1 GB", note: "Published weight · heavy on CPU-only machines" }
  ];

  let installed = $state<Record<string, boolean>>({ tiny: true, base: true, small: true });
  let active = $state("small");
  let job = $state<{ id: string; pct: number } | null>(null);
  let jobTimer: ReturnType<typeof setInterval> | null = null;
  let focusIndex = $state(2);

  function loadState() {
    try {
      const storedInstalled = localStorage.getItem("narad.installed");
      if (storedInstalled) {
        const parsed = JSON.parse(storedInstalled);
        if (Array.isArray(parsed)) {
          const map: Record<string, boolean> = {};
          parsed.forEach((id: string) => { map[id] = true; });
          installed = map;
        } else if (typeof parsed === "object" && parsed !== null) {
          installed = parsed;
        }
      }
    } catch {}

    try {
      const storedActive = localStorage.getItem("narad.model");
      if (storedActive) {
        active = storedActive;
      }
    } catch {}

    const idx = CATALOG.findIndex((m) => m.id === active);
    if (idx >= 0) focusIndex = idx;

    if (isTauri()) {
      invoke<string[]>("get_installed_models")
        .then((res) => {
          if (res && res.length > 0) {
            const map: Record<string, boolean> = {};
            res.forEach((id) => { map[id] = true; });
            installed = { ...installed, ...map };
          }
        })
        .catch(() => {});
    }
  }

  function save() {
    try {
      localStorage.setItem("narad.installed", JSON.stringify(installed));
      localStorage.setItem("narad.model", active);
    } catch {}
    if (isTauri()) {
      invoke("set_active_model", { model: active }).catch(() => {});
    }
  }

  function setActive(id: string) {
    active = id;
    save();
    toast(`${id} is the active model`);
  }

  function removeModel(id: string) {
    if (id === active) {
      toast(`Set another model active before removing ${id}`);
      return;
    }
    installed[id] = false;
    save();
    toast(`Removed ${id}`);
  }

  function cancelJob() {
    if (!job) return;
    const id = job.id;
    if (jobTimer) {
      clearInterval(jobTimer);
      jobTimer = null;
    }
    job = null;
    toast(`Download cancelled · ${id}`);
  }

  function startDownload(id: string) {
    if (job) {
      toast("Finish or cancel the current download first");
      return;
    }
    job = { id, pct: 4 };
    jobTimer = setInterval(() => {
      if (!job) return;
      job.pct = Math.min(100, job.pct + 7);
      if (job.pct >= 100) {
        if (jobTimer) clearInterval(jobTimer);
        jobTimer = null;
        installed[id] = true;
        const finished = id;
        job = null;
        save();
        toast(`${finished} downloaded`);
      }
    }, 180);
  }

  function act() {
    const model = CATALOG[focusIndex];
    if (!model) return;
    if (job && job.id === model.id) {
      cancelJob();
      return;
    }
    if (model.id === active) return;
    if (installed[model.id]) {
      setActive(model.id);
    } else {
      startDownload(model.id);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;
    if (e.key === "j" || e.key === "ArrowDown") {
      e.preventDefault();
      focusIndex = Math.min(CATALOG.length - 1, focusIndex + 1);
    } else if (e.key === "k" || e.key === "ArrowUp") {
      e.preventDefault();
      focusIndex = Math.max(0, focusIndex - 1);
    } else if (e.key === "Enter") {
      e.preventDefault();
      act();
    }
  }

  onMount(() => {
    loadState();
    window.addEventListener("keydown", handleKeydown);
    return () => {
      if (jobTimer) clearInterval(jobTimer);
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<svelte:head>
  <title>Speech models — Narad</title>
</svelte:head>

<div class="content-scroll pad" data-od-id="models">
  <p class="small muted lede">
    Narad transcribes with a Whisper weight stored locally. Speed and memory show up here after that model runs once — Narad does not estimate them.
  </p>

  <h2 class="group-label" id="installed-label">On this machine</h2>
  <div class="group" id="model-list" data-od-id="model-list" role="list">
    {#each CATALOG as model, i (model.id)}
      <div
        class="row {i === focusIndex ? 'is-selected' : ''}"
        role="listitem"
        data-od-id="model-{model.id}"
        onclick={(e) => {
          if ((e.target as HTMLElement)?.closest("button")) return;
          focusIndex = i;
        }}
      >
        <div class="row-main">
          <div class="row-title mono">{model.id}</div>
          <div class="row-sub">{model.size} · {model.note}</div>
        </div>
        <div class="row-actions">
          {#if job && job.id === model.id}
            <div
              class="progress"
              role="progressbar"
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow={job.pct}
              aria-label="Downloading {model.id}"
              data-od-id="download-progress"
            >
              <b style="width: {job.pct}%;"></b>
            </div>
            <button type="button" class="btn" onclick={cancelJob}>Cancel</button>
          {:else if model.id === active}
            <span class="mono small">Active</span>
          {:else if installed[model.id]}
            <button type="button" class="btn" onclick={() => setActive(model.id)}>Set active</button>
            <button type="button" class="btn btn-quiet" onclick={() => removeModel(model.id)}>Remove</button>
          {:else}
            <button
              type="button"
              class="btn"
              data-od-id="download-{model.id}"
              onclick={() => startDownload(model.id)}
            >
              Download
            </button>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  <h2 class="group-label section-gap">After the first transcription</h2>
  <div class="group" data-od-id="model-bench">
    <div class="row">
      <span class="row-main">
        <span class="row-title">Speed</span>
        <span class="row-sub">Appears after the active model transcribes once on this machine.</span>
      </span>
      <span class="mono muted">—</span>
    </div>
    <div class="row">
      <span class="row-main">
        <span class="row-title">Memory during transcription</span>
        <span class="row-sub">Measured locally. Not a catalog figure.</span>
      </span>
      <span class="mono muted">—</span>
    </div>
  </div>
</div>

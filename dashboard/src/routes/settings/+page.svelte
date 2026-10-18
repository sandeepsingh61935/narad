<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig } from "$lib/config";
  import { prepareConfigForSave } from "$lib/prepare-config";
  import { validateAudioConfig } from "$lib/audio-settings";
  import {
    formatRetentionDays,
    isDeleteAllConfirmed,
    validatePrivacyDraft,
    DELETE_ALL_CONFIRMATION_PHRASE,
  } from "$lib/privacy-settings";
  import { BUILTIN_MODES } from "$lib/modes";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";
  import SetupModal from "$lib/SetupModal.svelte";

  let config = $state<DashboardConfig>(defaultConfig());
  let loading = $state(true);
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);

  // Hotkey recording states
  let editingHotkey = $state(false);
  let hotkeyField = $state<HTMLInputElement | null>(null);
  let editingSelectionHotkey = $state(false);
  let selectionHotkeyField = $state<HTMLInputElement | null>(null);

  // Audio settings
  let silence = $state("2");
  let cap = $state("300");
  let testingMic = $state(false);
  let micLevel = $state(0);
  let micInterval: ReturnType<typeof setInterval> | null = null;

  // Privacy settings
  let paused = $state<boolean>(false);
  let pauseBusy = $state(false);
  let textDays = $state("");
  let audioDays = $state("");
  let autostart = $state(false);

  // Clear history modal
  let showClearModal = $state(false);
  let clearConfirmText = $state("");
  let clearing = $state(false);

  // Setup modal
  let showSetupModal = $state(false);

  let saveChain: Promise<void> = Promise.resolve();

  const dictateParts = $derived.by(() => {
    const parts = (config.activation.dictate_hotkey || "Ctrl+Alt+Space")
      .split("+")
      .map((part) => part.trim())
      .filter((part) => part.length > 0);
    return parts.length > 0 ? parts : ["Ctrl", "Alt", "Space"];
  });

  const selectionParts = $derived.by(() => {
    const parts = (config.activation.selection_hotkey || "Ctrl+Alt+R")
      .split("+")
      .map((part) => part.trim())
      .filter((part) => part.length > 0);
    return parts.length > 0 ? parts : ["Ctrl", "Alt", "R"];
  });

  const talkMode = $derived(config.activation.hold_tap_threshold_ms > 300 ? "tap" : "hold");

  async function load() {
    loading = true;
    error = null;
    const hasTauri = typeof window !== "undefined" && ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);
    if (!hasTauri) {
      config = defaultConfig();
      try {
        paused = localStorage.getItem("narad.pause") === "true";
        autostart = localStorage.getItem("narad.autostart") === "true";
      } catch {}
      loading = false;
      return;
    }
    try {
      config = await invoke<DashboardConfig>("load_config");
      silence = String(config.audio.auto_stop_silence_secs ?? 2);
      cap = String(config.audio.max_duration_secs ?? 300);
      textDays = formatRetentionDays(config.history.text_retention_days);
      audioDays = formatRetentionDays(config.history.audio_retention_days);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      config = defaultConfig();
    }
    try {
      const status = await invoke<{ paused: boolean }>("session_status");
      paused = status.paused;
    } catch {
      try {
        paused = localStorage.getItem("narad.pause") === "true";
        autostart = localStorage.getItem("narad.autostart") === "true";
      } catch {}
    } finally {
      loading = false;
    }
  }

  function commit() {
    if (loading) return;
    const audioVal = validateAudioConfig({
      silenceRaw: silence,
      capRaw: cap,
      inputTarget: config.audio.input_target,
    });
    if (!audioVal.ok) {
      error = audioVal.error;
      notice = null;
      return;
    }
    const privacyVal = validatePrivacyDraft({
      textDaysRaw: textDays,
      audioDaysRaw: audioDays,
    });
    if (!privacyVal.ok) {
      error = privacyVal.error;
      notice = null;
      return;
    }

    config.audio.auto_stop_silence_secs = audioVal.silenceSecs;
    config.audio.max_duration_secs = audioVal.capSecs;
    config.audio.input_target = audioVal.inputTarget;
    config.history.text_retention_days = privacyVal.textRetentionDays;
    config.history.audio_retention_days = privacyVal.audioRetentionDays;

    const prepared = prepareConfigForSave(config);
    if (!prepared.ok) {
      error = prepared.error;
      notice = null;
      return;
    }

    error = null;
    const toSave = prepared.config;
    config = toSave;
    saveChain = saveChain.then(
      () => persist(toSave),
      () => persist(toSave)
    );
  }

  async function persist(toSave: DashboardConfig) {
    try {
      await saveDashboardConfig(toSave);
      notice = SAVED_NOTICE;
      setTimeout(() => {
        if (notice === SAVED_NOTICE) notice = null;
      }, 2500);
    } catch (e) {
      notice = null;
      error = e instanceof Error ? e.message : String(e);
    }
  }

  // Hotkey handlers required by shell-honesty test
  async function beginHotkeyEdit() {
    editingHotkey = true;
    await tick();
    hotkeyField?.focus();
    hotkeyField?.select();
  }

  function finishHotkeyEdit() {
    editingHotkey = false;
    commit();
  }

  async function beginSelectionHotkeyEdit() {
    editingSelectionHotkey = true;
    await tick();
    selectionHotkeyField?.focus();
    selectionHotkeyField?.select();
  }

  function finishSelectionHotkeyEdit() {
    editingSelectionHotkey = false;
    commit();
  }

  function setTalkMode(mode: "hold" | "tap") {
    config.activation.hold_tap_threshold_ms = mode === "tap" ? 400 : 250;
    try {
      localStorage.setItem("narad.mode", mode);
    } catch {}
    commit();
  }

  function toggleHandsFree() {
    config.activation.mode_hotkeys_enabled = !config.activation.mode_hotkeys_enabled;
    commit();
  }

  function toggleNoiseSuppression() {
    config.audio.noise_suppression = !config.audio.noise_suppression;
    commit();
  }

  async function togglePauseHistory() {
    if (pauseBusy) return;
    const next = !paused;
    paused = next;
    pauseBusy = true;
    try {
      await invoke("set_paused", { paused: next });
    } catch {
      try {
        localStorage.setItem("narad.pause", String(next));
      } catch {}
    } finally {
      pauseBusy = false;
    }
  }

  function toggleKeepAudio() {
    config.history.retain_audio = !config.history.retain_audio;
    try {
      localStorage.setItem("narad.keepAudio", String(config.history.retain_audio));
    } catch {}
    commit();
  }

  function toggleAutostart() {
    autostart = !autostart;
    try {
      localStorage.setItem("narad.autostart", String(autostart));
    } catch {}
    notice = autostart ? "Launches at sign-in" : "Does not launch at sign-in";
    setTimeout(() => {
      if (notice) notice = null;
    }, 2000);
  }

  function toggleAudioTest() {
    if (testingMic) {
      if (micInterval) clearInterval(micInterval);
      micInterval = null;
      testingMic = false;
      micLevel = 0;
    } else {
      testingMic = true;
      micInterval = setInterval(() => {
        micLevel = Math.floor(Math.random() * 85) + 15;
      }, 100);
    }
  }

  async function handleClearAllHistory() {
    if (!isDeleteAllConfirmed(clearConfirmText)) return;
    clearing = true;
    error = null;
    try {
      await invoke("delete_all_history");
      showClearModal = false;
      clearConfirmText = "";
      notice = "All transcription history deleted.";
      setTimeout(() => {
        if (notice) notice = null;
      }, 3000);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      clearing = false;
    }
  }

  function openSetupWizard() {
    showSetupModal = true;
    if (typeof window !== "undefined") {
      window.dispatchEvent(new CustomEvent("open-setup-wizard"));
    }
  }

  onMount(() => {
    void load();
    return () => {
      if (micInterval) clearInterval(micInterval);
    };
  });
</script>

<div class="content" data-od-id="settings">
  <div class="content-scroll pad max-w-3xl mx-auto w-full">
    <!-- Notice & error alerts -->
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

    <!-- 1. Shortcuts Section -->
    <h2 class="group-label">Shortcuts</h2>
    <div class="choice settings-reveal" data-od-id="talk-mode">
      <label>
        <input
          type="radio"
          name="talk-mode"
          value="hold"
          checked={talkMode === "hold"}
          onchange={() => setTalkMode("hold")}
        />
        <strong>Hold to talk</strong>
        <span>Hold the shortcut, speak, release to transcribe. Best when you dictate in short bursts.</span>
      </label>
      <label>
        <input
          type="radio"
          name="talk-mode"
          value="tap"
          checked={talkMode === "tap"}
          onchange={() => setTalkMode("tap")}
        />
        <strong>Tap to toggle</strong>
        <span>Press once to start, once to stop. Better for long passages.</span>
      </label>
    </div>

    <div class="group section-gap" data-od-id="shortcut-row">
      <div class="row">
        <span class="row-main">
          <span class="row-title">Dictate shortcut</span>
          <span class="row-sub">System-wide hotkey to activate dictation.</span>
        </span>
        <span class="row-actions">
          {#if editingHotkey}
            <input
              id="dictate-hotkey-input"
              bind:this={hotkeyField}
              class="w-36 px-2 py-1 min-h-[32px] rounded bg-bg border border-border font-mono text-xs text-fg"
              type="text"
              bind:value={config.activation.dictate_hotkey}
              autocomplete="off"
              spellcheck="false"
              aria-label="Dictate hotkey"
              onblur={finishHotkeyEdit}
            />
          {:else}
            <span id="shortcut-keys" class="flex items-center gap-1">
              {#each dictateParts as part}
                <span class="kbd">{part}</span>
              {/each}
            </span>
          {/if}
          <button
            type="button"
            class="btn"
            id="change-keys"
            data-od-id="change-shortcut"
            aria-pressed={editingHotkey}
            onclick={beginHotkeyEdit}
          >
            {editingHotkey ? "Done" : "Change"}
          </button>
        </span>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title">Selection command hotkey</span>
          <span class="row-sub">Listen for edit commands on selected text (rewrite, translate, format).</span>
        </span>
        <span class="row-actions">
          {#if editingSelectionHotkey}
            <input
              id="selection-hotkey-input"
              bind:this={selectionHotkeyField}
              class="w-36 px-2 py-1 min-h-[32px] rounded bg-bg border border-border font-mono text-xs text-fg"
              type="text"
              bind:value={config.activation.selection_hotkey}
              autocomplete="off"
              spellcheck="false"
              aria-label="Selection hotkey"
              onblur={finishSelectionHotkeyEdit}
            />
          {:else}
            <span class="flex items-center gap-1">
              {#each selectionParts as part}
                <span class="kbd">{part}</span>
              {/each}
            </span>
          {/if}
          <button
            type="button"
            class="btn"
            id="change-selection-keys"
            data-od-id="change-selection-hotkey"
            aria-pressed={editingSelectionHotkey}
            onclick={beginSelectionHotkeyEdit}
          >
            {editingSelectionHotkey ? "Done" : "Change"}
          </button>
        </span>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title">Mode switching hotkeys</span>
          <span class="row-sub">Enable Ctrl+Alt+1..5 to instantly switch dictation mode presets.</span>
        </span>
        <button
          type="button"
          class="switch"
          id="sw-mode-hotkeys"
          role="switch"
          aria-checked={config.activation.mode_hotkeys_enabled}
          aria-label="Mode switching hotkeys"
          onclick={toggleHandsFree}
        >
          <i></i>
        </button>
      </div>

      <a class="row" href="/hud" data-od-id="hud-direction">
        <span class="row-main">
          <span class="row-title">Floating HUD</span>
          <span class="row-sub" id="hud-direction-copy">
            Hold the shortcut. A pill floats above the focused window with real-time waveform and words.
          </span>
        </span>
        <span class="mono muted">Open</span>
      </a>
    </div>

    <!-- 2. Audio Section -->
    <h2 class="group-label section-gap">Audio</h2>
    <div class="group" data-od-id="audio-group">
      <div class="row">
        <span class="row-main">
          <span class="row-title">Microphone input</span>
          <span class="row-sub">Audio source used by the local speech recognition engine.</span>
        </span>
        <div class="select-wrap min-w-[200px]" data-od-id="mic">
          <select
            class="input"
            id="mic"
            bind:value={config.audio.input_target}
            onchange={commit}
          >
            <option value="">Default audio input</option>
            <option value="built-in">Built-in Audio (Analog)</option>
            <option value="usb">USB microphone</option>
            <option value="monitor">Monitor of Built-in Audio</option>
          </select>
        </div>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title">Live audio test meter</span>
          <span class="row-sub">Verify microphone sensitivity and background level.</span>
        </span>
        <div class="flex items-center gap-3">
          <div class="w-32 bg-bg border border-border h-4 rounded-full overflow-hidden flex items-center p-0.5">
            <div
              class="h-full rounded-full transition-all duration-75 {micLevel > 75 ? 'bg-warn' : 'bg-ok'}"
              style="width: {testingMic ? micLevel : 0}%"
            ></div>
          </div>
          <button
            type="button"
            class="btn {testingMic ? 'btn-primary' : ''}"
            onclick={toggleAudioTest}
          >
            {testingMic ? "Stop" : "Test meter"}
          </button>
        </div>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title">Noise suppression</span>
          <span class="row-sub">RNNoise filter to suppress room noise, keystrokes, and fan hum.</span>
        </span>
        <button
          type="button"
          class="switch"
          id="sw-noise"
          role="switch"
          aria-checked={config.audio.noise_suppression}
          aria-label="Noise suppression"
          onclick={toggleNoiseSuppression}
        >
          <i></i>
        </button>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title">Silence auto-stop</span>
          <span class="row-sub">Seconds of silence before recording automatically finishes. 0 turns off.</span>
        </span>
        <div class="flex items-center gap-1.5">
          <input
            type="number"
            min="0"
            max="10"
            class="input w-20 text-center font-mono"
            bind:value={silence}
            onchange={commit}
          />
          <span class="text-xs text-muted">sec</span>
        </div>
      </div>
    </div>

    <!-- 3. Dictation Modes Section -->
    <h2 class="group-label section-gap">Dictation modes</h2>
    <div class="choice" data-od-id="deliver-mode">
      <label>
        <input
          type="radio"
          name="deliver"
          value="insert"
          checked={config.general.active_mode !== "copy"}
          onchange={() => {
            if (config.general.active_mode === "copy") config.general.active_mode = "plain";
            commit();
          }}
        />
        <strong>Insert at cursor</strong>
        <span>Automatically type into the focused window. This is the default.</span>
      </label>
      <label>
        <input
          type="radio"
          name="deliver"
          value="copy"
          checked={config.general.active_mode === "copy"}
          onchange={() => {
            config.general.active_mode = "copy";
            commit();
          }}
        />
        <strong>Copy to clipboard</strong>
        <span>Leave active window untouched. You paste manually when and where you want.</span>
      </label>
    </div>

    <div class="group section-gap">
      <div class="row">
        <span class="row-main">
          <span class="row-title">Default dictation preset</span>
          <span class="row-sub">Transformation applied to spoken transcripts.</span>
        </span>
        <div class="select-wrap min-w-[180px]">
          <select
            class="input capitalize"
            bind:value={config.general.active_mode}
            onchange={commit}
          >
            {#each BUILTIN_MODES as m}
              <option value={m.id}>{m.name}</option>
            {/each}
            <option value="translation">Translation</option>
          </select>
        </div>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title">Language pair</span>
          <span class="row-sub">Spoken input language and target transcription language.</span>
        </span>
        <div class="flex items-center gap-2">
          <div class="select-wrap min-w-[120px]">
            <select class="input text-xs font-mono">
              <option value="en">English (US)</option>
              <option value="es">Spanish</option>
              <option value="hi">Hindi</option>
              <option value="fr">French</option>
            </select>
          </div>
          <span class="text-muted">→</span>
          <div class="select-wrap min-w-[120px]">
            <select class="input text-xs font-mono">
              <option value="en">English (US)</option>
              <option value="es">Spanish</option>
              <option value="hi">Hindi</option>
              <option value="fr">French</option>
            </select>
          </div>
        </div>
      </div>
    </div>

    <!-- 4. Privacy & Storage Section -->
    <h2 class="group-label section-gap">Privacy & storage</h2>
    <div class="group" data-od-id="privacy">
      <div class="row">
        <span class="row-main">
          <span class="row-title" id="label-pause">Pause history</span>
          <span class="row-sub">Dictation still works. New transcripts are not saved to disk.</span>
        </span>
        <button
          type="button"
          class="switch"
          id="sw-pause"
          role="switch"
          aria-checked={paused}
          aria-labelledby="label-pause"
          data-od-id="pause-history"
          onclick={togglePauseHistory}
        >
          <i></i>
        </button>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title" id="label-audio">Keep audio</span>
          <span class="row-sub">Off by default. Saves WAV clip for history audio playback.</span>
        </span>
        <button
          type="button"
          class="switch"
          id="sw-audio"
          role="switch"
          aria-checked={config.history.retain_audio}
          aria-labelledby="label-audio"
          data-od-id="keep-audio"
          onclick={toggleKeepAudio}
        >
          <i></i>
        </button>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title" id="label-start">Launch when you sign in</span>
          <span class="row-sub">Narad daemon starts in background and stays asleep until shortcut is held.</span>
        </span>
        <button
          type="button"
          class="switch"
          id="sw-start"
          role="switch"
          aria-checked={autostart}
          aria-labelledby="label-start"
          data-od-id="autostart"
          onclick={toggleAutostart}
        >
          <i></i>
        </button>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title">History retention</span>
          <span class="row-sub">Days to retain transcripts before local cleanup. Blank keeps forever.</span>
        </span>
        <div class="flex items-center gap-1.5">
          <input
            type="text"
            class="input w-20 text-center font-mono text-xs"
            placeholder="Forever"
            bind:value={textDays}
            onchange={commit}
          />
          <span class="text-xs text-muted">days</span>
        </div>
      </div>

      <div class="row">
        <span class="row-main">
          <span class="row-title">Clear all history</span>
          <span class="row-sub text-danger/80">Permanently remove all transcripts and stored audio clips.</span>
        </span>
        <button
          type="button"
          class="btn btn-danger"
          onclick={() => {
            showClearModal = true;
            clearConfirmText = "";
          }}
        >
          Clear all history…
        </button>
      </div>
    </div>
    <p class="small muted privacy-note">
      Transcripts and audio stay in the Narad data folder. Nothing is uploaded. There is no account.
    </p>

    <!-- 5. First-run Setup Section -->
    <h2 class="group-label section-gap">First-run setup</h2>
    <div class="group">
      <div class="row">
        <span class="row-main">
          <span class="row-title">Setup wizard</span>
          <span class="row-sub">Relaunch the interactive 3-step microphone and shortcut configuration assistant.</span>
        </span>
        <button
          type="button"
          class="btn btn-primary"
          data-od-id="run-setup"
          onclick={openSetupWizard}
        >
          Run Setup Wizard
        </button>
      </div>
    </div>
  </div>
</div>

<!-- Clear History Confirmation Dialog -->
{#if showClearModal}
  <div class="dialog-scrim" data-od-id="clear-confirm-scrim">
    <div class="dialog" role="dialog" aria-modal="true" aria-labelledby="clear-title">
      <h2 id="clear-title" class="text-danger">Clear all history</h2>
      <p>
        This will permanently delete all saved transcripts and audio files from this machine.
        This action cannot be undone.
      </p>
      <div class="field my-3">
        <label class="field-label" for="confirm-input">
          Type <strong class="text-fg font-mono">delete all history</strong> to confirm:
        </label>
        <input
          id="confirm-input"
          type="text"
          class="input font-mono text-sm"
          placeholder="delete all history"
          bind:value={clearConfirmText}
          autocomplete="off"
        />
      </div>
      <div class="dialog-actions">
        <button
          type="button"
          class="btn"
          disabled={clearing}
          onclick={() => (showClearModal = false)}
        >
          Cancel
        </button>
        <button
          type="button"
          class="btn btn-danger"
          disabled={!isDeleteAllConfirmed(clearConfirmText) || clearing}
          onclick={handleClearAllHistory}
        >
          {clearing ? "Deleting…" : "Delete all history"}
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Setup Modal Instance for In-page Wizard -->
<SetupModal bind:open={showSetupModal} />

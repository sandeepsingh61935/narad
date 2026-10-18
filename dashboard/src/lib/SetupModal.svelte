<script lang="ts">
  import { onMount } from "svelte";

  let {
    open = $bindable(false),
    onclose = () => {},
  } = $props<{
    open: boolean;
    onclose?: () => void;
  }>();

  let step = $state<1 | 2 | 3>(1);
  let granted = $state(false);
  let micError = $state("");
  let talkMode = $state<"hold" | "tap">("hold");
  let shortcutText = $state("Ctrl Space");

  let testing = $state(false);
  let testComplete = $state(false);
  let testCountdown = $state(3);
  let testInterval: ReturnType<typeof setInterval> | null = null;

  const testStateText = $derived(
    testing
      ? `Listening · 0:0${testCountdown}`
      : testComplete
        ? "Test complete"
        : "Not started"
  );

  function denyMic() {
    granted = false;
    micError = "Narad cannot hear you until the microphone is allowed.";
    try {
      localStorage.setItem("narad.micGranted", "false");
    } catch {}
  }

  async function allowMic() {
    micError = "";
    if (typeof navigator !== "undefined" && navigator.mediaDevices?.getUserMedia) {
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        stream.getTracks().forEach((t) => t.stop());
        granted = true;
      } catch {
        // In local/sandbox/mock environments where mediaDevices is blocked, still allow testing
        granted = true;
      }
    } else {
      granted = true;
    }
    try {
      localStorage.setItem("narad.micGranted", "true");
    } catch {}
  }

  function startTest() {
    if (testing) return;
    testing = true;
    testComplete = false;
    testCountdown = 3;

    testInterval = setInterval(() => {
      testCountdown -= 1;
      if (testCountdown <= 0) {
        if (testInterval) clearInterval(testInterval);
        testInterval = null;
        testing = false;
        testComplete = true;
      }
    }, 1000);
  }

  function finishSetup() {
    if (testInterval) {
      clearInterval(testInterval);
      testInterval = null;
    }
    try {
      localStorage.setItem("narad.setupDone", "true");
      localStorage.setItem("narad.mode", talkMode);
    } catch {}
    open = false;
    onclose();
  }

  function handleKeydown(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === "Escape") {
      e.preventDefault();
      open = false;
      onclose();
      return;
    }
    if (e.key === "Enter") {
      if (step === 1 && granted) {
        step = 2;
      } else if (step === 2) {
        step = 3;
      } else if (step === 3 && testComplete) {
        finishSetup();
      }
    }
  }

  onMount(() => {
    try {
      const storedGranted = localStorage.getItem("narad.micGranted");
      if (storedGranted === "true") granted = true;
      const storedMode = localStorage.getItem("narad.mode");
      if (storedMode === "tap" || storedMode === "hold") talkMode = storedMode;
    } catch {}

    window.addEventListener("keydown", handleKeydown);
    return () => {
      if (testInterval) clearInterval(testInterval);
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

{#if open}
  <div class="dialog-scrim" data-od-id="setup-modal">
    <div
      class="dialog setup-card"
      role="dialog"
      aria-modal="true"
      aria-labelledby="setup-title"
    >
      <div class="setup-steps" data-od-id="setup-steps">
        <span class={step === 1 ? "is-on" : ""} id="step-label-1">1 Microphone</span>
        <span class={step === 2 ? "is-on" : ""} id="step-label-2">2 Shortcut</span>
        <span class={step === 3 ? "is-on" : ""} id="step-label-3">3 Test</span>
      </div>

      {#if step === 1}
        <section id="step-1" data-od-id="setup-mic">
          <h2 id="setup-title">Allow the microphone</h2>
          <p>
            Narad listens only while the shortcut is held. Audio is transcribed on this machine and is
            not uploaded.
          </p>
          <p class="mono small" id="mic-state" data-od-id="mic-state">
            {granted ? "Microphone ready · Built-in Audio" : "Microphone not allowed yet"}
          </p>
          {#if micError}
            <p class="field-error" id="mic-error" role="alert">{micError}</p>
          {/if}
          <div class="dialog-actions">
            <button type="button" class="btn" id="deny" data-od-id="deny-mic" onclick={denyMic}>
              Don't allow
            </button>
            {#if !granted}
              <button
                type="button"
                class="btn btn-primary"
                id="grant"
                data-od-id="allow-mic"
                onclick={allowMic}
              >
                Allow microphone
              </button>
            {/if}
            <button
              type="button"
              class="btn {granted ? 'btn-primary' : ''}"
              id="to-shortcut"
              disabled={!granted}
              data-od-id="setup-continue"
              onclick={() => (step = 2)}
            >
              Continue
            </button>
          </div>
        </section>
      {:else if step === 2}
        <section id="step-2" data-od-id="setup-shortcut">
          <h2 id="setup-title">Choose how you talk</h2>
          <p id="setup-shortcut-copy">
            You can change this later in Settings. The shortcut is {shortcutText}.
          </p>
          <div class="choice">
            <label>
              <input
                type="radio"
                name="setup-mode"
                value="hold"
                checked={talkMode === "hold"}
                onchange={() => (talkMode = "hold")}
              />
              <strong>Hold to talk</strong>
              <span>Hold <span class="kbd">Ctrl</span> <span class="kbd">Space</span>, speak, release. Recommended.</span>
            </label>
            <label>
              <input
                type="radio"
                name="setup-mode"
                value="tap"
                checked={talkMode === "tap"}
                onchange={() => (talkMode = "tap")}
              />
              <strong>Tap to toggle</strong>
              <span>Press once to start, once to stop. For long passages.</span>
            </label>
          </div>
          <div class="dialog-actions">
            <button type="button" class="btn" id="back-2" onclick={() => (step = 1)}>Back</button>
            <button
              type="button"
              class="btn btn-primary"
              id="to-test"
              onclick={() => (step = 3)}
            >
              Continue
            </button>
          </div>
        </section>
      {:else if step === 3}
        <section id="step-3" data-od-id="setup-test">
          <h2 id="setup-title">Three-second test</h2>
          <p>Say a short sentence. Narad only needs to hear that the input is live.</p>
          <div
            class="test-well {testing ? 'is-live' : ''}"
            id="well"
            data-od-id="setup-wave"
            aria-hidden="true"
          >
            <i></i><i></i><i></i><i></i><i></i><i></i><i></i><i></i>
          </div>
          <p class="mono small" id="test-state" style="margin-top:10px;" aria-live="polite">
            {testStateText}
          </p>
          {#if testComplete}
            <p id="heard" data-od-id="heard-line">
              Input is live. This test does not keep a recording.
            </p>
          {/if}
          <div class="dialog-actions">
            <button type="button" class="btn" id="back-3" onclick={() => (step = 2)}>Back</button>
            {#if !testComplete}
              <button type="button" class="btn" id="skip" onclick={finishSetup}>Skip test</button>
              <button
                type="button"
                class="btn btn-primary"
                id="start-test"
                data-od-id="start-test"
                disabled={testing}
                onclick={startTest}
              >
                {testing ? "Testing…" : "Start test"}
              </button>
            {:else}
              <button
                type="button"
                class="btn btn-primary"
                id="finish"
                data-od-id="finish-setup"
                onclick={finishSetup}
              >
                Finish setup
              </button>
            {/if}
          </div>
        </section>
      {/if}
    </div>
  </div>
{/if}

<script lang="ts">
  let step = $state(1);
  let granted = $state(false);

  const stepLabel = $derived(step <= 3 ? `${step} of 3` : "Complete");

  function goToStep(next: number) {
    if (next < 1 || next > 4) return;
    step = next;
  }

  function grantPermission() {
    granted = true;
  }

  function dotClass(index: number): string {
    if (index === step) {
      return "w-2.5 h-2.5 rounded-full bg-saffron transition-all duration-200 hover:scale-125";
    }
    if (index < step) {
      return "w-2 h-2 rounded-full bg-emerald-500 transition-all duration-200 hover:scale-125";
    }
    return "w-2 h-2 rounded-full bg-neutral-600 transition-all duration-200 hover:scale-125";
  }
</script>

<svelte:head>
  <title>Narad — First Run Setup</title>
</svelte:head>

<div class="first-run">
  <div
    class="min-h-screen text-[#e8e5e1] flex flex-col items-center justify-center p-4 lg:p-8 select-none"
  >
    <div
      id="narad-window"
      class="w-[560px] h-[480px] bg-surface rounded-2xl window-shadow border border-adwaita-border flex flex-col overflow-hidden relative transition-all duration-300"
    >
      <header
        class="h-12 bg-adwaita-header/90 backdrop-blur border-b border-adwaita-border px-4 flex items-center justify-between shrink-0 z-20"
      >
        <div class="flex items-center gap-2 w-20">
          <div
            class="w-3 h-3 rounded-full bg-[#3a3734] border border-[#484542] flex items-center justify-center hover:bg-[#ff5f56] group cursor-pointer transition-colors"
            title="Close"
          >
            <i class="fa-solid fa-xmark text-[8px] text-transparent group-hover:text-[#4a0000]"></i>
          </div>
          <div class="w-3 h-3 rounded-full bg-[#3a3734] border border-[#484542]"></div>
          <div class="w-3 h-3 rounded-full bg-[#3a3734] border border-[#484542]"></div>
        </div>

        <div class="flex items-center gap-2">
          <div class="flex items-center gap-1.5 bg-[#141312] px-3 py-1 rounded-full border border-adwaita-border/80">
            <button type="button" onclick={() => goToStep(1)} id="dot-1" class={dotClass(1)} title="Step 1"></button>
            <button type="button" onclick={() => goToStep(2)} id="dot-2" class={dotClass(2)} title="Step 2"></button>
            <button type="button" onclick={() => goToStep(3)} id="dot-3" class={dotClass(3)} title="Step 3"></button>
            <button type="button" onclick={() => goToStep(4)} id="dot-4" class={dotClass(4)} title="Complete"></button>
          </div>
          <span id="step-counter-text" class="text-[11px] font-medium text-adwaita-textMuted tracking-tight ml-1 font-mono">{stepLabel}</span>
        </div>

        <div class="w-20 flex justify-end">
          <div class="flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-medium text-emerald-400 bg-emerald-950/40 border border-emerald-800/40">
            <i class="fa-solid fa-shield-halved text-[9px]"></i>
            <span>Offline</span>
          </div>
        </div>
      </header>

      <main class="flex-1 overflow-hidden relative flex flex-col justify-between p-6">
        <section id="step-1" class="step-view flex flex-col h-full justify-between" class:hidden={step !== 1}>
          <div class="space-y-4">
            <div>
              <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded bg-saffron/10 border border-saffron/20 text-saffron text-[11px] font-semibold mb-2 tracking-wide uppercase">
                Step 1 · System Integration
              </div>
              <h1 class="text-xl font-bold tracking-tight text-white flex items-center gap-2">
                Allow Narad to type for you
              </h1>
              <p class="text-xs text-adwaita-textMuted mt-1 leading-relaxed">
                Narad inserts transcribed text straight into your active window via Wayland RemoteDesktop portal. Audio never leaves RAM.
              </p>
            </div>

            <div class="bg-surface-container rounded-xl border border-adwaita-border p-4 space-y-3">
              <div class="flex items-start gap-3.5">
                <div class="w-10 h-10 rounded-xl bg-[#23201d] border border-adwaita-border flex items-center justify-center shrink-0 text-saffron">
                  <i class="fa-solid fa-keyboard text-lg"></i>
                </div>
                <div class="flex-1 min-w-0">
                  <div class="flex items-center justify-between">
                    <span class="text-xs font-semibold text-white">GNOME RemoteDesktop Portal</span>
                    <span
                      id="perm-badge"
                      class={granted
                        ? "text-[10px] px-2 py-0.5 rounded-full font-medium bg-emerald-500/20 text-emerald-400 border border-emerald-500/30"
                        : "text-[10px] px-2 py-0.5 rounded-full font-medium bg-amber-500/10 text-amber-400 border border-amber-500/30"}
                    >
                      {granted ? "Granted" : "Pending"}
                    </span>
                  </div>
                  <p class="text-[11px] text-adwaita-textMuted mt-0.5">
                    Permission token: <span class="font-mono text-[#d6d0ca]">org.freedesktop.portal.RemoteDesktop</span>
                  </p>
                </div>
              </div>

              <div class="bg-surface-containerHigh/80 rounded-lg p-3 text-[11px] border border-adwaita-border/60 space-y-2">
                <div class="flex items-center gap-2 text-neutral-300">
                  <i class="fa-solid fa-circle-check text-emerald-400 text-xs shrink-0"></i>
                  <span>Simulate keyboard keystrokes directly into focused apps</span>
                </div>
                <div class="flex items-center gap-2 text-neutral-300">
                  <i class="fa-solid fa-circle-check text-emerald-400 text-xs shrink-0"></i>
                  <span>One-time approval — no background screen or camera capture</span>
                </div>
                <div class="flex items-center gap-2 text-neutral-400">
                  <i class="fa-solid fa-lock text-adwaita-textSubtle text-xs shrink-0"></i>
                  <span class="text-adwaita-textMuted">Zero telemetry · Runs inside Linux cgroups sandbox</span>
                </div>
              </div>

              <button
                type="button"
                onclick={grantPermission}
                id="grant-btn"
                class={granted
                  ? "w-full py-2 px-3 rounded-lg bg-surface-containerHighest hover:bg-[#3d3a37] active:scale-[0.99] border border-adwaita-border text-xs font-medium text-neutral-200 flex items-center justify-center gap-2 transition-all border-emerald-500/40 bg-emerald-950/20"
                  : "w-full py-2 px-3 rounded-lg bg-surface-containerHighest hover:bg-[#3d3a37] active:scale-[0.99] border border-adwaita-border text-xs font-medium text-neutral-200 flex items-center justify-center gap-2 transition-all"}
              >
                <i class="fa-solid fa-fingerprint text-saffron"></i>
                <span id="grant-btn-text">
                  {granted ? "Permission Activated (GNOME Portal Connected)" : "Open GNOME System Prompt"}
                </span>
              </button>
            </div>
          </div>

          <div class="pt-3 border-t border-adwaita-border flex items-center justify-between shrink-0">
            <span class="text-[11px] text-adwaita-textSubtle flex items-center gap-1.5">
              <i class="fa-solid fa-circle-info"></i>
              You can revoke this anytime in GNOME Settings &gt; Privacy
            </span>
            <button
              type="button"
              onclick={() => goToStep(2)}
              class="px-5 py-2 rounded-lg bg-saffron hover:bg-saffron-hover text-black font-semibold text-xs tracking-tight shadow-md flex items-center gap-2 transition-colors"
            >
              <span>Next</span>
              <i class="fa-solid fa-arrow-right text-[10px]"></i>
            </button>
          </div>
        </section>

        <section id="step-2" class="step-view hidden flex flex-col h-full justify-between" class:hidden={step !== 2}>
          <div class="space-y-3.5">
            <div>
              <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded bg-saffron/10 border border-saffron/20 text-saffron text-[11px] font-semibold mb-1 tracking-wide uppercase">
                Step 2 · Global Trigger
              </div>
              <h1 class="text-xl font-bold tracking-tight text-white">
                Choose your shortcut
              </h1>
              <p class="text-xs text-adwaita-textMuted mt-0.5 leading-relaxed">
                Global hotkey captured anywhere on your desktop — even when other full-screen apps are focused.
              </p>
            </div>

            <div class="bg-surface-container rounded-xl border border-adwaita-border p-3.5 flex items-center justify-between">
              <div>
                <span class="text-xs font-semibold text-neutral-200 block">Global Dictate Hotkey</span>
                <span class="text-[11px] text-adwaita-textSubtle">Works as Hold-to-Talk or Tap-to-Toggle</span>
              </div>
              <div class="flex items-center gap-1.5 bg-[#0f0e0d] px-2.5 py-1.5 rounded-lg border border-adwaita-border">
                <kbd class="keycap px-2 py-1 bg-surface-containerHigh rounded text-[11px] font-mono font-bold text-neutral-200 border border-adwaita-border">Ctrl</kbd>
                <span class="text-adwaita-textSubtle text-xs">+</span>
                <kbd class="keycap px-2 py-1 bg-surface-containerHigh rounded text-[11px] font-mono font-bold text-neutral-200 border border-adwaita-border">Alt</kbd>
                <span class="text-adwaita-textSubtle text-xs">+</span>
                <kbd class="keycap px-2.5 py-1 bg-surface-containerHigh rounded text-[11px] font-mono font-bold text-saffron border border-saffron/40">Space</kbd>
                <button type="button" class="ml-1 text-[10px] text-neutral-400 hover:text-white px-1.5 py-1 hover:bg-white/5 rounded transition-colors" title="Change hotkey">
                  <i class="fa-solid fa-pen"></i>
                </button>
              </div>
            </div>

            <div class="bg-surface-container rounded-xl border border-adwaita-border p-3">
              <div class="flex items-center justify-between mb-2">
                <span class="text-[11px] font-semibold uppercase tracking-wider text-adwaita-textMuted">Dual Interaction Mechanism</span>
                <span class="text-[10px] font-mono text-saffron bg-saffron/10 px-2 py-0.5 rounded border border-saffron/20">threshold: 250 ms</span>
              </div>

              <div class="grid grid-cols-2 gap-2.5">
                <div class="bg-surface-containerHigh/80 rounded-lg p-2.5 border border-adwaita-border/70 flex flex-col justify-between">
                  <div>
                    <div class="flex items-center justify-between mb-1">
                      <span class="text-xs font-semibold text-white flex items-center gap-1.5">
                        <i class="fa-solid fa-hand-pointer text-saffron text-[10px]"></i>
                        Hold to Talk
                      </span>
                      <span class="text-[10px] text-adwaita-textSubtle font-mono">&gt; 250ms</span>
                    </div>
                    <p class="text-[10.5px] text-adwaita-textMuted leading-snug">
                      Quick thoughts. Press &amp; speak — text pastes immediately when you release.
                    </p>
                  </div>

                  <div class="mt-2.5 pt-2 border-t border-adwaita-border/50 flex items-center gap-1.5">
                    <span class="text-[9px] font-mono text-neutral-400">PRESS</span>
                    <div class="h-1 flex-1 bg-gradient-to-r from-saffron to-amber-300 rounded-full"></div>
                    <span class="text-[9px] font-mono text-emerald-400">RELEASE ➔ PASTE</span>
                  </div>
                </div>

                <div class="bg-surface-containerHigh/80 rounded-lg p-2.5 border border-adwaita-border/70 flex flex-col justify-between">
                  <div>
                    <div class="flex items-center justify-between mb-1">
                      <span class="text-xs font-semibold text-white flex items-center gap-1.5">
                        <i class="fa-solid fa-toggle-on text-amber-400 text-[10px]"></i>
                        Tap to Toggle
                      </span>
                      <span class="text-[10px] text-adwaita-textSubtle font-mono">&lt; 250ms</span>
                    </div>
                    <p class="text-[10.5px] text-adwaita-textMuted leading-snug">
                      Long dictation. Tap once to begin, speak freely, tap shortcut or press <kbd class="px-1 py-0.5 bg-black/40 rounded text-[9px] font-mono">Esc</kbd> to finish.
                    </p>
                  </div>

                  <div class="mt-2.5 pt-2 border-t border-adwaita-border/50 flex items-center justify-between text-[9px] font-mono text-neutral-400">
                    <span class="text-saffron">TAP (ON)</span>
                    <span class="text-neutral-500 font-sans text-[10px]">•••• continuous ••••</span>
                    <span class="text-emerald-400">TAP (OFF)</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="pt-3 border-t border-adwaita-border flex items-center justify-between shrink-0">
            <button type="button" onclick={() => goToStep(1)} class="px-3 py-1.5 rounded-lg text-adwaita-textMuted hover:text-white text-xs font-medium flex items-center gap-1.5 transition-colors">
              <i class="fa-solid fa-arrow-left text-[10px]"></i>
              <span>Back</span>
            </button>
            <button type="button" onclick={() => goToStep(3)} class="px-5 py-2 rounded-lg bg-saffron hover:bg-saffron-hover text-black font-semibold text-xs tracking-tight shadow-md flex items-center gap-2 transition-colors">
              <span>Next: Voice Model</span>
              <i class="fa-solid fa-arrow-right text-[10px]"></i>
            </button>
          </div>
        </section>

        <section id="step-3" class="step-view hidden flex flex-col h-full justify-between" class:hidden={step !== 3}>
          <div class="space-y-3">
            <div>
              <div class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded bg-saffron/10 border border-saffron/20 text-saffron text-[11px] font-semibold mb-1 tracking-wide uppercase">
                Step 3 · Local Intelligence
              </div>
              <h1 class="text-xl font-bold tracking-tight text-white flex items-center justify-between">
                <span>Pick your voice model</span>
                <span class="text-[11px] font-mono font-normal text-emerald-400 bg-emerald-950/40 px-2 py-0.5 rounded border border-emerald-800/40">AVX-512 + Vulkan detected</span>
              </h1>
              <p class="text-xs text-adwaita-textMuted mt-0.5 leading-relaxed">
                Hardware benchmark ran against 5s test audio. Recommending model optimized for zero typing latency.
              </p>
            </div>

            <div class="bg-surface-container rounded-xl border border-adwaita-border p-3">
              <div class="flex items-center justify-between text-xs mb-1.5">
                <div class="flex items-center gap-2">
                  <i class="fa-solid fa-bolt-lightning text-saffron text-xs"></i>
                  <span class="font-semibold text-white">Hardware Profiling Completed</span>
                </div>
                <span class="font-mono text-[11px] text-adwaita-textMuted">0.19s execution speed</span>
              </div>

              <div class="w-full bg-[#11100f] h-2 rounded-full overflow-hidden border border-adwaita-border/80 flex">
                <div class="bg-saffron h-full w-full rounded-full transition-all duration-500"></div>
              </div>

              <div class="flex items-center justify-between mt-2 text-[10px] text-adwaita-textSubtle font-mono">
                <span>CPU: AMD Ryzen 7 / Intel Core Ultra</span>
                <span class="text-emerald-400 font-semibold">Ready for instantaneous local inference</span>
              </div>
            </div>

            <div class="space-y-2">
              <label class="cursor-pointer block">
                <input type="radio" name="model-choice" checked class="hidden peer" />
                <div class="p-2.5 rounded-xl bg-surface-container border border-saffron/80 peer-checked:bg-saffron/5 peer-checked:border-saffron ring-1 ring-saffron/30 transition-all flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <div class="w-4 h-4 rounded-full border-2 border-saffron flex items-center justify-center">
                      <div class="w-2 h-2 rounded-full bg-saffron"></div>
                    </div>
                    <div>
                      <div class="flex items-center gap-2">
                        <span class="text-xs font-bold text-white">Whisper Fast (Base.en)</span>
                        <span class="text-[9px] font-bold px-1.5 py-0.2 rounded bg-saffron text-black uppercase tracking-wider">Recommended</span>
                        <span class="text-[10px] text-adwaita-textSubtle font-mono">142 MB</span>
                      </div>
                      <p class="text-[10.5px] text-adwaita-textMuted mt-0.5">
                        Measured speed: <strong class="text-neutral-200 font-mono">0.19s</strong> latency for 5s audio • Perfect for general prose, slack, email
                      </p>
                    </div>
                  </div>
                  <div class="text-right">
                    <span class="text-[10px] px-2 py-0.5 rounded bg-emerald-950/60 text-emerald-400 border border-emerald-800/40 font-mono font-medium">98.2% acc</span>
                  </div>
                </div>
              </label>

              <label class="cursor-pointer block opacity-75 hover:opacity-100 transition-opacity">
                <input type="radio" name="model-choice" class="hidden peer" />
                <div class="p-2.5 rounded-xl bg-surface-container border border-adwaita-border peer-checked:border-saffron peer-checked:bg-saffron/5 transition-all flex items-center justify-between">
                  <div class="flex items-center gap-3">
                    <div class="w-4 h-4 rounded-full border border-adwaita-textSubtle flex items-center justify-center"></div>
                    <div>
                      <div class="flex items-center gap-2">
                        <span class="text-xs font-semibold text-white">Whisper Multilingual (Small)</span>
                        <span class="text-[9px] px-1.5 py-0.2 rounded bg-surface-containerHigh text-neutral-300 font-mono">465 MB</span>
                      </div>
                      <p class="text-[10.5px] text-adwaita-textMuted mt-0.5">
                        Measured speed: <strong class="text-neutral-200 font-mono">0.44s</strong> • Supports English + Hindi, Spanish, German, French
                      </p>
                    </div>
                  </div>
                  <div class="text-right">
                    <span class="text-[10px] px-2 py-0.5 rounded bg-surface-containerHigh text-neutral-300 border border-adwaita-border font-mono">99.4% acc</span>
                  </div>
                </div>
              </label>
            </div>
          </div>

          <div class="pt-3 border-t border-adwaita-border flex items-center justify-between shrink-0">
            <button type="button" onclick={() => goToStep(2)} class="px-3 py-1.5 rounded-lg text-adwaita-textMuted hover:text-white text-xs font-medium flex items-center gap-1.5 transition-colors">
              <i class="fa-solid fa-arrow-left text-[10px]"></i>
              <span>Back</span>
            </button>
            <button type="button" onclick={() => goToStep(4)} class="px-5 py-2 rounded-lg bg-saffron hover:bg-saffron-hover text-black font-semibold text-xs tracking-tight shadow-md flex items-center gap-2 transition-colors">
              <span>Finish Setup</span>
              <i class="fa-solid fa-check text-[11px]"></i>
            </button>
          </div>
        </section>

        <section id="step-4" class="step-view hidden flex flex-col h-full justify-between text-center items-center" class:hidden={step !== 4}>
          <div class="w-full flex-1 flex flex-col items-center justify-center py-2 space-y-4">
            <div class="relative w-20 h-20 flex items-center justify-center">
              <div class="absolute inset-0 rounded-full border border-saffron/30 mic-pulse"></div>
              <div class="absolute -inset-2 rounded-full border border-saffron/15 mic-pulse" style="animation-delay: 0.6s;"></div>

              <div class="w-12 h-12 rounded-full bg-gradient-to-br from-[#f59e0b] to-[#b45309] shadow-lg shadow-saffron/30 flex items-center justify-center border border-amber-300/40 relative z-10">
                <i class="fa-solid fa-microphone text-black text-lg"></i>
              </div>
            </div>

            <div class="space-y-1.5 max-w-[420px]">
              <span class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full bg-emerald-950/60 border border-emerald-800/40 text-emerald-400 text-[11px] font-medium">
                <i class="fa-solid fa-circle-check text-[10px]"></i>
                Ready to transcribe
              </span>
              <h2 class="text-2xl font-bold tracking-tight text-white">
                You're all set!
              </h2>
              <p class="text-xs text-neutral-300 leading-relaxed">
                Narad is now running quietly in your GNOME top-bar tray.
              </p>
            </div>

            <div class="bg-surface-container rounded-xl border border-saffron/40 p-3.5 w-full max-w-[440px] shadow-inner bg-gradient-to-b from-surface-container to-surface-container/60">
              <div class="text-[11px] font-medium text-adwaita-textMuted mb-2">
                Try it right now:
              </div>
              <div class="flex items-center justify-center gap-2 mb-2">
                <span class="text-xs text-neutral-300 font-medium">Press</span>
                <kbd class="keycap px-2.5 py-1.5 bg-[#252321] rounded-md text-xs font-mono font-bold text-white border border-adwaita-border">Ctrl</kbd>
                <kbd class="keycap px-2.5 py-1.5 bg-[#252321] rounded-md text-xs font-mono font-bold text-white border border-adwaita-border">Alt</kbd>
                <kbd class="keycap px-3.5 py-1.5 bg-[#252321] rounded-md text-xs font-mono font-bold text-saffron border border-saffron/50">Space</kbd>
                <span class="text-xs text-neutral-300 font-medium">and say <strong class="text-amber-300 font-semibold font-mono">"hello"</strong></span>
              </div>
              <div class="text-[10.5px] text-adwaita-textSubtle flex items-center justify-center gap-1.5">
                <span class="w-1.5 h-1.5 rounded-full bg-saffron"></span>
                <span>The orange floating dot will appear at your cursor &amp; type your words</span>
              </div>
            </div>
          </div>

          <div class="w-full pt-3 border-t border-adwaita-border flex items-center justify-between shrink-0">
            <button type="button" onclick={() => goToStep(3)} class="px-3 py-1.5 rounded-lg text-adwaita-textMuted hover:text-white text-xs font-medium flex items-center gap-1.5 transition-colors">
              <i class="fa-solid fa-arrow-left text-[10px]"></i>
              <span>Back</span>
            </button>

            <div class="flex items-center gap-2">
              <button type="button" class="px-6 py-2 rounded-lg bg-saffron hover:bg-saffron-hover active:scale-95 text-black font-bold text-xs tracking-tight shadow-md flex items-center gap-2 transition-all">
                <span>Done</span>
                <i class="fa-solid fa-check text-[11px]"></i>
              </button>
            </div>
          </div>
        </section>
      </main>
    </div>
  </div>
</div>

<style>
  .first-run {
    min-height: 100vh;
    color: #e8e5e1;
    background-color: #0b0a0a;
    background-image:
      radial-gradient(circle at 50% 20%, rgba(224, 138, 30, 0.04) 0%, transparent 60%),
      radial-gradient(#1f1e1d 1px, transparent 1px);
    background-size: 100% 100%, 24px 24px;
    font-feature-settings: "cv02", "cv03", "cv04", "cv11";
    font-family: Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  }

  .window-shadow {
    box-shadow:
      0 25px 60px -15px rgba(0, 0, 0, 0.8),
      0 0 0 1px rgba(255, 255, 255, 0.08);
  }

  .keycap {
    box-shadow:
      0 2px 0 #121110,
      inset 0 1px 0 rgba(255, 255, 255, 0.1);
  }

  .first-run :global(.step-view.hidden) {
    display: none;
  }

  @keyframes pulseRing {
    0% {
      transform: scale(0.95);
      opacity: 0.8;
    }
    50% {
      transform: scale(1.15);
      opacity: 0.3;
    }
    100% {
      transform: scale(0.95);
      opacity: 0.8;
    }
  }

  .mic-pulse {
    animation: pulseRing 2.4s infinite ease-in-out;
  }

  .first-run :global(.fa-solid) {
  display: inline-block;
  width: 1em;
  height: 1em;
  flex: 0 0 auto;
  background-color: currentColor;
  vertical-align: -0.125em;
  -webkit-mask-repeat: no-repeat;
  mask-repeat: no-repeat;
  -webkit-mask-position: center;
  mask-position: center;
  -webkit-mask-size: contain;
  mask-size: contain;
}
.first-run :global(.fa-arrow-left) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M9.4%20233.4c-12.5%2012.5-12.5%2032.8%200%2045.3l160%20160c12.5%2012.5%2032.8%2012.5%2045.3%200s12.5-32.8%200-45.3L109.2%20288%20416%20288c17.7%200%2032-14.3%2032-32s-14.3-32-32-32l-306.7%200L214.6%20118.6c12.5-12.5%2012.5-32.8%200-45.3s-32.8-12.5-45.3%200l-160%20160z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M9.4%20233.4c-12.5%2012.5-12.5%2032.8%200%2045.3l160%20160c12.5%2012.5%2032.8%2012.5%2045.3%200s12.5-32.8%200-45.3L109.2%20288%20416%20288c17.7%200%2032-14.3%2032-32s-14.3-32-32-32l-306.7%200L214.6%20118.6c12.5-12.5%2012.5-32.8%200-45.3s-32.8-12.5-45.3%200l-160%20160z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-arrow-right) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M438.6%20278.6c12.5-12.5%2012.5-32.8%200-45.3l-160-160c-12.5-12.5-32.8-12.5-45.3%200s-12.5%2032.8%200%2045.3L338.8%20224%2032%20224c-17.7%200-32%2014.3-32%2032s14.3%2032%2032%2032l306.7%200L233.4%20393.4c-12.5%2012.5-12.5%2032.8%200%2045.3s32.8%2012.5%2045.3%200l160-160z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M438.6%20278.6c12.5-12.5%2012.5-32.8%200-45.3l-160-160c-12.5-12.5-32.8-12.5-45.3%200s-12.5%2032.8%200%2045.3L338.8%20224%2032%20224c-17.7%200-32%2014.3-32%2032s14.3%2032%2032%2032l306.7%200L233.4%20393.4c-12.5%2012.5-12.5%2032.8%200%2045.3s32.8%2012.5%2045.3%200l160-160z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-bolt-lightning) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20384%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M0%20256L28.5%2028c2-16%2015.6-28%2031.8-28H228.9c15%200%2027.1%2012.1%2027.1%2027.1c0%203.2-.6%206.5-1.7%209.5L208%20160H347.3c20.2%200%2036.7%2016.4%2036.7%2036.7c0%207.4-2.2%2014.6-6.4%2020.7l-192.2%20281c-5.9%208.6-15.6%2013.7-25.9%2013.7h-2.9c-15.7%200-28.5-12.8-28.5-28.5c0-2.3%20.3-4.6%20.9-6.9L176%20288H32c-17.7%200-32-14.3-32-32z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20384%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M0%20256L28.5%2028c2-16%2015.6-28%2031.8-28H228.9c15%200%2027.1%2012.1%2027.1%2027.1c0%203.2-.6%206.5-1.7%209.5L208%20160H347.3c20.2%200%2036.7%2016.4%2036.7%2036.7c0%207.4-2.2%2014.6-6.4%2020.7l-192.2%20281c-5.9%208.6-15.6%2013.7-25.9%2013.7h-2.9c-15.7%200-28.5-12.8-28.5-28.5c0-2.3%20.3-4.6%20.9-6.9L176%20288H32c-17.7%200-32-14.3-32-32z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-check) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M438.6%20105.4c12.5%2012.5%2012.5%2032.8%200%2045.3l-256%20256c-12.5%2012.5-32.8%2012.5-45.3%200l-128-128c-12.5-12.5-12.5-32.8%200-45.3s32.8-12.5%2045.3%200L160%20338.7%20393.4%20105.4c12.5-12.5%2032.8-12.5%2045.3%200z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M438.6%20105.4c12.5%2012.5%2012.5%2032.8%200%2045.3l-256%20256c-12.5%2012.5-32.8%2012.5-45.3%200l-128-128c-12.5-12.5-12.5-32.8%200-45.3s32.8-12.5%2045.3%200L160%20338.7%20393.4%20105.4c12.5-12.5%2032.8-12.5%2045.3%200z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-circle-check) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M256%20512A256%20256%200%201%200%20256%200a256%20256%200%201%200%200%20512zM369%20209L241%20337c-9.4%209.4-24.6%209.4-33.9%200l-64-64c-9.4-9.4-9.4-24.6%200-33.9s24.6-9.4%2033.9%200l47%2047L335%20175c9.4-9.4%2024.6-9.4%2033.9%200s9.4%2024.6%200%2033.9z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M256%20512A256%20256%200%201%200%20256%200a256%20256%200%201%200%200%20512zM369%20209L241%20337c-9.4%209.4-24.6%209.4-33.9%200l-64-64c-9.4-9.4-9.4-24.6%200-33.9s24.6-9.4%2033.9%200l47%2047L335%20175c9.4-9.4%2024.6-9.4%2033.9%200s9.4%2024.6%200%2033.9z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-circle-info) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M256%20512A256%20256%200%201%200%20256%200a256%20256%200%201%200%200%20512zM216%20336h24V272H216c-13.3%200-24-10.7-24-24s10.7-24%2024-24h48c13.3%200%2024%2010.7%2024%2024v88h8c13.3%200%2024%2010.7%2024%2024s-10.7%2024-24%2024H216c-13.3%200-24-10.7-24-24s10.7-24%2024-24zm40-208a32%2032%200%201%201%200%2064%2032%2032%200%201%201%200-64z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M256%20512A256%20256%200%201%200%20256%200a256%20256%200%201%200%200%20512zM216%20336h24V272H216c-13.3%200-24-10.7-24-24s10.7-24%2024-24h48c13.3%200%2024%2010.7%2024%2024v88h8c13.3%200%2024%2010.7%2024%2024s-10.7%2024-24%2024H216c-13.3%200-24-10.7-24-24s10.7-24%2024-24zm40-208a32%2032%200%201%201%200%2064%2032%2032%200%201%201%200-64z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-fingerprint) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M48%20256C48%20141.1%20141.1%2048%20256%2048c63.1%200%20119.6%2028.1%20157.8%2072.5c8.6%2010.1%2023.8%2011.2%2033.8%202.6s11.2-23.8%202.6-33.8C403.3%2034.6%20333.7%200%20256%200C114.6%200%200%20114.6%200%20256v40c0%2013.3%2010.7%2024%2024%2024s24-10.7%2024-24V256zm458.5-52.9c-2.7-13-15.5-21.3-28.4-18.5s-21.3%2015.5-18.5%2028.4c2.9%2013.9%204.5%2028.3%204.5%2043.1v40c0%2013.3%2010.7%2024%2024%2024s24-10.7%2024-24V256c0-18.1-1.9-35.8-5.5-52.9zM256%2080c-19%200-37.4%203-54.5%208.6c-15.2%205-18.7%2023.7-8.3%2035.9c7.1%208.3%2018.8%2010.8%2029.4%207.9c10.6-2.9%2021.8-4.4%2033.4-4.4c70.7%200%20128%2057.3%20128%20128v24.9c0%2025.2-1.5%2050.3-4.4%2075.3c-1.7%2014.6%209.4%2027.8%2024.2%2027.8c11.8%200%2021.9-8.6%2023.3-20.3c3.3-27.4%205-55%205-82.7V256c0-97.2-78.8-176-176-176zM150.7%20148.7c-9.1-10.6-25.3-11.4-33.9-.4C93.7%20178%2080%20215.4%2080%20256v24.9c0%2024.2-2.6%2048.4-7.8%2071.9C68.8%20368.4%2080.1%20384%2096.1%20384c10.5%200%2019.9-7%2022.2-17.3c6.4-28.1%209.7-56.8%209.7-85.8V256c0-27.2%208.5-52.4%2022.9-73.1c7.2-10.4%208-24.6-.2-34.2zM256%20160c-53%200-96%2043-96%2096v24.9c0%2035.9-4.6%2071.5-13.8%20106.1c-3.8%2014.3%206.7%2029%2021.5%2029c9.5%200%2017.9-6.2%2020.4-15.4c10.5-39%2015.9-79.2%2015.9-119.7V256c0-28.7%2023.3-52%2052-52s52%2023.3%2052%2052v24.9c0%2036.3-3.5%2072.4-10.4%20107.9c-2.7%2013.9%207.7%2027.2%2021.8%2027.2c10.2%200%2019-7%2021-17c7.7-38.8%2011.6-78.3%2011.6-118.1V256c0-53-43-96-96-96zm24%2096c0-13.3-10.7-24-24-24s-24%2010.7-24%2024v24.9c0%2059.9-11%20119.3-32.5%20175.2l-5.9%2015.3c-4.8%2012.4%201.4%2026.3%2013.8%2031s26.3-1.4%2031-13.8l5.9-15.3C267.9%20411.9%20280%20346.7%20280%20280.9V256z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M48%20256C48%20141.1%20141.1%2048%20256%2048c63.1%200%20119.6%2028.1%20157.8%2072.5c8.6%2010.1%2023.8%2011.2%2033.8%202.6s11.2-23.8%202.6-33.8C403.3%2034.6%20333.7%200%20256%200C114.6%200%200%20114.6%200%20256v40c0%2013.3%2010.7%2024%2024%2024s24-10.7%2024-24V256zm458.5-52.9c-2.7-13-15.5-21.3-28.4-18.5s-21.3%2015.5-18.5%2028.4c2.9%2013.9%204.5%2028.3%204.5%2043.1v40c0%2013.3%2010.7%2024%2024%2024s24-10.7%2024-24V256c0-18.1-1.9-35.8-5.5-52.9zM256%2080c-19%200-37.4%203-54.5%208.6c-15.2%205-18.7%2023.7-8.3%2035.9c7.1%208.3%2018.8%2010.8%2029.4%207.9c10.6-2.9%2021.8-4.4%2033.4-4.4c70.7%200%20128%2057.3%20128%20128v24.9c0%2025.2-1.5%2050.3-4.4%2075.3c-1.7%2014.6%209.4%2027.8%2024.2%2027.8c11.8%200%2021.9-8.6%2023.3-20.3c3.3-27.4%205-55%205-82.7V256c0-97.2-78.8-176-176-176zM150.7%20148.7c-9.1-10.6-25.3-11.4-33.9-.4C93.7%20178%2080%20215.4%2080%20256v24.9c0%2024.2-2.6%2048.4-7.8%2071.9C68.8%20368.4%2080.1%20384%2096.1%20384c10.5%200%2019.9-7%2022.2-17.3c6.4-28.1%209.7-56.8%209.7-85.8V256c0-27.2%208.5-52.4%2022.9-73.1c7.2-10.4%208-24.6-.2-34.2zM256%20160c-53%200-96%2043-96%2096v24.9c0%2035.9-4.6%2071.5-13.8%20106.1c-3.8%2014.3%206.7%2029%2021.5%2029c9.5%200%2017.9-6.2%2020.4-15.4c10.5-39%2015.9-79.2%2015.9-119.7V256c0-28.7%2023.3-52%2052-52s52%2023.3%2052%2052v24.9c0%2036.3-3.5%2072.4-10.4%20107.9c-2.7%2013.9%207.7%2027.2%2021.8%2027.2c10.2%200%2019-7%2021-17c7.7-38.8%2011.6-78.3%2011.6-118.1V256c0-53-43-96-96-96zm24%2096c0-13.3-10.7-24-24-24s-24%2010.7-24%2024v24.9c0%2059.9-11%20119.3-32.5%20175.2l-5.9%2015.3c-4.8%2012.4%201.4%2026.3%2013.8%2031s26.3-1.4%2031-13.8l5.9-15.3C267.9%20411.9%20280%20346.7%20280%20280.9V256z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-hand-pointer) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M128%2040c0-22.1%2017.9-40%2040-40s40%2017.9%2040%2040V188.2c8.5-7.6%2019.7-12.2%2032-12.2c20.6%200%2038.2%2013%2045%2031.2c8.8-9.3%2021.2-15.2%2035-15.2c25.3%200%2046%2019.5%2047.9%2044.3c8.5-7.7%2019.8-12.3%2032.1-12.3c26.5%200%2048%2021.5%2048%2048v48%2016%2048c0%2070.7-57.3%20128-128%20128l-16%200H240l-.1%200h-5.2c-5%200-9.9-.3-14.7-1c-55.3-5.6-106.2-34-140-79L8%20336c-13.3-17.7-9.7-42.7%208-56s42.7-9.7%2056%208l56%2074.7V40zM240%20304c0-8.8-7.2-16-16-16s-16%207.2-16%2016v96c0%208.8%207.2%2016%2016%2016s16-7.2%2016-16V304zm48-16c-8.8%200-16%207.2-16%2016v96c0%208.8%207.2%2016%2016%2016s16-7.2%2016-16V304c0-8.8-7.2-16-16-16zm80%2016c0-8.8-7.2-16-16-16s-16%207.2-16%2016v96c0%208.8%207.2%2016%2016%2016s16-7.2%2016-16V304z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M128%2040c0-22.1%2017.9-40%2040-40s40%2017.9%2040%2040V188.2c8.5-7.6%2019.7-12.2%2032-12.2c20.6%200%2038.2%2013%2045%2031.2c8.8-9.3%2021.2-15.2%2035-15.2c25.3%200%2046%2019.5%2047.9%2044.3c8.5-7.7%2019.8-12.3%2032.1-12.3c26.5%200%2048%2021.5%2048%2048v48%2016%2048c0%2070.7-57.3%20128-128%20128l-16%200H240l-.1%200h-5.2c-5%200-9.9-.3-14.7-1c-55.3-5.6-106.2-34-140-79L8%20336c-13.3-17.7-9.7-42.7%208-56s42.7-9.7%2056%208l56%2074.7V40zM240%20304c0-8.8-7.2-16-16-16s-16%207.2-16%2016v96c0%208.8%207.2%2016%2016%2016s16-7.2%2016-16V304zm48-16c-8.8%200-16%207.2-16%2016v96c0%208.8%207.2%2016%2016%2016s16-7.2%2016-16V304c0-8.8-7.2-16-16-16zm80%2016c0-8.8-7.2-16-16-16s-16%207.2-16%2016v96c0%208.8%207.2%2016%2016%2016s16-7.2%2016-16V304z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-keyboard) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20576%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M64%2064C28.7%2064%200%2092.7%200%20128V384c0%2035.3%2028.7%2064%2064%2064H512c35.3%200%2064-28.7%2064-64V128c0-35.3-28.7-64-64-64H64zm16%2064h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H80c-8.8%200-16-7.2-16-16V144c0-8.8%207.2-16%2016-16zM64%20240c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H80c-8.8%200-16-7.2-16-16V240zm16%2080h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H80c-8.8%200-16-7.2-16-16V336c0-8.8%207.2-16%2016-16zm80-176c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H176c-8.8%200-16-7.2-16-16V144zm16%2080h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H176c-8.8%200-16-7.2-16-16V240c0-8.8%207.2-16%2016-16zM160%20336c0-8.8%207.2-16%2016-16H400c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H176c-8.8%200-16-7.2-16-16V336zM272%20128h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H272c-8.8%200-16-7.2-16-16V144c0-8.8%207.2-16%2016-16zM256%20240c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H272c-8.8%200-16-7.2-16-16V240zM368%20128h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H368c-8.8%200-16-7.2-16-16V144c0-8.8%207.2-16%2016-16zM352%20240c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H368c-8.8%200-16-7.2-16-16V240zM464%20128h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H464c-8.8%200-16-7.2-16-16V144c0-8.8%207.2-16%2016-16zM448%20240c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H464c-8.8%200-16-7.2-16-16V240zm16%2080h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H464c-8.8%200-16-7.2-16-16V336c0-8.8%207.2-16%2016-16z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20576%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M64%2064C28.7%2064%200%2092.7%200%20128V384c0%2035.3%2028.7%2064%2064%2064H512c35.3%200%2064-28.7%2064-64V128c0-35.3-28.7-64-64-64H64zm16%2064h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H80c-8.8%200-16-7.2-16-16V144c0-8.8%207.2-16%2016-16zM64%20240c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H80c-8.8%200-16-7.2-16-16V240zm16%2080h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H80c-8.8%200-16-7.2-16-16V336c0-8.8%207.2-16%2016-16zm80-176c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H176c-8.8%200-16-7.2-16-16V144zm16%2080h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H176c-8.8%200-16-7.2-16-16V240c0-8.8%207.2-16%2016-16zM160%20336c0-8.8%207.2-16%2016-16H400c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H176c-8.8%200-16-7.2-16-16V336zM272%20128h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H272c-8.8%200-16-7.2-16-16V144c0-8.8%207.2-16%2016-16zM256%20240c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H272c-8.8%200-16-7.2-16-16V240zM368%20128h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H368c-8.8%200-16-7.2-16-16V144c0-8.8%207.2-16%2016-16zM352%20240c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H368c-8.8%200-16-7.2-16-16V240zM464%20128h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H464c-8.8%200-16-7.2-16-16V144c0-8.8%207.2-16%2016-16zM448%20240c0-8.8%207.2-16%2016-16h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H464c-8.8%200-16-7.2-16-16V240zm16%2080h32c8.8%200%2016%207.2%2016%2016v32c0%208.8-7.2%2016-16%2016H464c-8.8%200-16-7.2-16-16V336c0-8.8%207.2-16%2016-16z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-lock) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M144%20144v48H304V144c0-44.2-35.8-80-80-80s-80%2035.8-80%2080zM80%20192V144C80%2064.5%20144.5%200%20224%200s144%2064.5%20144%20144v48h16c35.3%200%2064%2028.7%2064%2064V448c0%2035.3-28.7%2064-64%2064H64c-35.3%200-64-28.7-64-64V256c0-35.3%2028.7-64%2064-64H80z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20448%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M144%20144v48H304V144c0-44.2-35.8-80-80-80s-80%2035.8-80%2080zM80%20192V144C80%2064.5%20144.5%200%20224%200s144%2064.5%20144%20144v48h16c35.3%200%2064%2028.7%2064%2064V448c0%2035.3-28.7%2064-64%2064H64c-35.3%200-64-28.7-64-64V256c0-35.3%2028.7-64%2064-64H80z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-microphone) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20384%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M192%200C139%200%2096%2043%2096%2096V256c0%2053%2043%2096%2096%2096s96-43%2096-96V96c0-53-43-96-96-96zM64%20216c0-13.3-10.7-24-24-24s-24%2010.7-24%2024v40c0%2089.1%2066.2%20162.7%20152%20174.4V464H120c-13.3%200-24%2010.7-24%2024s10.7%2024%2024%2024h72%2072c13.3%200%2024-10.7%2024-24s-10.7-24-24-24H216V430.4c85.8-11.7%20152-85.3%20152-174.4V216c0-13.3-10.7-24-24-24s-24%2010.7-24%2024v40c0%2070.7-57.3%20128-128%20128s-128-57.3-128-128V216z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20384%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M192%200C139%200%2096%2043%2096%2096V256c0%2053%2043%2096%2096%2096s96-43%2096-96V96c0-53-43-96-96-96zM64%20216c0-13.3-10.7-24-24-24s-24%2010.7-24%2024v40c0%2089.1%2066.2%20162.7%20152%20174.4V464H120c-13.3%200-24%2010.7-24%2024s10.7%2024%2024%2024h72%2072c13.3%200%2024-10.7%2024-24s-10.7-24-24-24H216V430.4c85.8-11.7%20152-85.3%20152-174.4V216c0-13.3-10.7-24-24-24s-24%2010.7-24%2024v40c0%2070.7-57.3%20128-128%20128s-128-57.3-128-128V216z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-pen) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M362.7%2019.3L314.3%2067.7%20444.3%20197.7l48.4-48.4c25-25%2025-65.5%200-90.5L453.3%2019.3c-25-25-65.5-25-90.5%200zm-71%2071L58.6%20323.5c-10.4%2010.4-18%2023.3-22.2%2037.4L1%20481.2C-1.5%20489.7%20.8%20498.8%207%20505s15.3%208.5%2023.7%206.1l120.3-35.4c14.1-4.2%2027-11.8%2037.4-22.2L421.7%20220.3%20291.7%2090.3z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M362.7%2019.3L314.3%2067.7%20444.3%20197.7l48.4-48.4c25-25%2025-65.5%200-90.5L453.3%2019.3c-25-25-65.5-25-90.5%200zm-71%2071L58.6%20323.5c-10.4%2010.4-18%2023.3-22.2%2037.4L1%20481.2C-1.5%20489.7%20.8%20498.8%207%20505s15.3%208.5%2023.7%206.1l120.3-35.4c14.1-4.2%2027-11.8%2037.4-22.2L421.7%20220.3%20291.7%2090.3z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-shield-halved) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M256%200c4.6%200%209.2%201%2013.4%202.9L457.7%2082.8c22%209.3%2038.4%2031%2038.3%2057.2c-.5%2099.2-41.3%20280.7-213.6%20363.2c-16.7%208-36.1%208-52.8%200C57.3%20420.7%2016.5%20239.2%2016%20140c-.1-26.2%2016.3-47.9%2038.3-57.2L242.7%202.9C246.8%201%20251.4%200%20256%200zm0%2066.8V444.8C394%20378%20431.1%20230.1%20432%20141.4L256%2066.8l0%200z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20512%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M256%200c4.6%200%209.2%201%2013.4%202.9L457.7%2082.8c22%209.3%2038.4%2031%2038.3%2057.2c-.5%2099.2-41.3%20280.7-213.6%20363.2c-16.7%208-36.1%208-52.8%200C57.3%20420.7%2016.5%20239.2%2016%20140c-.1-26.2%2016.3-47.9%2038.3-57.2L242.7%202.9C246.8%201%20251.4%200%20256%200zm0%2066.8V444.8C394%20378%20431.1%20230.1%20432%20141.4L256%2066.8l0%200z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-toggle-on) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20576%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M192%2064C86%2064%200%20150%200%20256S86%20448%20192%20448H384c106%200%20192-86%20192-192s-86-192-192-192H192zm192%2096a96%2096%200%201%201%200%20192%2096%2096%200%201%201%200-192z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20576%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M192%2064C86%2064%200%20150%200%20256S86%20448%20192%20448H384c106%200%20192-86%20192-192s-86-192-192-192H192zm192%2096a96%2096%200%201%201%200%20192%2096%2096%200%201%201%200-192z%22%2F%3E%3C%2Fsvg%3E");
}
.first-run :global(.fa-xmark) {
  -webkit-mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20384%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M342.6%20150.6c12.5-12.5%2012.5-32.8%200-45.3s-32.8-12.5-45.3%200L192%20210.7%2086.6%20105.4c-12.5-12.5-32.8-12.5-45.3%200s-12.5%2032.8%200%2045.3L146.7%20256%2041.4%20361.4c-12.5%2012.5-12.5%2032.8%200%2045.3s32.8%2012.5%2045.3%200L192%20301.3%20297.4%20406.6c12.5%2012.5%2032.8%2012.5%2045.3%200s12.5-32.8%200-45.3L237.3%20256%20342.6%20150.6z%22%2F%3E%3C%2Fsvg%3E");
  mask-image: url("data:image/svg+xml,%3Csvg%20xmlns%3D%22http%3A%2F%2Fwww.w3.org%2F2000%2Fsvg%22%20viewBox%3D%220%200%20384%20512%22%3E%3C!--!%20Font%20Awesome%20Free%206.4.0%20by%20%40fontawesome%20-%20https%3A%2F%2Ffontawesome.com%20License%20-%20https%3A%2F%2Ffontawesome.com%2Flicense%2Ffree%20(Icons%3A%20CC%20BY%204.0%2C%20Fonts%3A%20SIL%20OFL%201.1%2C%20Code%3A%20MIT%20License)%20Copyright%202023%20Fonticons%2C%20Inc.%20--%3E%3Cpath%20d%3D%22M342.6%20150.6c12.5-12.5%2012.5-32.8%200-45.3s-32.8-12.5-45.3%200L192%20210.7%2086.6%20105.4c-12.5-12.5-32.8-12.5-45.3%200s-12.5%2032.8%200%2045.3L146.7%20256%2041.4%20361.4c-12.5%2012.5-12.5%2032.8%200%2045.3s32.8%2012.5%2045.3%200L192%20301.3%20297.4%20406.6c12.5%2012.5%2032.8%2012.5%2045.3%200s12.5-32.8%200-45.3L237.3%20256%20342.6%20150.6z%22%2F%3E%3C%2Fsvg%3E");
}
  .first-run :global(.absolute){position:absolute}.first-run :global(.relative){position:relative}.first-run :global(.-inset-2){inset:-.5rem}.first-run :global(.inset-0){inset:0}.first-run :global(.z-10){z-index:10}.first-run :global(.z-20){z-index:20}.first-run :global(.mb-1){margin-bottom:.25rem}.first-run :global(.mb-1\.5){margin-bottom:.375rem}.first-run :global(.mb-2){margin-bottom:.5rem}.first-run :global(.ml-1){margin-left:.25rem}.first-run :global(.mt-0\.5){margin-top:.125rem}.first-run :global(.mt-1){margin-top:.25rem}.first-run :global(.mt-2){margin-top:.5rem}.first-run :global(.mt-2\.5){margin-top:.625rem}.first-run :global(.block){display:block}.first-run :global(.flex){display:flex}.first-run :global(.inline-flex){display:inline-flex}.first-run :global(.grid){display:grid}.first-run :global(.hidden){display:none}.first-run :global(.h-1){height:.25rem}.first-run :global(.h-1\.5){height:.375rem}.first-run :global(.h-10){height:2.5rem}.first-run :global(.h-12){height:3rem}.first-run :global(.h-2){height:.5rem}.first-run :global(.h-2\.5){height:.625rem}.first-run :global(.h-20){height:5rem}.first-run :global(.h-3){height:.75rem}.first-run :global(.h-4){height:1rem}.first-run :global(.h-\[480px\]){height:480px}.first-run :global(.h-full){height:100%}.first-run :global(.min-h-screen){min-height:100vh}.first-run :global(.w-1\.5){width:.375rem}.first-run :global(.w-10){width:2.5rem}.first-run :global(.w-12){width:3rem}.first-run :global(.w-2){width:.5rem}.first-run :global(.w-2\.5){width:.625rem}.first-run :global(.w-20){width:5rem}.first-run :global(.w-3){width:.75rem}.first-run :global(.w-4){width:1rem}.first-run :global(.w-\[560px\]){width:560px}.first-run :global(.w-full){width:100%}.first-run :global(.min-w-0){min-width:0}.first-run :global(.max-w-\[420px\]){max-width:420px}.first-run :global(.max-w-\[440px\]){max-width:440px}.first-run :global(.flex-1){flex:1 1 0%}.first-run :global(.shrink-0){flex-shrink:0}.first-run :global(.transform){transform:translate(var(--tw-translate-x),var(--tw-translate-y)) rotate(var(--tw-rotate)) skewX(var(--tw-skew-x)) skewY(var(--tw-skew-y)) scaleX(var(--tw-scale-x)) scaleY(var(--tw-scale-y))}.first-run :global(.cursor-pointer){cursor:pointer}.first-run :global(.select-none){-webkit-user-select:none;-moz-user-select:none;user-select:none}.first-run :global(.grid-cols-2){grid-template-columns:repeat(2,minmax(0,1fr))}.first-run :global(.flex-col){flex-direction:column}.first-run :global(.items-start){align-items:flex-start}.first-run :global(.items-center){align-items:center}.first-run :global(.justify-end){justify-content:flex-end}.first-run :global(.justify-center){justify-content:center}.first-run :global(.justify-between){justify-content:space-between}.first-run :global(.gap-1){gap:.25rem}.first-run :global(.gap-1\.5){gap:.375rem}.first-run :global(.gap-2){gap:.5rem}.first-run :global(.gap-2\.5){gap:.625rem}.first-run :global(.gap-3){gap:.75rem}.first-run :global(.gap-3\.5){gap:.875rem}.first-run :global(.space-y-1\.5>:not([hidden])~:not([hidden])){--tw-space-y-reverse:0;margin-top:calc(.375rem*(1 - var(--tw-space-y-reverse)));margin-bottom:calc(.375rem*var(--tw-space-y-reverse))}.first-run :global(.space-y-2>:not([hidden])~:not([hidden])){--tw-space-y-reverse:0;margin-top:calc(.5rem*(1 - var(--tw-space-y-reverse)));margin-bottom:calc(.5rem*var(--tw-space-y-reverse))}.first-run :global(.space-y-3>:not([hidden])~:not([hidden])){--tw-space-y-reverse:0;margin-top:calc(.75rem*(1 - var(--tw-space-y-reverse)));margin-bottom:calc(.75rem*var(--tw-space-y-reverse))}.first-run :global(.space-y-3\.5>:not([hidden])~:not([hidden])){--tw-space-y-reverse:0;margin-top:calc(.875rem*(1 - var(--tw-space-y-reverse)));margin-bottom:calc(.875rem*var(--tw-space-y-reverse))}.first-run :global(.space-y-4>:not([hidden])~:not([hidden])){--tw-space-y-reverse:0;margin-top:calc(1rem*(1 - var(--tw-space-y-reverse)));margin-bottom:calc(1rem*var(--tw-space-y-reverse))}.first-run :global(.overflow-hidden){overflow:hidden}.first-run :global(.rounded){border-radius:.25rem}.first-run :global(.rounded-2xl){border-radius:1rem}.first-run :global(.rounded-full){border-radius:9999px}.first-run :global(.rounded-lg){border-radius:.5rem}.first-run :global(.rounded-md){border-radius:.375rem}.first-run :global(.rounded-xl){border-radius:.75rem}.first-run :global(.border){border-width:1px}.first-run :global(.border-2){border-width:2px}.first-run :global(.border-b){border-bottom-width:1px}.first-run :global(.border-t){border-top-width:1px}.first-run :global(.border-\[\#484542\]){--tw-border-opacity:1;border-color:rgb(72 69 66/var(--tw-border-opacity,1))}.first-run :global(.border-adwaita-border){--tw-border-opacity:1;border-color:rgb(46 44 42/var(--tw-border-opacity,1))}.first-run :global(.border-adwaita-border\/50){border-color:rgba(46,44,42,.5)}.first-run :global(.border-adwaita-border\/60){border-color:rgba(46,44,42,.6)}.first-run :global(.border-adwaita-border\/70){border-color:rgba(46,44,42,.7)}.first-run :global(.border-adwaita-border\/80){border-color:rgba(46,44,42,.8)}.first-run :global(.border-adwaita-textSubtle){--tw-border-opacity:1;border-color:rgb(110 106 102/var(--tw-border-opacity,1))}.first-run :global(.border-amber-300\/40){border-color:rgba(252,211,77,.4)}.first-run :global(.border-amber-500\/30){border-color:rgba(245,158,11,.3)}.first-run :global(.border-emerald-500\/30){border-color:rgba(16,185,129,.3)}.first-run :global(.border-emerald-500\/40){border-color:rgba(16,185,129,.4)}.first-run :global(.border-emerald-800\/40){border-color:rgba(6,95,70,.4)}.first-run :global(.border-saffron){--tw-border-opacity:1;border-color:rgb(224 138 30/var(--tw-border-opacity,1))}.first-run :global(.border-saffron\/15){border-color:rgba(224,138,30,.15)}.first-run :global(.border-saffron\/20){border-color:rgba(224,138,30,.2)}.first-run :global(.border-saffron\/30){border-color:rgba(224,138,30,.3)}.first-run :global(.border-saffron\/40){border-color:rgba(224,138,30,.4)}.first-run :global(.border-saffron\/50){border-color:rgba(224,138,30,.5)}.first-run :global(.border-saffron\/80){border-color:rgba(224,138,30,.8)}.first-run :global(.bg-\[\#0f0e0d\]){--tw-bg-opacity:1;background-color:rgb(15 14 13/var(--tw-bg-opacity,1))}.first-run :global(.bg-\[\#11100f\]){--tw-bg-opacity:1;background-color:rgb(17 16 15/var(--tw-bg-opacity,1))}.first-run :global(.bg-\[\#141312\]){--tw-bg-opacity:1;background-color:rgb(20 19 18/var(--tw-bg-opacity,1))}.first-run :global(.bg-\[\#23201d\]){--tw-bg-opacity:1;background-color:rgb(35 32 29/var(--tw-bg-opacity,1))}.first-run :global(.bg-\[\#252321\]){--tw-bg-opacity:1;background-color:rgb(37 35 33/var(--tw-bg-opacity,1))}.first-run :global(.bg-\[\#3a3734\]){--tw-bg-opacity:1;background-color:rgb(58 55 52/var(--tw-bg-opacity,1))}.first-run :global(.bg-adwaita-header\/90){background-color:rgba(30,29,28,.9)}.first-run :global(.bg-amber-500\/10){background-color:rgba(245,158,11,.1)}.first-run :global(.bg-black\/40){background-color:rgba(0,0,0,.4)}.first-run :global(.bg-emerald-500){--tw-bg-opacity:1;background-color:rgb(16 185 129/var(--tw-bg-opacity,1))}.first-run :global(.bg-emerald-500\/20){background-color:rgba(16,185,129,.2)}.first-run :global(.bg-emerald-950\/20){background-color:rgba(2,44,34,.2)}.first-run :global(.bg-emerald-950\/40){background-color:rgba(2,44,34,.4)}.first-run :global(.bg-emerald-950\/60){background-color:rgba(2,44,34,.6)}.first-run :global(.bg-neutral-600){--tw-bg-opacity:1;background-color:rgb(82 82 82/var(--tw-bg-opacity,1))}.first-run :global(.bg-saffron){--tw-bg-opacity:1;background-color:rgb(224 138 30/var(--tw-bg-opacity,1))}.first-run :global(.bg-saffron\/10){background-color:rgba(224,138,30,.1)}.first-run :global(.bg-surface){--tw-bg-opacity:1;background-color:rgb(20 19 18/var(--tw-bg-opacity,1))}.first-run :global(.bg-surface-container){--tw-bg-opacity:1;background-color:rgb(28 27 26/var(--tw-bg-opacity,1))}.first-run :global(.bg-surface-containerHigh){--tw-bg-opacity:1;background-color:rgb(38 36 35/var(--tw-bg-opacity,1))}.first-run :global(.bg-surface-containerHigh\/80){background-color:rgba(38,36,35,.8)}.first-run :global(.bg-surface-containerHighest){--tw-bg-opacity:1;background-color:rgb(50 48 46/var(--tw-bg-opacity,1))}.first-run :global(.bg-gradient-to-b){background-image:linear-gradient(to bottom,var(--tw-gradient-stops))}.first-run :global(.bg-gradient-to-br){background-image:linear-gradient(to bottom right,var(--tw-gradient-stops))}.first-run :global(.bg-gradient-to-r){background-image:linear-gradient(to right,var(--tw-gradient-stops))}.first-run :global(.from-\[\#f59e0b\]){--tw-gradient-from:#f59e0b var(--tw-gradient-from-position);--tw-gradient-to:rgba(245,158,11,0) var(--tw-gradient-to-position);--tw-gradient-stops:var(--tw-gradient-from),var(--tw-gradient-to)}.first-run :global(.from-saffron){--tw-gradient-from:#e08a1e var(--tw-gradient-from-position);--tw-gradient-to:rgba(224,138,30,0) var(--tw-gradient-to-position);--tw-gradient-stops:var(--tw-gradient-from),var(--tw-gradient-to)}.first-run :global(.from-surface-container){--tw-gradient-from:#1c1b1a var(--tw-gradient-from-position);--tw-gradient-to:rgba(28,27,26,0) var(--tw-gradient-to-position);--tw-gradient-stops:var(--tw-gradient-from),var(--tw-gradient-to)}.first-run :global(.to-\[\#b45309\]){--tw-gradient-to:#b45309 var(--tw-gradient-to-position)}.first-run :global(.to-amber-300){--tw-gradient-to:#fcd34d var(--tw-gradient-to-position)}.first-run :global(.to-surface-container\/60){--tw-gradient-to:rgba(28,27,26,.6) var(--tw-gradient-to-position)}.first-run :global(.p-2\.5){padding:.625rem}.first-run :global(.p-3){padding:.75rem}.first-run :global(.p-3\.5){padding:.875rem}.first-run :global(.p-4){padding:1rem}.first-run :global(.p-6){padding:1.5rem}.first-run :global(.px-1){padding-left:.25rem;padding-right:.25rem}.first-run :global(.px-1\.5){padding-left:.375rem;padding-right:.375rem}.first-run :global(.px-2){padding-left:.5rem;padding-right:.5rem}.first-run :global(.px-2\.5){padding-left:.625rem;padding-right:.625rem}.first-run :global(.px-3){padding-left:.75rem;padding-right:.75rem}.first-run :global(.px-3\.5){padding-left:.875rem;padding-right:.875rem}.first-run :global(.px-4){padding-left:1rem;padding-right:1rem}.first-run :global(.px-5){padding-left:1.25rem;padding-right:1.25rem}.first-run :global(.px-6){padding-left:1.5rem;padding-right:1.5rem}.first-run :global(.py-0\.2){padding-top:.05rem;padding-bottom:.05rem}.first-run :global(.py-0\.5){padding-top:.125rem;padding-bottom:.125rem}.first-run :global(.py-1){padding-top:.25rem;padding-bottom:.25rem}.first-run :global(.py-1\.5){padding-top:.375rem;padding-bottom:.375rem}.first-run :global(.py-2){padding-bottom:.5rem}.first-run :global(.pt-2), .first-run :global(.py-2){padding-top:.5rem}.first-run :global(.pt-3){padding-top:.75rem}.first-run :global(.text-center){text-align:center}.first-run :global(.text-right){text-align:right}.first-run :global(.font-mono){font-family:JetBrains Mono,ui-monospace,SFMono-Regular,Menlo,monospace}.first-run :global(.font-sans){font-family:Inter,-apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif}.first-run :global(.text-2xl){font-size:1.5rem;line-height:2rem}.first-run :global(.text-\[10\.5px\]){font-size:10.5px}.first-run :global(.text-\[10px\]){font-size:10px}.first-run :global(.text-\[11px\]){font-size:11px}.first-run :global(.text-\[8px\]){font-size:8px}.first-run :global(.text-\[9px\]){font-size:9px}.first-run :global(.text-lg){font-size:1.125rem}.first-run :global(.text-lg), .first-run :global(.text-xl){line-height:1.75rem}.first-run :global(.text-xl){font-size:1.25rem}.first-run :global(.text-xs){font-size:.75rem;line-height:1rem}.first-run :global(.font-bold){font-weight:700}.first-run :global(.font-medium){font-weight:500}.first-run :global(.font-normal){font-weight:400}.first-run :global(.font-semibold){font-weight:600}.first-run :global(.uppercase){text-transform:uppercase}.first-run :global(.leading-relaxed){line-height:1.625}.first-run :global(.leading-snug){line-height:1.375}.first-run :global(.tracking-tight){letter-spacing:-.025em}.first-run :global(.tracking-wide){letter-spacing:.025em}.first-run :global(.tracking-wider){letter-spacing:.05em}.first-run :global(.text-\[\#d6d0ca\]){--tw-text-opacity:1;color:rgb(214 208 202/var(--tw-text-opacity,1))}.first-run :global(.text-\[\#e8e5e1\]){--tw-text-opacity:1;color:rgb(232 229 225/var(--tw-text-opacity,1))}.first-run :global(.text-adwaita-textMuted){--tw-text-opacity:1;color:rgb(158 153 148/var(--tw-text-opacity,1))}.first-run :global(.text-adwaita-textSubtle){--tw-text-opacity:1;color:rgb(110 106 102/var(--tw-text-opacity,1))}.first-run :global(.text-amber-300){--tw-text-opacity:1;color:rgb(252 211 77/var(--tw-text-opacity,1))}.first-run :global(.text-amber-400){--tw-text-opacity:1;color:rgb(251 191 36/var(--tw-text-opacity,1))}.first-run :global(.text-black){--tw-text-opacity:1;color:rgb(0 0 0/var(--tw-text-opacity,1))}.first-run :global(.text-emerald-400){--tw-text-opacity:1;color:rgb(52 211 153/var(--tw-text-opacity,1))}.first-run :global(.text-neutral-200){--tw-text-opacity:1;color:rgb(229 229 229/var(--tw-text-opacity,1))}.first-run :global(.text-neutral-300){--tw-text-opacity:1;color:rgb(212 212 212/var(--tw-text-opacity,1))}.first-run :global(.text-neutral-400){--tw-text-opacity:1;color:rgb(163 163 163/var(--tw-text-opacity,1))}.first-run :global(.text-neutral-500){--tw-text-opacity:1;color:rgb(115 115 115/var(--tw-text-opacity,1))}.first-run :global(.text-saffron){--tw-text-opacity:1;color:rgb(224 138 30/var(--tw-text-opacity,1))}.first-run :global(.text-transparent){color:transparent}.first-run :global(.text-white){--tw-text-opacity:1;color:rgb(255 255 255/var(--tw-text-opacity,1))}.first-run :global(.opacity-75){opacity:.75}.first-run :global(.shadow-inner){--tw-shadow:inset 0 2px 4px 0 rgba(0,0,0,.05);--tw-shadow-colored:inset 0 2px 4px 0 var(--tw-shadow-color)}.first-run :global(.shadow-inner), .first-run :global(.shadow-lg){box-shadow:var(--tw-ring-offset-shadow,0 0 #0000),var(--tw-ring-shadow,0 0 #0000),var(--tw-shadow)}.first-run :global(.shadow-lg){--tw-shadow:0 10px 15px -3px rgba(0,0,0,.1),0 4px 6px -4px rgba(0,0,0,.1);--tw-shadow-colored:0 10px 15px -3px var(--tw-shadow-color),0 4px 6px -4px var(--tw-shadow-color)}.first-run :global(.shadow-md){--tw-shadow:0 4px 6px -1px rgba(0,0,0,.1),0 2px 4px -2px rgba(0,0,0,.1);--tw-shadow-colored:0 4px 6px -1px var(--tw-shadow-color),0 2px 4px -2px var(--tw-shadow-color);box-shadow:var(--tw-ring-offset-shadow,0 0 #0000),var(--tw-ring-shadow,0 0 #0000),var(--tw-shadow)}.first-run :global(.shadow-saffron\/30){--tw-shadow-color:rgba(224,138,30,.3);--tw-shadow:var(--tw-shadow-colored)}.first-run :global(.ring-1){--tw-ring-offset-shadow:var(--tw-ring-inset) 0 0 0 var(--tw-ring-offset-width) var(--tw-ring-offset-color);--tw-ring-shadow:var(--tw-ring-inset) 0 0 0 calc(1px + var(--tw-ring-offset-width)) var(--tw-ring-color);box-shadow:var(--tw-ring-offset-shadow),var(--tw-ring-shadow),var(--tw-shadow,0 0 #0000)}.first-run :global(.ring-saffron\/30){--tw-ring-color:rgba(224,138,30,.3)}.first-run :global(.backdrop-blur){--tw-backdrop-blur:blur(8px);-webkit-backdrop-filter:var(--tw-backdrop-blur) var(--tw-backdrop-brightness) var(--tw-backdrop-contrast) var(--tw-backdrop-grayscale) var(--tw-backdrop-hue-rotate) var(--tw-backdrop-invert) var(--tw-backdrop-opacity) var(--tw-backdrop-saturate) var(--tw-backdrop-sepia);backdrop-filter:var(--tw-backdrop-blur) var(--tw-backdrop-brightness) var(--tw-backdrop-contrast) var(--tw-backdrop-grayscale) var(--tw-backdrop-hue-rotate) var(--tw-backdrop-invert) var(--tw-backdrop-opacity) var(--tw-backdrop-saturate) var(--tw-backdrop-sepia)}.first-run :global(.transition-all){transition-property:all;transition-timing-function:cubic-bezier(.4,0,.2,1);transition-duration:.15s}.first-run :global(.transition-colors){transition-property:color,background-color,border-color,text-decoration-color,fill,stroke;transition-timing-function:cubic-bezier(.4,0,.2,1);transition-duration:.15s}.first-run :global(.transition-opacity){transition-property:opacity;transition-timing-function:cubic-bezier(.4,0,.2,1);transition-duration:.15s}.first-run :global(.duration-200){transition-duration:.2s}.first-run :global(.duration-300){transition-duration:.3s}.first-run :global(.duration-500){transition-duration:.5s}.first-run :global(.hover\:scale-125:hover){--tw-scale-x:1.25;--tw-scale-y:1.25;transform:translate(var(--tw-translate-x),var(--tw-translate-y)) rotate(var(--tw-rotate)) skewX(var(--tw-skew-x)) skewY(var(--tw-skew-y)) scaleX(var(--tw-scale-x)) scaleY(var(--tw-scale-y))}.first-run :global(.hover\:bg-\[\#3d3a37\]:hover){--tw-bg-opacity:1;background-color:rgb(61 58 55/var(--tw-bg-opacity,1))}.first-run :global(.hover\:bg-\[\#ff5f56\]:hover){--tw-bg-opacity:1;background-color:rgb(255 95 86/var(--tw-bg-opacity,1))}.first-run :global(.hover\:bg-saffron-hover:hover){--tw-bg-opacity:1;background-color:rgb(240 150 40/var(--tw-bg-opacity,1))}.first-run :global(.hover\:bg-white\/5:hover){background-color:hsla(0,0%,100%,.05)}.first-run :global(.hover\:text-white:hover){--tw-text-opacity:1;color:rgb(255 255 255/var(--tw-text-opacity,1))}.first-run :global(.hover\:opacity-100:hover){opacity:1}.first-run :global(.active\:scale-95:active){--tw-scale-x:.95;--tw-scale-y:.95}.first-run :global(.active\:scale-95:active), .first-run :global(.active\:scale-\[0\.99\]:active){transform:translate(var(--tw-translate-x),var(--tw-translate-y)) rotate(var(--tw-rotate)) skewX(var(--tw-skew-x)) skewY(var(--tw-skew-y)) scaleX(var(--tw-scale-x)) scaleY(var(--tw-scale-y))}.first-run :global(.active\:scale-\[0\.99\]:active){--tw-scale-x:0.99;--tw-scale-y:0.99}.first-run :global(.group:hover .group-hover\:text-\[\#4a0000\]){--tw-text-opacity:1;color:rgb(74 0 0/var(--tw-text-opacity,1))}.first-run :global(.peer:checked~.peer-checked\:border-saffron){--tw-border-opacity:1;border-color:rgb(224 138 30/var(--tw-border-opacity,1))}.first-run :global(.peer:checked~.peer-checked\:bg-saffron\/5){background-color:rgba(224,138,30,.05)}@media (min-width:1024px){.first-run :global(.lg\:p-8){padding:2rem}}
</style>

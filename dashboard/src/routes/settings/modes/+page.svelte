<script lang="ts">
  let modeLabel = $state("Clean");
  let spokenPrefix = $state("clean:");
  let prompt = $state(
    "Remove hesitation sounds ('um', 'uh', 'like'), clean false starts, fix minor grammatical agreement while strictly preserving technical vocabulary and original intent.",
  );
  let acoustic = $state("en-us");
  let translation = $state("original");
  let neural = $state(true);
  let hesitation = $state(true);
  let syntax = $state(false);
  let capitalization = $state(true);
</script>

{#snippet pipelineSwitch(on: boolean, label: string, toggle: () => void)}
  <button
    aria-checked={on}
    aria-label={label}
    class="w-11 h-6 rounded-full p-0.5 flex items-center transition-colors focus:outline-none {on
      ? 'bg-primary-container justify-end'
      : 'bg-surface-container-highest justify-start'}"
    role="switch"
    onclick={toggle}
  >
    <span
      class="w-5 h-5 rounded-full shadow {on
        ? 'bg-on-primary-container'
        : 'bg-on-surface-variant'}"
    ></span>
  </button>
{/snippet}

<div
  class="grid grid-cols-12 flex-1 min-w-0 min-h-[calc(100vh-3.5rem)] bg-surface text-on-surface divide-x divide-outline-variant/30"
>
  <div
    class="col-span-12 md:col-span-5 lg:col-span-4 flex flex-col bg-surface-container-low border-r border-outline-variant/30 overflow-y-auto"
  >
    <div
      class="h-14 px-space-md border-b border-outline-variant/30 flex items-center justify-between shrink-0 bg-surface-container-low/80 backdrop-blur sticky top-0 z-10"
    >
      <div>
        <h2 class="font-headline-md text-headline-md text-on-surface">Modes</h2>
        <p class="font-body-muted text-body-muted text-on-surface-variant -mt-0.5">
          Post-processing rules
        </p>
      </div>
      <button
        class="flex items-center gap-1 px-space-sm py-1 rounded-lg bg-surface-container hover:bg-surface-container-high border border-outline-variant/40 text-on-surface font-body-muted text-body-muted transition-colors"
        title="Add new mode"
        type="button"
      >
        <span class="material-symbols-outlined text-[16px]">add</span>
        <span>New</span>
      </button>
    </div>
    <div class="p-space-sm space-y-1.5">
      <div
        class="group flex items-start gap-space-sm p-space-sm rounded-xl hover:bg-surface-container/70 cursor-pointer border border-transparent transition-all"
      >
        <div
          class="w-8 h-8 rounded-lg bg-surface-container flex items-center justify-center shrink-0 mt-0.5 text-on-surface-variant"
        >
          <span class="material-symbols-outlined text-[18px]">text_fields</span>
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between gap-1">
            <span class="font-headline-md text-headline-md text-on-surface truncate">Plain</span>
            <span
              class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-1 py-0.5 rounded border border-outline-variant/30 shrink-0"
              >Ctrl+Alt+1</span
            >
          </div>
          <p class="font-body-muted text-body-muted text-on-surface-variant truncate mt-0.5">
            Direct acoustic tokens, 0ms latency
          </p>
        </div>
      </div>
      <div
        class="relative flex items-start gap-space-sm p-space-sm rounded-xl bg-surface-container-high border border-primary/40 shadow-sm cursor-pointer transition-all"
      >
        <div class="absolute left-0 top-3 bottom-3 w-1 bg-primary rounded-r"></div>
        <div
          class="w-8 h-8 rounded-lg bg-primary-container/20 text-primary flex items-center justify-center shrink-0 mt-0.5"
        >
          <span class="material-symbols-outlined text-[18px]">auto_fix_high</span>
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between gap-1">
            <span class="font-headline-md text-headline-md text-on-surface font-semibold truncate"
              >Clean</span
            >
            <div class="flex items-center gap-1 shrink-0">
              <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
              <span
                class="font-keycap text-keycap text-on-surface bg-surface-container-highest px-1.5 py-0.5 rounded border border-outline-variant/50"
                >Ctrl+Alt+2</span
              >
            </div>
          </div>
          <p class="font-body-muted text-body-muted text-on-surface-variant truncate mt-0.5">
            Filler removal & light grammar
          </p>
        </div>
      </div>
      <div
        class="group flex items-start gap-space-sm p-space-sm rounded-xl hover:bg-surface-container/70 cursor-pointer border border-transparent transition-all"
      >
        <div
          class="w-8 h-8 rounded-lg bg-surface-container flex items-center justify-center shrink-0 mt-0.5 text-on-surface-variant"
        >
          <span class="material-symbols-outlined text-[18px]">mail</span>
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between gap-1">
            <span class="font-headline-md text-headline-md text-on-surface truncate">Email</span>
            <span
              class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-1 py-0.5 rounded border border-outline-variant/30 shrink-0"
              >Ctrl+Alt+3</span
            >
          </div>
          <p class="font-body-muted text-body-muted text-on-surface-variant truncate mt-0.5">
            Polite sign-off, structured body
          </p>
        </div>
      </div>
      <div
        class="group flex items-start gap-space-sm p-space-sm rounded-xl hover:bg-surface-container/70 cursor-pointer border border-transparent transition-all"
      >
        <div
          class="w-8 h-8 rounded-lg bg-surface-container flex items-center justify-center shrink-0 mt-0.5 text-on-surface-variant"
        >
          <span class="material-symbols-outlined text-[18px]">format_list_bulleted</span>
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between gap-1">
            <span class="font-headline-md text-headline-md text-on-surface truncate">Notes</span>
            <span
              class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-1 py-0.5 rounded border border-outline-variant/30 shrink-0"
              >Ctrl+Alt+4</span
            >
          </div>
          <p class="font-body-muted text-body-muted text-on-surface-variant truncate mt-0.5">
            Bullet formatting, bullet syntax
          </p>
        </div>
      </div>
      <div
        class="group flex items-start gap-space-sm p-space-sm rounded-xl hover:bg-surface-container/70 cursor-pointer border border-transparent transition-all"
      >
        <div
          class="w-8 h-8 rounded-lg bg-surface-container flex items-center justify-center shrink-0 mt-0.5 text-on-surface-variant"
        >
          <span class="material-symbols-outlined text-[18px]">chat_bubble_outline</span>
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between gap-1">
            <span class="font-headline-md text-headline-md text-on-surface truncate">Message</span>
            <span
              class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-1 py-0.5 rounded border border-outline-variant/30 shrink-0"
              >Ctrl+Alt+5</span
            >
          </div>
          <p class="font-body-muted text-body-muted text-on-surface-variant truncate mt-0.5">
            Casual cadence, inline emoji parse
          </p>
        </div>
      </div>
      <div
        class="group flex items-start gap-space-sm p-space-sm rounded-xl hover:bg-surface-container/70 cursor-pointer border border-transparent transition-all"
      >
        <div
          class="w-8 h-8 rounded-lg bg-surface-container flex items-center justify-center shrink-0 mt-0.5 text-on-surface-variant"
        >
          <span class="material-symbols-outlined text-[18px]">code</span>
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between gap-1">
            <span class="font-headline-md text-headline-md text-on-surface truncate">Code</span>
            <span
              class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-1 py-0.5 rounded border border-outline-variant/30 shrink-0"
              >Ctrl+Alt+6</span
            >
          </div>
          <p class="font-body-muted text-body-muted text-on-surface-variant truncate mt-0.5">
            camelCase, snake_case syntax cues
          </p>
        </div>
      </div>
      <div
        class="group flex items-start gap-space-sm p-space-sm rounded-xl hover:bg-surface-container/70 cursor-pointer border border-transparent transition-all"
      >
        <div
          class="w-8 h-8 rounded-lg bg-surface-container flex items-center justify-center shrink-0 mt-0.5 text-on-surface-variant"
        >
          <span class="material-symbols-outlined text-[18px]">translate</span>
        </div>
        <div class="flex-1 min-w-0">
          <div class="flex items-center justify-between gap-1">
            <span class="font-headline-md text-headline-md text-on-surface truncate"
              >Translate → Hindi</span
            >
            <span
              class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-1 py-0.5 rounded border border-outline-variant/30 shrink-0"
              >Ctrl+Alt+7</span
            >
          </div>
          <p class="font-body-muted text-body-muted text-on-surface-variant truncate mt-0.5">
            IndicWav2Vec translation pipeline
          </p>
        </div>
      </div>
    </div>
  </div>

  <div class="col-span-12 md:col-span-7 lg:col-span-8 flex flex-col bg-surface overflow-y-auto">
    <div
      class="h-14 px-space-lg border-b border-outline-variant/30 flex items-center justify-between shrink-0 bg-surface/90 backdrop-blur sticky top-0 z-20"
    >
      <div class="flex items-center gap-space-sm">
        <div
          class="w-7 h-7 rounded-lg bg-primary-container/20 text-primary flex items-center justify-center"
        >
          <span class="material-symbols-outlined text-[18px]">auto_fix_high</span>
        </div>
        <div>
          <span class="font-headline-md text-headline-md text-on-surface font-semibold"
            >Clean Mode</span
          >
          <span
            class="ml-2 font-keycap text-keycap px-1.5 py-0.5 rounded bg-surface-container-high border border-outline-variant/40 text-on-surface-variant"
            >Default System Mode</span
          >
        </div>
      </div>
      <div class="flex items-center gap-space-sm">
        <button
          class="px-space-md py-1.5 rounded-lg font-body-default text-body-default text-on-surface-variant hover:text-on-surface hover:bg-surface-container transition-colors"
          type="button"
        >
          Reset
        </button>
        <button
          class="flex items-center gap-1.5 px-space-md py-1.5 rounded-lg bg-primary-container text-on-primary-container font-headline-md text-headline-md hover:bg-primary transition-colors shadow-sm"
          type="button"
        >
          <span class="material-symbols-outlined text-[16px]">check</span>
          <span>Save Changes</span>
        </button>
      </div>
    </div>
    <div class="p-space-lg max-w-3xl w-full mx-auto space-y-space-lg">
      <div
        class="p-space-md rounded-xl bg-surface-container-low border border-outline-variant/40 flex items-center justify-between gap-space-md"
      >
        <div class="flex items-center gap-space-sm">
          <div
            class="w-8 h-8 rounded-lg bg-primary/10 text-primary flex items-center justify-center shrink-0"
          >
            <span class="material-symbols-outlined text-[18px]">memory</span>
          </div>
          <div>
            <div class="flex items-center gap-2">
              <span class="font-headline-md text-headline-md text-on-surface">Local LLM Engine</span>
              <span
                class="font-label-code text-label-code px-1.5 py-0.2 rounded bg-surface-container-high text-primary border border-primary/30"
                >Q4_K_M GGUF</span
              >
            </div>
            <p class="font-body-muted text-body-muted text-on-surface-variant">
              Adds ~1.5s post-capture latency · Zero network calls
            </p>
          </div>
        </div>
        <div
          class="flex items-center gap-1.5 px-space-sm py-1 rounded bg-surface-container border border-outline-variant/30 text-emerald-400 font-label-code text-label-code shrink-0"
        >
          <span class="material-symbols-outlined text-[14px]">lock</span>
          <span>100% Offline</span>
        </div>
      </div>

      <div class="space-y-space-xs">
        <div class="px-space-xs flex items-center justify-between">
          <span class="font-headline-md text-headline-md text-on-surface">Identity & Triggers</span>
          <span class="font-body-muted text-body-muted text-on-surface-variant">Invocation keys</span>
        </div>
        <div
          class="rounded-xl bg-surface-container-low border border-outline-variant/30 divide-y divide-outline-variant/20 overflow-hidden"
        >
          <div class="p-space-md flex items-center justify-between gap-space-md">
            <div>
              <label class="font-body-default text-body-default text-on-surface block" for="mode-name"
                >Mode Label</label
              >
              <span class="font-body-muted text-body-muted text-on-surface-variant"
                >Appears in OSD bar and status indicator</span
              >
            </div>
            <input
              class="w-48 bg-surface-container px-space-sm py-1.5 rounded-lg border border-outline-variant/40 text-on-surface font-body-default text-body-default focus:border-primary focus:outline-none transition-colors"
              id="mode-name"
              type="text"
              bind:value={modeLabel}
            />
          </div>
          <div class="p-space-md flex items-center justify-between gap-space-md">
            <div>
              <span class="font-body-default text-body-default text-on-surface block">Direct Hotkey</span>
              <span class="font-body-muted text-body-muted text-on-surface-variant"
                >Switches to this mode and initiates transcription</span
              >
            </div>
            <div class="flex items-center gap-space-sm">
              <div class="flex items-center gap-1">
                <kbd
                  class="px-2 py-1 bg-surface-container-high border border-outline-variant/50 rounded font-keycap text-keycap text-on-surface shadow-xs"
                  >Ctrl</kbd
                >
                <kbd
                  class="px-2 py-1 bg-surface-container-high border border-outline-variant/50 rounded font-keycap text-keycap text-on-surface shadow-xs"
                  >Alt</kbd
                >
                <kbd
                  class="px-2 py-1 bg-surface-container-high border border-outline-variant/50 rounded font-keycap text-keycap text-on-surface shadow-xs"
                  >2</kbd
                >
              </div>
              <button
                class="px-space-sm py-1 rounded-lg bg-surface-container hover:bg-surface-container-high border border-outline-variant/40 text-on-surface font-body-muted text-body-muted transition-colors"
                type="button"
              >
                Change
              </button>
            </div>
          </div>
          <div class="p-space-md flex items-center justify-between gap-space-md">
            <div>
              <span class="font-body-default text-body-default text-on-surface block"
                >Spoken Prefix Trigger</span
              >
              <span class="font-body-muted text-body-muted text-on-surface-variant"
                >Switch mode inline when starting dictation with this word</span
              >
            </div>
            <div class="flex items-center gap-2">
              <span class="font-label-code text-label-code text-on-surface-variant">prefix:</span>
              <input
                class="w-28 bg-surface-container px-space-sm py-1.5 rounded-lg border border-outline-variant/40 text-on-surface font-label-code text-label-code focus:border-primary focus:outline-none"
                type="text"
                bind:value={spokenPrefix}
              />
            </div>
          </div>
        </div>
      </div>

      <div class="space-y-space-xs">
        <div class="px-space-xs">
          <span class="font-headline-md text-headline-md text-on-surface">Pipeline Filters</span>
        </div>
        <div
          class="rounded-xl bg-surface-container-low border border-outline-variant/30 divide-y divide-outline-variant/20 overflow-hidden"
        >
          <div class="p-space-md flex items-center justify-between gap-space-md">
            <div class="flex items-start gap-space-sm">
              <span class="material-symbols-outlined text-[20px] text-primary mt-0.5">neurology</span>
              <div>
                <span class="font-body-default text-body-default text-on-surface block font-medium"
                  >Neural Post-Processing</span
                >
                <span class="font-body-muted text-body-muted text-on-surface-variant"
                  >Pass tokens through quantized local language model</span
                >
              </div>
            </div>
            {@render pipelineSwitch(neural, "Neural Post-Processing", () => (neural = !neural))}
          </div>
          <div class="p-space-md flex items-center justify-between gap-space-md">
            <div class="flex items-start gap-space-sm">
              <span class="material-symbols-outlined text-[20px] text-on-surface-variant mt-0.5"
                >voice_selection</span
              >
              <div>
                <span class="font-body-default text-body-default text-on-surface block font-medium"
                  >Strip Hesitation Sounds</span
                >
                <span class="font-body-muted text-body-muted text-on-surface-variant"
                  >Prune "um", "uh", "you know", "like", and stammered syllables</span
                >
              </div>
            </div>
            {@render pipelineSwitch(hesitation, "Strip Hesitation Sounds", () => (hesitation = !hesitation))}
          </div>
          <div class="p-space-md flex items-center justify-between gap-space-md">
            <div class="flex items-start gap-space-sm">
              <span class="material-symbols-outlined text-[20px] text-on-surface-variant mt-0.5"
                >data_object</span
              >
              <div>
                <span class="font-body-default text-body-default text-on-surface block font-medium"
                  >Syntax Identifiers</span
                >
                <span class="font-body-muted text-body-muted text-on-surface-variant"
                  >Format terms like "kebab case user profile" into
                  <code class="font-label-code text-label-code text-primary">user-profile</code></span
                >
              </div>
            </div>
            {@render pipelineSwitch(syntax, "Syntax Identifiers", () => (syntax = !syntax))}
          </div>
          <div class="p-space-md flex items-center justify-between gap-space-md">
            <div class="flex items-start gap-space-sm">
              <span class="material-symbols-outlined text-[20px] text-on-surface-variant mt-0.5"
                >title</span
              >
              <div>
                <span class="font-body-default text-body-default text-on-surface block font-medium"
                  >Automatic Capitalization & Periods</span
                >
                <span class="font-body-muted text-body-muted text-on-surface-variant"
                  >Normalize sentence bounds based on pause heuristics</span
                >
              </div>
            </div>
            {@render pipelineSwitch(capitalization, "Automatic Capitalization & Periods", () => (capitalization = !capitalization))}
          </div>
        </div>
      </div>

      <div class="space-y-space-xs">
        <div class="px-space-xs flex items-center justify-between">
          <span class="font-headline-md text-headline-md text-on-surface">Refinement System Prompt</span>
          <span class="font-label-code text-label-code text-on-surface-variant">148 chars</span>
        </div>
        <div class="rounded-xl bg-surface-container-low border border-outline-variant/30 p-space-sm">
          <textarea
            class="w-full bg-surface-container p-space-md rounded-lg border border-outline-variant/40 font-label-code text-label-code text-on-surface leading-relaxed focus:border-primary focus:outline-none resize-none"
            rows="3"
            bind:value={prompt}
          ></textarea>
          <div class="flex items-center justify-between mt-space-xs px-1 text-on-surface-variant">
            <span class="font-body-muted text-body-muted">Model temperature: 0.1 (deterministic)</span>
            <button class="font-body-muted text-body-muted hover:text-on-surface text-primary" type="button"
              >Load Recommended Defaults</button
            >
          </div>
        </div>
      </div>

      <div class="space-y-space-xs">
        <div class="px-space-xs flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="font-headline-md text-headline-md text-on-surface">Live Diff Preview</span>
            <span
              class="font-label-code text-label-code px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-400 border border-emerald-500/20"
              >Verified</span
            >
          </div>
          <button
            class="flex items-center gap-1.5 px-space-sm py-1 rounded-lg bg-surface-container hover:bg-surface-container-high border border-outline-variant/40 text-on-surface font-body-muted text-body-muted transition-colors"
            type="button"
          >
            <span class="material-symbols-outlined text-[14px] text-primary">mic</span>
            <span>Test with microphone</span>
          </button>
        </div>
        <div
          class="rounded-xl bg-surface-container-low border border-outline-variant/30 overflow-hidden divide-y divide-outline-variant/20"
        >
          <div class="p-space-md bg-surface-container-low">
            <div class="flex items-center justify-between mb-1.5">
              <span
                class="font-keycap text-keycap text-on-surface-variant tracking-wider uppercase flex items-center gap-1"
              >
                <span class="w-1.5 h-1.5 rounded-full bg-outline"></span>
                Raw Acoustic Output (Parakeet CTC)
              </span>
              <span class="font-label-code text-label-code text-on-surface-variant">Latency: 42ms</span>
            </div>
            <div class="font-body-default text-body-default text-on-surface-variant leading-relaxed">
              <span class="bg-error/20 text-error line-through px-1 rounded mx-0.5">um</span>
              so
              <span class="bg-error/20 text-error line-through px-1 rounded mx-0.5">basically</span>
              I think we should
              <span class="bg-error/20 text-error line-through px-1 rounded mx-0.5">uh</span>
              ship it friday
            </div>
          </div>
          <div
            class="px-space-md py-1 bg-surface-container flex items-center justify-between text-on-surface-variant font-label-code text-label-code"
          >
            <div class="flex items-center gap-2">
              <span class="material-symbols-outlined text-[16px] text-primary">arrow_downward</span>
              <span>Applied: Stripped 3 filler tokens · Normalized capitalized weekday</span>
            </div>
            <span class="text-primary font-mono text-[11px]">+142ms total</span>
          </div>
          <div class="p-space-md bg-surface-container-high/40">
            <div class="flex items-center justify-between mb-1.5">
              <span
                class="font-keycap text-keycap text-primary tracking-wider uppercase flex items-center gap-1"
              >
                <span class="w-1.5 h-1.5 rounded-full bg-primary"></span>
                Pasted to Active Window
              </span>
              <button
                class="font-label-code text-label-code text-on-surface-variant hover:text-on-surface flex items-center gap-1"
                type="button"
              >
                <span class="material-symbols-outlined text-[14px]">content_copy</span>
                <span>Copy</span>
              </button>
            </div>
            <div class="font-body-default text-body-default text-on-surface font-medium leading-relaxed">
              "I think we should ship it on <span
                class="text-primary underline decoration-primary/40 underline-offset-4">Friday</span
              >."
            </div>
          </div>
        </div>
      </div>

      <div
        class="rounded-xl bg-surface-container-low border border-outline-variant/30 divide-y divide-outline-variant/20 overflow-hidden mb-space-lg"
      >
        <div class="p-space-md flex items-center justify-between gap-space-md">
          <div>
            <span class="font-body-default text-body-default text-on-surface block"
              >Acoustic Model Language</span
            >
            <span class="font-body-muted text-body-muted text-on-surface-variant"
              >Default tokenizer vocabulary</span
            >
          </div>
          <select
            class="bg-surface-container px-space-md py-1.5 rounded-lg border border-outline-variant/40 text-on-surface font-body-default text-body-default focus:border-primary focus:outline-none"
            bind:value={acoustic}
          >
            <option value="en-us">English (US) — Parakeet v1.1</option>
            <option value="en-uk">English (UK)</option>
            <option value="multi">Multilingual (Auto-Detect)</option>
          </select>
        </div>
        <div class="p-space-md flex items-center justify-between gap-space-md">
          <div>
            <span class="font-body-default text-body-default text-on-surface block"
              >Translation Target</span
            >
            <span class="font-body-muted text-body-muted text-on-surface-variant"
              >Cross-lingual synthesis</span
            >
          </div>
          <select
            class="bg-surface-container px-space-md py-1.5 rounded-lg border border-outline-variant/40 text-on-surface font-body-default text-body-default focus:border-primary focus:outline-none"
            bind:value={translation}
          >
            <option value="original">Original (No translation)</option>
            <option value="hi">Hindi (IndicWav2Vec)</option>
            <option value="es">Spanish (ES-LA)</option>
            <option value="de">German (DE)</option>
          </select>
        </div>
      </div>
    </div>
  </div>
</div>

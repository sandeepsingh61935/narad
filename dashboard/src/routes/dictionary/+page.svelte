<script lang="ts">
  type DictionaryTab = "vocab" | "replacements" | "snippets";

  let tab = $state<DictionaryTab>("vocab");

  function tabClass(id: DictionaryTab): string {
    const base =
      "tab-btn flex items-center gap-space-xs px-space-md py-1.5 rounded-lg font-headline-md text-headline-md transition-all";
    if (tab === id) {
      return `${base} active text-on-surface bg-surface-container-high shadow-sm`;
    }
    return `${base} text-on-surface-variant hover:text-on-surface`;
  }
</script>

<svelte:head>
  <link
    href="https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@20..48,100..700,0..1,-50..200&display=swap"
    rel="stylesheet"
  />
</svelte:head>

<main class="flex-1 bg-surface w-full">
  <div class="flex flex-col w-full">
    <div class="px-space-lg py-space-md flex flex-col gap-space-lg max-w-7xl mx-auto w-full">
      <div class="flex flex-col md:flex-row md:items-center justify-between gap-space-md">
        <div class="flex flex-col gap-0.5">
          <div class="flex items-center gap-space-xs">
            <span class="material-symbols-outlined text-primary text-[20px]">spellcheck</span>
            <span class="font-headline-lg text-headline-lg text-on-surface tracking-tight">Dictionary</span>
            <span class="px-1.5 py-0.5 rounded bg-surface-container-high font-label-code text-label-code text-on-surface-variant">SQLite Engine</span>
          </div>
          <div class="font-body-muted text-body-muted text-on-surface-variant flex items-center gap-space-xs">
            <span>Custom phonetic bias &amp; expansion database stored locally</span>
            <span class="w-1 h-1 rounded-full bg-outline-variant"></span>
            <span class="font-label-code text-label-code text-outline truncate">~/.local/share/narad/vocab.sqlite</span>
          </div>
        </div>
        <div class="flex items-center gap-space-sm flex-wrap">
          <button class="flex items-center gap-space-xs px-space-md py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-default text-body-default transition-colors" type="button">
            <span class="material-symbols-outlined text-[16px]">file_download</span>
            <span>Export</span>
            <span class="font-keycap text-keycap text-on-surface-variant bg-surface-container px-1 py-0.2 rounded">CSV</span>
          </button>
          <button class="flex items-center gap-space-xs px-space-md py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-default text-body-default transition-colors" type="button">
            <span class="material-symbols-outlined text-[16px]">file_upload</span>
            <span>Import</span>
          </button>
          <button class="flex items-center gap-space-xs px-space-md py-1.5 rounded-lg bg-primary-container hover:bg-primary text-on-primary-container font-headline-md text-headline-md transition-colors shadow-sm" id="add-word-btn" type="button">
            <span class="material-symbols-outlined text-[18px]">add</span>
            <span>Add Word</span>
            <kbd class="px-1.5 py-0.5 bg-on-primary-container/20 rounded font-keycap text-keycap text-on-primary-container">N</kbd>
          </button>
        </div>
      </div>
      <div class="flex flex-col lg:flex-row items-stretch lg:items-center justify-between gap-space-md">
        <div class="inline-flex p-1 rounded-xl bg-surface-container-lowest self-start">
          <button class={tabClass("vocab")} id="tab-vocab" type="button" onclick={() => (tab = "vocab")}>
            <span class="material-symbols-outlined text-[16px] text-primary">record_voice_over</span>
            <span>Vocabulary</span>
            <span class="px-1.5 py-0.2 rounded-full font-label-code text-label-code bg-surface-container-lowest text-primary">142</span>
          </button>
          <button class={tabClass("replacements")} id="tab-replacements" type="button" onclick={() => (tab = "replacements")}>
            <span class="material-symbols-outlined text-[16px]">find_replace</span>
            <span>Replacements</span>
            <span class="px-1.5 py-0.2 rounded-full font-label-code text-label-code bg-surface-container-high text-on-surface-variant">38</span>
          </button>
          <button class={tabClass("snippets")} id="tab-snippets" type="button" onclick={() => (tab = "snippets")}>
            <span class="material-symbols-outlined text-[16px]">data_object</span>
            <span>Snippets</span>
            <span class="px-1.5 py-0.2 rounded-full font-label-code text-label-code bg-surface-container-high text-on-surface-variant">14</span>
          </button>
        </div>
        <div class="flex items-center gap-space-sm flex-1 lg:max-w-md">
          <div class="flex items-center bg-surface-container px-space-md py-1.5 rounded-lg w-full gap-space-sm">
            <span class="material-symbols-outlined text-outline text-[18px]">search</span>
            <input class="bg-transparent border-none outline-none font-body-default text-body-default text-on-surface placeholder:text-outline w-full" id="dictionary-search" placeholder="Search words, phonetics, Hindi triggers..." type="text" />
            <kbd class="px-1.5 py-0.5 bg-surface-container-high rounded font-keycap text-keycap text-outline">Ctrl+F</kbd>
          </div>
          <div class="flex items-center gap-1 shrink-0">
            <button class="w-8 h-8 rounded-lg bg-surface-container hover:bg-surface-container-high flex items-center justify-center text-on-surface-variant transition-colors" title="Filter by Tag" type="button">
              <span class="material-symbols-outlined text-[18px]">filter_list</span>
            </button>
            <button class="w-8 h-8 rounded-lg bg-surface-container hover:bg-surface-container-high flex items-center justify-center text-on-surface-variant transition-colors" title="Sort Criteria" type="button">
              <span class="material-symbols-outlined text-[18px]">sort_by_alpha</span>
            </button>
          </div>
        </div>
      </div>
      <div class="grid grid-cols-1 lg:grid-cols-12 gap-space-lg">
        <div class="lg:col-span-8 flex flex-col gap-space-md">
          <div class="bg-surface-container-low rounded-xl p-space-md flex items-start gap-space-md">
            <span class="material-symbols-outlined text-primary text-[20px] shrink-0 mt-0.5">offline_bolt</span>
            <div class="flex flex-col gap-0.5">
              <span class="font-headline-md text-headline-md text-on-surface">Acoustic Prioritization</span>
              <span class="font-body-muted text-body-muted text-on-surface-variant">
                Custom vocabulary injects token priors into Whisper and Vosk decode beams. Increases recognition probability for proper nouns, Indian idioms, and tech jargon without requiring cloud recalculation.
              </span>
            </div>
          </div>
          <div class="hidden bg-surface-container rounded-xl p-space-md shadow-md flex-col gap-space-sm transition-all" id="inline-editor-card">
            <div class="flex items-center justify-between">
              <span class="font-headline-md text-headline-md text-on-surface flex items-center gap-space-xs">
                <span class="material-symbols-outlined text-primary text-[18px]">edit_note</span>
                <span>New Lexical Entry</span>
              </span>
              <span class="font-label-code text-label-code text-outline">schema: vocab.v2</span>
            </div>
            <div class="grid grid-cols-1 sm:grid-cols-4 gap-space-sm mt-1">
              <div class="sm:col-span-1">
                <label class="font-label-code text-label-code text-on-surface-variant block mb-1" for="input-term">Target Term</label>
                <input class="w-full bg-surface-container-lowest px-space-sm py-1.5 rounded-lg font-body-default text-body-default text-on-surface outline-none focus:bg-surface-container-high placeholder:text-outline" id="input-term" placeholder="e.g. WireGuard" type="text" />
              </div>
              <div class="sm:col-span-1">
                <label class="font-label-code text-label-code text-on-surface-variant block mb-1" for="input-phonetic">Phonetics / IPA</label>
                <input class="w-full bg-surface-container-lowest px-space-sm py-1.5 rounded-lg font-body-default text-body-default text-on-surface outline-none focus:bg-surface-container-high placeholder:text-outline" id="input-phonetic" placeholder="e.g. WYE-er-gard" type="text" />
              </div>
              <div class="sm:col-span-1">
                <label class="font-label-code text-label-code text-on-surface-variant block mb-1" for="input-boost">Acoustic Boost</label>
                <select class="w-full bg-surface-container-lowest px-space-sm py-1.5 rounded-lg font-body-default text-body-default text-on-surface outline-none focus:bg-surface-container-high cursor-pointer" id="input-boost">
                  <option value="+5.0dB High">+5.0dB (Strong)</option>
                  <option value="+4.0dB High">+4.0dB (High)</option>
                  <option selected value="+3.0dB Med">+3.0dB (Medium)</option>
                  <option value="+1.5dB Low">+1.5dB (Subtle)</option>
                </select>
              </div>
              <div class="sm:col-span-1">
                <label class="font-label-code text-label-code text-on-surface-variant block mb-1" for="input-format">Formatting</label>
                <select class="w-full bg-surface-container-lowest px-space-sm py-1.5 rounded-lg font-body-default text-body-default text-on-surface outline-none focus:bg-surface-container-high cursor-pointer" id="input-format">
                  <option value="Exact Case">Exact Case</option>
                  <option value="Title Case">Title Case</option>
                  <option value="ALL CAPS">ALL CAPS</option>
                  <option value="Monospace">Monospace</option>
                </select>
              </div>
            </div>
            <div class="flex items-center justify-end gap-space-sm mt-space-sm">
              <button class="px-space-md py-1 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-default text-body-default transition-colors" id="cancel-inline-btn" type="button">
                Cancel <kbd class="ml-1 font-keycap text-keycap text-outline">Esc</kbd>
              </button>
              <button class="px-space-md py-1 rounded-lg bg-primary text-on-primary font-headline-md text-headline-md hover:bg-primary-container transition-colors shadow-sm" id="save-inline-btn" type="button">
                Save Entry <kbd class="ml-1 font-keycap text-keycap text-on-primary">↵ Enter</kbd>
              </button>
            </div>
          </div>
          <div class="bg-surface-container rounded-xl overflow-hidden shadow-sm flex flex-col">
            <div class="grid grid-cols-12 px-space-md py-space-sm bg-surface-container-high text-outline font-label-code text-label-code select-none">
              <div class="col-span-4 sm:col-span-3">LEXICAL TERM</div>
              <div class="col-span-4 sm:col-span-4">PHONETIC HINT / TRANSCRIPTION</div>
              <div class="hidden sm:block sm:col-span-2">BOOST</div>
              <div class="hidden sm:block sm:col-span-2">FORMAT</div>
              <div class="col-span-4 sm:col-span-1 text-right">STATE</div>
            </div>
            <div class="flex flex-col" id="vocab-table-body">
              <div class="grid grid-cols-12 px-space-md py-space-sm items-center hover:bg-surface-container-high/60 transition-colors group">
                <div class="col-span-4 sm:col-span-3 flex items-center gap-space-xs min-w-0 pr-1">
                  <span class="w-1.5 h-1.5 rounded-full bg-primary shrink-0"></span>
                  <span class="font-headline-md text-headline-md text-on-surface truncate">Narad</span>
                  <span class="font-keycap text-keycap bg-surface-container-high text-on-surface-variant px-1 rounded hidden md:inline">Core</span>
                </div>
                <div class="col-span-4 sm:col-span-4 flex flex-col min-w-0 pr-1">
                  <span class="font-label-code text-label-code text-on-surface tracking-tight truncate">NAH-rud / nārada</span>
                  <span class="font-body-muted text-body-muted text-outline truncate text-[11px]">Sanskrit origin · High prior</span>
                </div>
                <div class="hidden sm:flex sm:col-span-2 items-center">
                  <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-primary/10 text-primary font-label-code text-label-code">
                    <span class="material-symbols-outlined text-[13px]">arrow_upward</span>
                    +5.0dB
                  </span>
                </div>
                <div class="hidden sm:block sm:col-span-2">
                  <span class="font-body-muted text-body-muted text-on-surface-variant bg-surface-container-lowest px-1.5 py-0.5 rounded">Title Case</span>
                </div>
                <div class="col-span-4 sm:col-span-1 flex items-center justify-end gap-1">
                  <button class="w-6 h-6 rounded flex items-center justify-center text-outline hover:text-on-surface hover:bg-surface-container-high opacity-0 group-hover:opacity-100 transition-all" title="Edit" type="button">
                    <span class="material-symbols-outlined text-[15px]">edit</span>
                  </button>
                  <div class="w-7 h-4 rounded-full bg-primary flex items-center px-0.5 cursor-pointer ml-1">
                    <div class="w-3 h-3 rounded-full bg-surface translate-x-3 transition-transform"></div>
                  </div>
                </div>
              </div>
              <div class="grid grid-cols-12 px-space-md py-space-sm items-center hover:bg-surface-container-high/60 transition-colors group bg-surface-container-low/30">
                <div class="col-span-4 sm:col-span-3 flex items-center gap-space-xs min-w-0 pr-1">
                  <span class="w-1.5 h-1.5 rounded-full bg-tertiary shrink-0"></span>
                  <span class="font-headline-md text-headline-md text-on-surface truncate">Kubernetes</span>
                </div>
                <div class="col-span-4 sm:col-span-4 flex flex-col min-w-0 pr-1">
                  <span class="font-label-code text-label-code text-on-surface tracking-tight truncate">koo-ber-NET-eez / k8s</span>
                  <span class="font-body-muted text-body-muted text-outline truncate text-[11px]">Cloud Infra · Technical</span>
                </div>
                <div class="hidden sm:flex sm:col-span-2 items-center">
                  <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-surface-container-high text-on-surface-variant font-label-code text-label-code">
                    <span class="material-symbols-outlined text-[13px]">arrow_upward</span>
                    +3.0dB
                  </span>
                </div>
                <div class="hidden sm:block sm:col-span-2">
                  <span class="font-body-muted text-body-muted text-on-surface-variant bg-surface-container-lowest px-1.5 py-0.5 rounded">Title Case</span>
                </div>
                <div class="col-span-4 sm:col-span-1 flex items-center justify-end gap-1">
                  <button class="w-6 h-6 rounded flex items-center justify-center text-outline hover:text-on-surface hover:bg-surface-container-high opacity-0 group-hover:opacity-100 transition-all" title="Edit" type="button">
                    <span class="material-symbols-outlined text-[15px]">edit</span>
                  </button>
                  <div class="w-7 h-4 rounded-full bg-primary flex items-center px-0.5 cursor-pointer ml-1">
                    <div class="w-3 h-3 rounded-full bg-surface translate-x-3 transition-transform"></div>
                  </div>
                </div>
              </div>
              <div class="grid grid-cols-12 px-space-md py-space-sm items-center hover:bg-surface-container-high/60 transition-colors group">
                <div class="col-span-4 sm:col-span-3 flex items-center gap-space-xs min-w-0 pr-1">
                  <span class="w-1.5 h-1.5 rounded-full bg-primary shrink-0"></span>
                  <span class="font-headline-md text-headline-md text-on-surface truncate">Sandeep</span>
                </div>
                <div class="col-span-4 sm:col-span-4 flex flex-col min-w-0 pr-1">
                  <span class="font-label-code text-label-code text-on-surface tracking-tight truncate">sun-DEEP / sandīp</span>
                  <span class="font-body-muted text-body-muted text-outline truncate text-[11px]">Hindi Proper · Personal</span>
                </div>
                <div class="hidden sm:flex sm:col-span-2 items-center">
                  <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-primary/10 text-primary font-label-code text-label-code">
                    <span class="material-symbols-outlined text-[13px]">arrow_upward</span>
                    +4.0dB
                  </span>
                </div>
                <div class="hidden sm:block sm:col-span-2">
                  <span class="font-body-muted text-body-muted text-on-surface-variant bg-surface-container-lowest px-1.5 py-0.5 rounded">Title Case</span>
                </div>
                <div class="col-span-4 sm:col-span-1 flex items-center justify-end gap-1">
                  <button class="w-6 h-6 rounded flex items-center justify-center text-outline hover:text-on-surface hover:bg-surface-container-high opacity-0 group-hover:opacity-100 transition-all" title="Edit" type="button">
                    <span class="material-symbols-outlined text-[15px]">edit</span>
                  </button>
                  <div class="w-7 h-4 rounded-full bg-primary flex items-center px-0.5 cursor-pointer ml-1">
                    <div class="w-3 h-3 rounded-full bg-surface translate-x-3 transition-transform"></div>
                  </div>
                </div>
              </div>
              <div class="grid grid-cols-12 px-space-md py-space-sm items-center hover:bg-surface-container-high/60 transition-colors group bg-surface-container-low/30">
                <div class="col-span-4 sm:col-span-3 flex items-center gap-space-xs min-w-0 pr-1">
                  <span class="w-1.5 h-1.5 rounded-full bg-tertiary shrink-0"></span>
                  <span class="font-headline-md text-headline-md text-on-surface font-label-code truncate">PostgreSQL</span>
                </div>
                <div class="col-span-4 sm:col-span-4 flex flex-col min-w-0 pr-1">
                  <span class="font-label-code text-label-code text-on-surface tracking-tight truncate">POST-gres-Q-L / postgres</span>
                  <span class="font-body-muted text-body-muted text-outline truncate text-[11px]">Database Engine · Exact</span>
                </div>
                <div class="hidden sm:flex sm:col-span-2 items-center">
                  <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-primary/10 text-primary font-label-code text-label-code">
                    <span class="material-symbols-outlined text-[13px]">arrow_upward</span>
                    +4.5dB
                  </span>
                </div>
                <div class="hidden sm:block sm:col-span-2">
                  <span class="font-body-muted text-body-muted text-on-surface-variant bg-surface-container-lowest px-1.5 py-0.5 rounded">Exact Case</span>
                </div>
                <div class="col-span-4 sm:col-span-1 flex items-center justify-end gap-1">
                  <button class="w-6 h-6 rounded flex items-center justify-center text-outline hover:text-on-surface hover:bg-surface-container-high opacity-0 group-hover:opacity-100 transition-all" title="Edit" type="button">
                    <span class="material-symbols-outlined text-[15px]">edit</span>
                  </button>
                  <div class="w-7 h-4 rounded-full bg-primary flex items-center px-0.5 cursor-pointer ml-1">
                    <div class="w-3 h-3 rounded-full bg-surface translate-x-3 transition-transform"></div>
                  </div>
                </div>
              </div>
              <div class="grid grid-cols-12 px-space-md py-space-sm items-center hover:bg-surface-container-high/60 transition-colors group">
                <div class="col-span-4 sm:col-span-3 flex items-center gap-space-xs min-w-0 pr-1">
                  <span class="w-1.5 h-1.5 rounded-full bg-secondary shrink-0"></span>
                  <span class="font-headline-md text-headline-md text-on-surface truncate">Ghar pe</span>
                </div>
                <div class="col-span-4 sm:col-span-4 flex flex-col min-w-0 pr-1">
                  <span class="font-label-code text-label-code text-on-surface tracking-tight truncate">ghar pay (घर पे)</span>
                  <span class="font-body-muted text-body-muted text-outline truncate text-[11px]">Hinglish Colloquial</span>
                </div>
                <div class="hidden sm:flex sm:col-span-2 items-center">
                  <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-surface-container-high text-on-surface-variant font-label-code text-label-code">
                    <span class="material-symbols-outlined text-[13px]">arrow_upward</span>
                    +3.5dB
                  </span>
                </div>
                <div class="hidden sm:block sm:col-span-2">
                  <span class="font-body-muted text-body-muted text-on-surface-variant bg-surface-container-lowest px-1.5 py-0.5 rounded">lowercase</span>
                </div>
                <div class="col-span-4 sm:col-span-1 flex items-center justify-end gap-1">
                  <button class="w-6 h-6 rounded flex items-center justify-center text-outline hover:text-on-surface hover:bg-surface-container-high opacity-0 group-hover:opacity-100 transition-all" title="Edit" type="button">
                    <span class="material-symbols-outlined text-[15px]">edit</span>
                  </button>
                  <div class="w-7 h-4 rounded-full bg-primary flex items-center px-0.5 cursor-pointer ml-1">
                    <div class="w-3 h-3 rounded-full bg-surface translate-x-3 transition-transform"></div>
                  </div>
                </div>
              </div>
              <div class="grid grid-cols-12 px-space-md py-space-sm items-center hover:bg-surface-container-high/60 transition-colors group bg-surface-container-low/30">
                <div class="col-span-4 sm:col-span-3 flex items-center gap-space-xs min-w-0 pr-1">
                  <span class="w-1.5 h-1.5 rounded-full bg-tertiary shrink-0"></span>
                  <span class="font-headline-md text-headline-md text-on-surface font-label-code truncate">OAuth2</span>
                </div>
                <div class="col-span-4 sm:col-span-4 flex flex-col min-w-0 pr-1">
                  <span class="font-label-code text-label-code text-on-surface tracking-tight truncate">oh-auth-two</span>
                  <span class="font-body-muted text-body-muted text-outline truncate text-[11px]">Security Protocol</span>
                </div>
                <div class="hidden sm:flex sm:col-span-2 items-center">
                  <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-surface-container-high text-on-surface-variant font-label-code text-label-code">
                    <span class="material-symbols-outlined text-[13px]">arrow_upward</span>
                    +3.0dB
                  </span>
                </div>
                <div class="hidden sm:block sm:col-span-2">
                  <span class="font-body-muted text-body-muted text-on-surface-variant bg-surface-container-lowest px-1.5 py-0.5 rounded">Exact Case</span>
                </div>
                <div class="col-span-4 sm:col-span-1 flex items-center justify-end gap-1">
                  <button class="w-6 h-6 rounded flex items-center justify-center text-outline hover:text-on-surface hover:bg-surface-container-high opacity-0 group-hover:opacity-100 transition-all" title="Edit" type="button">
                    <span class="material-symbols-outlined text-[15px]">edit</span>
                  </button>
                  <div class="w-7 h-4 rounded-full bg-primary flex items-center px-0.5 cursor-pointer ml-1">
                    <div class="w-3 h-3 rounded-full bg-surface translate-x-3 transition-transform"></div>
                  </div>
                </div>
              </div>
              <div class="grid grid-cols-12 px-space-md py-space-sm items-center hover:bg-surface-container-high/60 transition-colors group">
                <div class="col-span-4 sm:col-span-3 flex items-center gap-space-xs min-w-0 pr-1">
                  <span class="w-1.5 h-1.5 rounded-full bg-primary shrink-0"></span>
                  <span class="font-headline-md text-headline-md text-on-surface font-label-code truncate">libadwaita</span>
                </div>
                <div class="col-span-4 sm:col-span-4 flex flex-col min-w-0 pr-1">
                  <span class="font-label-code text-label-code text-on-surface tracking-tight truncate">lib-ad-WAY-tuh</span>
                  <span class="font-body-muted text-body-muted text-outline truncate text-[11px]">GNOME Library · GTK4</span>
                </div>
                <div class="hidden sm:flex sm:col-span-2 items-center">
                  <span class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-primary/10 text-primary font-label-code text-label-code">
                    <span class="material-symbols-outlined text-[13px]">arrow_upward</span>
                    +5.0dB
                  </span>
                </div>
                <div class="hidden sm:block sm:col-span-2">
                  <span class="font-body-muted text-body-muted text-on-surface-variant bg-surface-container-lowest px-1.5 py-0.5 rounded">lowercase</span>
                </div>
                <div class="col-span-4 sm:col-span-1 flex items-center justify-end gap-1">
                  <button class="w-6 h-6 rounded flex items-center justify-center text-outline hover:text-on-surface hover:bg-surface-container-high opacity-0 group-hover:opacity-100 transition-all" title="Edit" type="button">
                    <span class="material-symbols-outlined text-[15px]">edit</span>
                  </button>
                  <div class="w-7 h-4 rounded-full bg-primary flex items-center px-0.5 cursor-pointer ml-1">
                    <div class="w-3 h-3 rounded-full bg-surface translate-x-3 transition-transform"></div>
                  </div>
                </div>
              </div>
            </div>
            <div class="p-space-sm bg-surface-container-low flex items-center justify-between">
              <span class="font-label-code text-label-code text-outline">Showing 7 of 142 vocabulary weights</span>
              <div class="flex items-center gap-space-xs">
                <button class="px-space-sm py-1 rounded bg-surface-container hover:bg-surface-bright text-outline text-body-muted disabled:opacity-40" type="button">Previous</button>
                <span class="px-space-sm font-label-code text-label-code text-on-surface">1</span>
                <button class="px-space-sm py-1 rounded bg-surface-container hover:bg-surface-bright text-on-surface text-body-muted" type="button">Next</button>
              </div>
            </div>
          </div>
        </div>
        <div class="lg:col-span-4 flex flex-col gap-space-md">
          <div class="bg-surface-container rounded-xl p-space-md flex flex-col gap-space-sm shadow-sm">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-space-xs">
                <span class="material-symbols-outlined text-primary text-[18px]">find_replace</span>
                <span class="font-headline-md text-headline-md text-on-surface">Auto Replacements</span>
              </div>
              <span class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-1.5 py-0.5 rounded">38 rules</span>
            </div>
            <span class="font-body-muted text-body-muted text-outline">Direct string substitutions triggered during voice stream normalization.</span>
            <div class="flex flex-col gap-space-xs mt-1">
              <div class="p-space-sm bg-surface-container-low rounded-lg flex items-center justify-between hover:bg-surface-container-high transition-colors">
                <div class="flex flex-col min-w-0">
                  <span class="font-body-muted text-body-muted text-on-surface-variant flex items-center gap-1">
                    <span class="material-symbols-outlined text-[13px] text-outline">mic</span>
                    <span class="italic">"my email"</span>
                  </span>
                  <span class="font-label-code text-label-code text-primary truncate">sandy@example.com</span>
                </div>
                <span class="font-keycap text-keycap bg-surface-container px-1 py-0.5 rounded text-outline shrink-0">Exact</span>
              </div>
              <div class="p-space-sm bg-surface-container-low rounded-lg flex items-center justify-between hover:bg-surface-container-high transition-colors">
                <div class="flex flex-col min-w-0">
                  <span class="font-body-muted text-body-muted text-on-surface-variant flex items-center gap-1">
                    <span class="material-symbols-outlined text-[13px] text-outline">mic</span>
                    <span class="italic">"dot com"</span>
                  </span>
                  <span class="font-label-code text-label-code text-on-surface truncate">.com</span>
                </div>
                <span class="font-keycap text-keycap bg-surface-container px-1 py-0.5 rounded text-outline shrink-0">Regex</span>
              </div>
              <div class="p-space-sm bg-surface-container-low rounded-lg flex items-center justify-between hover:bg-surface-container-high transition-colors">
                <div class="flex flex-col min-w-0">
                  <span class="font-body-muted text-body-muted text-on-surface-variant flex items-center gap-1">
                    <span class="material-symbols-outlined text-[13px] text-outline">mic</span>
                    <span class="italic">"shukriya"</span>
                  </span>
                  <span class="font-body-default text-body-default text-tertiary truncate">धन्यवाद (Dhanyavaad)</span>
                </div>
                <span class="font-keycap text-keycap bg-surface-container px-1 py-0.5 rounded text-outline shrink-0">Translit</span>
              </div>
            </div>
            <button class="w-full py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-default text-body-default transition-colors flex items-center justify-center gap-1 text-center" type="button">
              <span>Manage Replacement Rules</span>
              <span class="material-symbols-outlined text-[16px]">arrow_forward</span>
            </button>
          </div>
          <div class="bg-surface-container rounded-xl p-space-md flex flex-col gap-space-sm shadow-sm">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-space-xs">
                <span class="material-symbols-outlined text-primary text-[18px]">data_object</span>
                <span class="font-headline-md text-headline-md text-on-surface">Voice Snippets</span>
              </div>
              <span class="font-keycap text-keycap text-on-surface-variant bg-surface-container-high px-1.5 py-0.5 rounded">14 templates</span>
            </div>
            <span class="font-body-muted text-body-muted text-outline">Multi-line expansion blocks populated instantly via trigger keywords.</span>
            <div class="p-space-sm bg-surface-container-low rounded-lg flex flex-col gap-1.5">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-1">
                  <span class="material-symbols-outlined text-[14px] text-primary">play_arrow</span>
                  <span class="font-body-muted text-body-muted text-on-surface font-medium">"insert signature"</span>
                </div>
                <span class="font-keycap text-keycap text-primary bg-primary/10 px-1 py-0.2 rounded">Global</span>
              </div>
              <div class="bg-surface-container-lowest p-space-sm rounded font-label-code text-label-code text-on-surface-variant select-all">
                Best regards,<br />
                Sandeep Sharma<br />
                Platform Engineering, Narad Core<br />
                <span class="text-outline">PGP: 0x4A8F...91BE</span>
              </div>
            </div>
            <div class="p-space-sm bg-surface-container-low rounded-lg flex flex-col gap-1.5">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-1">
                  <span class="material-symbols-outlined text-[14px] text-tertiary">play_arrow</span>
                  <span class="font-body-muted text-body-muted text-on-surface font-medium">"insert license block"</span>
                </div>
                <span class="font-keycap text-keycap text-tertiary bg-tertiary/10 px-1 py-0.2 rounded">IDE Only</span>
              </div>
              <div class="bg-surface-container-lowest p-space-sm rounded font-label-code text-label-code text-outline select-all">
                // SPDX-License-Identifier: GPL-3.0-or-later<br />
                // Copyright (c) 2025 Narad contributors
              </div>
            </div>
            <button class="w-full py-1.5 rounded-lg bg-surface-container-high hover:bg-surface-bright text-on-surface font-body-default text-body-default transition-colors flex items-center justify-center gap-1 text-center" type="button">
              <span>Configure Voice Snippets</span>
              <span class="material-symbols-outlined text-[16px]">arrow_forward</span>
            </button>
          </div>
          <div class="bg-surface-container-low rounded-xl p-space-md flex flex-col gap-space-xs">
            <div class="flex items-center justify-between text-outline font-label-code text-label-code">
              <span>ACOUSTIC GRAPH STATS</span>
              <span class="text-primary flex items-center gap-1">
                <span class="w-1.5 h-1.5 rounded-full bg-emerald-500"></span>
                Synchronized
              </span>
            </div>
            <div class="flex items-center justify-between py-1 font-body-muted text-body-muted">
              <span class="text-on-surface-variant">Active Phonetic Nodes</span>
              <span class="font-label-code text-label-code text-on-surface">1,482 triphones</span>
            </div>
            <div class="flex items-center justify-between py-1 font-body-muted text-body-muted">
              <span class="text-on-surface-variant">Cache Lookup Latency</span>
              <span class="font-label-code text-label-code text-on-surface">&lt; 0.42 ms</span>
            </div>
            <div class="flex items-center justify-between py-1 font-body-muted text-body-muted">
              <span class="text-on-surface-variant">Model Integration</span>
              <span class="font-label-code text-label-code text-on-surface">Vosk WFST + Whisper Logits</span>
            </div>
          </div>
        </div>
      </div>
      <div class="mt-space-md bg-surface-container-lowest p-space-sm rounded-lg flex flex-col sm:flex-row items-center justify-between gap-space-sm text-outline font-label-code text-label-code">
        <div class="flex items-center gap-space-sm">
          <span class="inline-flex items-center gap-1.5 text-on-surface">
            <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
            142 custom vocabulary terms active
          </span>
          <span class="hidden sm:inline">•</span>
          <span class="text-on-surface-variant">0 network lookups (100% offline)</span>
          <span class="hidden sm:inline">•</span>
          <span>Acoustic dictionary re-indexed 2m ago</span>
        </div>
        <div class="flex items-center gap-space-md">
          <span class="hover:text-on-surface cursor-pointer transition-colors">vacuum db</span>
          <span class="hover:text-on-surface cursor-pointer transition-colors">force re-index</span>
        </div>
      </div>
    </div>
  </div>
</main>

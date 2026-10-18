<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { defaultConfig, type DashboardConfig, type Replacement } from "$lib/config";
  import { SAVED_NOTICE, saveDashboardConfig } from "$lib/settings-form";
  import {
    cleanDictionaryPairs,
    findDuplicateSpoken,
    validateDictionaryPair,
  } from "$lib/dictionary";
  import { toast } from "$lib/toast.svelte";

  type CustomWord = {
    word: string;
    spoken: string;
    hits: number;
  };

  const SEED_WORDS: CustomWord[] = [
    { word: "drm_ioctl_kernel", spoken: "drm ioctl kernel", hits: 14 },
    { word: "Libadwaita", spoken: "lib adwaita", hits: 8 },
    { word: "oklch", spoken: "oak el see aitch", hits: 23 },
    { word: "Wayland", spoken: "", hits: 31 },
    { word: "Narad", spoken: "narrate", hits: 45 },
  ];

  let config = $state<DashboardConfig>(defaultConfig());
  let words = $state<CustomWord[]>([]);
  let loaded = $state<boolean>(false);
  let dialogOpen = $state<boolean>(false);
  let editingIndex = $state<number>(-1);
  let dialogWord = $state<string>("");
  let dialogSpoken = $state<string>("");
  let dialogError = $state<string>("");
  let wordInputEl: HTMLInputElement | null = $state(null);

  function syncFromConfig(cfg: DashboardConfig) {
    if (cfg.vocabulary && cfg.vocabulary.length > 0) {
      words = cfg.vocabulary.map((v) => ({
        word: v.written || v.spoken,
        spoken: v.written && v.written !== v.spoken ? v.spoken : "",
        hits: Math.floor(Math.random() * 20) + 1,
      }));
    } else {
      words = [...SEED_WORDS];
    }
  }

  function wordsToVocabulary(items: CustomWord[]): Replacement[] {
    return items.map((item) => ({
      spoken: item.spoken.trim() || item.word.trim(),
      written: item.word.trim(),
    }));
  }

  async function persistWords() {
    const vocabPairs = wordsToVocabulary(words);
    const cleaned = cleanDictionaryPairs(vocabPairs);
    config.vocabulary = cleaned;

    if (isTauri()) {
      const result = await saveDashboardConfig(config);
      if (!result.ok) {
        toast(`Error saving: ${result.error}`);
      }
    }
  }

  async function load() {
    if (isTauri()) {
      try {
        config = await invoke<DashboardConfig>("load_config");
        syncFromConfig(config);
        loaded = true;
      } catch {
        words = [...SEED_WORDS];
        loaded = true;
      }
    } else {
      words = [...SEED_WORDS];
      loaded = true;
    }
  }

  function openAddDialog() {
    editingIndex = -1;
    dialogWord = "";
    dialogSpoken = "";
    dialogError = "";
    dialogOpen = true;
    setTimeout(() => {
      wordInputEl?.focus();
    }, 50);
  }

  function openEditDialog(index: number) {
    const item = words[index];
    if (!item) return;
    editingIndex = index;
    dialogWord = item.word;
    dialogSpoken = item.spoken;
    dialogError = "";
    dialogOpen = true;
    setTimeout(() => {
      wordInputEl?.focus();
    }, 50);
  }

  function closeDialog() {
    dialogOpen = false;
    editingIndex = -1;
  }

  async function saveWord() {
    const word = dialogWord.trim();
    const spoken = dialogSpoken.trim();

    if (!word) {
      dialogError = "Enter the spelling Narad should keep.";
      wordInputEl?.focus();
      return;
    }

    if (word.length > 64) {
      dialogError = "Keep the word under 64 characters.";
      wordInputEl?.focus();
      return;
    }

    const testPair: Replacement = {
      spoken: spoken || word,
      written: word,
    };
    const validation = validateDictionaryPair(testPair);
    if (!validation.valid) {
      dialogError = validation.error || "Invalid word pair.";
      return;
    }

    const dup = words.some(
      (item, i) => i !== editingIndex && item.word.toLowerCase() === word.toLowerCase(),
    );
    if (dup) {
      dialogError = "That word is already in the list.";
      return;
    }

    const updatedItem: CustomWord = {
      word,
      spoken,
      hits: editingIndex >= 0 ? words[editingIndex].hits : 0,
    };

    if (editingIndex >= 0) {
      words[editingIndex] = updatedItem;
      words = [...words];
    } else {
      words = [updatedItem, ...words];
    }

    await persistWords();
    closeDialog();
    toast(`Saved "${word}"`);
  }

  async function removeWord(index: number) {
    const removed = words[index];
    if (!removed) return;
    words = words.filter((_, i) => i !== index);
    await persistWords();

    toast(`Removed "${removed.word}"`, async () => {
      words.splice(index, 0, removed);
      words = [...words];
      await persistWords();
    });
  }

  function handleKeydown(e: KeyboardEvent) {
    if (dialogOpen) {
      if (e.key === "Escape") {
        closeDialog();
      } else if (e.key === "Enter") {
        e.preventDefault();
        void saveWord();
      }
      return;
    }

    const target = e.target as HTMLElement | null;
    const isInput = target?.tagName === "INPUT" || target?.tagName === "TEXTAREA" || target?.tagName === "SELECT";
    if (isInput) return;

    if (e.key === "n" || e.key === "N") {
      e.preventDefault();
      openAddDialog();
    }
  }

  onMount(() => {
    void load();
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<main class="content h-full flex flex-col min-h-0 overflow-hidden" data-od-id="words">
  <div class="content-scroll pad">
    <div class="words-head">
      <div>
        <h1 class="text-xl font-semibold mb-1">Custom words</h1>
        <p class="small muted">
          Add a spelling the engine should keep. Useful for names, flags, and API symbols that otherwise get smoothed into plain speech.
        </p>
      </div>
      <button
        type="button"
        class="btn btn-primary"
        id="add"
        data-od-id="add-word"
        onclick={openAddDialog}
      >
        Add word <span class="kbd">N</span>
      </button>
    </div>

    {#if words.length > 0}
      <div class="group" id="word-list" data-od-id="word-list">
        {#each words as item, i (item.word)}
          <div class="row" data-od-id="word-{i}">
            <div class="row-main">
              <div class="row-title mono font-medium">{item.word}</div>
              <div class="row-sub">
                <span>{item.spoken ? `Spoken as "${item.spoken}"` : "No spoken hint"}</span>
                <span> · </span>
                <span class="mono">{item.hits} {item.hits === 1 ? "hit" : "hits"}</span>
              </div>
            </div>
            <div class="row-actions">
              <button
                type="button"
                class="btn"
                onclick={() => openEditDialog(i)}
              >
                Edit
              </button>
              <button
                type="button"
                class="btn btn-quiet btn-danger"
                onclick={() => void removeWord(i)}
              >
                Remove
              </button>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <p class="empty" id="word-empty" data-od-id="words-empty">
        No custom words yet. Add a term the engine should keep spelled as you write it.
      </p>
    {/if}
  </div>
</main>

<footer class="statusbar" data-od-id="statusbar">
  <span><span class="kbd">N</span> add word</span>
  <span><span class="kbd">Esc</span> close dialog</span>
  <span class="spacer mono">Saved on this machine</span>
</footer>

<!-- Add / Edit Word Modal Dialog -->
<div
  class="dialog-scrim"
  id="scrim"
  hidden={!dialogOpen}
  onclick={(e) => {
    if ((e.target as HTMLElement).id === "scrim") closeDialog();
  }}
>
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="word-dialog-title"
    data-od-id="word-dialog"
    onclick={(e) => e.stopPropagation()}
  >
    <h2 id="word-dialog-title">
      {editingIndex >= 0 ? "Edit word" : "Add word"}
    </h2>
    <p id="word-dialog-copy">
      Narad will prefer this spelling the next time it hears the phrase.
    </p>

    <div class="field">
      <label class="field-label" for="word">Word</label>
      <input
        class="input"
        id="word"
        autocomplete="off"
        spellcheck="false"
        bind:this={wordInputEl}
        bind:value={dialogWord}
        placeholder="e.g. drm_ioctl_kernel"
      />
    </div>

    <div class="field field-gap">
      <label class="field-label" for="spoken">
        Spoken as <span class="muted">optional</span>
      </label>
      <input
        class="input"
        id="spoken"
        autocomplete="off"
        bind:value={dialogSpoken}
        placeholder="e.g. drm ioctl kernel"
      />
    </div>

    {#if dialogError}
      <div class="field-error mt-2" id="word-error" role="alert">
        {dialogError}
      </div>
    {/if}

    <div class="dialog-actions">
      <button type="button" class="btn" id="cancel" onclick={closeDialog}>
        Cancel
      </button>
      <button
        type="button"
        class="btn btn-primary"
        id="save"
        data-od-id="save-word"
        onclick={() => void saveWord()}
      >
        Save word
      </button>
    </div>
  </div>
</div>

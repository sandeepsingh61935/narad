<script lang="ts">
  import { onMount } from "svelte";
  import { goto } from "$app/navigation";
  import { SAVED_NOTICE } from "$lib/settings-form";

  // Alias / redirect to /words for vocabulary and replacements management
  const aliasNotice = SAVED_NOTICE;

  onMount(() => {
    void goto("/words", { replaceState: true });
  });
</script>

<div class="pad flex flex-col gap-4">
  <h1 class="text-xl font-semibold">Dictionary</h1>
  <p class="muted">
    Custom words, vocabulary, and replacements have moved to
    <a href="/words" class="text-accent underline">Custom words (/words)</a>.
    Redirecting…
  </p>
</div>

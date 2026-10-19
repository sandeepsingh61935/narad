<script lang="ts">
  import { toasts } from "$lib/toast.svelte";
</script>

<div class="toast-host" data-toast-host>
  {#each toasts.items as item (item.id)}
    <div
      class="toast {item.isOut ? 'is-out' : 'is-in'}"
      role="status"
      aria-live="polite"
      data-od-id="toast"
    >
      <span class="toast-msg">{item.message}</span>
      {#if item.onUndo}
        <button
          type="button"
          class="btn btn-quiet"
          onclick={() => {
            item.onUndo?.();
            toasts.dismiss(item.id);
          }}
        >
          Undo
        </button>
      {/if}
      <i class="toast-bar"></i>
    </div>
  {/each}
</div>

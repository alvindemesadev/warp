<script lang="ts">
  let { message = "" }: { message: string } = $props();
</script>

{#if message}
  <div class="toast">{message}</div>
{/if}

<style>
  .toast {
    position: fixed;
    bottom: 18px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 300;
    background: var(--modal-bg);
    border: 1px solid var(--modal-border);
    border-radius: 10px;
    padding: 8px 14px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-primary);
    box-shadow: var(--modal-shadow);
    pointer-events: none;
    transition: background 0.2s;
  }
</style>

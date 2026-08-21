<script lang="ts">
  let { message = "" }: { message: string } = $props();
</script>

{#if message}
  <div class="toast">{message}</div>
{/if}

<style>
  .toast {
    position: fixed; bottom: 18px; left: 50%; transform: translateX(-50%); z-index: 300;
    background: #2c2c2e; border: 1px solid rgba(255,255,255,0.1); border-radius: 10px;
    padding: 8px 14px; font-size: 12px; color: var(--text-primary);
    box-shadow: 0 6px 28px rgba(0,0,0,0.5); pointer-events: none;
  }
</style>

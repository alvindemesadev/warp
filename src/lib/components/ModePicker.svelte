<script lang="ts">
  import type { Mode } from "$lib/types";
  let { mode = $bindable("copy") }: { mode: Mode } = $props();

  const MODES: { id: Mode; label: string; desc: string; warning?: string }[] = [
    { id: "copy", label: "Copy", desc: "Duplicate files to destination" },
    { id: "move", label: "Move", desc: "Transfer and remove from source" },
    {
      id: "sync",
      label: "Sync",
      desc: "Mirror source → destination",
      warning: "Files only in destination will be DELETED",
    },
  ];
</script>

<div class="mode-picker">
  {#each MODES as m}
    <button
      onclick={() => (mode = m.id)}
      title={m.warning ?? m.desc}
      class="mode-btn"
      class:mode-btn--active={mode === m.id}
      aria-pressed={mode === m.id}
    >
      {m.label}
    </button>
  {/each}
</div>
<p class="mode-desc" class:mode-desc--warn={mode === "sync"}>
  {MODES.find((m) => m.id === mode)?.warning ?? MODES.find((m) => m.id === mode)?.desc}
</p>

<style>
  .mode-picker {
    background: var(--seg-bg);
    border: 1px solid var(--glass-border);
    border-radius: 12px;
    padding: 4px;
    display: flex;
    gap: 3px;
    transition:
      background 0.2s,
      border-color 0.2s;
  }
  .mode-btn {
    flex: 1;
    padding: 6px 8px;
    border-radius: 9px;
    font-size: 12px;
    font-weight: 600;
    border: none;
    cursor: pointer;
    transition:
      background 0.15s,
      color 0.15s,
      box-shadow 0.15s;
    outline: none;
    background: transparent;
    color: var(--text-secondary);
    transform: none;
  }
  .mode-btn--active {
    background: var(--seg-active-bg);
    color: var(--text-primary);
    box-shadow: var(--seg-active-shadow);
  }
  .mode-btn:not(.mode-btn--active):hover {
    background: var(--seg-hover);
    color: var(--text-primary);
  }
  .mode-desc {
    text-align: center;
    font-size: 11px;
    margin: -6px 0 0;
    color: var(--text-tertiary);
    min-height: 14px;
  }
  .mode-desc--warn {
    color: var(--orange);
  }
</style>

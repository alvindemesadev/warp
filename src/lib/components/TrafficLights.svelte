<script lang="ts">
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let {
    recentCount = 0,
    isProcessing = false,
    lastSummary = null,
    updateState = "idle",
    updateVersion = "",
    onRecentToggle,
    onUpdateOpen
  }: {
    recentCount: number;
    isProcessing: boolean;
    lastSummary: unknown;
    updateState: string;
    updateVersion: string;
    onRecentToggle: () => void;
    onUpdateOpen: () => void;
  } = $props();

  const win = getCurrentWindow();
</script>

<div class="traffic-bar" data-tauri-drag-region>
  <button
    onclick={() => win.close()} aria-label="Close" class="traffic traffic--red"
    onmouseenter={(e) => ((e.currentTarget.querySelector('span') as HTMLElement).style.opacity='1')}
    onmouseleave={(e) => ((e.currentTarget.querySelector('span') as HTMLElement).style.opacity='0')}
  ><span class="traffic-x">✕</span></button>

  <button
    onclick={() => win.minimize()} aria-label="Minimize" class="traffic traffic--yellow"
    onmouseenter={(e) => ((e.currentTarget.querySelector('span') as HTMLElement).style.opacity='1')}
    onmouseleave={(e) => ((e.currentTarget.querySelector('span') as HTMLElement).style.opacity='0')}
  ><span class="traffic-min">−</span></button>

  <div class="traffic-actions">
    {#if updateState === "checking"}
      <span class="chip chip--muted">CHECKING…</span>
    {:else if updateState === "available" && updateVersion}
      <button
        onclick={onUpdateOpen}
        title={`Update available — Warp v${updateVersion} is ready to install`}
        class="chip chip--update"
      >⬆ UPDATE v{updateVersion}</button>
    {/if}
    {#if recentCount > 0 && !isProcessing && !lastSummary}
      <button
        onclick={onRecentToggle}
        title="Recent transfers"
        class="chip chip--recent"
      >RECENT</button>
    {/if}
  </div>
</div>

<style>
  .traffic-bar {
    position: fixed; top: 0; left: 0; right: 0; height: 36px; z-index: 50;
    display: flex; align-items: center; padding: 0 14px; gap: 6px; cursor: default;
  }
  .traffic {
    width: 12px; height: 12px; border-radius: 50%; border: none; padding: 0;
    cursor: pointer; display: flex; align-items: center; justify-content: center; flex-shrink: 0;
  }
  .traffic--red { background: var(--red); }
  .traffic--yellow { background: var(--yellow); }
  .traffic-x { opacity: 0; font-size: 7px; font-weight: 900; color: rgba(0,0,0,0.6); line-height: 1; pointer-events: none; }
  .traffic-min { opacity: 0; font-size: 9px; font-weight: 900; color: rgba(0,0,0,0.5); line-height: 1; margin-top: -1px; pointer-events: none; }
  .traffic-actions { margin-left: auto; margin-right: 4px; display: flex; align-items: center; gap: 6px; }
  .chip {
    padding: 2px 6px; border-radius: 5px; border: none; font-size: 9px; font-weight: 600;
    letter-spacing: 0.04em; font-family: var(--font-sf); cursor: pointer;
  }
  .chip--muted { color: var(--text-tertiary); background: rgba(255,255,255,0.07); }
  .chip--update {
    padding: 2px 7px; background: rgba(10,132,255,0.22); color: #64b5ff; font-weight: 700;
    animation: update-pulse 2.2s ease-in-out infinite;
  }
  .chip--update:hover { background: rgba(10,132,255,0.35); }
  .chip--recent { background: rgba(255,255,255,0.07); color: rgba(255,255,255,0.4); }
  .chip--recent:hover { color: rgba(255,255,255,0.7); }
</style>

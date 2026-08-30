<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let {
    recentCount = 0,
    isProcessing = false,
    lastSummary = null,
    updateState = "idle",
    updateVersion = "",
    onRecentToggle,
    onUpdateOpen,
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

  async function handleClose() {
    if (isProcessing) {
      // Guard: orphan robocopy would keep running if window closed mid-transfer
      const ok = window.confirm("Transfer in progress — close Warp and cancel the transfer?");
      if (!ok) return;
      // Kill the backend child process BEFORE the window goes away. The Rust
      // side also kills on window destroy/app exit as a safety net.
      try {
        await invoke("cancel_warp");
      } catch {}
    }
    try {
      win.close();
    } catch {}
  }
</script>

<div class="traffic-bar" data-tauri-drag-region>
  <button
    onclick={handleClose}
    aria-label="Close"
    class="traffic traffic--red"
    onmouseenter={(e) =>
      ((e.currentTarget.querySelector("span") as HTMLElement).style.opacity = "1")}
    onmouseleave={(e) =>
      ((e.currentTarget.querySelector("span") as HTMLElement).style.opacity = "0")}
    ><span class="traffic-x">✕</span></button
  >

  <button
    onclick={() => win.minimize()}
    aria-label="Minimize"
    class="traffic traffic--yellow"
    onmouseenter={(e) =>
      ((e.currentTarget.querySelector("span") as HTMLElement).style.opacity = "1")}
    onmouseleave={(e) =>
      ((e.currentTarget.querySelector("span") as HTMLElement).style.opacity = "0")}
    ><span class="traffic-min">−</span></button
  >

  <div class="traffic-actions">
    {#if updateState === "available" && updateVersion}
      <button
        onclick={onUpdateOpen}
        title={`Update available — Warp v${updateVersion} is ready to install`}
        class="chip chip--update">⬆ UPDATE</button
      >
    {/if}
    {#if recentCount > 0 && !isProcessing && !lastSummary}
      <button onclick={onRecentToggle} title="Recent transfers" class="chip chip--recent"
        >RECENT</button
      >
    {/if}
  </div>
</div>

<style>
  .traffic-bar {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 44px;
    z-index: 50;
    display: flex;
    align-items: center;
    padding: 6px 14px 0;
    gap: 8px;
    cursor: default;
    background: transparent;
    border-bottom: none;
  }
  .traffic {
    width: 13px;
    height: 13px;
    border-radius: 50%;
    border: none;
    padding: 0;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.35),
      inset 0 1px 0 rgba(255, 255, 255, 0.22);
    transition:
      transform 0.12s,
      filter 0.12s;
    margin-top: -8px;
  }
  .traffic:hover {
    transform: scale(1.08);
    filter: brightness(1.1);
  }
  .traffic:active {
    transform: scale(0.96);
  }
  .traffic--red {
    background: #ff5f57;
  }
  .traffic--yellow {
    background: #ffbd2e;
  }
  .traffic-x {
    opacity: 0;
    font-size: 7px;
    font-weight: 900;
    color: rgba(0, 0, 0, 0.65);
    line-height: 1;
    pointer-events: none;
  }
  .traffic-min {
    opacity: 0;
    font-size: 10px;
    font-weight: 900;
    color: rgba(0, 0, 0, 0.55);
    line-height: 1;
    margin-top: -1px;
    pointer-events: none;
  }
  .traffic-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .chip {
    height: 24px;
    padding: 0 10px;
    border-radius: 7px;
    border: 1px solid var(--glass-border);
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.06em;
    font-family: var(--font-sf);
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-secondary);
    transition: all 0.15s;
  }
  .chip--update {
    background: rgba(10, 132, 255, 0.14);
    color: #64b5ff;
    border-color: rgba(10, 132, 255, 0.22);
    font-weight: 700;
    animation: update-pulse 2.2s ease-in-out infinite;
  }
  .chip--update:hover {
    background: rgba(10, 132, 255, 0.22);
    color: #7cc4ff;
  }
  .chip--recent {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-secondary);
  }
  .chip--recent:hover {
    background: rgba(255, 255, 255, 0.08);
    color: var(--text-primary);
  }
</style>

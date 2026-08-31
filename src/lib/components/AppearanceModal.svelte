<script lang="ts">
  import { ui } from "$lib/stores/ui.svelte";

  let {
    onClose,
  }: {
    onClose: () => void;
  } = $props();
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-label="Appearance settings" tabindex="-1">
  <button type="button" aria-label="Dismiss" onclick={onClose} class="backdrop"></button>
  <div class="panel animate-pop-in">
    <div class="head">
      <div class="head-left">
        <svg width="14" height="14" viewBox="0 0 20 20" fill="none" class="head-icon">
          <circle cx="10" cy="10" r="7" stroke="var(--accent)" stroke-width="1.8" />
          <path d="M10 3v14a7 7 0 000-14z" fill="var(--accent)" />
        </svg>
        <p class="head-title">Appearance</p>
      </div>
      <button onclick={onClose} class="close-btn" aria-label="Close appearance modal">✕</button>
    </div>

    <div class="body">
      <!-- Theme group -->
      <div class="section">
        <span class="section-label">Theme</span>
        <div class="seg" role="group" aria-label="Color theme">
          <button
            class="seg-btn"
            class:on={ui.theme === "dark"}
            onclick={() => ui.setTheme("dark")}
          >
            <svg width="12" height="12" viewBox="0 0 20 20" fill="none" class="btn-icon">
              <path
                d="M17.293 13.293A8 8 0 016.707 2.707a8.001 8.001 0 1010.586 10.586z"
                fill="currentColor"
              />
            </svg>
            Dark
          </button>
          <button
            class="seg-btn"
            class:on={ui.theme === "light"}
            onclick={() => ui.setTheme("light")}
          >
            <svg width="12" height="12" viewBox="0 0 20 20" fill="none" class="btn-icon">
              <circle cx="10" cy="10" r="4" stroke="currentColor" stroke-width="1.6" />
              <path
                d="M10 2v2m0 12v2M2 10h2m12 0h2m-2.636-5.364l-1.414 1.414M6.05 13.95l-1.414 1.414m0-10.728l1.414 1.414m7.9 7.9l1.414 1.414"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linecap="round"
              />
            </svg>
            Light
          </button>
        </div>
      </div>

      <!-- Size group -->
      <div class="section">
        <span class="section-label">Interface Size</span>
        <div class="seg" role="group" aria-label="Interface scaling size">
          {#each [{ v: 1.0, label: "Normal" }, { v: 1.15, label: "Medium" }, { v: 1.3, label: "Large" }] as s}
            <button
              class="seg-btn"
              class:on={Math.abs(ui.scale - s.v) < 0.04}
              onclick={() => ui.setScale(s.v)}
            >
              {s.label}
            </button>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: flex-start;
    justify-content: flex-end;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    border: none;
    padding: 0;
    margin: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(4px);
    cursor: default;
  }
  .panel {
    position: fixed;
    top: 50px;
    right: 14px;
    width: 270px;
    background: var(--modal-bg);
    border: 1px solid var(--modal-border);
    box-shadow: var(--modal-shadow);
    border-radius: 14px;
    overflow: hidden;
    z-index: 101;
  }
  .head {
    padding: 11px 14px;
    border-bottom: 1px solid var(--glass-border);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .head-left {
    display: flex;
    align-items: center;
    gap: 7px;
  }
  .head-icon {
    flex-shrink: 0;
  }
  .head-title {
    font-size: 11px;
    font-weight: 700;
    color: var(--text-secondary);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin: 0;
  }
  .close-btn {
    font-size: 11px;
    font-weight: 700;
    color: var(--text-secondary);
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    font-family: var(--font-sf);
    transition:
      color 0.12s,
      background 0.12s;
  }
  .close-btn:hover {
    color: var(--text-primary);
    background: var(--glass-hover);
  }
  .close-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .body {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .section-label {
    font-size: 9.5px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-secondary);
    padding-left: 2px;
  }
  .seg {
    display: flex;
    gap: 2px;
    padding: 3px;
    background: var(--seg-bg);
    border: 1px solid var(--glass-border);
    border-radius: 9px;
    width: 100%;
  }
  .seg-btn {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 5px;
    padding: 5px 8px;
    border-radius: 7px;
    font-size: 11px;
    font-weight: 600;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition:
      background 0.15s,
      color 0.15s,
      box-shadow 0.15s;
    font-family: var(--font-sf);
    white-space: nowrap;
  }
  .seg-btn:hover {
    background: var(--seg-hover);
    color: var(--text-primary);
  }
  .seg-btn:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .seg-btn.on {
    background: var(--seg-active-bg);
    color: var(--text-primary);
    box-shadow: var(--seg-active-shadow);
  }
  .btn-icon {
    flex-shrink: 0;
  }
</style>

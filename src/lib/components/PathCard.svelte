<script lang="ts">
  import { basename, fmtBytes, fmtFiles } from "$lib/format";
  import type { PathInfo } from "$lib/types";
  let {
    sourcePath,
    destPath,
    sourceInfo,
    destInfo,
    isScanning,
    isScanningDest,
    dragTarget = $bindable(null),
    isDragging,
    onBrowseSource,
    onBrowseDest,
    onClearSource,
    onClearDest,
    onSwap,
  }: {
    sourcePath: string;
    destPath: string;
    sourceInfo: PathInfo | null;
    destInfo: PathInfo | null;
    isScanning: boolean;
    isScanningDest: boolean;
    dragTarget: string | null;
    isDragging: boolean;
    onBrowseSource: () => void;
    onBrowseDest: () => void;
    onClearSource: () => void;
    onClearDest: () => void;
    onSwap: () => void;
  } = $props();

  function isSpecialPath(p: string): string | null {
    const lower = p.toLowerCase();
    if (lower.includes("onedrive")) return "OneDrive path — ensure files are downloaded locally";
    if (p.startsWith("\\\\")) return "Network path — speed may be limited";
    return null;
  }
  const sourceWarning = $derived(sourcePath ? isSpecialPath(sourcePath) : null);
  const destWarning = $derived(destPath ? isSpecialPath(destPath) : null);
  const sourceUsb = $derived(sourceInfo?.removable ?? false);
  const destUsb = $derived(destInfo?.removable ?? false);
</script>

<div class="card" class:card--drag-active={isDragging && sourcePath && destPath}>
  {#if isDragging && sourcePath && destPath}
    <div class="card-hint">DROP TO REPLACE A SLOT</div>
  {/if}

  <!-- Source row -->
  <div
    data-drop="source"
    role="button"
    tabindex="0"
    aria-label="Source folder drop zone"
    aria-dropeffect={dragTarget === "source" ? "copy" : "none"}
    class="row row--source"
    class:row--drag={dragTarget === "source"}
    onclick={onBrowseSource}
    onkeydown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onBrowseSource();
      }
    }}
  >
    <div
      class="icon"
      class:icon--source={!!sourcePath && !sourceInfo?.isFile}
      class:icon--empty={!sourcePath}
    >
      <svg width="17" height="17" viewBox="0 0 20 20" fill="none">
        {#if sourcePath && !sourceInfo?.isFile}
          <path
            d="M2 6.5A2.5 2.5 0 014.5 4H8l1.5 2h6A2.5 2.5 0 0118 8.5v5A2.5 2.5 0 0115.5 16h-11A2.5 2.5 0 012 13.5V6.5Z"
            fill="var(--accent)"
          />
        {:else if sourcePath && sourceInfo?.isFile}
          <path
            d="M5 3h7l4 4v10a1 1 0 01-1 1H5a1 1 0 01-1-1V4a1 1 0 011-1z"
            stroke="var(--red)"
            stroke-width="1.4"
            fill="none"
          />
        {:else}
          <path
            d="M2 6.5A2.5 2.5 0 014.5 4H8l1.5 2h6A2.5 2.5 0 0118 8.5v5A2.5 2.5 0 0115.5 16h-11A2.5 2.5 0 012 13.5V6.5Z"
            stroke="var(--text-tertiary)"
            stroke-width="1.4"
            fill="none"
          />
        {/if}
      </svg>
    </div>
    <div class="text">
      <p class="label">Source</p>
      {#if sourcePath}
        <p class="path" class:path--error={sourceInfo?.isFile} title={sourcePath}>
          {basename(sourcePath)}
        </p>
        <p class="meta">
          {#if isScanning}<span class="pulse">Scanning…</span>
          {:else if sourceInfo?.isFile}<span class="error">Drop a folder, not a file</span>
          {:else if sourceInfo}{fmtFiles(sourceInfo.files)} · {fmtBytes(sourceInfo.bytes)}
          {:else}<span class="warn">⚠ Folder not found or unreadable</span>{/if}
        </p>
        {#if sourceWarning}<p class="hint hint--warn">⚠ {sourceWarning}</p>{/if}
        {#if sourceUsb}<p class="hint hint--warn">
            ⚠ USB drive — reduced threads for optimal throughput
          </p>{/if}
      {:else}
        <p class="placeholder">Drop or <span class="link">browse</span></p>
      {/if}
    </div>
    {#if sourcePath}
      <button
        onclick={(e) => {
          e.stopPropagation();
          onClearSource();
        }}
        aria-label="Clear source"
        class="clear"
        ><svg width="9" height="9" viewBox="0 0 8 8" fill="none"
          ><path
            d="M1 1l6 6M7 1L1 7"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
          /></svg
        ></button
      >
    {/if}
  </div>

  <!-- Swap -->
  <div class="swap-wrap">
    <button
      onclick={onSwap}
      title="Swap source and destination"
      disabled={!sourcePath && !destPath}
      class="swap"
      class:swap--enabled={!!(sourcePath || destPath)}
    >
      <svg width="10" height="10" viewBox="0 0 10 10" fill="none"
        ><path
          d="M3 1.5v7M1.5 7L3 8.5 4.5 7M7 8.5v-7M5.5 3L7 1.5 8.5 3"
          stroke="var(--text-secondary)"
          stroke-width="1.2"
          stroke-linecap="round"
          stroke-linejoin="round"
        /></svg
      >
    </button>
  </div>

  <!-- Dest row -->
  <div
    data-drop="dest"
    role="button"
    tabindex="0"
    aria-label="Destination folder drop zone"
    aria-dropeffect={dragTarget === "dest" ? "copy" : "none"}
    class="row row--dest"
    class:row--drag={dragTarget === "dest"}
    onclick={onBrowseDest}
    onkeydown={(e) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onBrowseDest();
      }
    }}
  >
    <div
      class="icon"
      class:icon--dest={!!destPath && !destInfo?.isFile}
      class:icon--error={!!destPath && destInfo?.isFile}
      class:icon--empty={!destPath}
    >
      <svg width="17" height="17" viewBox="0 0 20 20" fill="none">
        {#if destPath && destInfo?.isFile}
          <path
            d="M5 3h7l4 4v10a1 1 0 01-1 1H5a1 1 0 01-1-1V4a1 1 0 011-1z"
            stroke="var(--red)"
            stroke-width="1.4"
            fill="none"
          />
        {:else if destPath}
          <path
            d="M2 6.5A2.5 2.5 0 014.5 4H8l1.5 2h6A2.5 2.5 0 0118 8.5v5A2.5 2.5 0 0115.5 16h-11A2.5 2.5 0 012 13.5V6.5Z"
            fill="var(--green)"
          />
        {:else}
          <path
            d="M2 6.5A2.5 2.5 0 014.5 4H8l1.5 2h6A2.5 2.5 0 0118 8.5v5A2.5 2.5 0 0115.5 16h-11A2.5 2.5 0 012 13.5V6.5Z"
            stroke="var(--text-tertiary)"
            stroke-width="1.4"
            fill="none"
          />
        {/if}
      </svg>
    </div>
    <div class="text">
      <p class="label">Destination</p>
      {#if destPath}
        <p class="path" class:path--error={destInfo?.isFile} title={destPath}>
          {basename(destPath)}
        </p>
        <p class="meta">
          {#if isScanningDest}<span class="pulse">Scanning…</span>
          {:else if destInfo?.isFile}<span class="error">Drop a folder, not a file</span>
          {:else if destInfo && destInfo.files > 0}{fmtFiles(destInfo.files)} · {fmtBytes(
              destInfo.bytes,
            )} already here
          {:else if destInfo}Empty folder
          {:else}<span class="warn">⚠ Folder not found or unreadable</span>{/if}
        </p>
        {#if destWarning}<p class="hint hint--warn">⚠ {destWarning}</p>{/if}
        {#if destUsb}<p class="hint hint--warn">
            ⚠ USB drive — reduced threads for optimal throughput
          </p>{/if}
      {:else}
        <p class="placeholder">Drop or <span class="link">browse</span></p>
      {/if}
    </div>
    {#if destPath}
      <button
        onclick={(e) => {
          e.stopPropagation();
          onClearDest();
        }}
        aria-label="Clear destination"
        class="clear"
        ><svg width="9" height="9" viewBox="0 0 8 8" fill="none"
          ><path
            d="M1 1l6 6M7 1L1 7"
            stroke="currentColor"
            stroke-width="1.6"
            stroke-linecap="round"
          /></svg
        ></button
      >
    {/if}
  </div>
</div>

<style>
  .card {
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    backdrop-filter: blur(48px) saturate(180%);
    border-radius: 16px;
    overflow: visible;
    position: relative;
    box-shadow: var(--card-shadow);
    transition:
      border-color 0.15s,
      background 0.2s,
      box-shadow 0.2s;
  }
  .card--drag-active {
    border-color: var(--accent);
  }
  .card-hint {
    position: absolute;
    top: -9px;
    left: 50%;
    transform: translateX(-50%);
    z-index: 20;
    background: var(--accent);
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 2px 8px;
    border-radius: 6px;
    white-space: nowrap;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 13px 14px;
    transition: background 0.15s;
    width: 100%;
    text-align: left;
    border: none;
    cursor: pointer;
    background: transparent;
    font: inherit;
  }
  .row--source {
    border-bottom: 1px solid var(--glass-border);
    border-radius: 16px 16px 0 0;
  }
  .row--dest {
    border-radius: 0 0 16px 16px;
  }
  .row--drag {
    background: rgba(10, 132, 255, 0.08);
  }
  .icon {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    background: var(--seg-bg);
  }
  .icon--source {
    background: rgba(10, 132, 255, 0.15);
  }
  .icon--dest {
    background: rgba(48, 209, 88, 0.15);
  }
  .icon--error {
    background: rgba(255, 69, 58, 0.15);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .label {
    font-size: 9px;
    font-weight: 700;
    color: var(--text-secondary);
    letter-spacing: 0.07em;
    text-transform: uppercase;
    margin: 0 0 2px;
  }
  .path {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .path--error {
    color: var(--red);
  }
  .meta {
    font-size: 10px;
    color: var(--text-secondary);
    margin: 1px 0 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .placeholder {
    font-size: 13px;
    color: var(--text-secondary);
    margin: 0;
  }
  .link {
    background: none;
    border: none;
    color: var(--accent);
    font-size: 13px;
    font-family: var(--font-sf);
    cursor: pointer;
    padding: 0;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .hint {
    font-size: 10px;
    font-weight: 600;
    color: var(--orange);
    margin: 2px 0 0;
  }
  .pulse {
    animation: pulse-soft 1.8s ease-in-out infinite;
  }
  .error {
    color: var(--red);
  }
  .warn {
    color: var(--orange);
  }
  .clear {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--seg-bg);
    color: var(--text-secondary);
    border: none;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    cursor: pointer;
    opacity: 0.7;
    transition:
      opacity 0.15s,
      background 0.15s;
  }
  .clear:hover {
    opacity: 1;
    background: var(--glass-hover);
  }
  .swap-wrap {
    height: 0;
    position: relative;
    display: flex;
    justify-content: center;
  }
  .swap {
    position: absolute;
    top: -11px;
    z-index: 10;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--swap-bg);
    border: 1px solid var(--glass-border);
    box-shadow:
      0 1px 4px rgba(0, 0, 0, 0.25),
      0 0 1px rgba(0, 0, 0, 0.2);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: default;
    transition:
      background 0.15s,
      border-color 0.15s,
      transform 0.15s;
  }
  .swap:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .swap--enabled {
    cursor: pointer;
  }
  .swap--enabled:hover {
    background: var(--swap-hover);
    border-color: var(--accent);
    transform: scale(1.1);
  }
</style>

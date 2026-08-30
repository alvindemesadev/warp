<script lang="ts">
  import { basename, fmtEta } from "$lib/format";
  let {
    progress,
    currentFile,
    speed,
    filesDone,
    filesTotal,
    etaSeconds,
    isIndeterminate,
    isQueueRunning,
    queueIndex,
    queueTotal,
    sourcePath,
    destPath,
    transferredFiles,
    onCancel,
    activeWorkers = 0,
    shardsDone = 0,
    shardsTotal = 0,
    paused = false,
    onTogglePause,
  }: {
    progress: number;
    currentFile: string;
    speed: string;
    filesDone: number;
    filesTotal: number;
    etaSeconds: number;
    isIndeterminate: boolean;
    isQueueRunning: boolean;
    queueIndex: number;
    queueTotal: number;
    sourcePath: string;
    destPath: string;
    transferredFiles: string[];
    onCancel: () => void;
    activeWorkers?: number;
    shardsDone?: number;
    shardsTotal?: number;
    paused?: boolean;
    onTogglePause?: () => void;
  } = $props();

  // Drive the bar width through a CSS custom property (set via CSSOM) instead
  // of an inline style attribute so the strict CSP (no 'unsafe-inline') holds.
  let fillEl = $state<HTMLDivElement | null>(null);
  $effect(() => {
    fillEl?.style.setProperty("--p", `${progress}%`);
  });
</script>

<div class="card">
  {#if isQueueRunning}
    <div class="queue-head">
      <span class="queue-label">Job {queueIndex} of {queueTotal}</span>
      <span class="queue-path">{basename(sourcePath)} → {basename(destPath)}</span>
    </div>
  {/if}
  <div
    class="top"
    role="status"
    aria-live="polite"
    aria-busy={!isIndeterminate && progress < 100 ? "true" : "false"}
  >
    <div class="spinner" aria-hidden="true"></div>
    <span class="file" aria-live="polite">{currentFile || "Transferring..."}</span>
    <span class="speed" aria-live="polite">{speed || `${progress}%`}</span>
  </div>
  <div
    class="bar"
    role="progressbar"
    aria-valuenow={progress}
    aria-valuemin={0}
    aria-valuemax={100}
    aria-label={isIndeterminate ? "Empty folder — indeterminate progress" : `Transfer ${progress}%`}
  >
    {#if isIndeterminate}
      <div class="fill fill--indeterminate" title="Empty folder — no bytes to measure"></div>
    {:else}
      <div class="fill" bind:this={fillEl}></div>
    {/if}
  </div>
  {#if (activeWorkers ?? 0) > 0 || paused}
    <div class="pool-line">
      {#if (activeWorkers ?? 0) > 1}
        <span class="pool-chip pool-chip--workers">⚡ {activeWorkers} copying in parallel</span>
        {#if (shardsTotal ?? 0) > 0}
          <span class="pool-chip">{shardsDone}/{shardsTotal} folders</span>
        {/if}
      {:else if (activeWorkers ?? 0) === 1 && (shardsTotal ?? 0) > 0 && shardsDone < (shardsTotal ?? 0)}
        <span class="pool-chip pool-chip--workers">⚡ finishing last folder</span>
        <span class="pool-chip">{shardsDone}/{shardsTotal} folders</span>
      {/if}
      {#if paused}
        <span class="pool-chip pool-chip--paused"
          >⏸ Paused — finishing active folder{(activeWorkers ?? 0) > 1 ? "s" : ""}</span
        >
      {/if}
    </div>
  {/if}
  <div class="bottom">
    <span class="meta">
      {filesTotal > 0
        ? `${filesDone.toLocaleString()} / ${filesTotal.toLocaleString()} files`
        : `${basename(sourcePath)} → ${basename(destPath)}`}
      {#if etaSeconds > 0}<span class="eta"> · {fmtEta(etaSeconds)}</span>{/if}
    </span>
    <div class="bottom-right">
      <span class="pct">{progress}%</span>
      {#if onTogglePause}
        <button
          onclick={onTogglePause}
          class="pausebtn"
          title={paused ? "Resume dispatching folders" : "Pause after the active folders finish"}
          >{paused ? "RESUME" : "PAUSE"}</button
        >
      {/if}
      <button onclick={onCancel} class="cancel">CANCEL</button>
    </div>
  </div>
  {#if transferredFiles.length > 0}
    <div class="live">
      <p class="live-title">
        Transferring files — {Math.min(transferredFiles.length, 5)} of {transferredFiles.length} recent
      </p>
      <div class="live-list">
        {#each transferredFiles.slice(0, 5) as f, i}
          <p class="live-file" class:live-file--new={i === 0} title={f}>{f}</p>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .card {
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    backdrop-filter: blur(40px);
    border-radius: 14px;
    padding: 13px 15px;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }
  .queue-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .queue-label {
    font-size: 9px;
    font-weight: 700;
    color: var(--accent);
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }
  .queue-path {
    font-size: 10px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 70%;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    flex-shrink: 0;
    border: 2px solid rgba(10, 132, 255, 0.22);
    border-top-color: var(--accent);
    animation: spin-smooth 1.2s linear infinite;
  }
  .file {
    flex: 1;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .speed {
    font-size: 12px;
    font-weight: 600;
    color: var(--accent);
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }
  .bar {
    height: 4px;
    background: var(--surface-3);
    border-radius: 100px;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    width: var(--p, 0%);
    background: linear-gradient(90deg, var(--accent), #5e5ce6, var(--accent));
    border-radius: 100px;
    transition: width 0.4s cubic-bezier(0.4, 0, 0.2, 1);
  }
  .fill--indeterminate {
    width: 60%;
    background: linear-gradient(90deg, transparent, var(--accent), transparent);
    animation: shimmer 2.5s linear infinite;
    background-size: 300% auto;
  }
  .bottom {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .pool-line {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
  }
  .pool-chip {
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.04em;
    padding: 2px 7px;
    border-radius: 5px;
    background: var(--seg-bg);
    color: var(--text-tertiary);
  }
  .pool-chip--workers {
    background: rgba(10, 132, 255, 0.15);
    color: var(--accent);
  }
  .pool-chip--paused {
    background: rgba(255, 214, 10, 0.15);
    color: var(--yellow);
  }
  .meta {
    font-size: 10px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .eta {
    opacity: 0.7;
  }
  .bottom-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .pct {
    font-size: 10px;
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
  }
  .cancel {
    font-size: 9px;
    font-weight: 700;
    color: var(--red);
    background: rgba(255, 69, 58, 0.1);
    border: 1px solid rgba(255, 69, 58, 0.25);
    border-radius: 5px;
    padding: 2px 7px;
    cursor: pointer;
    letter-spacing: 0.04em;
    font-family: var(--font-sf);
  }
  .cancel:hover {
    background: rgba(255, 69, 58, 0.2);
  }
  .pausebtn {
    font-size: 9px;
    font-weight: 700;
    color: var(--yellow);
    background: rgba(255, 214, 10, 0.1);
    border: 1px solid rgba(255, 214, 10, 0.25);
    border-radius: 5px;
    padding: 2px 7px;
    cursor: pointer;
    letter-spacing: 0.04em;
    font-family: var(--font-sf);
  }
  .pausebtn:hover {
    background: rgba(255, 214, 10, 0.2);
  }
  .live {
    border-top: 1px solid var(--glass-border);
    padding-top: 10px;
    margin-top: 2px;
    min-width: 0;
  }
  .live-title {
    font-size: 10px;
    font-weight: 700;
    color: var(--text-tertiary);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin: 0 0 8px;
  }
  .live-list {
    display: block;
    padding: 6px;
    background: var(--live-box-bg);
    border: 1px solid var(--live-box-border);
    border-radius: 10px;
    overflow: hidden;
  }
  .live-file {
    display: block;
    font-size: 12px;
    font-family: Consolas, monospace;
    font-weight: 500;
    color: var(--live-file-color);
    margin: 0;
    padding: 4px 8px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    line-height: 1.4;
    background: var(--live-file-bg);
    border: 1px solid var(--glass-border);
    border-radius: 6px;
    margin-bottom: 4px;
    transition: background 0.1s;
  }
  .live-file:last-child {
    margin-bottom: 0;
  }
  .live-file--new {
    color: var(--accent);
    background: rgba(10, 132, 255, 0.12);
    border: 1px solid rgba(10, 132, 255, 0.25);
    font-weight: 700;
  }
  .live-file:hover {
    background: var(--live-file-hover);
  }
</style>

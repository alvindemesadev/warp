<script lang="ts">
  import { basename, fmtEta } from "$lib/format";
  let {
    progress, currentFile, speed, filesDone, filesTotal, etaSeconds,
    isIndeterminate, isQueueRunning, queueIndex, queueTotal,
    sourcePath, destPath, transferredFiles, onCancel
  }: {
    progress: number; currentFile: string; speed: string; filesDone: number; filesTotal: number;
    etaSeconds: number; isIndeterminate: boolean; isQueueRunning: boolean; queueIndex: number; queueTotal: number;
    sourcePath: string; destPath: string; transferredFiles: string[]; onCancel: () => void;
  } = $props();
</script>

<div class="card">
  {#if isQueueRunning}
    <div class="queue-head">
      <span class="queue-label">Job {queueIndex} of {queueTotal}</span>
      <span class="queue-path">{basename(sourcePath)} → {basename(destPath)}</span>
    </div>
  {/if}
  <div class="top">
    <div class="spinner"></div>
    <span class="file">{currentFile || "Transferring…"}</span>
    <span class="speed">{speed || `${progress}%`}</span>
  </div>
  <div class="bar">
    {#if isIndeterminate}
      <div class="fill fill--indeterminate"></div>
    {:else}
      <div class="fill" style:width="{progress}%"></div>
    {/if}
  </div>
  <div class="bottom">
    <span class="meta">
      {filesTotal > 0 ? `${filesDone.toLocaleString()} / ${filesTotal.toLocaleString()} files` : `${basename(sourcePath)} → ${basename(destPath)}`}
      {#if etaSeconds > 0}<span class="eta"> · {fmtEta(etaSeconds)}</span>{/if}
    </span>
    <div class="bottom-right">
      <span class="pct">{progress}%</span>
      <button onclick={onCancel} class="cancel">CANCEL</button>
    </div>
  </div>
  {#if transferredFiles.length > 0}
    <div class="live">
      <p class="live-title">Transferring files</p>
      <div class="live-list">
        {#each transferredFiles.slice(0, 60) as f}
          <p class="live-file">{f}</p>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .card { background: var(--glass-bg); border: 1px solid var(--glass-border); backdrop-filter: blur(40px); border-radius: 14px; padding: 13px 15px; display: flex; flex-direction: column; gap: 9px; }
  .queue-head { display: flex; align-items: center; justify-content: space-between; }
  .queue-label { font-size: 9px; font-weight: 700; color: var(--accent); letter-spacing: 0.06em; text-transform: uppercase; }
  .queue-path { font-size: 10px; color: var(--text-tertiary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 70%; }
  .top { display: flex; align-items: center; gap: 9px; }
  .spinner { width: 14px; height: 14px; border-radius: 50%; flex-shrink: 0; border: 2px solid rgba(10,132,255,0.22); border-top-color: var(--accent); animation: spin-smooth 1.2s linear infinite; }
  .file { flex: 1; font-size: 12px; font-weight: 500; color: var(--text-secondary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; min-width: 0; }
  .speed { font-size: 12px; font-weight: 600; color: var(--accent); flex-shrink: 0; font-variant-numeric: tabular-nums; }
  .bar { height: 3px; background: rgba(255,255,255,0.06); border-radius: 100px; overflow: hidden; }
  .fill { height: 100%; background: linear-gradient(90deg,var(--accent),#5e5ce6,var(--accent)); border-radius: 100px; transition: width 0.4s cubic-bezier(0.4,0,0.2,1); }
  .fill--indeterminate { width: 60%; background: linear-gradient(90deg,transparent,var(--accent),transparent); animation: shimmer 2.5s linear infinite; background-size: 300% auto; }
  .bottom { display: flex; justify-content: space-between; align-items: center; }
  .meta { font-size: 10px; color: var(--text-tertiary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .eta { opacity: 0.7; }
  .bottom-right { display: flex; align-items: center; gap: 8px; }
  .pct { font-size: 10px; color: var(--text-tertiary); font-variant-numeric: tabular-nums; }
  .cancel { font-size: 9px; font-weight: 700; color: var(--red); background: rgba(255,69,58,0.1); border: 1px solid rgba(255,69,58,0.2); border-radius: 5px; padding: 2px 7px; cursor: pointer; letter-spacing: 0.04em; font-family: var(--font-sf); }
  .cancel:hover { background: rgba(255,69,58,0.2); }
  .live { border-top: 1px solid var(--glass-border); padding-top: 8px; margin-top: 1px; }
  .live-title { font-size: 9px; font-weight: 700; color: var(--text-tertiary); letter-spacing: 0.06em; text-transform: uppercase; margin: 0 0 5px; }
  .live-list { max-height: 120px; overflow-y: auto; display: flex; flex-direction: column; gap: 1px; }
  .live-file { font-size: 10px; font-family: monospace; color: var(--text-tertiary); margin: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>

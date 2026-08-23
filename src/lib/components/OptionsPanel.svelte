<script lang="ts">
  import type { Conflict, FolderMode } from "$lib/types";
  import { THROTTLE_OPTIONS, WORKER_OPTIONS, normalizeThrottleInput } from "$lib/transfer";
  let {
    folderMode = $bindable("into"),
    conflict = $bindable("overwrite"),
    throttle = $bindable(0),
    verify = $bindable(false),
    mode = "copy",
    destPath = "",
    sourcePath = "",
    customSpeed = $bindable(false),
    customSpeedValue = $bindable(50),
    workers = $bindable(0)
  }: {
    folderMode: FolderMode; conflict: Conflict; throttle: number; verify: boolean;
    mode: string; destPath: string; sourcePath: string;
    customSpeed: boolean; customSpeedValue: number;
    workers: number;
  } = $props();

  import { basename } from "$lib/format";
</script>

<div class="opts">
  <div class="opts-row">
    <div class="opt-group">
      <span class="opt-label">Destination</span>
      <div class="seg" role="group" aria-label="Destination behavior">
        {#each [
          { id: "into",  label: "Inside folder",  title: `Files land in: ${destPath && sourcePath ? basename(destPath) + '\\' + basename(sourcePath) + '\\' : 'destination\\source_name\\'}` },
          { id: "merge", label: "Merge contents", title: `Files land directly in: ${destPath ? basename(destPath) + '\\' : 'destination\\'}` },
        ] as opt}
          <button class="seg-btn" class:on={folderMode===opt.id} title={opt.title} onclick={() => folderMode = opt.id as FolderMode}>{opt.label}</button>
        {/each}
      </div>
    </div>
    {#if mode !== "move"}
      <div class="opt-group">
        <span class="opt-label">If a file exists</span>
        <div class="seg" role="group" aria-label="Conflict resolution">
          {#each [
            { id: "overwrite", label: "Overwrite", title: "Replace existing files in the destination" },
            { id: "skip",      label: "Skip",      title: "Keep existing files; only copy new ones" },
          ] as opt}
            <button class="seg-btn" class:on={conflict===opt.id} title={opt.title} onclick={() => conflict = opt.id as Conflict}>{opt.label}</button>
          {/each}
        </div>
      </div>
    {/if}
  </div>
  <div class="opts-row opts-secondary">
    <div class="opt-group">
      <span class="opt-label">Max speed</span>
      <div class="seg" role="group" aria-label="Maximum transfer speed">
        {#each THROTTLE_OPTIONS as opt}
          <button class="seg-btn" class:on={!customSpeed && throttle===opt.value}
            title={opt.value === 0 ? "No speed limit — transfer at full speed" : `Cap the transfer at about ${opt.label}`}
            onclick={() => { customSpeed = false; throttle = opt.value; }}>{opt.label}</button>
        {/each}
        <button class="seg-btn" class:on={customSpeed} title="Set your own speed limit"
          onclick={() => { customSpeed = true; const n = normalizeThrottleInput(customSpeedValue); throttle = n > 0 ? n : 50; customSpeedValue = throttle; }}>Custom</button>
        {#if customSpeed}
          <input class="seg-input" type="number" min="1" max="500" bind:value={customSpeedValue}
            oninput={() => { const n = normalizeThrottleInput(customSpeedValue); customSpeedValue = n; throttle = n; }}
            aria-label="Custom speed limit in megabytes per second" />
          <span class="seg-unit">MB/s</span>
        {/if}
      </div>
    </div>
    <div class="opt-group">
      <span class="opt-label">Verify</span>
      <div class="seg" role="group" aria-label="Verify after transfer">
        <button class="seg-btn" class:on={!verify} title="No verification pass" onclick={() => verify = false}>Off</button>
        <button class="seg-btn" class:on-green={verify}
          title="After a copy/sync, re-compare source and destination to confirm every file arrived (size + timestamp check)"
          onclick={() => verify = true}>On</button>
      </div>
    </div>
    <div class="opt-group">
      <span class="opt-label">Workers</span>
      <div class="seg" role="group" aria-label="Parallel workers" class:seg--disabled={mode === "sync"}>
        {#each WORKER_OPTIONS as opt}
          <button class="seg-btn" class:on={workers === opt.value} disabled={mode === "sync"}
            title={mode === "sync" ? "Sync runs as a single job for safety" : opt.title}
            onclick={() => workers = opt.value}>{opt.label}</button>
        {/each}
      </div>
      {#if mode !== "sync"}
        <p class="opt-hint">Parallel copies folders concurrently. Sync &amp; throttled jobs always run single.</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .opts { display: flex; flex-direction: column; gap: 12px; align-items: center; }
  .opts-row { display: flex; flex-wrap: wrap; gap: 10px 18px; justify-content: center; align-items: flex-start; }
  .opts-secondary { opacity: 0.92; }
  .opt-group { display: flex; flex-direction: column; gap: 5px; align-items: flex-start; }
  .opt-label { font-size: 9px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--text-tertiary); padding-left: 3px; }
  .seg { display: inline-flex; align-items: center; gap: 2px; padding: 3px; background: rgba(255,255,255,0.05); border: 1px solid var(--glass-border); border-radius: 9px; }
  .seg-btn { padding: 4px 11px; border-radius: 7px; font-size: 11px; font-weight: 600; border: none; background: transparent; color: var(--text-secondary); cursor: pointer; transition: background 0.15s, color 0.15s; font-family: var(--font-sf); white-space: nowrap; }
  .seg-btn:hover { background: rgba(255,255,255,0.07); color: var(--text-primary); }
  .seg-btn.on { background: rgba(255,255,255,0.16); color: var(--text-primary); box-shadow: 0 1px 3px rgba(0,0,0,0.35); }
  .seg-btn.on-green { background: rgba(48,209,88,0.2); color: var(--green); box-shadow: 0 1px 3px rgba(0,0,0,0.35); }
  .seg-input { width: 56px; padding: 4px 6px; margin-left: 2px; border-radius: 6px; border: 1px solid var(--glass-border); background: rgba(0,0,0,0.3); color: var(--text-primary); font-size: 11px; font-family: var(--font-sf); outline: none; }
  .seg-input:focus { border-color: var(--accent); }
  .seg-unit { font-size: 10px; color: var(--text-tertiary); padding: 0 4px; }
  .seg--disabled { opacity: 0.45; }
  .seg-btn:disabled { cursor: not-allowed; color: var(--text-tertiary); background: transparent; box-shadow: none; }
  .opt-hint { font-size: 9px; color: var(--text-tertiary); max-width: 220px; line-height: 1.5; }
</style>

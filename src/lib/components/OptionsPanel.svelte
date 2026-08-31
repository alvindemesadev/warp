<script lang="ts">
  import type { Conflict, FolderMode } from "$lib/types";
  import { WORKER_OPTIONS, normalizeThrottleInput } from "$lib/transfer";
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
    workers = $bindable(0),
    filter = $bindable(""),
    sourceInfo = null,
    destInfo = null,
  }: {
    folderMode: FolderMode;
    conflict: Conflict;
    throttle: number;
    verify: boolean;
    mode: string;
    destPath: string;
    sourcePath: string;
    customSpeed: boolean;
    customSpeedValue: number;
    workers: number;
    filter?: string;
    sourceInfo?: import("$lib/types").PathInfo | null;
    destInfo?: import("$lib/types").PathInfo | null;
  } = $props();

  const hasPaths = $derived(!!sourcePath && !!destPath);
  const effectiveWorkers = $derived(
    workers !== 0
      ? workers
      : !hasPaths
        ? 4
        : sourceInfo?.removable ||
            destInfo?.removable ||
            /^[D-Z]:/i.test(sourcePath) ||
            /^[D-Z]:/i.test(destPath)
          ? 2
          : sourcePath.startsWith("\\\\") || destPath.startsWith("\\\\")
            ? 3
            : 4,
  );

  import { basename } from "$lib/format";
</script>

<div class="opts">
  <div class="opts-row">
    <div class="opt-group">
      <span class="opt-label">Destination</span>
      <div class="seg" role="group" aria-label="Destination behavior">
        {#each [{ id: "into", label: "Inside folder", title: `Files land in: ${destPath && sourcePath ? basename(destPath) + "\\" + basename(sourcePath) + "\\" : "destination\\source_name\\"}` }, { id: "merge", label: "Merge contents", title: `Files land directly in: ${destPath ? basename(destPath) + "\\" : "destination\\"}` }] as opt}
          <button
            class="seg-btn"
            class:on={folderMode === opt.id}
            title={opt.title}
            onclick={() => (folderMode = opt.id as FolderMode)}>{opt.label}</button
          >
        {/each}
      </div>
    </div>
    <div class="opt-group">
      <span class="opt-label">If a file exists</span>
      <div
        class="seg"
        role="group"
        aria-label="Conflict resolution"
        class:seg--disabled={mode === "move"}
      >
        {#each [{ id: "overwrite", label: "Overwrite", title: "Replace existing files in the destination" }, { id: "skip", label: "Skip", title: "Keep existing files; only copy new ones" }] as opt}
          <button
            class="seg-btn"
            class:on={conflict === opt.id}
            disabled={mode === "move"}
            title={mode === "move" ? "Not applicable for Move" : opt.title}
            onclick={() => (conflict = opt.id as Conflict)}>{opt.label}</button
          >
        {/each}
      </div>
    </div>
  </div>
  <div class="opts-row opts-secondary">
    <div class="opt-group">
      <span class="opt-label">Max speed</span>
      <div class="seg" role="group" aria-label="Maximum transfer speed">
        <button
          class="seg-btn"
          class:on={!customSpeed}
          title="No speed limit — transfer at full speed"
          onclick={() => {
            customSpeed = false;
            throttle = 0;
          }}>Unlimited</button
        >
        <button
          class="seg-btn"
          class:on={customSpeed}
          title="Set your own speed limit"
          onclick={() => {
            customSpeed = true;
            const n = normalizeThrottleInput(customSpeedValue);
            throttle = n > 0 ? n : 50;
            customSpeedValue = throttle;
          }}>Custom</button
        >
        <input
          class="seg-input seg-input--no-spin"
          type="number"
          min="1"
          max="500"
          bind:value={customSpeedValue}
          oninput={() => {
            const n = normalizeThrottleInput(customSpeedValue);
            customSpeedValue = n;
            throttle = n;
          }}
          aria-label="Custom speed limit in megabytes per second"
          class:seg-input--hidden={!customSpeed}
        />
        <span class="seg-unit" class:seg-unit--hidden={!customSpeed}>MB/s</span>
      </div>
    </div>
    <div class="opt-group">
      <span class="opt-label">Verify</span>
      <div class="seg" role="group" aria-label="Verify after transfer">
        <button
          class="seg-btn"
          class:on={!verify}
          title="No verification pass"
          onclick={() => (verify = false)}>Off</button
        >
        <button
          class="seg-btn"
          class:on-green={verify}
          title="After a copy/sync, re-compare source and destination to confirm every file arrived (size + timestamp check)"
          onclick={() => (verify = true)}>On</button
        >
      </div>
    </div>
  </div>
  <div class="opts-row">
    <div class="right-stack">
      <div class="opt-group">
        <span class="opt-label">Workers</span>
        <div class="seg" role="group" aria-label="Parallel workers" class:seg--disabled={false}>
          {#each WORKER_OPTIONS as opt}
            {@const isUsb =
              hasPaths &&
              (sourceInfo?.removable ||
                destInfo?.removable ||
                /^[D-Z]:/i.test(sourcePath) ||
                /^[D-Z]:/i.test(destPath))}
            {@const isSyncTwoPhase = mode === "sync"}
            <button
              class="seg-btn"
              class:on={workers === opt.value}
              class:on-blue={workers === 2 && opt.value === 2}
              class:on-green={workers === 4 && opt.value === 4}
              class:on-yellow={workers === 6 && opt.value === 6}
              class:on-orange={workers === 8 && opt.value === 8}
              disabled={isUsb && opt.value !== 0 && opt.value !== 2}
              title={isUsb && opt.value !== 0 && opt.value !== 2
                ? "USB — only Auto and 2 are available"
                : isSyncTwoPhase && opt.value !== 0
                  ? `Sync: ${opt.value} workers for delete -> ${opt.value} workers for copy (sequential, never 4+4)`
                  : isSyncTwoPhase
                    ? "Sync: Auto picks workers for two-phase delete -> copy"
                    : opt.title}
              onclick={() => (workers = opt.value)}>{opt.label}</button
            >
          {/each}
        </div>
        <p class="opt-hint" class:opt-hint--hidden={!(workers === 0 && hasPaths)}>
          Auto -> {effectiveWorkers} workers ({sourceInfo?.removable || destInfo?.removable
            ? "USB"
            : sourcePath.startsWith("\\\\") || destPath.startsWith("\\\\")
              ? "network"
              : "local"})
        </p>
      </div>
      <div class="opt-group">
        <span class="opt-label">Skip junk</span>
        <div class="seg" role="group" aria-label="Skip junk files">
          <button
            class="seg-btn"
            class:on={filter === ""}
            title="Don't skip junk"
            onclick={() => (filter = "")}>Off</button
          >
          <button
            class="seg-btn"
            class:on-green={filter !== ""}
            title="Skip *.tmp, *.log, node_modules, .git, __pycache__, .DS_Store, Thumbs.db"
            onclick={() => {
              filter = "*.tmp; *.log; node_modules; .git; __pycache__; .DS_Store; Thumbs.db";
            }}>On</button
          >
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .opts {
    display: flex;
    flex-direction: column;
    gap: 10px;
    align-items: center;
  }
  .opts-row {
    display: flex;
    flex-wrap: wrap;
    gap: 8px 14px;
    justify-content: center;
    align-items: flex-start;
  }
  .opts-secondary {
    opacity: 0.95;
  }
  .opt-group {
    display: flex;
    flex-direction: column;
    gap: 5px;
    align-items: flex-start;
  }
  .opt-label {
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-secondary);
    padding-left: 3px;
  }
  .seg {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 3px;
    background: var(--seg-bg);
    border: 1px solid var(--glass-border);
    border-radius: 9px;
    min-width: 0;
    flex-wrap: nowrap;
    transition:
      background 0.2s,
      border-color 0.2s;
  }
  .seg-btn {
    padding: 4px 11px;
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
    flex-shrink: 0;
  }
  .seg-btn:hover {
    background: var(--seg-hover);
    color: var(--text-primary);
  }
  .seg-btn.on {
    background: var(--seg-active-bg);
    color: var(--text-primary);
    box-shadow: var(--seg-active-shadow);
  }
  .seg-btn.on-green {
    background: rgba(48, 209, 88, 0.2);
    color: var(--green);
    box-shadow: var(--seg-active-shadow);
  }
  .seg-btn.on-blue {
    background: rgba(10, 132, 255, 0.2);
    color: var(--accent);
    box-shadow: var(--seg-active-shadow);
  }
  .seg-btn.on-yellow {
    background: rgba(255, 214, 10, 0.2);
    color: var(--yellow);
    box-shadow: var(--seg-active-shadow);
  }
  .seg-btn.on-orange {
    background: rgba(255, 159, 10, 0.2);
    color: var(--orange);
    box-shadow: var(--seg-active-shadow);
  }
  .seg-input {
    width: 56px;
    padding: 4px 6px;
    border-radius: 6px;
    border: 1px solid var(--glass-border);
    background: var(--input-bg);
    color: var(--text-primary);
    font-size: 11px;
    font-family: var(--font-sf);
    outline: none;
    text-align: center;
    transition:
      opacity 0.12s,
      width 0.12s,
      background 0.2s;
  }
  .seg-input--hidden {
    width: 0;
    padding: 0;
    border-width: 0;
    opacity: 0;
    pointer-events: none;
  }
  .seg-input--no-spin::-webkit-outer-spin-button,
  .seg-input--no-spin::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }
  .seg-input--no-spin {
    -moz-appearance: textfield;
    appearance: textfield;
  }
  .seg-input:focus {
    border-color: var(--accent);
  }
  .seg-unit {
    font-size: 10px;
    color: var(--text-secondary);
    padding: 0 4px;
    transition:
      opacity 0.12s,
      width 0.12s,
      padding 0.12s;
  }
  .seg-unit--hidden {
    opacity: 0;
    pointer-events: none;
    width: 0;
    padding: 0;
    overflow: hidden;
  }
  .seg--disabled {
    opacity: 0.45;
  }
  .seg-btn:disabled {
    cursor: not-allowed;
    color: var(--text-secondary);
    background: transparent;
    box-shadow: none;
  }
  .right-stack {
    display: flex;
    flex-direction: row;
    gap: 14px;
    align-items: flex-start;
  }
  .opt-hint {
    font-size: 9px;
    color: var(--text-secondary);
    max-width: 220px;
    line-height: 1.5;
    min-height: 14px;
  }
  .opt-hint--hidden {
    visibility: hidden;
  }
</style>

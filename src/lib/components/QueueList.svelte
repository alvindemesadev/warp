<script lang="ts">
  import { basename } from "$lib/format";
  import type { QueueJob } from "$lib/types";
  let { queue, onRemove, onClear }: { queue: QueueJob[]; onRemove: (id: number) => void; onClear: () => void } = $props();
</script>

<div class="queue">
  <div class="queue-head">
    <p class="queue-title">Queue · {queue.length}</p>
    <button onclick={onClear} class="clear">Clear</button>
  </div>
  {#each queue as job, i}
    <div class="row">
      <span class="idx">{i + 1}</span>
      <div class="info">
        <p class="path">{basename(job.source)} → {basename(job.dest)}</p>
        <p class="meta">{job.mode}{job.verify ? ' · verify' : ''}{job.throttle ? ` · ${job.throttle} MB/s` : ''}</p>
      </div>
      <button onclick={() => onRemove(job.id)} aria-label="Remove from queue" class="rm">
        <svg width="6" height="6" viewBox="0 0 8 8" fill="none"><path d="M1 1l6 6M7 1L1 7" stroke="white" stroke-width="1.6" stroke-linecap="round"/></svg>
      </button>
    </div>
  {/each}
</div>

<style>
  .queue { background: var(--surface-2); border: 1px solid var(--glass-border); border-radius: 12px; padding: 8px; display: flex; flex-direction: column; gap: 4px; }
  .queue-head { display: flex; align-items: center; justify-content: space-between; padding: 2px 4px 4px; }
  .queue-title { font-size: 10px; font-weight: 700; color: var(--text-tertiary); letter-spacing: 0.06em; text-transform: uppercase; margin: 0; }
  .clear { font-size: 10px; color: var(--red); background: none; border: none; cursor: pointer; font-family: var(--font-sf); }
  .row { display: flex; align-items: center; gap: 8px; padding: 6px 8px; background: rgba(255,255,255,0.03); border-radius: 8px; }
  .idx { font-size: 10px; color: var(--text-tertiary); flex-shrink: 0; width: 16px; font-variant-numeric: tabular-nums; }
  .info { flex: 1; min-width: 0; }
  .path { font-size: 11px; color: var(--text-secondary); margin: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .meta { font-size: 9px; color: var(--text-tertiary); margin: 1px 0 0; }
  .rm { width: 16px; height: 16px; border-radius: 50%; background: rgba(255,255,255,0.08); border: none; padding: 0; display: flex; align-items: center; justify-content: center; flex-shrink: 0; cursor: pointer; opacity: 0.6; }
  .rm:hover { opacity: 1; }
</style>

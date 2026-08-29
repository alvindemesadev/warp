<script lang="ts">
  import { basename, fmtBytes, fmtDuration, timeAgo } from "$lib/format";
  import type { RecentEntry } from "$lib/types";
  let { entries, onLoad, onClear, onClose }: { entries: RecentEntry[]; onLoad: (r: RecentEntry) => void; onClear: () => void; onClose: () => void } = $props();
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-label="Recent transfers" tabindex="-1">
  <button type="button" aria-label="Dismiss" onclick={onClose} class="backdrop"></button>
  <div class="panel">
    <div class="head">
      <p class="head-title">Recent</p>
      <button onclick={onClear} class="clear">Clear</button>
    </div>
    {#each entries as r}
      <button onclick={() => onLoad(r)} class="row">
        <div class="row-top">
          <span class="row-name">{basename(r.source)} → {basename(r.dest)}</span>
          <span class="row-time">{timeAgo(r.timestamp)}</span>
        </div>
        <span class="row-meta">{r.mode} · {fmtBytes(r.bytes)} · {fmtDuration(r.duration_ms)}</span>
      </button>
    {/each}
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 100; }
  .backdrop { position: fixed; inset: 0; border: none; padding: 0; margin: 0; background: rgba(0,0,0,0.5); cursor: default; }
  .panel { position: fixed; top: 50px; right: 14px; width: 300px; background: #1c1c1e; border: 1px solid rgba(255,255,255,0.1); border-radius: 14px; overflow: hidden; }
  .head { padding: 12px 14px; border-bottom: 1px solid rgba(255,255,255,0.07); display: flex; align-items: center; justify-content: space-between; }
  .head-title { font-size: 11px; font-weight: 700; color: var(--text-tertiary); letter-spacing: 0.06em; text-transform: uppercase; margin: 0; }
  .clear { font-size: 10px; color: var(--red); background: none; border: none; cursor: pointer; font-family: var(--font-sf); }
  .row { width: 100%; padding: 10px 14px; text-align: left; background: transparent; border: none; border-bottom: 1px solid rgba(255,255,255,0.05); cursor: pointer; display: flex; flex-direction: column; gap: 2px; }
  .row:hover { background: rgba(255,255,255,0.04); }
  .row-top { display: flex; align-items: center; justify-content: space-between; }
  .row-name { font-size: 12px; font-weight: 500; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; max-width: 200px; }
  .row-time { font-size: 10px; color: var(--text-tertiary); flex-shrink: 0; margin-left: 8px; }
  .row-meta { font-size: 10px; color: var(--text-tertiary); }
</style>

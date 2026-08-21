<script lang="ts">
  import { basename } from "$lib/format";
  import type { Preset } from "$lib/types";
  let { presets, onLoad, onDelete, onClose }: { presets: Preset[]; onLoad: (p: Preset) => void; onDelete: (name: string) => void; onClose: () => void } = $props();
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-label="Presets" tabindex="-1">
  <button type="button" aria-label="Dismiss" onclick={onClose} class="backdrop"></button>
  <div class="panel">
    <div class="head"><p class="head-title">Presets</p></div>
    {#if presets.length === 0}
      <p class="empty">No presets yet. Set up a transfer and click “Save preset”.</p>
    {:else}
      {#each presets as p}
        <div class="row">
          <button onclick={() => onLoad(p)} class="preset-btn">
            <span class="preset-name">{p.name}</span>
            <span class="preset-meta">{basename(p.source)} → {basename(p.dest)} · {p.mode}{p.verify ? ' · verify' : ''}{p.throttle ? ` · ${p.throttle} MB/s` : ''}</span>
          </button>
          <button onclick={() => onDelete(p.name)} aria-label="Delete preset" class="delete">Delete</button>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 100; }
  .backdrop { position: fixed; inset: 0; border: none; padding: 0; margin: 0; background: rgba(0,0,0,0.5); cursor: default; }
  .panel { position: absolute; top: 50%; left: 50%; transform: translate(-50%,-50%); width: 320px; max-height: 70vh; overflow-y: auto; background: #1c1c1e; border: 1px solid rgba(255,255,255,0.1); border-radius: 14px; }
  .head { padding: 12px 14px; border-bottom: 1px solid rgba(255,255,255,0.07); }
  .head-title { font-size: 11px; font-weight: 700; color: var(--text-tertiary); letter-spacing: 0.06em; text-transform: uppercase; margin: 0; }
  .empty { font-size: 11px; color: var(--text-tertiary); padding: 16px 14px; margin: 0; text-align: center; }
  .row { display: flex; align-items: center; gap: 8px; padding: 10px 14px; border-bottom: 1px solid rgba(255,255,255,0.05); }
  .preset-btn { flex: 1; min-width: 0; text-align: left; background: transparent; border: none; cursor: pointer; padding: 0; font-family: var(--font-sf); }
  .preset-name { display: block; font-size: 12px; font-weight: 600; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .preset-meta { display: block; font-size: 10px; color: var(--text-tertiary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .delete { font-size: 10px; color: var(--red); background: none; border: none; cursor: pointer; flex-shrink: 0; font-family: var(--font-sf); }
</style>

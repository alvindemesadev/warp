<script lang="ts">
  import { fmtBytes, fmtDuration } from "$lib/format";
  import type { WarpSummary } from "$lib/types";
  let { results, cancelled, onDone }: { results: WarpSummary[]; cancelled: boolean; onDone: () => void } = $props();
  const anyFailed = $derived(results.some((r) => r.failed > 0 || !!r.errorMessage));
  const headerColor = $derived(cancelled ? "255,159,10" : anyFailed ? "255,69,58" : "48,209,88");
</script>

<div class="wrap animate-fade-up">
  <div class="card">
    <div class="head">
      <div class="icon" style:background={`rgba(${headerColor},0.14)`}>
        {#if cancelled}
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none"><path d="M6 6l8 8M14 6l-8 8" stroke="#ff9f0a" stroke-width="1.8" stroke-linecap="round"/></svg>
        {:else if anyFailed}
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none"><circle cx="10" cy="10" r="7" stroke="#ff453a" stroke-width="1.5"/><path d="M10 6.5v4m0 2.5v.5" stroke="#ff453a" stroke-width="1.6" stroke-linecap="round"/></svg>
        {:else}
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none"><path d="M5.5 10.5l3 3 6-6.5" stroke="#30d158" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
        {/if}
      </div>
      <div class="head-text">
        <p class="title">{cancelled ? "Queue cancelled" : anyFailed ? "Queue finished with errors" : "Queue complete"}</p>
        <p class="sub">{results.length} {results.length === 1 ? "job" : "jobs"} · {results.reduce((n, r) => n + r.transferred, 0).toLocaleString()} files · {fmtBytes(results.reduce((n, r) => n + r.bytesTransferred, 0))}</p>
      </div>
    </div>
    <div class="list">
      {#each results as r, i}
        <div class="row" class:row--border={i < results.length - 1}>
          <div class="row-icon" style:background={r.cancelled?'rgba(255,159,10,0.14)':r.failed>0?'rgba(255,69,58,0.14)':'rgba(48,209,88,0.14)'}>
            {#if r.cancelled}
              <svg width="11" height="11" viewBox="0 0 20 20" fill="none"><path d="M6 6l8 8M14 6l-8 8" stroke="#ff9f0a" stroke-width="2" stroke-linecap="round"/></svg>
            {:else if r.failed > 0}
              <svg width="11" height="11" viewBox="0 0 20 20" fill="none"><circle cx="10" cy="10" r="7" stroke="#ff453a" stroke-width="1.8"/></svg>
            {:else}
              <svg width="11" height="11" viewBox="0 0 20 20" fill="none"><path d="M5.5 10.5l3 3 6-6.5" stroke="#30d158" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/></svg>
            {/if}
          </div>
          <div class="row-text">
            <p class="row-main">{r.transferred.toLocaleString()} transferred · {fmtBytes(r.bytesTransferred)} · {fmtDuration(r.durationMs)}</p>
            {#if r.verified}
              <p class="row-verify" class:row-verify--ok={r.verifyMismatches===0}>{r.verifyMismatches === 0 ? "✓ Verified" : `⚠ ${r.verifyMismatches} mismatch${r.verifyMismatches === 1 ? '' : 'es'}`}</p>
            {/if}
            {#if r.errorMessage}<p class="row-err">{r.errorMessage}</p>{/if}
          </div>
        </div>
      {/each}
    </div>
  </div>
  <button onclick={onDone} class="btn">Done</button>
</div>

<style>
  .wrap { display: flex; flex-direction: column; gap: 11px; }
  .card { background: var(--glass-bg); border: 1px solid var(--glass-border); backdrop-filter: blur(48px) saturate(180%); border-radius: 16px; overflow: hidden; }
  .head { padding: 15px 16px; display: flex; align-items: center; gap: 12px; border-bottom: 1px solid var(--glass-border); }
  .icon { width: 38px; height: 38px; border-radius: 11px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; }
  .head-text { flex: 1; min-width: 0; }
  .title { font-size: 15px; font-weight: 600; color: var(--text-primary); margin: 0; letter-spacing: -0.01em; }
  .sub { font-size: 11px; color: var(--text-tertiary); margin: 3px 0 0; }
  .list { max-height: 220px; overflow-y: auto; }
  .row { display: flex; align-items: center; gap: 10px; padding: 10px 14px; }
  .row--border { border-bottom: 1px solid var(--glass-border); }
  .row-icon { width: 20px; height: 20px; border-radius: 6px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; }
  .row-text { flex: 1; min-width: 0; }
  .row-main { font-size: 11px; color: var(--text-secondary); margin: 0; }
  .row-verify { font-size: 9px; margin: 1px 0 0; }
  .row-verify--ok { color: var(--green); }
  .row-verify:not(.row-verify--ok) { color: var(--orange); }
  .row-err { font-size: 9px; margin: 1px 0 0; color: var(--red); }
  .btn { width: 100%; padding: 12px; border-radius: 14px; border: none; font-size: 14px; font-weight: 600; background: var(--accent); color: white; box-shadow: 0 2px 20px rgba(10,132,255,0.28); cursor: pointer; transition: all 0.15s; outline: none; }
  .btn:hover { background: var(--accent-hover); }
</style>

<script lang="ts">
  import { fmtBytes, fmtDuration } from "$lib/format";
  import type { WarpSummary } from "$lib/types";
  let {
    summary,
    mode,
    sourcePath,
    destPath,
    errorLogs = [],
    onReset,
  }: {
    summary: WarpSummary;
    mode: string;
    sourcePath: string;
    destPath: string;
    errorLogs: string[];
    onReset: () => void;
  } = $props();
</script>

<div class="wrap animate-fade-up">
  <div class="card">
    <div class="head">
      <div
        class="icon"
        class:icon--cancel={summary.cancelled}
        class:icon--error={summary.failed > 0}
        class:icon--ok={!summary.cancelled && summary.failed === 0}
      >
        {#if summary.cancelled}
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none"
            ><path
              d="M6 6l8 8M14 6l-8 8"
              stroke="#ff9f0a"
              stroke-width="1.8"
              stroke-linecap="round"
            /></svg
          >
        {:else if summary.failed > 0}
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none"
            ><circle cx="10" cy="10" r="7" stroke="#ff453a" stroke-width="1.5" /><path
              d="M10 6.5v4m0 2.5v.5"
              stroke="#ff453a"
              stroke-width="1.6"
              stroke-linecap="round"
            /></svg
          >
        {:else}
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none"
            ><path
              d="M5.5 10.5l3 3 6-6.5"
              stroke="#30d158"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            /></svg
          >
        {/if}
      </div>
      <div class="head-text">
        <p class="title">
          {summary.cancelled
            ? "Transfer cancelled"
            : summary.errorMessage
              ? "Transfer failed"
              : summary.failed > 0
                ? "Completed with errors"
                : "Transfer complete"}
        </p>
        <p class="sub">
          {fmtDuration(summary.durationMs)} · {mode}{#if summary.bytesTransferred > 0}
            · {fmtBytes(summary.bytesTransferred)}{/if}
          {#if (summary.workersUsed ?? 1) > 1}
            · <span class="sub-accent">⚡ {summary.workersUsed} workers</span>{/if}
          {#if (summary.retriedOk ?? 0) > 0}
            · <span class="sub-green">↻ {summary.retriedOk} recovered</span>{/if}
        </p>
        {#if summary.verified}
          <p
            class="verify"
            class:verify--ok={summary.verifyMismatches === 0}
            class:verify--warn={summary.verifyMismatches !== 0}
          >
            {summary.verifyMismatches === 0
              ? "✓ Verified — all files match"
              : `⚠ Verified — ${summary.verifyMismatches} file${summary.verifyMismatches === 1 ? "" : "s"} differ`}
          </p>
        {/if}
      </div>
    </div>
    <div class="stats">
      {#each [{ label: mode === "move" ? "Moved" : mode === "sync" ? "Synced" : "Copied", value: summary.transferred }, { label: "Skipped", value: summary.skipped }, { label: "Failed", value: summary.failed }] as stat, i}
        <div class="stat" class:stat--border={i < 2}>
          <p
            class="stat-value"
            class:stat-value--accent={i === 0}
            class:stat-value--secondary={i === 1}
            class:stat-value--red={i === 2 && summary.failed > 0}
            class:stat-value--tertiary={i === 2 && summary.failed === 0}
          >
            {stat.value.toLocaleString()}
          </p>
          <p class="stat-label">{stat.label}</p>
        </div>
      {/each}
    </div>
  </div>
  {#if mode === "move" && summary.skipped > 0 && !summary.cancelled}
    <div class="info-box">
      <p class="info-text">
        ℹ︎ {summary.skipped.toLocaleString()} file{summary.skipped === 1 ? " was" : "s were"} skipped (already
        exists) and remain in the source. Only {summary.transferred.toLocaleString()} moved.
      </p>
    </div>
  {/if}
  {#if summary.errorMessage}
    <div class="error-box">
      <p class="error-title">Transfer Error (code {summary.errorCode})</p>
      <p class="error-msg">{summary.errorMessage}</p>
    </div>
  {/if}
  <div class="paths">
    {#each [{ label: "From", path: sourcePath }, { label: "To", path: destPath }] as row}
      <div class="path-row">
        <span class="path-label">{row.label}</span>
        <span class="path-val" title={row.path}>{row.path}</span>
      </div>
    {/each}
  </div>
  {#if errorLogs.length > 0}
    <div class="logs">
      <div class="logs-head">
        <p class="logs-title">{errorLogs.length} Error{errorLogs.length !== 1 ? "s" : ""}</p>
        <button
          onclick={async () => {
            try {
              await navigator.clipboard.writeText(errorLogs.join("\n"));
            } catch {}
          }}
          class="logs-copy"
          title="Copy errors to clipboard">Copy</button
        >
      </div>
      <div class="logs-list">
        {#each errorLogs as log}
          <p class="log">{log}</p>
        {/each}
      </div>
      <p class="logs-hint">Log file: %TEMP%\warp.log — open with Notepad to diagnose</p>
    </div>
  {/if}

  <button onclick={onReset} class="btn">New Transfer</button>
</div>

<style>
  .wrap {
    display: flex;
    flex-direction: column;
    gap: 11px;
  }
  .card {
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    backdrop-filter: blur(48px) saturate(180%);
    border-radius: 16px;
    overflow: hidden;
  }
  .head {
    padding: 15px 16px;
    display: flex;
    align-items: center;
    gap: 12px;
    border-bottom: 1px solid var(--glass-border);
  }
  .icon {
    width: 38px;
    height: 38px;
    border-radius: 11px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .icon--ok {
    background: rgba(48, 209, 88, 0.14);
  }
  .icon--error {
    background: rgba(255, 69, 58, 0.14);
  }
  .icon--cancel {
    background: rgba(255, 159, 10, 0.14);
  }
  .head-text {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
    letter-spacing: -0.01em;
  }
  .sub {
    font-size: 11px;
    color: var(--text-tertiary);
    margin: 3px 0 0;
  }
  .sub-accent {
    color: var(--accent);
  }
  .sub-green {
    color: var(--green);
  }
  .verify {
    font-size: 10px;
    font-weight: 600;
    margin: 4px 0 0;
  }
  .verify--ok {
    color: var(--green);
  }
  .verify--warn {
    color: var(--orange);
  }
  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
  }
  .stat {
    padding: 13px 10px;
    text-align: center;
  }
  .stat--border {
    border-right: 1px solid var(--glass-border);
  }
  .stat-value {
    font-size: 22px;
    font-weight: 700;
    margin: 0;
    line-height: 1;
    letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums;
  }
  .stat-value--accent {
    color: var(--accent);
  }
  .stat-value--secondary {
    color: var(--text-secondary);
  }
  .stat-value--red {
    color: var(--red);
  }
  .stat-value--tertiary {
    color: var(--text-tertiary);
  }
  .stat-label {
    font-size: 9px;
    font-weight: 600;
    color: var(--text-tertiary);
    margin: 4px 0 0;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .info-box {
    background: rgba(10, 132, 255, 0.08);
    border: 1px solid rgba(10, 132, 255, 0.2);
    border-radius: 12px;
    padding: 10px 14px;
  }
  .info-text {
    font-size: 11px;
    color: var(--accent);
    margin: 0;
    line-height: 1.5;
  }
  .error-box {
    background: rgba(255, 69, 58, 0.08);
    border: 1px solid rgba(255, 69, 58, 0.2);
    border-radius: 12px;
    padding: 10px 14px;
  }
  .error-title {
    font-size: 10px;
    font-weight: 700;
    color: var(--red);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    margin: 0 0 4px;
  }
  .error-msg {
    font-size: 11px;
    color: rgba(255, 69, 58, 0.8);
    margin: 0;
    line-height: 1.5;
  }
  .paths {
    background: rgba(255, 255, 255, 0.03);
    border: 1px solid var(--glass-border);
    border-radius: 12px;
    padding: 11px 14px;
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .path-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .path-label {
    font-size: 9px;
    font-weight: 700;
    color: var(--text-tertiary);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    width: 26px;
    flex-shrink: 0;
  }
  .path-val {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .logs {
    background: rgba(255, 69, 58, 0.05);
    border: 1px solid rgba(255, 69, 58, 0.15);
    border-radius: 12px;
    padding: 11px 14px;
  }
  .logs-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 0 0 7px;
  }
  .logs-title {
    font-size: 9px;
    font-weight: 700;
    color: var(--red);
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin: 0;
  }
  .logs-copy {
    font-size: 9px;
    font-weight: 600;
    color: var(--red);
    background: rgba(255, 69, 58, 0.12);
    border: 1px solid rgba(255, 69, 58, 0.2);
    border-radius: 5px;
    padding: 2px 6px;
    cursor: pointer;
  }
  .logs-hint {
    font-size: 9px;
    color: var(--text-tertiary);
    margin: 6px 0 0;
  }
  .logs-list {
    max-height: 72px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .log {
    font-size: 10px;
    font-family: monospace;
    color: rgba(255, 69, 58, 0.65);
    margin: 0;
  }
  .btn {
    width: 100%;
    padding: 12px;
    border-radius: 14px;
    border: none;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.01em;
    background: var(--accent);
    color: white;
    box-shadow: 0 2px 20px rgba(10, 132, 255, 0.28);
    cursor: pointer;
    transition: all 0.15s;
    outline: none;
  }
  .btn:hover {
    background: var(--accent-hover);
  }
</style>

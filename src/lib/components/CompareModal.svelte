<script lang="ts">
  import { fmtBytes, fmtFiles } from "$lib/format";
  let {
    filesToCopy,
    bytesToCopy,
    skipped,
    extra,
    onClose,
  }: {
    filesToCopy: number;
    bytesToCopy: number;
    skipped: number;
    extra: number;
    onClose: () => void;
  } = $props();
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-label="Compare">
  <button type="button" aria-label="Dismiss" onclick={onClose} class="backdrop"></button>
  <div class="panel">
    <p class="title">Compare — Dry Run</p>
    <p class="line">{fmtFiles(filesToCopy)} - {fmtBytes(bytesToCopy)} <strong>will copy</strong></p>
    <p class="line sub">{skipped} skipped - {extra} extra in dest (Sync would delete)</p>
    <button onclick={onClose} class="btn">Close</button>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
  }
  .backdrop {
    position: fixed;
    inset: 0;
    border: none;
    background: rgba(0, 0, 0, 0.5);
    cursor: default;
  }
  .panel {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 340px;
    background: #1c1c1e;
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 14px;
    padding: 16px;
    text-align: center;
  }
  .title {
    font-size: 12px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 8px;
  }
  .line {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 2px 0;
  }
  .sub {
    color: var(--text-tertiary);
  }
  .btn {
    margin-top: 12px;
    padding: 6px 14px;
    border-radius: 8px;
    border: none;
    background: var(--accent);
    color: white;
    cursor: pointer;
  }
</style>

<script lang="ts">
  import { basename } from "$lib/format";
  let {
    pendingPath,
    onCancel,
    onPick,
  }: { pendingPath: string; onCancel: () => void; onPick: (slot: "source" | "dest") => void } =
    $props();
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-label="Replace which slot?" tabindex="-1">
  <button type="button" aria-label="Dismiss" onclick={onCancel} class="overlay-backdrop"></button>
  <div class="modal">
    <p class="title">Replace which slot?</p>
    <p class="sub">{basename(pendingPath)}</p>
    <div class="actions">
      <button onclick={() => onPick("source")} class="btn btn--source">← Source</button>
      <button onclick={() => onPick("dest")} class="btn btn--dest">Destination →</button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }
  .overlay-backdrop {
    position: fixed;
    inset: 0;
    border: none;
    padding: 0;
    margin: 0;
    background: rgba(0, 0, 0, 0.45);
    backdrop-filter: blur(6px);
    cursor: default;
  }
  .modal {
    position: relative;
    background: var(--modal-bg);
    border: 1px solid var(--modal-border);
    box-shadow: var(--modal-shadow);
    border-radius: 18px;
    padding: 22px;
    max-width: 320px;
    width: 100%;
  }
  .title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0 0 6px;
  }
  .sub {
    font-size: 11px;
    color: var(--text-tertiary);
    margin: 0 0 16px;
  }
  .actions {
    display: flex;
    gap: 8px;
  }
  .btn {
    flex: 1;
    padding: 10px;
    border-radius: 10px;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition:
      opacity 0.15s,
      background 0.15s;
  }
  .btn--source {
    border: 1px solid rgba(10, 132, 255, 0.3);
    background: rgba(10, 132, 255, 0.12);
    color: var(--accent);
  }
  .btn--source:hover {
    background: rgba(10, 132, 255, 0.2);
  }
  .btn--dest {
    border: 1px solid rgba(48, 209, 88, 0.3);
    background: rgba(48, 209, 88, 0.12);
    color: var(--green);
  }
  .btn--dest:hover {
    background: rgba(48, 209, 88, 0.2);
  }
</style>

<script lang="ts">
  import { basename } from "$lib/format";
  let {
    destPath,
    onCancel,
    onConfirm,
  }: { destPath: string; onCancel: () => void; onConfirm: () => void } = $props();
</script>

<div
  class="overlay"
  role="dialog"
  aria-modal="true"
  aria-label="Dangerous combination"
  tabindex="-1"
>
  <button type="button" aria-label="Dismiss" onclick={onCancel} class="overlay-backdrop"></button>
  <div class="modal">
    <div class="modal-head">
      <div class="icon icon--red">
        <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
          <path
            d="M10 3l7.5 13H2.5L10 3z"
            stroke="#ff453a"
            stroke-width="1.5"
            fill="none"
            stroke-linejoin="round"
          />
          <path d="M10 8v4m0 2.5v.5" stroke="#ff453a" stroke-width="1.5" stroke-linecap="round" />
        </svg>
      </div>
      <div>
        <p class="modal-title">Dangerous combination</p>
        <p class="modal-sub">Sync + Merge will delete files</p>
      </div>
    </div>
    <p class="modal-body">
      Sync + Merge Contents will mirror the source directly into the destination root. Files already
      in
      <strong class="strong">{basename(destPath) || "destination"}</strong> that are not in the
      source will be
      <strong class="strong strong--red">permanently deleted</strong>.
    </p>
    <div class="modal-actions">
      <button onclick={onCancel} class="btn btn--ghost">Cancel</button>
      <button onclick={onConfirm} class="btn btn--danger">Continue</button>
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
    padding: 24px;
    max-width: 340px;
    width: 100%;
  }
  .modal-head {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 14px;
  }
  .icon {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .icon--red {
    background: rgba(255, 69, 58, 0.15);
  }
  .modal-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }
  .modal-sub {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 3px 0 0;
  }
  .modal-body {
    font-size: 12px;
    color: var(--text-secondary);
    line-height: 1.5;
    margin: 0 0 18px;
  }
  .strong {
    color: var(--text-primary);
  }
  .strong--red {
    color: var(--red);
  }
  .modal-actions {
    display: flex;
    gap: 8px;
  }
  .btn {
    flex: 1;
    padding: 10px;
    border-radius: 10px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s;
  }
  .btn--ghost {
    border: 1px solid var(--glass-border);
    background: var(--surface-2);
    color: var(--text-primary);
  }
  .btn--ghost:hover {
    background: var(--glass-hover);
  }
  .btn--danger {
    border: none;
    background: var(--red);
    color: white;
    font-weight: 600;
  }
  .btn--danger:hover {
    opacity: 0.85;
  }
</style>

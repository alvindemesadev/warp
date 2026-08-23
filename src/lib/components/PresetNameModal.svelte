<script lang="ts">
  let { name = $bindable(""), onCancel, onSave }: { name: string; onCancel: () => void; onSave: () => void } = $props();

  function focusInput(node: HTMLInputElement) { node.focus(); }
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-label="Save preset" tabindex="-1">
  <button type="button" aria-label="Dismiss" onclick={onCancel} class="overlay-backdrop"></button>
  <div class="modal">
    <p class="title">Name this preset</p>
    <p class="sub">Saves the current source, destination, and options for one-click reuse.</p>
    <input type="text" use:focusInput bind:value={name} placeholder="Preset name"
      onkeydown={(e) => { if (e.key === "Enter") onSave(); }}
      class="input" aria-label="Preset name" />
    <div class="actions">
      <button onclick={onCancel} class="btn btn--ghost">Cancel</button>
      <button onclick={onSave} disabled={!name.trim()} class="btn btn--primary">Save</button>
    </div>
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 200; display: flex; align-items: center; justify-content: center; padding: 24px; }
  .overlay-backdrop { position: fixed; inset: 0; border: none; padding: 0; margin: 0; background: rgba(0,0,0,0.65); cursor: default; }
  .modal { position: relative; background: #1c1c1e; border: 1px solid rgba(255,255,255,0.1); border-radius: 18px; padding: 22px; max-width: 320px; width: 100%; }
  .title { font-size: 13px; font-weight: 600; color: var(--text-primary); margin: 0 0 4px; }
  .sub { font-size: 11px; color: var(--text-tertiary); margin: 0 0 14px; }
  .input { width: 100%; padding: 9px 11px; border-radius: 9px; border: 1px solid rgba(255,255,255,0.12); background: rgba(255,255,255,0.05); color: var(--text-primary); font-size: 13px; font-family: var(--font-sf); outline: none; }
  .actions { display: flex; gap: 8px; margin-top: 14px; }
  .btn { flex: 1; padding: 10px; border-radius: 10px; font-size: 13px; font-weight: 500; cursor: pointer; }
  .btn--ghost { border: 1px solid rgba(255,255,255,0.1); background: transparent; color: var(--text-secondary); }
  .btn--ghost:hover { background: rgba(255,255,255,0.05); }
  .btn--primary { border: none; background: var(--accent); color: white; font-weight: 600; }
  .btn--primary:hover { background: var(--accent-hover); }
  .btn--primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>

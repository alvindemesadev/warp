<script lang="ts">
  let {
    version = "",
    currentVersion = "",
    body = "",
    state = "idle",
    progress = 0,
    onDismiss,
    onInstall
  }: {
    version: string;
    currentVersion: string;
    body: string;
    state: string;
    progress: number;
    onDismiss: () => void;
    onInstall: () => void;
  } = $props();
</script>

<div class="overlay" role="dialog" aria-modal="true" aria-label="Update available" tabindex="-1">
  <button type="button" aria-label="Dismiss" onclick={() => { if (state !== "downloading" && state !== "installing") onDismiss(); }} class="overlay-backdrop"></button>
  <div class="modal">
    <div class="head">
      <div class="icon">
        <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
          <path d="M10 3v10m0 0l-4-4m4 4l4-4" stroke="#0a84ff" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M3.5 14.5v1a2 2 0 002 2h9a2 2 0 002-2v-1" stroke="#0a84ff" stroke-width="1.7" stroke-linecap="round"/>
        </svg>
      </div>
      <div>
        <p class="title">{state === "downloading" ? "Downloading update…" : state === "installing" ? "Installing update…" : "Update available"}</p>
        <p class="sub">Warp v{currentVersion} → <strong class="accent">v{version}</strong></p>
      </div>
    </div>

    {#if body && state !== "downloading" && state !== "installing"}
      <div class="notes"><p class="notes-text">{body}</p></div>
    {/if}

    {#if state === "downloading"}
      <div class="dl">
        <div class="bar"><div class="fill" style:width="{progress > 0 ? progress : 100}%" style:animation={progress === 0 ? 'update-indeterminate 1.2s ease-in-out infinite' : ''}></div></div>
        <p class="dl-label">{progress > 0 ? `${progress}%` : "Downloading installer…"}</p>
      </div>
    {:else if state === "installing"}
      <div class="installed">
        <div class="spinner"></div>
        <p class="installed-text">Installed — Warp will restart to finish the update.</p>
      </div>
    {/if}

    {#if state !== "downloading" && state !== "installing"}
      <p class="hint">Download the latest version and install it inside Warp. Your transfers and settings are kept.</p>
      <div class="actions">
        <button onclick={onDismiss} class="btn btn--ghost">Later</button>
        <button onclick={onInstall} class="btn btn--primary">Install Update</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .overlay { position: fixed; inset: 0; z-index: 200; display: flex; align-items: center; justify-content: center; padding: 24px; }
  .overlay-backdrop { position: fixed; inset: 0; border: none; padding: 0; margin: 0; background: rgba(0,0,0,0.65); cursor: default; }
  .modal { position: relative; background: #1c1c1e; border: 1px solid rgba(255,255,255,0.1); border-radius: 18px; padding: 24px; max-width: 340px; width: 100%; }
  .head { display: flex; align-items: center; gap: 12px; margin-bottom: 14px; }
  .icon { width: 38px; height: 38px; border-radius: 10px; background: rgba(10,132,255,0.15); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
  .title { font-size: 14px; font-weight: 600; color: var(--text-primary); margin: 0; }
  .sub { font-size: 11px; color: var(--text-tertiary); margin: 3px 0 0; }
  .accent { color: var(--accent); }
  .notes { max-height: 140px; overflow-y: auto; background: rgba(255,255,255,0.04); border: 1px solid rgba(255,255,255,0.07); border-radius: 10px; padding: 10px 12px; margin-bottom: 14px; }
  .notes-text { font-size: 11px; color: var(--text-secondary); line-height: 1.55; margin: 0; white-space: pre-wrap; word-break: break-word; }
  .dl { margin-bottom: 14px; }
  .bar { height: 4px; background: rgba(255,255,255,0.08); border-radius: 100px; overflow: hidden; }
  .fill { height: 100%; background: linear-gradient(90deg,var(--accent),#5e5ce6); border-radius: 100px; transition: width 0.3s; }
  .dl-label { font-size: 10px; color: var(--text-tertiary); margin: 7px 0 0; text-align: center; }
  .installed { display: flex; align-items: center; gap: 9px; background: rgba(48,209,88,0.08); border: 1px solid rgba(48,209,88,0.2); border-radius: 10px; padding: 10px 12px; margin-bottom: 14px; }
  .spinner { width: 12px; height: 12px; border-radius: 50%; flex-shrink: 0; border: 2px solid rgba(48,209,88,0.25); border-top-color: var(--green); animation: spin-smooth 1.2s linear infinite; }
  .installed-text { font-size: 11px; color: var(--green); margin: 0; }
  .hint { font-size: 11px; color: var(--text-secondary); line-height: 1.5; margin: 0 0 18px; }
  .actions { display: flex; gap: 8px; }
  .btn { flex: 1; padding: 10px; border-radius: 10px; font-size: 13px; font-weight: 500; cursor: pointer; }
  .btn--ghost { border: 1px solid rgba(255,255,255,0.1); background: transparent; color: var(--text-secondary); }
  .btn--ghost:hover { background: rgba(255,255,255,0.05); }
  .btn--primary { border: none; background: var(--accent); color: white; font-weight: 600; }
  .btn--primary:hover { background: var(--accent-hover); }
</style>

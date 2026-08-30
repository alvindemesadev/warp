<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { basename } from "$lib/format";
  import { transfer } from "$lib/stores/transfer.svelte";
  import { updater } from "$lib/stores/updater.svelte";
  import { ui } from "$lib/stores/ui.svelte";
  import { comparePaths } from "$lib/services/warp";
  import {
    listenWarpProgress,
    listenWarpError,
    listenWarpVerifying,
    listenHealthWarning,
  } from "$lib/services/warp";

  import Background from "$lib/components/Background.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import TrafficLights from "$lib/components/TrafficLights.svelte";
  import SyncWarningModal from "$lib/components/SyncWarningModal.svelte";
  import DropConflictModal from "$lib/components/DropConflictModal.svelte";
  import UpdateModal from "$lib/components/UpdateModal.svelte";
  import RecentPanel from "$lib/components/RecentPanel.svelte";
  import PathCard from "$lib/components/PathCard.svelte";
  import ModePicker from "$lib/components/ModePicker.svelte";
  import OptionsPanel from "$lib/components/OptionsPanel.svelte";
  import ProgressCard from "$lib/components/ProgressCard.svelte";
  import ResultCards from "$lib/components/ResultCards.svelte";
  import CompareModal from "$lib/components/CompareModal.svelte";

  const win = getCurrentWindow();

  onMount(() => {
    getVersion()
      .then((v) => updater.initUpdater(v))
      .catch(() => {});
    transfer.initFromStorage();

    const unlisten: Array<() => void> = [];
    (async () => {
      unlisten.push(await listenWarpProgress((p) => transfer.handleProgress(p)));
      unlisten.push(await listenWarpError((m) => transfer.handleWarpError(m)));
      unlisten.push(await listenWarpVerifying(() => transfer.handleWarpVerifying()));
      unlisten.push(await listenHealthWarning((slow) => transfer.handleHealthWarning(slow)));
      unlisten.push(
        await win.onDragDropEvent((e) => {
          const t = e.payload.type;
          if (t === "over") {
            ui.isDragging = true;
            const pos = (e.payload as unknown as { position?: { x: number; y: number } }).position;
            if (pos) {
              const scale = window.devicePixelRatio || 1;
              const x = pos.x / scale;
              const y = pos.y / scale;
              const srcEl = document.querySelector('[data-drop="source"]') as HTMLElement | null;
              const destEl = document.querySelector('[data-drop="dest"]') as HTMLElement | null;
              if (srcEl) {
                const r = srcEl.getBoundingClientRect();
                if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) {
                  ui.dragTarget = "source";
                  return;
                }
              }
              if (destEl) {
                const r = destEl.getBoundingClientRect();
                if (x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) {
                  ui.dragTarget = "dest";
                  return;
                }
              }
              ui.dragTarget = null;
            }
          } else if (t === "drop") {
            const paths = ((e.payload as { paths?: string[] }).paths as string[]) ?? [];
            const target = ui.dragTarget;
            ui.dragTarget = null;
            ui.isDragging = false;
            if (paths.length > 0) {
              const p = paths[0];
              if (!p) return;
              if (target === "source") transfer.setSource(p);
              else if (target === "dest") transfer.setDest(p);
              else if (!transfer.sourcePath) transfer.setSource(p);
              else if (!transfer.destPath) transfer.setDest(p);
              else {
                ui.dropConflict = true;
                ui._pendingDrop = p;
              }
            }
          } else {
            ui.dragTarget = null;
            ui.isDragging = false;
          }
        }),
      );
    })();

    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        if (transfer.isProcessing) transfer.cancelTransfer();
        else if (transfer.lastSummary) reset();
        else if (ui.showSyncWarning) ui.showSyncWarning = false;
        else if (ui.dropConflict) ui.dropConflict = false;
      }
      if ((e.metaKey || e.ctrlKey) && e.key === "o") {
        e.preventDefault();
        void transfer.browseSource();
      }
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "o") {
        e.preventDefault();
        void transfer.browseDest();
      }
      if (e.key === "Enter" && transfer.canStart && !ui.showSyncWarning) {
        e.preventDefault();
        handleStart();
      }
    }
    window.addEventListener("keydown", onKey);
    const _autoCheck = setTimeout(() => updater.checkForUpdates(true), 4000);
    return () => {
      unlisten.forEach((fn) => fn());
      window.removeEventListener("keydown", onKey);
      clearTimeout(_autoCheck);
    };
  });

  function handleStart() {
    if (transfer.mode === "sync" && !ui.showSyncWarning) {
      ui.showSyncWarning = true;
      return;
    }
    ui.showSyncWarning = false;
    transfer.startWarp();
  }

  function reset() {
    transfer.resetTransferOnly();
    ui.resetUi();
  }

  let showCompare = $state(false);
  let compareResult = $state<{
    filesToCopy: number;
    bytesToCopy: number;
    skipped: number;
    extra: number;
  } | null>(null);
  async function doCompare() {
    if (!transfer.sourcePath || !transfer.destPath) return;
    try {
      const r = await comparePaths({
        source: transfer.sourcePath,
        destination: transfer.destPath,
        mode: transfer.mode,
        filter: transfer.filter || null,
      });
      compareResult = r;
      showCompare = true;
    } catch {
      ui.showSyncWarning = false;
    }
  }
</script>

<Background />

{#if ui.showSyncWarning}
  <SyncWarningModal
    destPath={transfer.destPath}
    onCancel={() => (ui.showSyncWarning = false)}
    onConfirm={() => {
      ui.showSyncWarning = false;
      transfer.startWarp();
    }}
  />
{/if}
{#if ui.dropConflict}
  <DropConflictModal
    pendingPath={ui._pendingDrop}
    onCancel={() => {
      ui.dropConflict = false;
      ui._pendingDrop = "";
    }}
    onPick={(slot) =>
      ui.applyDropToPending(
        slot,
        (p) => transfer.setSource(p),
        (p) => transfer.setDest(p),
      )}
  />
{/if}
{#if ui.showRecent}
  <RecentPanel
    entries={transfer.recentTransfers}
    onLoad={(r) => {
      transfer.loadRecent(r);
      ui.showRecent = false;
    }}
    onClear={() => {
      transfer.recentTransfers = [];
      localStorage.removeItem("warp-recent");
      ui.showRecent = false;
    }}
    onClose={() => (ui.showRecent = false)}
  />
{/if}
{#if updater.showUpdateModal && updater.updateInfo}
  <UpdateModal
    version={updater.updateInfo.version}
    currentVersion={updater.APP_VERSION}
    body={updater.updateInfo.body ?? ""}
    phase={updater.updateState}
    progress={updater.updateProgress}
    onDismiss={() => (updater.showUpdateModal = false)}
    onInstall={updater.installUpdate}
  />
{/if}
{#if showCompare && compareResult}
  <CompareModal
    filesToCopy={compareResult.filesToCopy}
    bytesToCopy={compareResult.bytesToCopy}
    skipped={compareResult.skipped}
    extra={compareResult.extra}
    onClose={() => (showCompare = false)}
  />
{/if}
<Toast message={updater.toast} />
<TrafficLights
  recentCount={transfer.recentTransfers.length}
  isProcessing={transfer.isProcessing}
  lastSummary={transfer.lastSummary}
  updateState={updater.updateState}
  updateVersion={updater.updateInfo?.version ?? ""}
  onRecentToggle={() => (ui.showRecent = !ui.showRecent)}
  onUpdateOpen={() => (updater.showUpdateModal = true)}
/>

<main class="page">
  <div class="shell">
    <div class="header">
      <h1 class="header-title">Warp</h1>
      <p class="header-sub">High-speed file transfer</p>
    </div>

    {#if transfer.isProcessing}
      <ProgressCard
        progress={transfer.progress}
        currentFile={transfer.currentFile}
        speed={transfer.speed}
        filesDone={transfer.filesDone}
        filesTotal={transfer.filesTotal}
        etaSeconds={transfer.etaSeconds}
        isIndeterminate={transfer.isIndeterminate}
        isQueueRunning={false}
        queueIndex={0}
        queueTotal={0}
        sourcePath={transfer.sourcePath}
        destPath={transfer.destPath}
        transferredFiles={transfer.transferredFiles}
        onCancel={() => transfer.cancelTransfer()}
        activeWorkers={transfer.liveWorkers}
        shardsDone={transfer.shardsDone}
        shardsTotal={transfer.shardsTotal}
        paused={transfer.paused}
        onTogglePause={() => transfer.togglePause()}
      />
    {:else if !transfer.lastSummary}
      <PathCard
        sourcePath={transfer.sourcePath}
        destPath={transfer.destPath}
        sourceInfo={transfer.sourceInfo}
        destInfo={transfer.destInfo}
        isScanning={transfer.isScanning}
        isScanningDest={transfer.isScanningDest}
        bind:dragTarget={ui.dragTarget}
        isDragging={ui.isDragging}
        onBrowseSource={() => transfer.browseSource()}
        onBrowseDest={() => transfer.browseDest()}
        onClearSource={() => {
          transfer.sourcePath = "";
          transfer.sourceInfo = null;
        }}
        onClearDest={() => {
          transfer.destPath = "";
          transfer.destInfo = null;
        }}
        onSwap={() => transfer.swapPaths()}
      />

      <ModePicker bind:mode={transfer.mode} />

      <OptionsPanel
        bind:folderMode={transfer.folderMode}
        bind:conflict={transfer.conflict}
        bind:throttle={transfer.throttle}
        bind:verify={transfer.verify}
        bind:customSpeed={transfer.customSpeed}
        bind:customSpeedValue={transfer.customSpeedValue}
        bind:workers={transfer.workers}
        bind:filter={transfer.filter}
        mode={transfer.mode}
        destPath={transfer.destPath}
        sourcePath={transfer.sourcePath}
        sourceInfo={transfer.sourceInfo}
        destInfo={transfer.destInfo}
      />

      {#if transfer.overlappingPath}
        <div class="warn warn--red">
          <p class="warn-text">
            ! <strong>Invalid paths:</strong>
            {transfer.overlappingPath}. Choose a different destination.
          </p>
        </div>
      {/if}
      {#if transfer.crossDriveMove}
        <div class="warn warn--orange">
          <p class="warn-text">
            ! <strong>Cross-drive move:</strong> Robocopy will copy files to {transfer.sourceInfo
              ?.drive ?? "dest"} then delete from {transfer.destInfo?.drive ?? "source"}. If
            cancelled mid-transfer, source files may be partially deleted. Consider using
            <strong>Copy</strong> instead.
          </p>
        </div>
      {/if}
      {#if transfer.slowDrive}
        <div class="warn warn--orange">
          <p class="warn-text">! <strong>Slow drive:</strong> Using 2 lanes for best speed</p>
        </div>
      {/if}

      <div class="engage-row">
        <button
          onclick={doCompare}
          disabled={!transfer.canStart}
          class="engage engage--secondary"
          class:engage--disabled={!transfer.canStart}
          title="Dry-run: show what would copy without copying">Compare</button
        >
        <button
          onclick={handleStart}
          disabled={!transfer.canStart}
          class="engage"
          class:engage--accent={transfer.canStart}
          class:engage--disabled={!transfer.canStart}>{transfer.startLabel}</button
        >
      </div>

      <p class="hint">
        {#if transfer.sourcePath && transfer.destPath && transfer.sourceInfo && !transfer.sourceInfo.isFile && !transfer.destInfo?.isFile}
          {@const effectiveDest =
            transfer.folderMode === "into" &&
            basename(transfer.destPath).toLowerCase() !==
              basename(transfer.sourcePath).toLowerCase()
              ? transfer.destPath.replace(/\\+$/, "") + "\\" + basename(transfer.sourcePath)
              : transfer.destPath}
          {basename(transfer.sourcePath)} -> <span class="mono">{effectiveDest}</span>
        {:else if transfer.sourcePath && !transfer.destPath}
          Now drop or browse a destination folder
        {/if}
      </p>
    {:else}
      <ResultCards
        summary={transfer.lastSummary}
        mode={transfer.mode}
        sourcePath={transfer.sourcePath}
        destPath={transfer.destPath}
        errorLogs={transfer.errorLogs}
        onReset={reset}
      />
    {/if}

    <button
      onclick={() => updater.checkForUpdates()}
      disabled={updater.updateState === "checking"}
      title="Check for updates"
      class="check-updates"
    >
      {updater.updateState === "checking" ? "Checking..." : "Check for updates"}
    </button>
  </div>
</main>

<style>
  .page {
    min-height: 100vh;
    max-height: 100vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: safe center;
    padding: 52px 20px 12px;
    font-family: var(--font-sf);
    cursor: default;
  }
  .shell {
    width: 100%;
    max-width: 500px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    flex-shrink: 0;
  }
  .header {
    text-align: center;
    margin-bottom: 20px;
  }
  .header-title {
    font-size: 40px;
    font-weight: 700;
    letter-spacing: -0.04em;
    color: var(--text-primary);
    margin: 0;
    line-height: 1;
    cursor: default;
    user-select: none;
  }
  .header-sub {
    margin: 6px 0 0;
    font-size: 14px;
    font-weight: 500;
    color: var(--text-tertiary);
    letter-spacing: 0.01em;
  }
  .warn {
    border-radius: 12px;
    padding: 12px 16px;
    border-width: 1.5px;
  }
  .warn--orange {
    background: rgba(255, 159, 10, 0.12);
    border: 1.5px solid rgba(255, 159, 10, 0.3);
  }
  .warn--red {
    background: rgba(255, 69, 58, 0.12);
    border: 1.5px solid rgba(255, 69, 58, 0.35);
  }
  .warn-text {
    font-size: 12px;
    font-weight: 600;
    margin: 0;
    line-height: 1.5;
  }
  .warn--orange .warn-text {
    color: var(--orange);
  }
  .warn--red .warn-text {
    color: var(--red);
  }
  .engage {
    width: 100%;
    padding: 12px;
    border-radius: 14px;
    border: none;
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.01em;
    transition: all 0.15s;
    outline: none;
  }
  .engage--accent {
    background: var(--accent);
    color: #fff;
    box-shadow: 0 2px 20px rgba(10, 132, 255, 0.28);
    cursor: pointer;
  }
  .engage--accent:hover {
    background: var(--accent-hover);
  }
  .engage--secondary {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text-secondary);
    border: 1px solid var(--glass-border);
    cursor: pointer;
  }
  .engage--secondary:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text-primary);
  }
  .engage--disabled {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text-tertiary);
    cursor: not-allowed;
    box-shadow: none;
  }
  .engage-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-top: 14px;
  }
  .hint {
    text-align: center;
    font-size: 11px;
    color: var(--text-tertiary);
    margin: -5px 0 0;
  }
  .mono {
    font-family: monospace;
    font-size: 10px;
  }
  .check-updates {
    display: block;
    margin: 8px auto 0;
    background: none;
    border: none;
    padding: 4px 12px;
    font-size: 11px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.32);
    cursor: pointer;
    font-family: var(--font-sf);
    letter-spacing: 0.02em;
    transition: color 0.15s;
  }
  .check-updates:hover {
    color: var(--accent);
  }
  .check-updates:disabled {
    cursor: default;
  }
  :global(*) {
    box-sizing: border-box;
  }
  :global(body) {
    margin: 0;
    overflow: auto;
    background: transparent;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
  :global(::-webkit-scrollbar) {
    width: 4px;
  }
  :global(::-webkit-scrollbar-track) {
    background: transparent;
  }
  :global(::-webkit-scrollbar-thumb) {
    background: rgba(255, 255, 255, 0.12);
    border-radius: 4px;
  }
  :global(button:focus-visible) {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  @keyframes update-pulse {
    0%,
    100% {
      box-shadow: 0 0 0 0 rgba(10, 132, 255, 0.35);
    }
    50% {
      box-shadow: 0 0 0 4px rgba(10, 132, 255, 0);
    }
  }
  @keyframes update-indeterminate {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(300%);
    }
  }
</style>

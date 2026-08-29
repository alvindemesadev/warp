<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { basename, fmtBytes, fmtDuration, fmtEta, fmtFiles, timeAgo } from "$lib/format";
  import { loadPresets, loadRecent as loadRecentEntries, savePresets as persistPresets, saveRecentEntries as persistRecent, loadQueue, saveQueue } from "$lib/storage";
  import { normalizeThrottleInput } from "$lib/transfer";
  import type { Mode, Conflict, FolderMode, PathInfo, WarpProgress, WarpSummary, QueueJob, Preset, RecentEntry } from "$lib/types";

  import Background from "$lib/components/Background.svelte";
  import Toast from "$lib/components/Toast.svelte";
  import TrafficLights from "$lib/components/TrafficLights.svelte";
  import SyncWarningModal from "$lib/components/SyncWarningModal.svelte";
  import DropConflictModal from "$lib/components/DropConflictModal.svelte";
  import PresetNameModal from "$lib/components/PresetNameModal.svelte";
  import UpdateModal from "$lib/components/UpdateModal.svelte";
  import RecentPanel from "$lib/components/RecentPanel.svelte";
  import PresetsPanel from "$lib/components/PresetsPanel.svelte";
  import PathCard from "$lib/components/PathCard.svelte";
  import ModePicker from "$lib/components/ModePicker.svelte";
  import OptionsPanel from "$lib/components/OptionsPanel.svelte";
  import ProgressCard from "$lib/components/ProgressCard.svelte";
  import QueueList from "$lib/components/QueueList.svelte";
  import ResultCards from "$lib/components/ResultCards.svelte";
  import QueueSummary from "$lib/components/QueueSummary.svelte";

  // ── State ──────────────────────────────────────────────────────────────────
  let sourcePath = $state("");
  let destPath   = $state("");
  let sourceInfo = $state<PathInfo | null>(null);
  let destInfo   = $state<PathInfo | null>(null);
  let mode       = $state<Mode>("copy");
  let conflict   = $state<Conflict>("overwrite");
  let folderMode = $state<FolderMode>("into");
  let throttle   = $state(0);
  let verify     = $state(false);
  let workers    = $state(0); // parallel workers: 0 = Auto, 2..=8 explicit

  let progress    = $state(0);
  let currentFile = $state("");
  let speed       = $state("");
  let filesDone   = $state(0);
  let filesTotal  = $state(0);
  let etaSeconds  = $state(0);
  let transferredFiles = $state<string[]>([]);
  let liveWorkers = $state(0);
  let shardsDone  = $state(0);
  let shardsTotal = $state(0);
  let paused      = $state(false);

  let isProcessing = $state(false);
  let isScanning   = $state(false);
  let isScanningDest = $state(false);
  let isVerifying  = $state(false);
  let dragTarget   = $state<"source" | "dest" | null>(null);
  let isDragging   = $state(false);
  let dropConflict = $state(false);
  let isIndeterminate = $state(false);

  let showSyncWarning  = $state(false);
  let lastSummary = $state<WarpSummary | null>(null);
  let errorLogs   = $state<string[]>([]);
  let recentTransfers = $state<RecentEntry[]>([]);
  let showRecent = $state(false);
  let queue = $state<QueueJob[]>([]);
  let isQueueRunning = $state(false);
  let queueIndex = $state(0);
  let queueTotal = $state(0);
  let queueResults = $state<WarpSummary[]>([]);
  let showQueueSummary = $state(false);
  let queueCancelled = $state(false);
  let _jobId = 0;
  let _runId = 0; // generation counter — stale transfer results must not clobber newer UI state
  let presets = $state<Preset[]>([]);
  let showPresets = $state(false);
  let customSpeed = $state(false);
  let customSpeedValue = $state(50);

  function syncSpeedMode(t: number) {
    const isPreset = [0,100,25,5].includes(t);
    customSpeed = t > 0 && !isPreset;
    if (customSpeed) customSpeedValue = t;
  }

  const win = getCurrentWindow();

  // ── Lifecycle ──────────────────────────────────────────────────────────────
  onMount(() => {
    getVersion().then((v) => (APP_VERSION = v)).catch(() => {});
    recentTransfers = loadRecentEntries();
    presets = loadPresets();
    queue = loadQueue();
    _jobId = queue.reduce((m, j) => Math.max(m, j.id), 0);
    const unlisten: Array<() => void> = [];
    (async () => {
      unlisten.push(await listen<WarpProgress>("warp-progress", ({ payload }) => {
        progress    = payload.percentage;
        currentFile = basename(payload.currentFile);
        if (payload.speed) speed = payload.speed;
        filesDone  = payload.filesDone;
        filesTotal = payload.filesTotal;
        isIndeterminate = payload.indeterminate;
        liveWorkers = payload.activeWorkers ?? 0;
        shardsDone  = payload.shardsDone ?? 0;
        shardsTotal = payload.shardsTotal ?? 0;
        if (payload.currentFile) {
          transferredFiles = [basename(payload.currentFile), ...transferredFiles].slice(0, 200);
        }
        if (!payload.indeterminate && payload.bytesPerSec > 0 && payload.totalBytes > 0) {
          const remaining = Math.max(0, payload.totalBytes - payload.bytesDone);
          etaSeconds = Math.round(remaining / payload.bytesPerSec);
        } else {
          etaSeconds = 0;
        }
      }));
      unlisten.push(await listen<string>("warp-error", ({ payload }) => {
        errorLogs = [...errorLogs, payload];
      }));
      unlisten.push(await listen("warp-verifying", () => {
        isVerifying = true;
        currentFile = "Verifying…";
      }));
      unlisten.push(await win.onDragDropEvent((e) => {
        const t = e.payload.type;
        if (t === "over") {
          isDragging = true;
          if (!sourcePath) dragTarget = "source";
          else if (!destPath) dragTarget = "dest";
          else dragTarget = null;
        } else if (t === "drop") {
          const paths = (e.payload as any).paths as string[] ?? [];
          dragTarget = null;
          isDragging = false;
          if (paths.length > 0) {
            const p = paths[0];
            if (!sourcePath) setSource(p);
            else if (!destPath) setDest(p);
            else { dropConflict = true; _pendingDrop = p; }
          }
        } else {
          dragTarget = null;
          isDragging = false;
        }
      }));
    })();

    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        if (isProcessing) cancelTransfer();
        else if (lastSummary) reset();
        else if (showSyncWarning) showSyncWarning = false;
        else if (dropConflict) dropConflict = false;
        else if (showPresetModal) showPresetModal = false;
      }
      if ((e.metaKey || e.ctrlKey) && e.key === "o") { e.preventDefault(); browseSource(); }
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "o") { e.preventDefault(); browseDest(); }
      if (e.key === "Enter" && canStart && !showSyncWarning) { e.preventDefault(); handleStart(); }
    }
    window.addEventListener("keydown", onKey);
    const _autoCheck = setTimeout(() => checkForUpdates(true), 4000);
    return () => {
      unlisten.forEach(fn => fn());
      window.removeEventListener("keydown", onKey);
      clearTimeout(_autoCheck);
      clearTimeout(_toastTimer);
    };
  });

  let _pendingDrop = $state("");
  function applyDropToPending(slot: "source" | "dest") {
    if (slot === "source") setSource(_pendingDrop);
    else setDest(_pendingDrop);
    dropConflict = false;
    _pendingDrop = "";
  }

  async function setSource(p: string) {
    sourcePath = p;
    sourceInfo = null;
    if (!p) return;
    isScanning = true;
    try { const info = await invoke<PathInfo>("get_path_info", { path: p }); sourceInfo = info; } catch { sourceInfo = null; }
    isScanning = false;
  }
  async function setDest(p: string) {
    destPath = p;
    destInfo = null;
    if (!p) return;
    isScanningDest = true;
    try { const info = await invoke<PathInfo>("get_path_info", { path: p }); destInfo = info; } catch { destInfo = null; }
    isScanningDest = false;
  }
  function swapPaths() {
    const tmpPath = sourcePath; const tmpInfo = sourceInfo;
    sourcePath = destPath; sourceInfo = destInfo;
    destPath = tmpPath; destInfo = tmpInfo;
  }
  async function browseSource() {
    const selected = await openDialog({ directory: true, multiple: false, title: "Select Source Folder" });
    if (selected && typeof selected === "string") setSource(selected);
  }
  async function browseDest() {
    const selected = await openDialog({ directory: true, multiple: false, title: "Select Destination Folder" });
    if (selected && typeof selected === "string") setDest(selected);
  }

  function handleStart() {
    if (mode === "sync" && !showSyncWarning) { showSyncWarning = true; return; }
    showSyncWarning = false;
    startWarp();
  }
  async function startWarp() {
    if (!sourcePath || !destPath || isProcessing) return;
    const id = ++_runId;
    isProcessing = true; progress = 0; speed = ""; filesDone = 0; filesTotal = 0; etaSeconds = 0; transferredFiles = []; isVerifying = false; currentFile = "Scanning…"; lastSummary = null; errorLogs = [];
    paused = false; liveWorkers = 0; shardsDone = 0; shardsTotal = 0;
    try {
      const s = await invoke<WarpSummary>("warp_file_op", { source: sourcePath, destination: destPath, mode, conflict, folderMode, throttle, verify, workers });
      // A cancelled job resolves after the user may have started something new
      // (or reset) — only apply results that belong to the current run.
      if (id !== _runId) return;
      lastSummary = s;
      if (!s.cancelled) {
        progress = 100; currentFile = ""; isIndeterminate = false;
        saveRecent({ source: sourcePath, dest: destPath, mode, transferred: s.transferred, bytes: s.bytesTransferred, duration_ms: s.durationMs, timestamp: Date.now() });
        notifyDone(s);
      } else { progress = 0; isIndeterminate = false; }
    } catch (err) {
      if (id !== _runId) return;
      lastSummary = { totalFiles: 0, transferred: 0, skipped: 0, failed: 0, durationMs: 0, bytesTransferred: 0, cancelled: false, errorCode: -1, errorMessage: `Could not start the transfer: ${String(err)}`, verified: false, verifyMismatches: 0 };
      isIndeterminate = false;
    } finally {
      if (id === _runId) { isProcessing = false; isVerifying = false; }
    }
  }

  async function cancelTransfer() {
    // Show feedback immediately but leave isProcessing alone — whichever
    // warp_file_op call is pending finalizes UI state when the killed
    // robocopy returns its summary. Flipping flags here would let a new
    // transfer start while the old invoke is still resolving.
    currentFile = "Cancelling…";
    paused = false;
    try { await invoke("cancel_warp"); } catch {}
  }

  async function togglePause() {
    const next = !paused;
    try {
      await invoke("pause_warp", { paused: next });
      paused = next;
    } catch {}
  }
  function reset() {
    sourcePath = destPath = ""; sourceInfo = destInfo = null;
    progress = 0; speed = ""; currentFile = ""; filesDone = filesTotal = 0; etaSeconds = 0; transferredFiles = [];
    isProcessing = false; isScanning = false; isScanningDest = false; isVerifying = false; isIndeterminate = false;
    lastSummary = null; errorLogs = []; dragTarget = null; isDragging = false; dropConflict = false;
    showSyncWarning = false; showQueueSummary = false; queueResults = []; queueCancelled = false;
    paused = false; liveWorkers = 0; shardsDone = 0; shardsTotal = 0;
  }
  function currentJobConfig(): Omit<QueueJob, "id"> {
    return { source: sourcePath, dest: destPath, mode, conflict, folderMode, throttle, verify, workers };
  }
  function addToQueue() {
    if (!canStart) return;
    queue = [...queue, { id: ++_jobId, ...currentJobConfig() }];
    saveQueue(queue);
    sourcePath = destPath = ""; sourceInfo = destInfo = null;
  }
  function removeFromQueue(id: number) { queue = queue.filter((j) => j.id !== id); saveQueue(queue); }
  function clearQueue() { queue = []; saveQueue(queue); }
  async function runQueue() {
    if (isProcessing || isQueueRunning) return;
    const jobs: QueueJob[] = [...queue];
    if (canStart) jobs.push({ id: ++_jobId, ...currentJobConfig() });
    if (jobs.length === 0) return;
    isQueueRunning = true; queueResults = []; queueTotal = jobs.length; queueCancelled = false; lastSummary = null; showQueueSummary = false;
    for (let i = 0; i < jobs.length; i++) {
      const job = jobs[i]; queueIndex = i + 1;
      sourcePath = job.source; destPath = job.dest; mode = job.mode; conflict = job.conflict; folderMode = job.folderMode; throttle = job.throttle; syncSpeedMode(job.throttle); verify = job.verify; workers = job.workers ?? 0;
      isProcessing = true; progress = 0; speed = ""; filesDone = 0; filesTotal = 0; etaSeconds = 0; transferredFiles = []; isVerifying = false; currentFile = "Scanning…"; errorLogs = [];
      paused = false; liveWorkers = 0; shardsDone = 0; shardsTotal = 0;
      try {
        const s = await invoke<WarpSummary>("warp_file_op", { source: job.source, destination: job.dest, mode: job.mode, conflict: job.conflict, folderMode: job.folderMode, throttle: job.throttle, verify: job.verify, workers: job.workers ?? 0 });
        queueResults = [...queueResults, s];
        if (s.cancelled) { queueCancelled = true; isProcessing = false; break; }
        saveRecent({ source: job.source, dest: job.dest, mode: job.mode, transferred: s.transferred, bytes: s.bytesTransferred, duration_ms: s.durationMs, timestamp: Date.now() });
      } catch (err) {
        queueResults = [...queueResults, { totalFiles: 0, transferred: 0, skipped: 0, failed: 0, durationMs: 0, bytesTransferred: 0, cancelled: false, errorCode: -1, errorMessage: `Could not start the transfer: ${String(err)}`, verified: false, verifyMismatches: 0 }];
      }
      isProcessing = false; isVerifying = false;
    }
    queue = []; saveQueue(queue); queueIndex = 0; isQueueRunning = false; isIndeterminate = false; isVerifying = false; progress = 100; showQueueSummary = true; notifyQueueDone();
  }
  let showPresetModal = $state(false);
  let presetName = $state("");
  function openPresetModal() {
    if (!sourcePath || !destPath) return;
    presetName = `${basename(sourcePath)} → ${basename(destPath)}`;
    showPresetModal = true;
  }
  function confirmSavePreset() {
    const name = presetName.trim(); if (!name) return;
    const entry: Preset = { name, ...currentJobConfig() };
    const next = [...presets.filter((p) => p.name !== name), entry];
    presets = next; persistPresets(next); showPresetModal = false;
  }
  function loadPreset(p: Preset) {
    setSource(p.source); setDest(p.dest); mode = p.mode; conflict = p.conflict; folderMode = p.folderMode; throttle = p.throttle ?? 0; syncSpeedMode(throttle); verify = p.verify ?? false; workers = p.workers ?? 0; showPresets = false;
  }
  function deletePreset(name: string) { const next = presets.filter((p) => p.name !== name); presets = next; persistPresets(next); }
  function saveRecent(entry: RecentEntry) { const updated = [entry, ...recentTransfers].slice(0, 5); recentTransfers = updated; persistRecent(updated); }
  function loadRecent(r: RecentEntry) { setSource(r.source); setDest(r.dest); mode = r.mode; showRecent = false; }
  async function notifyDone(s: WarpSummary) {
    try {
      let granted = await isPermissionGranted(); if (!granted) granted = (await requestPermission()) === "granted";
      if (granted) { const verb = mode === "move" ? "Moved" : mode === "sync" ? "Synced" : "Copied"; sendNotification({ title: "Warp — Transfer Complete", body: `${verb} ${s.transferred.toLocaleString()} files · ${fmtBytes(s.bytesTransferred)} in ${fmtDuration(s.durationMs)}` }); }
    } catch {}
  }
  async function notifyQueueDone() {
    try {
      let granted = await isPermissionGranted(); if (!granted) granted = (await requestPermission()) === "granted";
      if (granted) { const files = queueResults.reduce((n, r) => n + r.transferred, 0); const anyFailed = queueResults.some((r) => r.failed > 0 || r.errorMessage); const title = queueCancelled ? "Warp — Queue Cancelled" : anyFailed ? "Warp — Queue Finished (with errors)" : "Warp — Queue Complete"; sendNotification({ title, body: `${queueResults.length} ${queueResults.length === 1 ? "job" : "jobs"} · ${files.toLocaleString()} files transferred` }); }
    } catch {}
  }
  let updateState = $state<"idle" | "checking" | "available" | "downloading" | "installing">("idle");
  let updateInfo = $state<{ version: string; body: string } | null>(null);
  let _pendingUpdate: Update | null = null;
  let showUpdateModal = $state(false);
  let updateProgress = $state(0);
  let toast = $state("");
  let _toastTimer: ReturnType<typeof setTimeout> | undefined;
  function showToast(msg: string) { toast = msg; clearTimeout(_toastTimer); _toastTimer = setTimeout(() => (toast = ""), 3500); }
  async function checkForUpdates(auto = false) {
    if (updateState === "checking" || updateState === "downloading" || updateState === "installing") return;
    updateState = "checking";
    try {
      const update = await check();
      if (update) { _pendingUpdate = update; updateInfo = { version: update.version, body: update.body ?? "" }; updateState = "available"; showUpdateModal = true; }
      else { updateState = "idle"; if (!auto) showToast(`You're up to date (v${APP_VERSION})`); }
    } catch { updateState = "idle"; if (!auto) showToast("Couldn't check for updates — check your connection"); }
  }
  async function installUpdate() {
    if (!_pendingUpdate || updateState === "downloading" || updateState === "installing") return;
    updateState = "downloading"; updateProgress = 0; let downloaded = 0; let contentLength = 0;
    const dlStart = Date.now();
    const MIN_DL_MS = 1600;
    try {
      await _pendingUpdate.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started": contentLength = event.data.contentLength ?? 0; break;
          case "Progress": downloaded += event.data.chunkLength; if (contentLength > 0) updateProgress = Math.min(100, Math.round((downloaded / contentLength) * 100)); break;
          case "Finished": updateProgress = 100; break;
        }
      });
      const elapsed = Date.now() - dlStart;
      if (elapsed < MIN_DL_MS) await new Promise(r => setTimeout(r, MIN_DL_MS - elapsed));
      updateState = "installing";
      await new Promise(r => setTimeout(r, 900));
    } catch {
      const elapsed = Date.now() - dlStart;
      if (elapsed < MIN_DL_MS) await new Promise(r => setTimeout(r, MIN_DL_MS - elapsed));
      updateState = "idle";
      showToast("Update download failed — check your connection and try again");
    }
  }
  let APP_VERSION = $state("1.2.4");
  const MODES: { id: Mode; label: string; desc: string; warning?: string }[] = [
    { id: "copy", label: "Copy", desc: "Duplicate files to destination" },
    { id: "move", label: "Move", desc: "Transfer and remove from source" },
    { id: "sync", label: "Sync", desc: "Mirror source → destination", warning: "Files only in destination will be DELETED" },
  ];
  function normalizePath(p: string): string {
    return p.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
  }
  const overlappingPath = $derived.by(() => {
    if (!sourcePath || !destPath) return null;
    const a = normalizePath(sourcePath);
    const b = normalizePath(destPath);
    // Also consider effective dest when folderMode is "into" (dest will contain source name)
    const sourceName = basename(sourcePath).toLowerCase();
    const effectiveB = folderMode === "into" && sourceName && !b.endsWith("/" + sourceName) ? `${b}/${sourceName}` : b;
    if (a === b || a === effectiveB) return "Source and destination are the same folder";
    if (effectiveB.startsWith(a + "/")) return "Destination is inside the source — would copy into itself";
    if (a.startsWith(b + "/")) return "Source is inside the destination — may cause recursion";
    return null;
  });
  const crossDriveMove = $derived(mode === "move" && !!sourceInfo?.drive && !!destInfo?.drive && sourceInfo.drive.toLowerCase() !== destInfo.drive.toLowerCase());
  const mergeSyncDanger = $derived(mode === "sync" && folderMode === "merge");
  const canStart = $derived(!!sourcePath && !!destPath && !isProcessing && !sourceInfo?.isFile && !destInfo?.isFile && !overlappingPath);
  const startLabel = $derived(
    overlappingPath ? overlappingPath
    : !sourcePath || !destPath ? "Drop source and destination to begin"
    : sourceInfo?.isFile ? "Source must be a folder, not a file"
    : destInfo?.isFile ? "Destination must be a folder, not a file"
    : `${MODES.find(m => m.id === mode)?.label} Files`
  );
</script>

<Background />

{#if showSyncWarning}
  <SyncWarningModal destPath={destPath} onCancel={() => showSyncWarning = false} onConfirm={() => { showSyncWarning = false; startWarp(); }} />
{/if}
{#if dropConflict}
  <DropConflictModal pendingPath={_pendingDrop} onCancel={() => { dropConflict=false; _pendingDrop=""; }} onPick={applyDropToPending} />
{/if}
{#if showRecent}
  <RecentPanel entries={recentTransfers} onLoad={loadRecent} onClear={() => { recentTransfers=[]; localStorage.removeItem('warp-recent'); showRecent=false; }} onClose={() => showRecent=false} />
{/if}
{#if showPresets}
  <PresetsPanel presets={presets} onLoad={loadPreset} onDelete={deletePreset} onClose={() => showPresets = false} />
{/if}
{#if showPresetModal}
  <PresetNameModal bind:name={presetName} onCancel={() => showPresetModal = false} onSave={confirmSavePreset} />
{/if}
{#if showUpdateModal && updateInfo}
  <UpdateModal version={updateInfo.version} currentVersion={APP_VERSION} body={updateInfo.body ?? ""} phase={updateState} progress={updateProgress} onDismiss={() => showUpdateModal = false} onInstall={installUpdate} />
{/if}
<Toast message={toast} />
<TrafficLights
  recentCount={recentTransfers.length}
  isProcessing={isProcessing}
  lastSummary={lastSummary}
  updateState={updateState}
  updateVersion={updateInfo?.version ?? ""}
  onRecentToggle={() => showRecent = !showRecent}
  onUpdateOpen={() => showUpdateModal = true}
/>

<main class="page">
  <div class="shell">
    <div class="header">
      <h1 class="header-title">Warp</h1>
      <p class="header-sub">High-speed file transfer</p>
    </div>

    {#if showQueueSummary}
      <QueueSummary results={queueResults} cancelled={queueCancelled} onDone={reset} />
    {:else if isProcessing}
      <!-- Focus mode: the card owns the window while transferring. The setup
           form stays mounted below only when idle, so nothing can overlap. -->
      <ProgressCard
        {progress} {currentFile} {speed} {filesDone} {filesTotal} {etaSeconds} {isIndeterminate} {isQueueRunning} {queueIndex} {queueTotal} {sourcePath} {destPath} {transferredFiles}
        onCancel={cancelTransfer}
        activeWorkers={liveWorkers} {shardsDone} {shardsTotal} {paused} onTogglePause={togglePause}
      />
    {:else if !lastSummary}
      <PathCard
        {sourcePath} {destPath} {sourceInfo} {destInfo} {isScanning} {isScanningDest} {dragTarget} {isDragging}
        onBrowseSource={browseSource} onBrowseDest={browseDest}
        onClearSource={() => { sourcePath=""; sourceInfo=null; }}
        onClearDest={() => { destPath=""; destInfo=null; }}
        onSwap={swapPaths}
      />

      <ModePicker bind:mode />

      <OptionsPanel
        bind:folderMode bind:conflict bind:throttle bind:verify bind:customSpeed bind:customSpeedValue bind:workers
        {mode} {destPath} {sourcePath}
      />

      <div class="actions">
        <button onclick={openPresetModal} disabled={!sourcePath || !destPath} title="Save the current source, destination, and options as a reusable preset" class="chip chip--action" class:chip--disabled={!sourcePath || !destPath}>SAVE PRESET</button>
        {#if presets.length > 0}
          <button onclick={() => showPresets = true} class="chip chip--action">PRESETS ({presets.length})</button>
        {/if}
        <button onclick={addToQueue} disabled={!canStart} title="Add this transfer to the queue and clear the form for the next one" class="chip chip--action" class:chip--accent={canStart} class:chip--disabled={!canStart}>ADD TO QUEUE</button>
      </div>

      {#if queue.length > 0}
        <QueueList queue={queue} onRemove={removeFromQueue} onClear={clearQueue} />
      {/if}

      {#if overlappingPath}
        <div class="warn warn--red">
          <p class="warn-text">⚠ <strong>Invalid paths:</strong> {overlappingPath}. Choose a different destination.</p>
        </div>
      {/if}
      {#if crossDriveMove}
        <div class="warn warn--orange">
          <p class="warn-text">⚠ <strong>Cross-drive move:</strong> Robocopy will copy files to {sourceInfo?.drive ?? "dest"} then delete from {destInfo?.drive ?? "source"}. If cancelled mid-transfer, source files may be partially deleted. Consider using <strong>Copy</strong> instead.</p>
        </div>
      {/if}
      {#if mergeSyncDanger}
        <div class="warn warn--red">
          <p class="warn-text">⚠ <strong>Dangerous combination:</strong> Sync + Merge Contents will mirror the source directly into the destination root. Files already in {destPath ? basename(destPath) : "destination"} that are not in the source will be <strong>permanently deleted</strong>.</p>
        </div>
      {/if}

      {#if queue.length > 0}
        <button onclick={runQueue} class="engage engage--accent">Run Queue ({queue.length}{canStart ? ' + current' : ''} {queue.length + (canStart ? 1 : 0) === 1 ? 'job' : 'jobs'})</button>
      {:else}
        <button onclick={handleStart} disabled={!canStart} class="engage" class:engage--accent={canStart} class:engage--disabled={!canStart}>{startLabel}</button>
      {/if}

      <p class="hint">
        {#if sourcePath && destPath && sourceInfo && !sourceInfo.isFile && !destInfo?.isFile}
          {@const effectiveDest = folderMode === 'into' && basename(destPath).toLowerCase() !== basename(sourcePath).toLowerCase() ? destPath.replace(/\\+$/, '') + '\\' + basename(sourcePath) : destPath}
          → <span class="mono">{effectiveDest}</span>
        {:else if sourcePath && !destPath}
          Now drop or browse a destination folder
        {/if}
      </p>
    {:else}
      <ResultCards summary={lastSummary} {mode} {sourcePath} {destPath} {errorLogs} onReset={reset} />
    {/if}

    <p class="kbd-hint">
      {#if isProcessing}Esc to cancel
      {:else if lastSummary}Esc to reset
      {:else}Ctrl+O source · Ctrl+Shift+O destination · Enter to start{/if}
    </p>
    <button onclick={() => checkForUpdates()} disabled={updateState === "checking"} title="Check for updates" class="check-updates"> {updateState === "checking" ? "Checking…" : "Check for updates"} </button>
  </div>
</main>

<style>
  .page { min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 52px 20px 12px; font-family: var(--font-sf); cursor: default; }
  .shell { width: 100%; max-width: 500px; display: flex; flex-direction: column; gap: 14px; }
  .header { text-align: center; margin-bottom: 2px; }
  .chip { height: 26px; padding: 0 9px; border-radius: 7px; border: none; font-size: 9px; font-weight: 600; letter-spacing: 0.04em; font-family: var(--font-sf); cursor: pointer; display: inline-flex; align-items: center; justify-content: center; }
  .chip--muted { color: var(--text-tertiary); background: rgba(255,255,255,0.07); }
  .chip--update { background: rgba(10,132,255,0.22); color: #64b5ff; font-weight: 700; animation: update-pulse 2.2s ease-in-out infinite; }
  .chip--update:hover { background: rgba(10,132,255,0.35); }
  .chip--recent { border: 1px solid var(--glass-border); background: rgba(255,255,255,0.04); color: var(--text-secondary); }
  .chip--recent:hover { background: rgba(255,255,255,0.08); color: var(--text-primary); }
  .icon-btn { width: 26px; height: 26px; border-radius: 7px; border: 1px solid var(--glass-border); background: rgba(255,255,255,0.04); color: var(--text-secondary); display: flex; align-items: center; justify-content: center; cursor: pointer; }
  .icon-btn:hover { background: rgba(255,255,255,0.08); color: var(--text-primary); }
  .header-title { font-size: 40px; font-weight: 700; letter-spacing: -0.04em; color: var(--text-primary); margin: 0; line-height: 1; cursor: default; user-select: none; }
  .header-sub { margin: 5px 0 0; font-size: 12px; font-weight: 500; color: var(--text-tertiary); }
  .actions { display: flex; align-items: center; justify-content: center; gap: 8px; flex-wrap: wrap; }
  .chip--action { height: 26px; padding: 0 10px; border: 1px solid var(--glass-border); background: rgba(255,255,255,0.04); color: var(--text-secondary); letter-spacing: 0.04em; }
  .chip--action:hover:not(:disabled) { background: rgba(255,255,255,0.08); color: var(--text-primary); }
  .chip--accent { border-color: rgba(10,132,255,0.3); background: rgba(10,132,255,0.12); color: #64b5ff; }
  .chip--accent:hover:not(:disabled) { background: rgba(10,132,255,0.22); }
  .chip--disabled { opacity: 0.45; cursor: not-allowed; }
  .warn { border-radius: 10px; padding: 8px 12px; }
  .warn--orange { background: rgba(255,159,10,0.08); border: 1px solid rgba(255,159,10,0.2); }
  .warn--red { background: rgba(255,69,58,0.08); border: 1px solid rgba(255,69,58,0.2); }
  .warn-text { font-size: 10px; margin: 0; line-height: 1.5; }
  .warn--orange .warn-text { color: var(--orange); }
  .warn--red .warn-text { color: var(--red); }
  .engage { width: 100%; padding: 12px; border-radius: 14px; border: none; font-size: 14px; font-weight: 600; letter-spacing: -0.01em; transition: all 0.15s; outline: none; }
  .engage--accent { background: var(--accent); color: #fff; box-shadow: 0 2px 20px rgba(10,132,255,0.28); cursor: pointer; }
  .engage--accent:hover { background: var(--accent-hover); }
  .engage--disabled { background: rgba(255,255,255,0.05); color: var(--text-tertiary); cursor: not-allowed; box-shadow: none; }
  .hint { text-align: center; font-size: 11px; color: var(--text-tertiary); margin: -5px 0 0; }
  .mono { font-family: monospace; font-size: 10px; }
  .kbd-hint { text-align: center; font-size: 11px; color: var(--text-tertiary); margin: -2px 0 0; letter-spacing: 0.02em; }
  .check-updates { display: block; margin: 8px auto 0; background: none; border: none; padding: 4px 12px; font-size: 11px; font-weight: 600; color: rgba(255,255,255,0.32); cursor: pointer; font-family: var(--font-sf); letter-spacing: 0.02em; transition: color 0.15s; }
  .check-updates:hover { color: var(--accent); }
  .check-updates:disabled { cursor: default; }
  :global(*) { box-sizing: border-box; }
  :global(body) { margin: 0; overflow: hidden; background: transparent; -webkit-font-smoothing: antialiased; -moz-osx-font-smoothing: grayscale; }
  :global(::-webkit-scrollbar) { width: 4px; }
  :global(::-webkit-scrollbar-track) { background: transparent; }
  :global(::-webkit-scrollbar-thumb) { background: rgba(255,255,255,0.12); border-radius: 4px; }
  :global(button:focus-visible) { outline: 2px solid var(--accent); outline-offset: 2px; }
  @keyframes update-pulse { 0%, 100% { box-shadow: 0 0 0 0 rgba(10,132,255,0.35); } 50% { box-shadow: 0 0 0 4px rgba(10,132,255,0); } }
  @keyframes update-indeterminate { 0% { transform: translateX(-100%); } 100% { transform: translateX(300%); } }
</style>

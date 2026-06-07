<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { sendNotification, isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";

  // ── Types ──────────────────────────────────────────────────────────────────
  type Mode = "copy" | "move" | "sync";
  type Conflict = "overwrite" | "skip";

  type PathInfo = { files: number; bytes: number; isFile: boolean; drive: string };

  type WarpProgress = {
    percentage: number;
    currentFile: string;
    speed: string;
    filesDone: number;
    filesTotal: number;
    indeterminate: boolean;
  };

  type WarpSummary = {
    totalFiles: number;
    transferred: number;
    skipped: number;
    failed: number;
    durationMs: number;
    bytesTransferred: number;
    cancelled: boolean;
    errorCode: number;
    errorMessage: string;
  };

  type RecentEntry = {
    source: string;
    dest: string;
    mode: Mode;
    transferred: number;
    bytes: number;
    duration_ms: number;
    timestamp: number;
  };

  // ── State ──────────────────────────────────────────────────────────────────
  let sourcePath = $state("");
  let destPath   = $state("");
  let sourceInfo = $state<PathInfo | null>(null);
  let destInfo   = $state<PathInfo | null>(null);
  let mode       = $state<Mode>("copy");
  let conflict   = $state<Conflict>("overwrite");
  let folderMode = $state<"into" | "merge">("into"); // "into" = create subfolder, "merge" = copy contents directly

  let progress    = $state(0);
  let currentFile = $state("");
  let speed       = $state("");
  let filesDone   = $state(0);
  let filesTotal  = $state(0);

  let isProcessing = $state(false);
  let isScanning   = $state(false);
  let dragTarget   = $state<"source" | "dest" | null>(null);
  let dropConflict = $state(false);
  let isIndeterminate = $state(false); // progress bar in pulse mode

  // Modals
  let showSyncWarning  = $state(false);
  let showConflictOpts = $state(false);

  // Results
  let lastSummary = $state<WarpSummary | null>(null);
  let errorLogs   = $state<string[]>([]);

  // #26: recent transfers
  let recentTransfers = $state<RecentEntry[]>([]);
  let showRecent = $state(false);

  const win = getCurrentWindow();

  // ── Lifecycle ──────────────────────────────────────────────────────────────
  onMount(() => {
    // Load recent from localStorage
    try {
      const saved = localStorage.getItem("warp-recent");
      if (saved) recentTransfers = JSON.parse(saved);
    } catch {}

    const unlisten: Array<() => void> = [];

    (async () => {
      unlisten.push(await listen<WarpProgress>("warp-progress", ({ payload }) => {
        progress    = payload.percentage;
        currentFile = basename(payload.currentFile);
        if (payload.speed) speed = payload.speed;
        filesDone  = payload.filesDone;
        filesTotal = payload.filesTotal;
        isIndeterminate = payload.indeterminate;
      }));

      unlisten.push(await listen<string>("warp-error", ({ payload }) => {
        errorLogs = [...errorLogs, payload];
      }));

      unlisten.push(await win.onDragDropEvent((e) => {
        const t = e.payload.type;
        if (t === "over") {
          if (!sourcePath) dragTarget = "source";
          else if (!destPath) dragTarget = "dest";
          else dragTarget = null;
        } else if (t === "drop") {
          const paths = (e.payload as any).paths as string[] ?? [];
          dragTarget = null;
          if (paths.length > 0) {
            const p = paths[0];
            if (!sourcePath) {
              setSource(p);
            } else if (!destPath) {
              setDest(p);
            } else {
              // #11: both paths already set — show conflict prompt
              dropConflict = true;
              _pendingDrop = p;
            }
          }
        } else {
          dragTarget = null;
        }
      }));
    })();

    // #24: Keyboard shortcuts
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        if (isProcessing) { cancelTransfer(); }
        else if (lastSummary) { reset(); }
        else if (showSyncWarning) { showSyncWarning = false; }
        else if (dropConflict) { dropConflict = false; }
      }
      if ((e.metaKey || e.ctrlKey) && e.key === "o") {
        e.preventDefault();
        browseSource();
      }
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === "o") {
        e.preventDefault();
        browseDest();
      }
      if (e.key === "Enter" && canStart && !showSyncWarning) {
        e.preventDefault();
        handleStart();
      }
    }
    window.addEventListener("keydown", onKey);

    return () => {
      unlisten.forEach(fn => fn());
      window.removeEventListener("keydown", onKey);
    };
  });

  // pending drop path when both slots are full
  let _pendingDrop = "";

  function applyDropToPending(slot: "source" | "dest") {
    if (slot === "source") setSource(_pendingDrop);
    else setDest(_pendingDrop);
    dropConflict = false;
    _pendingDrop = "";
  }

  // ── Path helpers ───────────────────────────────────────────────────────────
  async function setSource(p: string) {
    sourcePath = p;
    sourceInfo = null;
    if (!p) return;
    isScanning = true;
    try {
      const info = await invoke<PathInfo>("get_path_info", { path: p });
      sourceInfo = info;
    } catch { sourceInfo = null; }
    isScanning = false;
  }

  async function setDest(p: string) {
    destPath = p;
    destInfo = null;
    if (!p) return;
    try {
      const info = await invoke<PathInfo>("get_path_info", { path: p });
      destInfo = info;
    } catch { destInfo = null; }
  }

  // #10: swap source and destination
  function swapPaths() {
    const tmpPath = sourcePath;
    const tmpInfo = sourceInfo;
    sourcePath = destPath;
    sourceInfo = destInfo;
    destPath = tmpPath;
    destInfo = tmpInfo;
  }

  // #2: folder picker
  async function browseSource() {
    const selected = await openDialog({ directory: true, multiple: false, title: "Select Source Folder" });
    if (selected && typeof selected === "string") setSource(selected);
  }

  async function browseDest() {
    const selected = await openDialog({ directory: true, multiple: false, title: "Select Destination Folder" });
    if (selected && typeof selected === "string") setDest(selected);
  }

  // ── Transfer ───────────────────────────────────────────────────────────────
  function handleStart() {
    // #5: sync warning
    if (mode === "sync" && !showSyncWarning) {
      showSyncWarning = true;
      return;
    }
    showSyncWarning = false;
    startWarp();
  }

  async function startWarp() {
    if (!sourcePath || !destPath || isProcessing) return;
    isProcessing = true;
    progress = 0;
    speed = "";
    filesDone = 0;
    filesTotal = 0;
    currentFile = "Scanning…";
    lastSummary = null;
    errorLogs = [];

    try {
      const s = await invoke<WarpSummary>("warp_file_op", {
        source: sourcePath,
        destination: destPath,
        mode,
        conflict,
        folderMode,
      });
      lastSummary = s;
      if (!s.cancelled) {
        progress = 100;
        currentFile = "";
        isIndeterminate = false;

        // #26: save to recent
        saveRecent({
          source: sourcePath,
          dest: destPath,
          mode,
          transferred: s.transferred,
          bytes: s.bytesTransferred,
          duration_ms: s.durationMs,
          timestamp: Date.now(),
        });

        // #25: system notification
        notifyDone(s);
      } else {
        progress = 0;
        isIndeterminate = false;
      }
    } catch (err) {
      currentFile = String(err);
      isIndeterminate = false;
    } finally {
      isProcessing = false;
    }
  }

  // #1: cancel
  async function cancelTransfer() {
    await invoke("cancel_warp");
    isProcessing = false;
    currentFile = "Cancelled";
    progress = 0;
    isIndeterminate = false;
  }

  function reset() {
    sourcePath = destPath = "";
    sourceInfo = destInfo = null;
    progress = 0; speed = ""; currentFile = "";
    filesDone = filesTotal = 0;
    isProcessing = false; isScanning = false;
    isIndeterminate = false;
    lastSummary = null; errorLogs = [];
    dragTarget = null; dropConflict = false;
    showSyncWarning = false;
  }

  // ── Recent transfers ───────────────────────────────────────────────────────
  function saveRecent(entry: RecentEntry) {
    const updated = [entry, ...recentTransfers].slice(0, 5);
    recentTransfers = updated;
    try { localStorage.setItem("warp-recent", JSON.stringify(updated)); } catch {}
  }

  function loadRecent(r: RecentEntry) {
    setSource(r.source);
    setDest(r.dest);
    mode = r.mode;
    showRecent = false;
  }

  // ── Notifications ──────────────────────────────────────────────────────────
  async function notifyDone(s: WarpSummary) {
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        const permission = await requestPermission();
        granted = permission === "granted";
      }
      if (granted) {
        const verb = mode === "move" ? "Moved" : mode === "sync" ? "Synced" : "Copied";
        sendNotification({
          title: "Warp — Transfer Complete",
          body: `${verb} ${s.transferred.toLocaleString()} files · ${fmtBytes(s.bytesTransferred)} in ${fmtDuration(s.durationMs)}`,
        });
      }
    } catch {}
  }

  // ── Formatters ─────────────────────────────────────────────────────────────
  function basename(p: string) {
    return p.replace(/\\/g, "/").split("/").filter(Boolean).pop() ?? p;
  }

  function fmtBytes(b: number): string {
    if (b >= 1_073_741_824) return `${(b / 1_073_741_824).toFixed(1)} GB`;
    if (b >= 1_048_576)     return `${(b / 1_048_576).toFixed(1)} MB`;
    if (b >= 1024)          return `${(b / 1024).toFixed(0)} KB`;
    return `${b} B`;
  }

  function fmtFiles(n: number): string {
    return n === 1 ? "1 file" : `${n.toLocaleString()} files`;
  }

  // #9: sub-second duration
  function fmtDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const s = ms / 1000;
    if (s < 60) return `${s.toFixed(1)}s`;
    return `${Math.floor(s / 60)}m ${Math.round(s % 60)}s`;
  }

  function timeAgo(ts: number): string {
    const diff = Date.now() - ts;
    if (diff < 60_000) return "just now";
    if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
    if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
    return `${Math.floor(diff / 86_400_000)}d ago`;
  }

  // ── Constants ──────────────────────────────────────────────────────────────
  const APP_VERSION = "1.0.0";

  const MODES: { id: Mode; label: string; desc: string; warning?: string }[] = [
    { id: "copy", label: "Copy", desc: "Duplicate files to destination" },
    { id: "move", label: "Move", desc: "Transfer and remove from source" },
    {
      id: "sync",
      label: "Sync",
      desc: "Mirror source → destination",
      warning: "Files only in destination will be DELETED",
    },
  ];

  // #18: OneDrive / network path detection
  function isSpecialPath(p: string): string | null {
    const lower = p.toLowerCase();
    if (lower.includes("onedrive")) return "OneDrive path — ensure files are downloaded locally";
    if (p.startsWith("\\\\")) return "Network path — speed may be limited";
    return null;
  }

  const sourceWarning = $derived(sourcePath ? isSpecialPath(sourcePath) : null);
  const destWarning   = $derived(destPath   ? isSpecialPath(destPath)   : null);

  // Cross-drive move warning — robocopy copies then deletes; cancel mid-way leaves partial state
  const crossDriveMove = $derived(
    mode === "move" &&
    !!sourceInfo?.drive && !!destInfo?.drive &&
    sourceInfo.drive.toLowerCase() !== destInfo.drive.toLowerCase()
  );

  // Merge + Sync is dangerous — could delete files in root of destination
  const mergeSyncDanger = $derived(mode === "sync" && folderMode === "merge");

  const canStart      = $derived(
    !!sourcePath && !!destPath && !isProcessing &&
    !sourceInfo?.isFile
  );

  const startLabel = $derived(
    !sourcePath || !destPath ? "Drop source and destination to begin"
    : sourceInfo?.isFile    ? "Source must be a folder, not a file"
    : `${MODES.find(m => m.id === mode)?.label} Files`
  );
</script>

<!-- ── Traffic lights ─────────────────────────────────────────────────────── -->
<div
  data-tauri-drag-region
  class="fixed top-0 left-0 right-0 h-9 z-50 flex items-center px-3.5 gap-[6px]"
  style="cursor:default;"
>
  <button
    onclick={() => win.close()} aria-label="Close"
    style="width:12px;height:12px;border-radius:50%;background:var(--red);border:none;padding:0;cursor:pointer;display:flex;align-items:center;justify-content:center;flex-shrink:0;"
    onmouseenter={(e) => (e.currentTarget.querySelector('span') as HTMLElement).style.opacity='1'}
    onmouseleave={(e) => (e.currentTarget.querySelector('span') as HTMLElement).style.opacity='0'}
  ><span style="opacity:0;font-size:7px;font-weight:900;color:rgba(0,0,0,0.6);line-height:1;pointer-events:none;">✕</span></button>

  <button
    onclick={() => win.minimize()} aria-label="Minimize"
    style="width:12px;height:12px;border-radius:50%;background:var(--yellow);border:none;padding:0;cursor:pointer;display:flex;align-items:center;justify-content:center;flex-shrink:0;"
    onmouseenter={(e) => (e.currentTarget.querySelector('span') as HTMLElement).style.opacity='1'}
    onmouseleave={(e) => (e.currentTarget.querySelector('span') as HTMLElement).style.opacity='0'}
  ><span style="opacity:0;font-size:9px;font-weight:900;color:rgba(0,0,0,0.5);line-height:1;margin-top:-1px;pointer-events:none;">−</span></button>

  <!-- #26: recent button -->
  {#if recentTransfers.length > 0 && !isProcessing && !lastSummary}
    <button
      onclick={() => showRecent = !showRecent}
      title="Recent transfers"
      style="margin-left:auto;margin-right:4px;padding:2px 6px;border-radius:5px;border:none;background:rgba(255,255,255,0.07);color:rgba(255,255,255,0.4);font-size:9px;font-weight:600;cursor:pointer;letter-spacing:0.04em;font-family:var(--font-sf);"
      onmouseenter={(e)=>(e.currentTarget as HTMLElement).style.color='rgba(255,255,255,0.7)'}
      onmouseleave={(e)=>(e.currentTarget as HTMLElement).style.color='rgba(255,255,255,0.4)'}
    >RECENT</button>
  {/if}
</div>

<!-- ── Background ─────────────────────────────────────────────────────────── -->
<div class="fixed inset-0 -z-10" style="background:#000;">
  <svg class="absolute inset-0 w-full h-full" style="opacity:0.025;" xmlns="http://www.w3.org/2000/svg">
    <filter id="n"><feTurbulence type="fractalNoise" baseFrequency="0.7" numOctaves="4" stitchTiles="stitch"/><feColorMatrix type="saturate" values="0"/></filter>
    <rect width="100%" height="100%" filter="url(#n)"/>
  </svg>
  <div class="absolute" style="top:-80px;left:50%;transform:translateX(-50%);width:500px;height:280px;background:radial-gradient(ellipse,rgba(10,132,255,0.15) 0%,transparent 70%);border-radius:50%;filter:blur(1px);pointer-events:none;"></div>
</div>

<!-- ── Sync warning modal (#5) ────────────────────────────────────────────── -->
{#if showSyncWarning}
  <div
    style="position:fixed;inset:0;z-index:200;background:rgba(0,0,0,0.65);display:flex;align-items:center;justify-content:center;padding:24px;"
    onclick={() => showSyncWarning = false}
    role="dialog" aria-modal="true"
  >
    <div
      style="background:#1c1c1e;border:1px solid rgba(255,255,255,0.1);border-radius:18px;padding:24px;max-width:340px;width:100%;"
      onclick={(e) => e.stopPropagation()}
    >
      <div style="display:flex;align-items:center;gap:12px;margin-bottom:14px;">
        <div style="width:38px;height:38px;border-radius:10px;background:rgba(255,69,58,0.15);display:flex;align-items:center;justify-content:center;flex-shrink:0;">
          <svg width="18" height="18" viewBox="0 0 20 20" fill="none">
            <path d="M10 3l7.5 13H2.5L10 3z" stroke="#ff453a" stroke-width="1.5" fill="none" stroke-linejoin="round"/>
            <path d="M10 8v4m0 2.5v.5" stroke="#ff453a" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </div>
        <div>
          <p style="font-size:14px;font-weight:600;color:var(--text-primary);margin:0;">Sync will delete files</p>
          <p style="font-size:11px;color:var(--text-tertiary);margin:3px 0 0;">This cannot be undone</p>
        </div>
      </div>
      <p style="font-size:12px;color:var(--text-secondary);line-height:1.5;margin:0 0 18px;">
        Sync mirrors the source exactly. Any file in <strong style="color:var(--text-primary);">{basename(destPath)}</strong> that doesn't exist in the source will be <strong style="color:var(--red);">permanently deleted</strong>.
      </p>
      <div style="display:flex;gap:8px;">
        <button
          onclick={() => showSyncWarning = false}
          style="flex:1;padding:10px;border-radius:10px;border:1px solid rgba(255,255,255,0.1);background:transparent;color:var(--text-secondary);font-size:13px;font-weight:500;cursor:pointer;"
          onmouseenter={(e)=>(e.currentTarget as HTMLElement).style.background='rgba(255,255,255,0.05)'}
          onmouseleave={(e)=>(e.currentTarget as HTMLElement).style.background='transparent'}
        >Cancel</button>
        <button
          onclick={() => { showSyncWarning = false; startWarp(); }}
          style="flex:1;padding:10px;border-radius:10px;border:none;background:var(--red);color:white;font-size:13px;font-weight:600;cursor:pointer;"
          onmouseenter={(e)=>(e.currentTarget as HTMLElement).style.opacity='0.85'}
          onmouseleave={(e)=>(e.currentTarget as HTMLElement).style.opacity='1'}
        >Sync & Delete</button>
      </div>
    </div>
  </div>
{/if}

<!-- ── Drop conflict modal (#11) ──────────────────────────────────────────── -->
{#if dropConflict}
  <div
    style="position:fixed;inset:0;z-index:200;background:rgba(0,0,0,0.65);display:flex;align-items:center;justify-content:center;padding:24px;"
    onclick={() => { dropConflict=false; _pendingDrop=""; }}
    role="dialog" aria-modal="true"
  >
    <div
      style="background:#1c1c1e;border:1px solid rgba(255,255,255,0.1);border-radius:18px;padding:22px;max-width:320px;width:100%;"
      onclick={(e) => e.stopPropagation()}
    >
      <p style="font-size:13px;font-weight:600;color:var(--text-primary);margin:0 0 6px;">Replace which slot?</p>
      <p style="font-size:11px;color:var(--text-tertiary);margin:0 0 16px;">{basename(_pendingDrop)}</p>
      <div style="display:flex;gap:8px;">
        <button
          onclick={() => applyDropToPending("source")}
          style="flex:1;padding:10px;border-radius:10px;border:1px solid rgba(10,132,255,0.3);background:rgba(10,132,255,0.1);color:var(--accent);font-size:12px;font-weight:600;cursor:pointer;"
        >← Source</button>
        <button
          onclick={() => applyDropToPending("dest")}
          style="flex:1;padding:10px;border-radius:10px;border:1px solid rgba(48,209,88,0.3);background:rgba(48,209,88,0.1);color:var(--green);font-size:12px;font-weight:600;cursor:pointer;"
        >Destination →</button>
      </div>
    </div>
  </div>
{/if}

<!-- ── Recent transfers panel (#26) ───────────────────────────────────────── -->
{#if showRecent}
  <div
    style="position:fixed;inset:0;z-index:100;background:rgba(0,0,0,0.5);"
    onclick={() => showRecent=false}
    role="dialog" aria-modal="true"
  >
    <div
      style="position:absolute;top:36px;right:8px;width:300px;background:#1c1c1e;border:1px solid rgba(255,255,255,0.1);border-radius:14px;overflow:hidden;"
      onclick={(e) => e.stopPropagation()}
    >
      <div style="padding:12px 14px;border-bottom:1px solid rgba(255,255,255,0.07);display:flex;align-items:center;justify-content:space-between;">
        <p style="font-size:11px;font-weight:700;color:var(--text-tertiary);letter-spacing:0.06em;text-transform:uppercase;margin:0;">Recent</p>
        <button onclick={() => { recentTransfers=[]; localStorage.removeItem('warp-recent'); showRecent=false; }}
          style="font-size:10px;color:var(--red);background:none;border:none;cursor:pointer;font-family:var(--font-sf);">Clear</button>
      </div>
      {#each recentTransfers as r}
        <button
          onclick={() => loadRecent(r)}
          style="width:100%;padding:10px 14px;text-align:left;background:transparent;border:none;border-bottom:1px solid rgba(255,255,255,0.05);cursor:pointer;display:flex;flex-direction:column;gap:2px;"
          onmouseenter={(e)=>(e.currentTarget as HTMLElement).style.background='rgba(255,255,255,0.04)'}
          onmouseleave={(e)=>(e.currentTarget as HTMLElement).style.background='transparent'}
        >
          <div style="display:flex;align-items:center;justify-content:space-between;">
            <span style="font-size:12px;font-weight:500;color:var(--text-primary);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:200px;">{basename(r.source)} → {basename(r.dest)}</span>
            <span style="font-size:10px;color:var(--text-tertiary);flex-shrink:0;margin-left:8px;">{timeAgo(r.timestamp)}</span>
          </div>
          <span style="font-size:10px;color:var(--text-tertiary);">{r.mode} · {fmtBytes(r.bytes)} · {fmtDuration(r.duration_ms)}</span>
        </button>
      {/each}
    </div>
  </div>
{/if}

<!-- ── Main ────────────────────────────────────────────────────────────────── -->
<main style="
  min-height:100vh;
  display:flex;
  flex-direction:column;
  align-items:center;
  justify-content:center;
  padding:36px 20px 20px;
  font-family:var(--font-sf);
  cursor:default;
">
  <div style="width:100%;max-width:440px;display:flex;flex-direction:column;gap:14px;">

    <!-- Header -->
    <div style="text-align:center;margin-bottom:2px;">
      <h1 style="font-size:40px;font-weight:700;letter-spacing:-0.04em;color:var(--text-primary);margin:0;line-height:1;cursor:default;user-select:none;">Warp</h1>
      <p style="margin:5px 0 0;font-size:12px;font-weight:500;color:var(--text-tertiary);">
        High-speed file transfer <span style="opacity:0.4;">v{APP_VERSION}</span>
      </p>
    </div>

    {#if !lastSummary}

    <!-- Source + Dest card -->
    <div style="background:var(--glass-bg);border:1px solid var(--glass-border);backdrop-filter:blur(48px) saturate(180%);border-radius:16px;overflow:visible;position:relative;">

      <!-- Source row -->
      <div style="display:flex;align-items:center;gap:12px;padding:13px 14px;border-bottom:1px solid var(--glass-border);background:{dragTarget==='source'?'rgba(10,132,255,0.08)':'transparent'};transition:background 0.15s;border-radius:16px 16px 0 0;">
        <!-- Folder icon -->
        <div style="width:34px;height:34px;border-radius:9px;background:{sourcePath?'rgba(10,132,255,0.18)':'rgba(255,255,255,0.06)'};display:flex;align-items:center;justify-content:center;flex-shrink:0;">
          <svg width="17" height="17" viewBox="0 0 20 20" fill="none">
            {#if sourcePath && !sourceInfo?.isFile}
              <path d="M2 6.5A2.5 2.5 0 014.5 4H8l1.5 2h6A2.5 2.5 0 0118 8.5v5A2.5 2.5 0 0115.5 16h-11A2.5 2.5 0 012 13.5V6.5Z" fill="#0a84ff"/>
            {:else if sourcePath && sourceInfo?.isFile}
              <path d="M5 3h7l4 4v10a1 1 0 01-1 1H5a1 1 0 01-1-1V4a1 1 0 011-1z" stroke="#ff453a" stroke-width="1.4" fill="none"/>
            {:else}
              <path d="M2 6.5A2.5 2.5 0 014.5 4H8l1.5 2h6A2.5 2.5 0 0118 8.5v5A2.5 2.5 0 0115.5 16h-11A2.5 2.5 0 012 13.5V6.5Z" stroke="rgba(255,255,255,0.2)" stroke-width="1.4" fill="none"/>
            {/if}
          </svg>
        </div>
        <!-- Text -->
        <div style="flex:1;min-width:0;">
          <p style="font-size:9px;font-weight:700;color:var(--text-tertiary);letter-spacing:0.07em;text-transform:uppercase;margin:0 0 2px;">Source</p>
          {#if sourcePath}
            <p style="font-size:13px;font-weight:500;color:{sourceInfo?.isFile?'var(--red)':'var(--text-primary)'};margin:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;" title={sourcePath}>{basename(sourcePath)}</p>
            <p style="font-size:10px;color:var(--text-tertiary);margin:1px 0 0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;">
              {#if isScanning}<span class="animate-pulse-soft">Scanning…</span>
              {:else if sourceInfo?.isFile}<span style="color:var(--red);">Drop a folder, not a file</span>
              {:else if sourceInfo}{fmtFiles(sourceInfo.files)} · {fmtBytes(sourceInfo.bytes)}
              {:else}{sourcePath}{/if}
            </p>
            {#if sourceWarning}
              <p style="font-size:9px;color:var(--orange);margin:2px 0 0;">⚠ {sourceWarning}</p>
            {/if}
          {:else}
            <p style="font-size:13px;color:var(--text-tertiary);margin:0;">Drop or <button onclick={browseSource} style="background:none;border:none;color:var(--accent);font-size:13px;font-family:var(--font-sf);cursor:pointer;padding:0;text-decoration:underline;text-underline-offset:2px;">browse</button></p>
          {/if}
        </div>
        {#if sourcePath}
          <button onclick={() => { sourcePath=""; sourceInfo=null; }} aria-label="Clear source"
            style="width:18px;height:18px;border-radius:50%;background:rgba(255,255,255,0.08);border:none;padding:0;display:flex;align-items:center;justify-content:center;flex-shrink:0;cursor:pointer;opacity:0.5;transition:opacity 0.15s;"
            onmouseenter={(e)=>(e.currentTarget as HTMLElement).style.opacity='1'}
            onmouseleave={(e)=>(e.currentTarget as HTMLElement).style.opacity='0.5'}
          ><svg width="7" height="7" viewBox="0 0 8 8" fill="none"><path d="M1 1l6 6M7 1L1 7" stroke="white" stroke-width="1.6" stroke-linecap="round"/></svg></button>
        {/if}
      </div>

      <!-- Swap + arrow connector (#10) -->
      <div style="height:0;position:relative;display:flex;justify-content:center;">
        <button
          onclick={swapPaths}
          title="Swap source and destination"
          disabled={!sourcePath && !destPath}
          style="
            position:absolute;top:-11px;z-index:10;
            width:22px;height:22px;border-radius:50%;
            background:#111;border:1px solid var(--glass-border);
            display:flex;align-items:center;justify-content:center;
            cursor:{sourcePath||destPath?'pointer':'default'};
            transition:background 0.15s, border-color 0.15s;
          "
          onmouseenter={(e)=>{ if(sourcePath||destPath){ (e.currentTarget as HTMLElement).style.background='#222'; (e.currentTarget as HTMLElement).style.borderColor='rgba(255,255,255,0.2)'; } }}
          onmouseleave={(e)=>{ (e.currentTarget as HTMLElement).style.background='#111'; (e.currentTarget as HTMLElement).style.borderColor='var(--glass-border)'; }}
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
            <path d="M3 1.5v7M1.5 7L3 8.5 4.5 7M7 8.5v-7M5.5 3L7 1.5 8.5 3" stroke="rgba(255,255,255,0.35)" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      </div>

      <!-- Dest row -->
      <div style="display:flex;align-items:center;gap:12px;padding:13px 14px;background:{dragTarget==='dest'?'rgba(10,132,255,0.08)':'transparent'};transition:background 0.15s;border-radius:0 0 16px 16px;">
        <div style="width:34px;height:34px;border-radius:9px;background:{destPath?'rgba(48,209,88,0.15)':'rgba(255,255,255,0.06)'};display:flex;align-items:center;justify-content:center;flex-shrink:0;">
          <svg width="17" height="17" viewBox="0 0 20 20" fill="none">
            {#if destPath}
              <path d="M2 6.5A2.5 2.5 0 014.5 4H8l1.5 2h6A2.5 2.5 0 0118 8.5v5A2.5 2.5 0 0115.5 16h-11A2.5 2.5 0 012 13.5V6.5Z" fill="#30d158"/>
            {:else}
              <path d="M2 6.5A2.5 2.5 0 014.5 4H8l1.5 2h6A2.5 2.5 0 0118 8.5v5A2.5 2.5 0 0115.5 16h-11A2.5 2.5 0 012 13.5V6.5Z" stroke="rgba(255,255,255,0.2)" stroke-width="1.4" fill="none"/>
            {/if}
          </svg>
        </div>
        <div style="flex:1;min-width:0;">
          <p style="font-size:9px;font-weight:700;color:var(--text-tertiary);letter-spacing:0.07em;text-transform:uppercase;margin:0 0 2px;">Destination</p>
          {#if destPath}
            <p style="font-size:13px;font-weight:500;color:var(--text-primary);margin:0;white-space:nowrap;overflow:hidden;text-overflow:ellipsis;" title={destPath}>{basename(destPath)}</p>
            <p style="font-size:10px;color:var(--text-tertiary);margin:1px 0 0;">
              {#if destInfo && destInfo.files > 0}{fmtFiles(destInfo.files)} · {fmtBytes(destInfo.bytes)} already here
              {:else if destInfo}Empty folder
              {:else}{destPath}{/if}
            </p>
            {#if destWarning}
              <p style="font-size:9px;color:var(--orange);margin:2px 0 0;">⚠ {destWarning}</p>
            {/if}
          {:else}
            <p style="font-size:13px;color:var(--text-tertiary);margin:0;">Drop or <button onclick={browseDest} style="background:none;border:none;color:var(--accent);font-size:13px;font-family:var(--font-sf);cursor:pointer;padding:0;text-decoration:underline;text-underline-offset:2px;">browse</button></p>
          {/if}
        </div>
        {#if destPath}
          <button onclick={() => { destPath=""; destInfo=null; }} aria-label="Clear destination"
            style="width:18px;height:18px;border-radius:50%;background:rgba(255,255,255,0.08);border:none;padding:0;display:flex;align-items:center;justify-content:center;flex-shrink:0;cursor:pointer;opacity:0.5;transition:opacity 0.15s;"
            onmouseenter={(e)=>(e.currentTarget as HTMLElement).style.opacity='1'}
            onmouseleave={(e)=>(e.currentTarget as HTMLElement).style.opacity='0.5'}
          ><svg width="7" height="7" viewBox="0 0 8 8" fill="none"><path d="M1 1l6 6M7 1L1 7" stroke="white" stroke-width="1.6" stroke-linecap="round"/></svg></button>
        {/if}
      </div>
    </div>

    <!-- Mode picker -->
    <div style="background:var(--surface-2);border:1px solid var(--glass-border);border-radius:12px;padding:4px;display:flex;gap:3px;">
      {#each MODES as m}
        <button
          onclick={() => mode = m.id}
          title={m.warning ?? m.desc}
          style="flex:1;padding:6px 8px;border-radius:9px;font-size:12px;font-weight:600;border:none;cursor:pointer;transition:all 0.15s;outline:none;
            background:{mode===m.id?'rgba(255,255,255,0.10)':'transparent'};
            color:{mode===m.id?'var(--text-primary)':'var(--text-tertiary)'};
            box-shadow:{mode===m.id?'0 1px 4px rgba(0,0,0,0.35)':'none'};"
          aria-pressed={mode === m.id}
          onmouseenter={(e)=>{ if(mode!==m.id)(e.currentTarget as HTMLElement).style.color='var(--text-secondary)'; }}
          onmouseleave={(e)=>{ if(mode!==m.id)(e.currentTarget as HTMLElement).style.color='var(--text-tertiary)'; }}
        >
          {m.label}
          {#if m.warning}<span style="font-size:9px;margin-left:2px;opacity:0.7;">⚠</span>{/if}
        </button>
      {/each}
    </div>

    <!-- Mode description + sync warning (#12) -->
    <p style="text-align:center;font-size:11px;margin:-6px 0 0;
      color:{mode==='sync'?'rgba(255,159,10,0.8)':'var(--text-tertiary)'};">
      {MODES.find(m => m.id === mode)?.warning ?? MODES.find(m => m.id === mode)?.desc}
    </p>

    <!-- #3: Conflict + folder mode options -->
    {#if mode !== "move" || true}
      <div style="display:flex;align-items:center;justify-content:center;gap:12px;flex-wrap:wrap;">
        <!-- Folder mode toggle -->
        <div style="display:flex;align-items:center;gap:6px;">
          <p style="font-size:10px;color:var(--text-tertiary);margin:0;">Destination:</p>
          {#each [
            { id: "into",  label: "Inside folder", title: `Result: ${destPath && sourcePath ? basename(destPath) + '\\' + basename(sourcePath) + '\\' : 'dest\\source_name\\'}` },
            { id: "merge", label: "Merge contents", title: `Result: ${destPath ? basename(destPath) + '\\' : 'dest\\'} (files go directly inside)` },
          ] as opt}
            <button
              onclick={() => folderMode = opt.id as "into" | "merge"}
              title={opt.title}
              style="padding:3px 8px;border-radius:6px;font-size:10px;font-weight:600;border:none;cursor:pointer;transition:all 0.15s;
                background:{folderMode===opt.id?'rgba(255,255,255,0.10)':'transparent'};
                color:{folderMode===opt.id?'var(--text-primary)':'var(--text-tertiary)'};
                border:1px solid {folderMode===opt.id?'rgba(255,255,255,0.12)':'transparent'};"
            >{opt.label}</button>
          {/each}
        </div>

        <!-- Divider -->
        {#if mode !== "move"}
          <div style="width:1px;height:14px;background:rgba(255,255,255,0.08);"></div>
          <!-- Conflict option -->
          <div style="display:flex;align-items:center;gap:6px;">
            <p style="font-size:10px;color:var(--text-tertiary);margin:0;">Conflict:</p>
            {#each [{id:'overwrite',label:'Overwrite'},{id:'skip',label:'Skip'}] as opt}
              <button
                onclick={() => conflict = opt.id as Conflict}
                style="padding:3px 8px;border-radius:6px;font-size:10px;font-weight:600;border:none;cursor:pointer;transition:all 0.15s;
                  background:{conflict===opt.id?'rgba(255,255,255,0.10)':'transparent'};
                  color:{conflict===opt.id?'var(--text-primary)':'var(--text-tertiary)'};
                  border:1px solid {conflict===opt.id?'rgba(255,255,255,0.12)':'transparent'};"
              >{opt.label}</button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Cross-drive move warning -->
    {#if crossDriveMove}
      <div style="background:rgba(255,159,10,0.08);border:1px solid rgba(255,159,10,0.2);border-radius:10px;padding:8px 12px;">
        <p style="font-size:10px;color:var(--orange);margin:0;line-height:1.5;">
          ⚠ <strong>Cross-drive move:</strong> Robocopy will copy files to {sourceInfo?.drive ?? "dest"} then delete from {destInfo?.drive ?? "source"}.
          If cancelled mid-transfer, source files may be partially deleted.
          Consider using <strong>Copy</strong> instead.
        </p>
      </div>
    {/if}

    <!-- Merge + Sync danger warning -->
    {#if mergeSyncDanger}
      <div style="background:rgba(255,69,58,0.08);border:1px solid rgba(255,69,58,0.2);border-radius:10px;padding:8px 12px;">
        <p style="font-size:10px;color:var(--red);margin:0;line-height:1.5;">
          ⚠ <strong>Dangerous combination:</strong> Sync + Merge Contents will mirror the source directly into the destination root.
          Files already in {destPath ? basename(destPath) : "destination"} that are not in the source will be <strong>permanently deleted</strong>.
        </p>
      </div>
    {/if}

    <!-- Progress / Engage -->
    {#if isProcessing}
      <div style="background:var(--glass-bg);border:1px solid var(--glass-border);backdrop-filter:blur(40px);border-radius:14px;padding:13px 15px;display:flex;flex-direction:column;gap:9px;">
        <!-- Top: spinner + file + speed -->
        <div style="display:flex;align-items:center;gap:9px;">
          <div class="animate-spin-smooth" style="width:14px;height:14px;border-radius:50%;flex-shrink:0;border:2px solid rgba(10,132,255,0.22);border-top-color:var(--accent);"></div>
          <span style="flex:1;font-size:12px;font-weight:500;color:var(--text-secondary);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;min-width:0;">{currentFile || "Transferring…"}</span>
          <span style="font-size:12px;font-weight:600;color:var(--accent);flex-shrink:0;font-variant-numeric:tabular-nums;">{speed || `${progress}%`}</span>
        </div>
        <!-- Progress bar -->
        <div style="height:3px;background:rgba(255,255,255,0.06);border-radius:100px;overflow:hidden;">
          {#if isIndeterminate}
            <!-- Indeterminate: animated sweep for empty/tiny folders -->
            <div class="animate-shimmer" style="height:100%;width:60%;background:linear-gradient(90deg,transparent,var(--accent),transparent);border-radius:100px;"></div>
          {:else}
            <div class="animate-shimmer" style="height:100%;width:{progress}%;background:linear-gradient(90deg,var(--accent),#5e5ce6,var(--accent));border-radius:100px;transition:width 0.4s cubic-bezier(0.4,0,0.2,1);"></div>
          {/if}
        </div>
        <!-- Bottom: file count + overall % + cancel (#1) -->
        <div style="display:flex;justify-content:space-between;align-items:center;">
          <span style="font-size:10px;color:var(--text-tertiary);">
            {filesTotal > 0 ? `${filesDone.toLocaleString()} / ${filesTotal.toLocaleString()} files` : `${basename(sourcePath)} → ${basename(destPath)}`}
          </span>
          <div style="display:flex;align-items:center;gap:8px;">
            <span style="font-size:10px;color:var(--text-tertiary);font-variant-numeric:tabular-nums;">{progress}%</span>
            <button
              onclick={cancelTransfer}
              style="font-size:9px;font-weight:700;color:var(--red);background:rgba(255,69,58,0.1);border:1px solid rgba(255,69,58,0.2);border-radius:5px;padding:2px 7px;cursor:pointer;letter-spacing:0.04em;font-family:var(--font-sf);"
              onmouseenter={(e)=>(e.currentTarget as HTMLElement).style.background='rgba(255,69,58,0.2)'}
              onmouseleave={(e)=>(e.currentTarget as HTMLElement).style.background='rgba(255,69,58,0.1)'}
            >CANCEL</button>
          </div>
        </div>
      </div>

    {:else}
      <button
        onclick={handleStart}
        disabled={!canStart}
        style="width:100%;padding:12px;border-radius:14px;border:none;font-size:14px;font-weight:600;letter-spacing:-0.01em;transition:all 0.15s;outline:none;
          cursor:{canStart?'pointer':'not-allowed'};
          background:{canStart?'var(--accent)':'rgba(255,255,255,0.05)'};
          color:{canStart?'#fff':'var(--text-tertiary)'};
          box-shadow:{canStart?'0 2px 20px rgba(10,132,255,0.28)':'none'};"
        onmouseenter={(e)=>{ if(canStart)(e.currentTarget as HTMLElement).style.background='var(--accent-hover)'; }}
        onmouseleave={(e)=>{ if(canStart)(e.currentTarget as HTMLElement).style.background='var(--accent)'; }}
        onmousedown={(e)=>{ if(canStart)(e.currentTarget as HTMLElement).style.transform='scale(0.98)'; }}
        onmouseup={(e)=>{ if(canStart)(e.currentTarget as HTMLElement).style.transform='scale(1)'; }}
      >{startLabel}</button>
    {/if}

    <!-- Status hint -->
    {#if !isProcessing}
      <p style="text-align:center;font-size:11px;color:var(--text-tertiary);margin:-5px 0 0;">
        {#if sourcePath && destPath && sourceInfo && !sourceInfo.isFile}
          {@const effectiveDest = folderMode === 'into' && basename(destPath).toLowerCase() !== basename(sourcePath).toLowerCase()
            ? destPath.replace(/\\+$/, '') + '\\' + basename(sourcePath)
            : destPath}
          → <span style="font-family:monospace;font-size:10px;">{effectiveDest}</span>
        {:else if sourcePath && !destPath}
          Now drop or browse a destination folder
        {:else if !sourcePath}
          Drop folders onto the window, or use the browse links
        {/if}
      </p>
    {/if}

    <!-- ── Summary ─────────────────────────────────────────────────────── -->
    {:else}
      <div class="animate-fade-up" style="display:flex;flex-direction:column;gap:11px;">

        <!-- Result card -->
        <div style="background:var(--glass-bg);border:1px solid var(--glass-border);backdrop-filter:blur(48px) saturate(180%);border-radius:16px;overflow:hidden;">
          <!-- Header -->
          <div style="padding:15px 16px;display:flex;align-items:center;gap:12px;border-bottom:1px solid var(--glass-border);">
            <div style="width:38px;height:38px;border-radius:11px;flex-shrink:0;
              background:{lastSummary!.cancelled?'rgba(255,159,10,0.14)':lastSummary!.failed>0?'rgba(255,69,58,0.14)':'rgba(48,209,88,0.14)'};
              display:flex;align-items:center;justify-content:center;">
              {#if lastSummary!.cancelled}
                <svg width="18" height="18" viewBox="0 0 20 20" fill="none"><path d="M6 6l8 8M14 6l-8 8" stroke="#ff9f0a" stroke-width="1.8" stroke-linecap="round"/></svg>
              {:else if lastSummary!.failed > 0}
                <svg width="18" height="18" viewBox="0 0 20 20" fill="none"><circle cx="10" cy="10" r="7" stroke="#ff453a" stroke-width="1.5"/><path d="M10 6.5v4m0 2.5v.5" stroke="#ff453a" stroke-width="1.6" stroke-linecap="round"/></svg>
              {:else}
                <svg width="18" height="18" viewBox="0 0 20 20" fill="none"><path d="M5.5 10.5l3 3 6-6.5" stroke="#30d158" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>
              {/if}
            </div>
            <div style="flex:1;min-width:0;">
              <p style="font-size:15px;font-weight:600;color:var(--text-primary);margin:0;letter-spacing:-0.01em;">
                {lastSummary!.cancelled ? "Transfer cancelled" : lastSummary!.errorMessage ? "Transfer failed" : lastSummary!.failed > 0 ? "Completed with errors" : "Transfer complete"}
              </p>
              <p style="font-size:11px;color:var(--text-tertiary);margin:3px 0 0;">
                {fmtDuration(lastSummary!.durationMs)} · {MODES.find(m=>m.id===mode)?.label}
                {#if lastSummary!.bytesTransferred > 0} · {fmtBytes(lastSummary!.bytesTransferred)}{/if}
              </p>
            </div>
          </div>

          <!-- Stats -->
          <div style="display:grid;grid-template-columns:repeat(3,1fr);">
            {#each [
              { label: mode==="move"?"Moved":mode==="sync"?"Synced":"Copied", value: lastSummary!.transferred, color:"var(--accent)" },
              { label:"Skipped", value: lastSummary!.skipped, color:"var(--text-secondary)" },
              { label:"Failed",  value: lastSummary!.failed,  color: lastSummary!.failed>0?"var(--red)":"var(--text-tertiary)" },
            ] as stat, i}
              <div style="padding:13px 10px;text-align:center;{i<2?'border-right:1px solid var(--glass-border);':''}">
                <p style="font-size:22px;font-weight:700;color:{stat.color};margin:0;line-height:1;letter-spacing:-0.03em;font-variant-numeric:tabular-nums;">{stat.value.toLocaleString()}</p>
                <p style="font-size:9px;font-weight:600;color:var(--text-tertiary);margin:4px 0 0;letter-spacing:0.04em;text-transform:uppercase;">{stat.label}</p>
              </div>
            {/each}
          </div>
        </div>

        <!-- Error message (disk full, access denied, etc.) -->
        {#if lastSummary!.errorMessage}
          <div style="background:rgba(255,69,58,0.08);border:1px solid rgba(255,69,58,0.2);border-radius:12px;padding:10px 14px;">
            <p style="font-size:10px;font-weight:700;color:var(--red);letter-spacing:0.04em;text-transform:uppercase;margin:0 0 4px;">Transfer Error (code {lastSummary!.errorCode})</p>
            <p style="font-size:11px;color:rgba(255,69,58,0.8);margin:0;line-height:1.5;">{lastSummary!.errorMessage}</p>
          </div>
        {/if}

        <!-- Paths recap -->
        <div style="background:rgba(255,255,255,0.03);border:1px solid var(--glass-border);border-radius:12px;padding:11px 14px;display:flex;flex-direction:column;gap:5px;">
          {#each [{label:"From",path:sourcePath},{label:"To",path:destPath}] as row}
            <div style="display:flex;align-items:center;gap:8px;">
              <span style="font-size:9px;font-weight:700;color:var(--text-tertiary);letter-spacing:0.06em;text-transform:uppercase;width:26px;flex-shrink:0;">{row.label}</span>
              <span style="font-size:11px;color:var(--text-secondary);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;" title={row.path}>{row.path}</span>
            </div>
          {/each}
        </div>

        <!-- Errors -->
        {#if errorLogs.length > 0}
          <div style="background:rgba(255,69,58,0.05);border:1px solid rgba(255,69,58,0.15);border-radius:12px;padding:11px 14px;">
            <p style="font-size:9px;font-weight:700;color:var(--red);letter-spacing:0.06em;text-transform:uppercase;margin:0 0 7px;">{errorLogs.length} Error{errorLogs.length!==1?"s":""}</p>
            <div style="max-height:72px;overflow-y:auto;display:flex;flex-direction:column;gap:2px;">
              {#each errorLogs as log}
                <p style="font-size:10px;font-family:monospace;color:rgba(255,69,58,0.65);margin:0;">{log}</p>
              {/each}
            </div>
          </div>
        {/if}

        <!-- New transfer -->
        <button onclick={reset}
          style="width:100%;padding:12px;border-radius:14px;border:none;font-size:14px;font-weight:600;letter-spacing:-0.01em;background:var(--accent);color:white;box-shadow:0 2px 20px rgba(10,132,255,0.28);cursor:pointer;transition:all 0.15s;outline:none;"
          onmouseenter={(e)=>(e.currentTarget as HTMLElement).style.background='var(--accent-hover)'}
          onmouseleave={(e)=>(e.currentTarget as HTMLElement).style.background='var(--accent)'}
          onmousedown={(e)=>(e.currentTarget as HTMLElement).style.transform='scale(0.98)'}
          onmouseup={(e)=>(e.currentTarget as HTMLElement).style.transform='scale(1)'}
        >New Transfer</button>

      </div>
    {/if}

    <!-- Keyboard shortcuts hint -->
    <p style="text-align:center;font-size:9px;color:rgba(255,255,255,0.12);margin:-2px 0 0;letter-spacing:0.02em;">
      {#if isProcessing}Esc to cancel
      {:else if lastSummary}Esc to reset
      {:else}⌘O source · ⌘⇧O destination · Enter to start{/if}
    </p>

  </div>
</main>

<style>
  :global(*) { box-sizing: border-box; }
  :global(body) {
    margin: 0;
    overflow: hidden;
    background: transparent;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
  }
  :global(::-webkit-scrollbar) { width: 4px; }
  :global(::-webkit-scrollbar-track) { background: transparent; }
  :global(::-webkit-scrollbar-thumb) { background: rgba(255,255,255,0.12); border-radius: 4px; }
  :global(button:focus-visible) { outline: 2px solid var(--accent); outline-offset: 2px; }
</style>

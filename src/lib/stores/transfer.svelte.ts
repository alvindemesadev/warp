/// <reference types="svelte" />
// Transfer store — single job state + progress. Svelte 5 runes ($state/$derived) via class.
// Extracted from +page.svelte:32-... for Phase 2.

import { basename, fmtBytes, fmtDuration } from "$lib/format";
import {
  loadRecent as loadRecentEntries,
  saveRecentEntries as persistRecent,
  getNotifyPref,
} from "$lib/storage";
import type {
  Mode,
  Conflict,
  FolderMode,
  PathInfo,
  WarpProgress,
  WarpSummary,
  RecentEntry,
} from "$lib/types";
import {
  getOverlappingPath,
  isCrossDriveMove,
  isMergeSyncDanger,
  canStartTransfer,
  getStartLabel,
} from "$lib/services/validation";
import {
  getPathInfo,
  warpFileOp,
  cancelWarp as cancelWarpInvoke,
  pauseWarp,
} from "$lib/services/warp";
import {
  sendNotification,
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";

export class TransferStore {
  sourcePath = $state("");
  destPath = $state("");
  sourceInfo = $state<PathInfo | null>(null);
  destInfo = $state<PathInfo | null>(null);
  mode = $state<Mode>("copy");
  conflict = $state<Conflict>("overwrite");
  folderMode = $state<FolderMode>("into");
  throttle = $state(0);
  verify = $state(false);
  workers = $state(0);
  filter = $state("");
  customSpeed = $state(false);
  customSpeedValue = $state(50);

  progress = $state(0);
  currentFile = $state("");
  speed = $state("");
  filesDone = $state(0);
  filesTotal = $state(0);
  etaSeconds = $state(0);
  transferredFiles = $state<string[]>([]);
  liveWorkers = $state(0);
  shardsDone = $state(0);
  shardsTotal = $state(0);
  paused = $state(false);
  slowDrive = $state(false);

  isProcessing = $state(false);
  isScanning = $state(false);
  isScanningDest = $state(false);
  isVerifying = $state(false);
  isIndeterminate = $state(false);
  lastSummary = $state<WarpSummary | null>(null);
  errorLogs = $state<string[]>([]);
  recentTransfers = $state<RecentEntry[]>([]);

  _runId = 0;

  overlappingPath = $derived.by(() =>
    getOverlappingPath(this.sourcePath, this.destPath, this.folderMode),
  );
  crossDriveMove = $derived.by(() => isCrossDriveMove(this.mode, this.sourceInfo, this.destInfo));
  mergeSyncDanger = $derived(isMergeSyncDanger(this.mode, this.folderMode));
  canStart = $derived.by(() =>
    canStartTransfer(
      this.sourcePath,
      this.destPath,
      this.isProcessing,
      this.sourceInfo,
      this.destInfo,
      this.overlappingPath,
    ),
  );
  startLabel = $derived.by(() =>
    getStartLabel(
      this.sourcePath,
      this.destPath,
      this.sourceInfo,
      this.destInfo,
      this.overlappingPath,
      this.mode,
    ),
  );

  syncSpeedMode(t: number) {
    const isPreset = [0, 100, 25, 5].includes(t);
    this.customSpeed = t > 0 && !isPreset;
    if (this.customSpeed) this.customSpeedValue = t;
  }

  resetTransferOnly() {
    this.sourcePath = this.destPath = "";
    this.sourceInfo = this.destInfo = null;
    this.progress = 0;
    this.speed = "";
    this.currentFile = "";
    this.filesDone = this.filesTotal = 0;
    this.etaSeconds = 0;
    this.transferredFiles = [];
    this.isProcessing = false;
    this.isScanning = false;
    this.isScanningDest = false;
    this.isVerifying = false;
    this.isIndeterminate = false;
    this.lastSummary = null;
    this.errorLogs = [];
    this.paused = false;
    this.slowDrive = false;
    this.liveWorkers = 0;
    this.shardsDone = 0;
    this.shardsTotal = 0;
  }

  async setSource(p: string) {
    this.sourcePath = p;
    this.sourceInfo = null;
    if (!p) return;
    this.isScanning = true;
    try {
      const info = await getPathInfo(p);
      this.sourceInfo = info;
      if (this.workers !== 0 && (info.removable || /^[D-Z]:/i.test(p) || p.startsWith("\\\\"))) {
        this.workers = 0;
      }
    } catch {
      this.sourceInfo = null;
      if (this.workers !== 0 && (/^[D-Z]:/i.test(p) || p.startsWith("\\\\"))) {
        this.workers = 0;
      }
    }
    this.isScanning = false;
  }

  async setDest(p: string) {
    this.destPath = p;
    this.destInfo = null;
    if (!p) return;
    this.isScanningDest = true;
    try {
      const info = await getPathInfo(p);
      this.destInfo = info;
      if (this.workers !== 0 && (info.removable || /^[D-Z]:/i.test(p) || p.startsWith("\\\\"))) {
        this.workers = 0;
      }
    } catch {
      this.destInfo = null;
      if (this.workers !== 0 && (/^[D-Z]:/i.test(p) || p.startsWith("\\\\"))) {
        this.workers = 0;
      }
    }
    this.isScanningDest = false;
  }

  swapPaths() {
    const tmpPath = this.sourcePath;
    const tmpInfo = this.sourceInfo;
    this.sourcePath = this.destPath;
    this.sourceInfo = this.destInfo;
    this.destPath = tmpPath;
    this.destInfo = tmpInfo;
  }

  async browseSource() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Source Folder",
    });
    if (selected && typeof selected === "string") this.setSource(selected);
  }

  async browseDest() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Destination Folder",
    });
    if (selected && typeof selected === "string") this.setDest(selected);
  }

  handleProgress(payload: WarpProgress) {
    this.progress = payload.percentage;
    this.currentFile = basename(payload.currentFile);
    if (payload.speed) this.speed = payload.speed;
    this.filesDone = payload.filesDone;
    this.filesTotal = payload.filesTotal;
    this.isIndeterminate = payload.indeterminate;
    this.liveWorkers = payload.activeWorkers ?? 0;
    this.shardsDone = payload.shardsDone ?? 0;
    this.shardsTotal = payload.shardsTotal ?? 0;
    if (payload.currentFile) {
      this.transferredFiles = [basename(payload.currentFile), ...this.transferredFiles].slice(
        0,
        200,
      );
    }
    if (!payload.indeterminate && payload.bytesPerSec > 0 && payload.totalBytes > 0) {
      const remaining = Math.max(0, payload.totalBytes - payload.bytesDone);
      this.etaSeconds = Math.round(remaining / payload.bytesPerSec);
    } else {
      this.etaSeconds = 0;
    }
  }

  handleWarpError(payload: string) {
    this.errorLogs = [...this.errorLogs, payload];
  }

  handleWarpVerifying() {
    this.isVerifying = true;
    this.currentFile = "Verifying...";
  }

  handleHealthWarning(slow: boolean) {
    this.slowDrive = slow;
  }

  initFromStorage() {
    this.recentTransfers = loadRecentEntries();
  }

  saveRecent(entry: RecentEntry) {
    const updated = [entry, ...this.recentTransfers].slice(0, 5);
    this.recentTransfers = updated;
    persistRecent(updated);
  }

  loadRecent(entry: RecentEntry) {
    this.setSource(entry.source);
    this.setDest(entry.dest);
    this.mode = entry.mode;
  }

  async notifyDone(s: WarpSummary) {
    if (getNotifyPref() === "never") return;
    try {
      let granted = await isPermissionGranted();
      if (!granted) {
        if (getNotifyPref() === "never") return;
        granted = (await requestPermission()) === "granted";
      }
      if (granted) {
        const verb = this.mode === "move" ? "Moved" : this.mode === "sync" ? "Synced" : "Copied";
        sendNotification({
          title: "Warp — Transfer Complete",
          body: `${verb} ${s.transferred.toLocaleString()} files - ${fmtBytes(s.bytesTransferred)} in ${fmtDuration(s.durationMs)}`,
        });
      }
    } catch {}
  }

  async startWarp(options?: { onSuccess?: (s: WarpSummary) => void }) {
    if (!this.sourcePath || !this.destPath || this.isProcessing) return;
    const id = ++this._runId;
    this.isProcessing = true;
    this.progress = 0;
    this.speed = "";
    this.filesDone = 0;
    this.filesTotal = 0;
    this.etaSeconds = 0;
    this.transferredFiles = [];
    this.isVerifying = false;
    this.currentFile = "Scanning...";
    this.lastSummary = null;
    this.errorLogs = [];
    this.paused = false;
    this.slowDrive = false;
    this.liveWorkers = 0;
    this.shardsDone = 0;
    this.shardsTotal = 0;
    try {
      const s = await warpFileOp({
        source: this.sourcePath,
        destination: this.destPath,
        mode: this.mode,
        conflict: this.conflict,
        folderMode: this.folderMode,
        throttle: this.throttle,
        verify: this.verify,
        workers: this.workers,
        filter: this.filter || undefined,
      });
      if (id !== this._runId) return;
      this.lastSummary = s;
      if (!s.cancelled) {
        this.progress = 100;
        this.currentFile = "";
        this.isIndeterminate = false;
        this.saveRecent({
          source: this.sourcePath,
          dest: this.destPath,
          mode: this.mode,
          transferred: s.transferred,
          bytes: s.bytesTransferred,
          duration_ms: s.durationMs,
          timestamp: Date.now(),
        });
        this.notifyDone(s);
        options?.onSuccess?.(s);
      } else {
        this.progress = 0;
        this.isIndeterminate = false;
      }
    } catch (err) {
      if (id !== this._runId) return;
      this.lastSummary = {
        totalFiles: 0,
        transferred: 0,
        skipped: 0,
        failed: 0,
        durationMs: 0,
        bytesTransferred: 0,
        cancelled: false,
        errorCode: -1,
        errorMessage: `Could not start the transfer: ${String(err)}`,
        verified: false,
        verifyMismatches: 0,
      };
      this.isIndeterminate = false;
    } finally {
      if (id === this._runId) {
        this.isProcessing = false;
        this.isVerifying = false;
      }
    }
  }

  async cancelTransfer() {
    this.currentFile = "Cancelling...";
    this.paused = false;
    try {
      await cancelWarpInvoke();
    } catch {}
  }

  async togglePause() {
    const next = !this.paused;
    try {
      await pauseWarp(next);
      this.paused = next;
    } catch {}
  }

  currentJobConfig(): {
    source: string;
    dest: string;
    mode: Mode;
    conflict: Conflict;
    folderMode: FolderMode;
    throttle: number;
    verify: boolean;
    workers?: number | undefined;
    filter?: string | undefined;
  } {
    return {
      source: this.sourcePath,
      dest: this.destPath,
      mode: this.mode,
      conflict: this.conflict,
      folderMode: this.folderMode,
      throttle: this.throttle,
      verify: this.verify,
      workers: this.workers,
      filter: this.filter || undefined,
    };
  }
}

export const transfer = new TransferStore();

// Centralized shared types — single source of truth for Mode/Conflict/PathInfo
// and every payload crossing the Tauri IPC boundary. storage.ts and
// transfer.ts import from here (and re-export for backwards compatibility).

export type Mode = "copy" | "move" | "sync";
export type Conflict = "overwrite" | "skip";
export type FolderMode = "into" | "merge";

export type PathInfo = {
  files: number;
  bytes: number;
  isFile: boolean;
  drive: string;
  removable: boolean;
};

export type WarpProgress = {
  percentage: number;
  currentFile: string;
  speed: string;
  filesDone: number;
  filesTotal: number;
  indeterminate: boolean;
  bytesPerSec: number;
  bytesDone: number;
  totalBytes: number;
  /** Parallel engine only (1 = sequential). */
  activeWorkers?: number;
  shardsDone?: number;
  shardsTotal?: number;
};

export type WarpSummary = {
  totalFiles: number;
  transferred: number;
  skipped: number;
  failed: number;
  durationMs: number;
  bytesTransferred: number;
  cancelled: boolean;
  errorCode: number;
  errorMessage: string;
  verified: boolean;
  verifyMismatches: number;
  /** Parallel engine only (1 = sequential). */
  workersUsed?: number;
  /** Files recovered by the automatic retry pass. */
  retriedOk?: number;
};

export type RecentEntry = {
  source: string;
  dest: string;
  mode: Mode;
  transferred: number;
  bytes: number;
  duration_ms: number;
  timestamp: number;
};

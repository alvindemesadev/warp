// Centralized shared types — single source for Mode/Conflict/PathInfo etc.
// Re-exports validation-aware types from storage/transfer and adds Tauri-specific.

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
};

export type QueueJob = {
  id: number;
  source: string;
  dest: string;
  mode: Mode;
  conflict: Conflict;
  folderMode: FolderMode;
  throttle: number;
  verify: boolean;
};

export type Preset = {
  name: string;
  source: string;
  dest: string;
  mode: Mode;
  conflict: Conflict;
  folderMode: FolderMode;
  throttle: number;
  verify: boolean;
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

// Thin wrappers around Tauri invokes — isolates invoke strings and types for testability.
import { invoke } from "@tauri-apps/api/core";

export type PathInfo = { files: number; bytes: number; isFile: boolean; drive: string; removable: boolean };

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

export async function getPathInfo(path: string): Promise<PathInfo> {
  return invoke<PathInfo>("get_path_info", { path });
}

export async function warpFileOp(args: {
  source: string;
  destination: string;
  mode: string;
  conflict: string;
  folderMode: string;
  throttle: number;
  verify: boolean;
}): Promise<WarpSummary> {
  return invoke<WarpSummary>("warp_file_op", args);
}

export async function cancelWarp(): Promise<void> {
  return invoke("cancel_warp");
}

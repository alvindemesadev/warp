// Thin wrappers around Tauri invokes — isolates invoke strings and types for testability.
// Types are canonical in ./types — re-export for backwards compat.
import { invoke } from "@tauri-apps/api/core";
import type { PathInfo, WarpSummary } from "./types";

export type { PathInfo, WarpSummary } from "./types";

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

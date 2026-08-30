// Warp service — thin invoke wrappers + event plumbing.
// Keeps `+page.svelte` free of `invoke`/`listen` strings.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { PathInfo, WarpProgress, WarpSummary } from "$lib/types";

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
  workers: number;
  filter?: string;
}): Promise<WarpSummary> {
  return invoke<WarpSummary>("warp_file_op", args);
}

export async function cancelWarp(): Promise<void> {
  return invoke("cancel_warp");
}

export async function pauseWarp(paused: boolean): Promise<void> {
  return invoke("pause_warp", { paused });
}

export async function listenHealthWarning(cb: (slow: boolean) => void): Promise<WarpEventUnlisten> {
  return listen<{ slow: boolean; mbps: number }>("warp-health-warning", ({ payload }) =>
    cb(payload.slow),
  );
}

export type WarpEventUnlisten = () => void;

export async function listenWarpProgress(
  cb: (p: WarpProgress) => void,
): Promise<WarpEventUnlisten> {
  return listen<WarpProgress>("warp-progress", ({ payload }) => cb(payload));
}

export async function listenWarpError(cb: (msg: string) => void): Promise<WarpEventUnlisten> {
  return listen<string>("warp-error", ({ payload }) => cb(payload));
}

export async function listenWarpVerifying(cb: () => void): Promise<WarpEventUnlisten> {
  return listen("warp-verifying", () => cb());
}

export function getCurrentWindowApi() {
  return getCurrentWindow();
}

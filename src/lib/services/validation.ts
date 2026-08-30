// Validation helpers — pure, testable, no Svelte/Tauri deps.
// Extracted from +page.svelte:382 et al. for Phase 2.

import { basename } from "$lib/format";
import type { PathInfo, Mode, FolderMode } from "$lib/types";

function normalizePath(p: string): string {
  return p.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

/** Returns warning string or null — mirrors Rust check_overlap + effective dest logic. */
export function getOverlappingPath(
  sourcePath: string,
  destPath: string,
  folderMode: FolderMode,
): string | null {
  if (!sourcePath || !destPath) return null;
  const a = normalizePath(sourcePath);
  const b = normalizePath(destPath);
  const sourceName = basename(sourcePath).toLowerCase();
  const effectiveB =
    folderMode === "into" && sourceName && !b.endsWith("/" + sourceName) ? `${b}/${sourceName}` : b;
  if (a === b || a === effectiveB) return "Source and destination are the same folder";
  if (effectiveB.startsWith(a + "/"))
    return "Destination is inside the source — would copy into itself";
  if (a.startsWith(b + "/")) return "Source is inside the destination — may cause recursion";
  return null;
}

export function isCrossDriveMove(
  mode: Mode,
  sourceInfo: PathInfo | null,
  destInfo: PathInfo | null,
): boolean {
  return (
    mode === "move" &&
    !!sourceInfo?.drive &&
    !!destInfo?.drive &&
    sourceInfo.drive.toLowerCase() !== destInfo.drive.toLowerCase()
  );
}

export function isMergeSyncDanger(mode: Mode, folderMode: FolderMode): boolean {
  return mode === "sync" && folderMode === "merge";
}

export function canStartTransfer(
  sourcePath: string,
  destPath: string,
  isProcessing: boolean,
  sourceInfo: PathInfo | null,
  destInfo: PathInfo | null,
  overlappingPath: string | null,
): boolean {
  return (
    !!sourcePath &&
    !!destPath &&
    !isProcessing &&
    !sourceInfo?.isFile &&
    !destInfo?.isFile &&
    !overlappingPath
  );
}

export function getStartLabel(
  sourcePath: string,
  destPath: string,
  sourceInfo: PathInfo | null,
  destInfo: PathInfo | null,
  overlappingPath: string | null,
  mode: Mode,
): string {
  const label = mode === "move" ? "Move" : mode === "sync" ? "Sync" : "Copy";
  if (overlappingPath) return `${label} Files`;
  if (!sourcePath || !destPath) return "Drop folders";
  if (sourceInfo?.isFile) return "Source must be a folder, not a file";
  if (destInfo?.isFile) return "Destination must be a folder, not a file";
  return `${label} Files`;
}

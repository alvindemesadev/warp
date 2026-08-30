// Transfer-related helpers — extracted from +page.svelte for testability.
// Types live in ./types (single source of truth); re-exported for compat.
import type { Mode, Conflict } from "./types";

export type { Mode, Conflict };

export const THROTTLE_OPTIONS = [
  { value: 0, label: "Unlimited" },
  { value: 100, label: "100 MB/s" },
  { value: 25, label: "25 MB/s" },
  { value: 5, label: "5 MB/s" },
] as const;

/** Parallel worker choices for the segmented control. 0 = Auto. */
export const WORKER_OPTIONS = [
  { value: 0, label: "Auto", title: "Pick workers from the drive types involved (recommended)" },
  { value: 2, label: "2", title: "2 parallel folder workers" },
  {
    value: 4,
    label: "4",
    title: "4 parallel folder workers — best on NVMe/SSD with many small files",
  },
  { value: 6, label: "6", title: "6 parallel folder workers — maximum parallelism" },
  { value: 8, label: "8", title: "8 parallel folder workers — may contend on slower disks" },
] as const;

/** Returns true if throttle value matches a preset option. */
export function isPresetThrottle(v: number): boolean {
  return THROTTLE_OPTIONS.some((o) => o.value === v);
}

/** Clamp custom speed to 1..500, 0 = unlimited. */
export function normalizeThrottleInput(v: number): number {
  if (!Number.isFinite(v)) return 50;
  if (v <= 0) return 0;
  return Math.min(500, Math.max(1, Math.round(v)));
}

/** Clamp workers to the valid set: 0 (Auto) or 2..=8. */
export function normalizeWorkersInput(v: number): number {
  if (!Number.isFinite(v)) return 0;
  const n = Math.round(v);
  if (n <= 0) return 0;
  return Math.min(8, Math.max(2, n));
}

/** OneDrive / network heuristic. */
export function isSpecialPath(p: string): string | null {
  const lower = p.toLowerCase();
  if (lower.includes("onedrive")) return "OneDrive path — ensure files are downloaded locally";
  if (p.startsWith("\\\\")) return "Network path — speed may be limited";
  return null;
}

/** For queue/presets: describe current job config. */
export function describeJob(mode: Mode, verify: boolean, throttle: number): string {
  const parts: string[] = [mode];
  if (verify) parts.push("verify");
  if (throttle) parts.push(`${throttle} MB/s`);
  return parts.join(" - ");
}

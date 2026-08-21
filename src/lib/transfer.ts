// Transfer-related helpers — extracted from +page.svelte for testability.

export type Mode = "copy" | "move" | "sync";
export type Conflict = "overwrite" | "skip";

export const THROTTLE_OPTIONS = [
  { value: 0, label: "Unlimited" },
  { value: 100, label: "100 MB/s" },
  { value: 25, label: "25 MB/s" },
  { value: 5, label: "5 MB/s" },
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
  return parts.join(" · ");
}

// Storage helpers with validation — never trust localStorage.
// Corrupt JSON or old schema should not crash the app; we clear the bad key and return fallback.
// Types live in ./types (single source of truth); re-exported for compat.
// Phase 2.4: zod schemas + versioned persistence `{ v: 1, data: [...] }` with backwards compat.

import { z } from "zod";
import type { Mode, Conflict, RecentEntry } from "./types";

export type { Mode, Conflict, RecentEntry };

const STORAGE_VERSION = 1;

const ModeSchema = z.enum(["copy", "move", "sync"]);

const RecentEntrySchema = z.object({
  source: z.string(),
  dest: z.string(),
  mode: ModeSchema,
  transferred: z.number().finite(),
  bytes: z.number().finite(),
  duration_ms: z.number().finite(),
  timestamp: z.number().finite(),
});

export function isValidRecentEntry(v: unknown): v is RecentEntry {
  return RecentEntrySchema.safeParse(v).success;
}

function unwrapVersioned<T>(parsed: unknown, validate: (v: unknown) => boolean): T[] {
  if (Array.isArray(parsed)) return parsed.filter(validate) as T[];
  if (
    parsed &&
    typeof parsed === "object" &&
    "v" in (parsed as Record<string, unknown>) &&
    "data" in (parsed as Record<string, unknown>)
  ) {
    const o = parsed as Record<string, unknown>;
    if (o["v"] === STORAGE_VERSION && Array.isArray(o["data"])) {
      return (o["data"] as unknown[]).filter(validate) as T[];
    }
    // Unknown version — treat as corrupt, caller will clear
    return [];
  }
  return [];
}

function isVersionedArray(parsed: unknown): boolean {
  return (
    !!parsed &&
    typeof parsed === "object" &&
    "v" in (parsed as Record<string, unknown>) &&
    "data" in (parsed as Record<string, unknown>)
  );
}

export function loadRecent(): RecentEntry[] {
  try {
    const raw = localStorage.getItem("warp-recent");
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    const valid = unwrapVersioned<RecentEntry>(parsed, isValidRecentEntry);
    if (valid.length === 0) {
      const wasArray = Array.isArray(parsed);
      const wasVersioned = isVersionedArray(parsed);
      const hadItems = wasArray
        ? parsed.length > 0
        : wasVersioned
          ? ((parsed as { data: unknown[] }).data?.length ?? 0) > 0
          : false;
      if (hadItems) localStorage.removeItem("warp-recent");
    }
    if (!Array.isArray(parsed) && !isVersionedArray(parsed))
      throw new Error("not array nor versioned");
    return valid;
  } catch {
    try {
      localStorage.removeItem("warp-recent");
    } catch {}
    return [];
  }
}

export function saveRecentEntries(entries: RecentEntry[]): void {
  try {
    localStorage.setItem("warp-recent", JSON.stringify({ v: STORAGE_VERSION, data: entries }));
  } catch {}
}

export function normalizeThrottle(v: unknown): number {
  const n = typeof v === "number" ? v : parseInt(String(v), 10);
  if (!Number.isFinite(n) || n <= 0) return 0;
  return Math.min(500, Math.max(0, Math.round(n)));
}

export type NotifyPref = "ask" | "always" | "never";

export function getNotifyPref(): NotifyPref {
  try {
    const raw = localStorage.getItem("warp-notify-pref");
    if (raw === "always" || raw === "never" || raw === "ask") return raw;
  } catch {}
  return "ask";
}

export function setNotifyPref(pref: NotifyPref): void {
  try {
    localStorage.setItem("warp-notify-pref", pref);
  } catch {}
}

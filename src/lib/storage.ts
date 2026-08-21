// Storage helpers with validation — never trust localStorage.
// Corrupt JSON or old schema should not crash the app; we clear the bad key and return fallback.

export type Mode = "copy" | "move" | "sync";
export type Conflict = "overwrite" | "skip";

export type Preset = {
  name: string;
  source: string;
  dest: string;
  mode: Mode;
  conflict: Conflict;
  folderMode: "into" | "merge";
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

const VALID_MODES = new Set(["copy", "move", "sync"]);
const VALID_CONFLICTS = new Set(["overwrite", "skip"]);
const VALID_FOLDER = new Set(["into", "merge"]);

function isValidMode(v: unknown): v is Mode {
  return typeof v === "string" && VALID_MODES.has(v);
}
function isValidConflict(v: unknown): v is Conflict {
  return typeof v === "string" && VALID_CONFLICTS.has(v);
}

export function isValidPreset(v: unknown): v is Preset {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  return (
    typeof o.name === "string" && o.name.trim().length > 0 &&
    typeof o.source === "string" && typeof o.dest === "string" &&
    isValidMode(o.mode) &&
    isValidConflict(o.conflict) &&
    VALID_FOLDER.has(o.folderMode as string) &&
    typeof o.throttle === "number" && Number.isFinite(o.throttle) && o.throttle >= 0 &&
    typeof o.verify === "boolean"
  );
}

export function isValidRecentEntry(v: unknown): v is RecentEntry {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  return (
    typeof o.source === "string" && typeof o.dest === "string" &&
    isValidMode(o.mode) &&
    typeof o.transferred === "number" && Number.isFinite(o.transferred) &&
    typeof o.bytes === "number" && Number.isFinite(o.bytes) &&
    typeof o.duration_ms === "number" && Number.isFinite(o.duration_ms) &&
    typeof o.timestamp === "number" && Number.isFinite(o.timestamp)
  );
}

/** Safe load: returns [] on corrupt/missing, clears bad key. */
export function loadPresets(): Preset[] {
  try {
    const raw = localStorage.getItem("warp-presets");
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) throw new Error("not array");
    const valid = parsed.filter(isValidPreset);
    if (valid.length !== parsed.length) {
      // Persist cleaned list next save; don't clear entirely if some valid.
      if (valid.length === 0 && parsed.length > 0) localStorage.removeItem("warp-presets");
    }
    return valid;
  } catch {
    try { localStorage.removeItem("warp-presets"); } catch {}
    return [];
  }
}

export function loadRecent(): RecentEntry[] {
  try {
    const raw = localStorage.getItem("warp-recent");
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) throw new Error("not array");
    const valid = parsed.filter(isValidRecentEntry);
    if (valid.length !== parsed.length) {
      if (valid.length === 0 && parsed.length > 0) localStorage.removeItem("warp-recent");
    }
    return valid;
  } catch {
    try { localStorage.removeItem("warp-recent"); } catch {}
    return [];
  }
}

export function savePresets(presets: Preset[]): void {
  try { localStorage.setItem("warp-presets", JSON.stringify(presets)); } catch {}
}

export function saveRecentEntries(entries: RecentEntry[]): void {
  try { localStorage.setItem("warp-recent", JSON.stringify(entries)); } catch {}
}

export type QueueJob = {
  id: number;
  source: string;
  dest: string;
  mode: Mode;
  conflict: Conflict;
  folderMode: "into" | "merge";
  throttle: number;
  verify: boolean;
};

export function isValidQueueJob(v: unknown): v is QueueJob {
  if (typeof v !== "object" || v === null) return false;
  const o = v as Record<string, unknown>;
  return (
    typeof o.id === "number" && Number.isFinite(o.id) &&
    typeof o.source === "string" && typeof o.dest === "string" &&
    isValidMode(o.mode) &&
    isValidConflict(o.conflict) &&
    VALID_FOLDER.has(o.folderMode as string) &&
    typeof o.throttle === "number" && Number.isFinite(o.throttle) &&
    typeof o.verify === "boolean"
  );
}

export function loadQueue(): QueueJob[] {
  try {
    const raw = localStorage.getItem("warp-queue");
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) throw new Error("not array");
    const valid = parsed.filter(isValidQueueJob);
    if (valid.length !== parsed.length && valid.length === 0) localStorage.removeItem("warp-queue");
    return valid;
  } catch {
    try { localStorage.removeItem("warp-queue"); } catch {}
    return [];
  }
}

export function saveQueue(queue: QueueJob[]): void {
  try {
    if (queue.length === 0) localStorage.removeItem("warp-queue");
    else localStorage.setItem("warp-queue", JSON.stringify(queue));
  } catch {}
}

export function normalizeThrottle(v: unknown): number {
  const n = typeof v === "number" ? v : parseInt(String(v), 10);
  if (!Number.isFinite(n) || n <= 0) return 0;
  return Math.min(500, Math.max(0, Math.round(n)));
}

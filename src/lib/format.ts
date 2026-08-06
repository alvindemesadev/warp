// Pure display helpers shared by the UI. Kept free of any Svelte/Tauri imports
// so they can be unit-tested in isolation (see format.test.ts).

/** Last path segment, handling both `\` (Windows) and `/` separators. */
export function basename(p: string): string {
  return p.replace(/\\/g, "/").split("/").filter(Boolean).pop() ?? p;
}

/** Compact human-readable byte count: 1.5 GB, 320 MB, 12 KB, 42 B. */
export function fmtBytes(b: number): string {
  if (b >= 1_073_741_824) return `${(b / 1_073_741_824).toFixed(1)} GB`;
  if (b >= 1_048_576) return `${(b / 1_048_576).toFixed(1)} MB`;
  if (b >= 1024) return `${(b / 1024).toFixed(0)} KB`;
  return `${b} B`;
}

/** "1 file" / "2,048 files" (localized number separators). */
export function fmtFiles(n: number): string {
  return n === 1 ? "1 file" : `${n.toLocaleString()} files`;
}

/** Millisecond duration → "320ms", "4.5s", "3m 12s". */
export function fmtDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = ms / 1000;
  if (s < 60) return `${s.toFixed(1)}s`;
  return `${Math.floor(s / 60)}m ${Math.round(s % 60)}s`;
}

/** Seconds remaining → "", "45s left", "12m 5s left", "2h 10m left". */
export function fmtEta(secs: number): string {
  if (secs <= 0) return "";
  if (secs < 60) return `${secs}s left`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m ${secs % 60}s left`;
  return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m left`;
}

/** Timestamp → "just now", "5m ago", "3h ago", "2d ago". */
export function timeAgo(ts: number): string {
  const diff = Date.now() - ts;
  if (diff < 60_000) return "just now";
  if (diff < 3_600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86_400_000) return `${Math.floor(diff / 3_600_000)}h ago`;
  return `${Math.floor(diff / 86_400_000)}d ago`;
}

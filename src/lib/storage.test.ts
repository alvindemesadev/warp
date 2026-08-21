import { describe, it, expect, beforeEach, vi } from "vitest";
import { isValidPreset, isValidRecentEntry, normalizeThrottle } from "./storage";

// Mock localStorage for node env
class MemoryStorage {
  private m = new Map<string, string>();
  getItem(k: string) { return this.m.get(k) ?? null; }
  setItem(k: string, v: string) { this.m.set(k, v); }
  removeItem(k: string) { this.m.delete(k); }
  clear() { this.m.clear(); }
}

describe("isValidPreset", () => {
  it("accepts valid preset", () => {
    expect(isValidPreset({
      name: "Backup Photos",
      source: "C:\\src", dest: "D:\\dst",
      mode: "copy", conflict: "overwrite", folderMode: "into",
      throttle: 0, verify: false
    })).toBe(true);
  });
  it("rejects bad mode and empty name", () => {
    expect(isValidPreset({ name: "", source: "a", dest: "b", mode: "copy", conflict: "overwrite", folderMode: "into", throttle: 0, verify: false })).toBe(false);
    expect(isValidPreset({ name: "x", source: "a", dest: "b", mode: "bad", conflict: "overwrite", folderMode: "into", throttle: 0, verify: false })).toBe(false);
  });
  it("rejects invalid throttle/verify types", () => {
    expect(isValidPreset({ name: "x", source: "a", dest: "b", mode: "copy", conflict: "skip", folderMode: "merge", throttle: NaN, verify: false })).toBe(false);
    expect(isValidPreset({ name: "x", source: "a", dest: "b", mode: "copy", conflict: "skip", folderMode: "merge", throttle: 5, verify: "yes" as any })).toBe(false);
  });
  it("rejects missing fields", () => {
    expect(isValidPreset({ name: "x" } as any)).toBe(false);
    expect(isValidPreset(null)).toBe(false);
  });
});

describe("isValidRecentEntry", () => {
  it("accepts valid recent", () => {
    expect(isValidRecentEntry({ source: "a", dest: "b", mode: "sync", transferred: 3, bytes: 1024, duration_ms: 100, timestamp: Date.now() })).toBe(true);
  });
  it("rejects invalid numbers", () => {
    expect(isValidRecentEntry({ source: "a", dest: "b", mode: "copy", transferred: NaN, bytes: 0, duration_ms: 0, timestamp: 0 })).toBe(false);
    expect(isValidRecentEntry({ source: "a", dest: "b", mode: "copy", transferred: 1, bytes: 0, duration_ms: 0, timestamp: "now" as any })).toBe(false);
  });
});

describe("normalizeThrottle", () => {
  it("clamps", () => {
    expect(normalizeThrottle(0)).toBe(0);
    expect(normalizeThrottle(50)).toBe(50);
    expect(normalizeThrottle(999)).toBe(500);
    expect(normalizeThrottle(-5)).toBe(0);
    expect(normalizeThrottle(NaN)).toBe(0);
    expect(normalizeThrottle("25" as any)).toBe(25);
  });
});

describe("loadPresets/loadRecent with mock storage", () => {
  beforeEach(() => {
    (globalThis as any).localStorage = new MemoryStorage();
  });
  it("returns [] on corrupt JSON and clears key", async () => {
    const { loadPresets } = await import("./storage");
    localStorage.setItem("warp-presets", "not json");
    expect(loadPresets()).toEqual([]);
    expect(localStorage.getItem("warp-presets")).toBeNull();
  });
  it("filters invalid entries", async () => {
    const { loadPresets, loadRecent } = await import("./storage");
    localStorage.setItem("warp-presets", JSON.stringify([
      { name: "good", source: "a", dest: "b", mode: "copy", conflict: "overwrite", folderMode: "into", throttle: 0, verify: false },
      { name: "", source: "a", dest: "b", mode: "copy", conflict: "overwrite", folderMode: "into", throttle: 0, verify: false },
    ]));
    expect(loadPresets().length).toBe(1);
    localStorage.setItem("warp-recent", JSON.stringify([
      { source: "a", dest: "b", mode: "copy", transferred: 1, bytes: 10, duration_ms: 5, timestamp: 123 },
      { source: "a", dest: "b", mode: "bad", transferred: 1, bytes: 10, duration_ms: 5, timestamp: 123 },
    ]));
    expect(loadRecent().length).toBe(1);
  });
});

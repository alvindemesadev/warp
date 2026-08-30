import { describe, it, expect, beforeEach } from "vitest";
import { isValidRecentEntry, normalizeThrottle } from "./storage";

// Mock localStorage for node env
class MemoryStorage {
  private m = new Map<string, string>();
  getItem(k: string) {
    return this.m.get(k) ?? null;
  }
  setItem(k: string, v: string) {
    this.m.set(k, v);
  }
  removeItem(k: string) {
    this.m.delete(k);
  }
  clear() {
    this.m.clear();
  }
}

describe("isValidRecentEntry", () => {
  it("accepts valid recent", () => {
    expect(
      isValidRecentEntry({
        source: "a",
        dest: "b",
        mode: "sync",
        transferred: 3,
        bytes: 1024,
        duration_ms: 100,
        timestamp: Date.now(),
      }),
    ).toBe(true);
  });
  it("rejects invalid numbers", () => {
    expect(
      isValidRecentEntry({
        source: "a",
        dest: "b",
        mode: "copy",
        transferred: NaN,
        bytes: 0,
        duration_ms: 0,
        timestamp: 0,
      }),
    ).toBe(false);
    expect(
      isValidRecentEntry({
        source: "a",
        dest: "b",
        mode: "copy",
        transferred: 1,
        bytes: 0,
        duration_ms: 0,
        timestamp: "now" as any,
      }),
    ).toBe(false);
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

describe("loadRecent with mock storage", () => {
  beforeEach(() => {
    (globalThis as any).localStorage = new MemoryStorage();
  });
  it("returns [] on corrupt JSON and clears key", async () => {
    const { loadRecent } = await import("./storage");
    localStorage.setItem("warp-recent", "not json");
    expect(loadRecent()).toEqual([]);
    expect(localStorage.getItem("warp-recent")).toBeNull();
  });
  it("filters invalid entries", async () => {
    const { loadRecent } = await import("./storage");
    localStorage.setItem(
      "warp-recent",
      JSON.stringify([
        {
          source: "a",
          dest: "b",
          mode: "copy",
          transferred: 1,
          bytes: 10,
          duration_ms: 5,
          timestamp: 123,
        },
        {
          source: "a",
          dest: "b",
          mode: "bad",
          transferred: 1,
          bytes: 10,
          duration_ms: 5,
          timestamp: 123,
        },
      ]),
    );
    expect(loadRecent().length).toBe(1);
  });

  it("handles versioned format {v,data} and legacy []", async () => {
    const { loadRecent, saveRecentEntries } = await import("./storage");
    const good = {
      source: "a",
      dest: "b",
      mode: "copy",
      transferred: 1,
      bytes: 10,
      duration_ms: 5,
      timestamp: 123,
    };
    saveRecentEntries([good as any]);
    const raw = localStorage.getItem("warp-recent");
    expect(raw).toContain('"v":1');
    expect(loadRecent().length).toBe(1);
    localStorage.setItem("warp-recent", JSON.stringify([good]));
    expect(loadRecent().length).toBe(1);
  });

  it("ignores __proto__ pollution", async () => {
    const { loadRecent } = await import("./storage");
    const payload = JSON.stringify([
      {
        source: "a",
        dest: "b",
        mode: "copy",
        transferred: 1,
        bytes: 10,
        duration_ms: 5,
        timestamp: 123,
        __proto__: { polluted: true },
      },
    ]);
    localStorage.setItem("warp-recent", payload);
    expect(loadRecent().length).toBe(1);
    expect((Object.prototype as any).polluted).toBeUndefined();
  });

  it("handles quota exceeded on save", async () => {
    const { saveRecentEntries } = await import("./storage");
    const orig = (globalThis as any).localStorage.setItem;
    (globalThis as any).localStorage.setItem = () => {
      throw new DOMException("QuotaExceededError");
    };
    expect(() =>
      saveRecentEntries([
        {
          source: "a",
          dest: "b",
          mode: "copy",
          transferred: 1,
          bytes: 10,
          duration_ms: 5,
          timestamp: 123,
        } as any,
      ]),
    ).not.toThrow();
    (globalThis as any).localStorage.setItem = orig;
  });
});

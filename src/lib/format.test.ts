import { describe, expect, it } from "vitest";
import { basename, fmtBytes, fmtDuration, fmtEta, fmtFiles, timeAgo } from "./format";

describe("basename", () => {
  it("handles Windows and Unix separators", () => {
    expect(basename("C:\\folder\\file.txt")).toBe("file.txt");
    expect(basename("/usr/local/bin/tool")).toBe("tool");
  });

  it("handles trailing separators and bare names", () => {
    expect(basename("C:\\folder\\")).toBe("folder");
    expect(basename("single")).toBe("single");
  });

  it("handles long-path prefix", () => {
    expect(basename("\\\\?\\C:\\very\\long\\file.txt")).toBe("file.txt");
    expect(basename("\\\\?\\UNC\\server\\share\\a.txt")).toBe("a.txt");
  });
});

describe("fmtBytes", () => {
  it("scales units", () => {
    expect(fmtBytes(42)).toBe("42 B");
    expect(fmtBytes(2048)).toBe("2 KB");
    expect(fmtBytes(5 * 1_048_576)).toBe("5.0 MB");
    expect(fmtBytes(3 * 1_073_741_824)).toBe("3.0 GB");
  });

  it("handles boundaries", () => {
    expect(fmtBytes(0)).toBe("0 B");
    expect(fmtBytes(1023)).toBe("1023 B");
    expect(fmtBytes(1024)).toBe("1 KB");
    expect(fmtBytes(1_073_741_823)).toBe("1024.0 MB");
    expect(fmtBytes(1_073_741_824)).toBe("1.0 GB");
  });
});

describe("fmtFiles", () => {
  it("singularizes", () => {
    expect(fmtFiles(1)).toBe("1 file");
    expect(fmtFiles(2048)).toBe("2,048 files");
  });
});

describe("fmtDuration", () => {
  it("shows sub-second durations", () => {
    expect(fmtDuration(320)).toBe("320ms");
    expect(fmtDuration(0)).toBe("0ms");
  });

  it("formats seconds and minutes", () => {
    expect(fmtDuration(4500)).toBe("4.5s");
    expect(fmtDuration(192_000)).toBe("3m 12s");
  });

  it("handles 60s exact and large", () => {
    expect(fmtDuration(60_000)).toBe("1m 0s");
    expect(fmtDuration(3_600_000)).toBe("60m 0s");
  });
});

describe("fmtEta", () => {
  it("returns empty for no ETA", () => {
    expect(fmtEta(0)).toBe("");
    expect(fmtEta(-5)).toBe("");
  });

  it("formats seconds, minutes, and hours", () => {
    expect(fmtEta(45)).toBe("45s left");
    expect(fmtEta(725)).toBe("12m 5s left");
    expect(fmtEta(7800)).toBe("2h 10m left");
  });

  it("handles 3599/3600 boundaries", () => {
    expect(fmtEta(3599)).toBe("59m 59s left");
    expect(fmtEta(3600)).toBe("1h 0m left");
    expect(fmtEta(3601)).toBe("1h 0m left");
  });
});

describe("timeAgo", () => {
  it("formats relative timestamps", () => {
    const now = Date.now();
    expect(timeAgo(now - 30_000)).toBe("just now");
    expect(timeAgo(now - 5 * 60_000)).toBe("5m ago");
    expect(timeAgo(now - 3 * 3_600_000)).toBe("3h ago");
    expect(timeAgo(now - 2 * 86_400_000)).toBe("2d ago");
  });
});

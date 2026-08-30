import { describe, it, expect } from "vitest";
import {
  getOverlappingPath,
  isCrossDriveMove,
  isMergeSyncDanger,
  canStartTransfer,
  getStartLabel,
} from "./validation";
import type { PathInfo } from "$lib/types";

function info(drive: string, isFile = false): PathInfo {
  return { files: 1, bytes: 100, isFile, drive, removable: false };
}

describe("getOverlappingPath", () => {
  it("same folder", () => {
    expect(getOverlappingPath("C:\\a", "C:\\a", "into")).toBeTruthy();
    expect(getOverlappingPath("C:\\a", "C:\\a\\", "into")).toBeTruthy();
    expect(getOverlappingPath("C:\\Photos", "C:\\Backup\\Photos", "into")).toBeNull(); // dest already ends with Photos -> not double-nest
  });
  it("dest inside source (into)", () => {
    expect(getOverlappingPath("C:\\a", "C:\\a\\b", "into")).toMatch(/inside the source/);
    expect(getOverlappingPath("C:\\a", "C:\\a\\Photos", "into")).toMatch(/inside the source/);
    // into adds source basename: source C:\a, dest C:\a -> effective C:\a\a vs C:\a -> inside
    expect(getOverlappingPath("C:\\src", "C:\\src\\dest", "into")).toBeTruthy();
  });
  it("source inside dest", () => {
    expect(getOverlappingPath("C:\\a\\b\\c", "C:\\a", "into")).toMatch(/inside the destination/);
  });
  it("merge vs into", () => {
    // merge: effectiveB = dest as-is, so C:\a vs C:\a\Photos (merge) is inside source, but Photosis not appended
    expect(getOverlappingPath("C:\\a", "C:\\a\\b", "merge")).toMatch(/inside the source/);
    // into with different basename should not trigger same
    expect(getOverlappingPath("C:\\a", "C:\\b", "into")).toBeNull();
  });
  it("case insensitive and trailing slash", () => {
    expect(getOverlappingPath("C:\\A", "c:\\a", "into")).toBeTruthy();
    expect(getOverlappingPath("C:\\a\\", "C:\\a", "into")).toBeTruthy();
    expect(getOverlappingPath("C:\\a", "C:\\b\\", "merge")).toBeNull();
  });
  it("empty paths", () => {
    expect(getOverlappingPath("", "C:\\a", "into")).toBeNull();
    expect(getOverlappingPath("C:\\a", "", "into")).toBeNull();
  });
});

describe("isCrossDriveMove", () => {
  it("true when move across drives", () => {
    expect(isCrossDriveMove("move", info("C:"), info("D:"))).toBe(true);
    expect(isCrossDriveMove("move", info("c:"), info("d:"))).toBe(true);
  });
  it("false otherwise", () => {
    expect(isCrossDriveMove("copy", info("C:"), info("D:"))).toBe(false);
    expect(isCrossDriveMove("move", info("C:"), info("C:"))).toBe(false);
    expect(isCrossDriveMove("move", null, info("D:"))).toBe(false);
    expect(isCrossDriveMove("move", info("C:"), null)).toBe(false);
  });
});

describe("isMergeSyncDanger", () => {
  it("only sync+merge", () => {
    expect(isMergeSyncDanger("sync", "merge")).toBe(true);
    expect(isMergeSyncDanger("sync", "into")).toBe(false);
    expect(isMergeSyncDanger("copy", "merge")).toBe(false);
  });
});

describe("canStartTransfer", () => {
  it("requires both paths and not processing/file/overlap", () => {
    expect(canStartTransfer("a", "b", false, null, null, null)).toBe(true);
    expect(canStartTransfer("", "b", false, null, null, null)).toBe(false);
    expect(canStartTransfer("a", "", false, null, null, null)).toBe(false);
    expect(canStartTransfer("a", "b", true, null, null, null)).toBe(false);
    expect(canStartTransfer("a", "b", false, info("C:", true), null, null)).toBe(false);
    expect(canStartTransfer("a", "b", false, null, info("D:", true), null)).toBe(false);
    expect(canStartTransfer("a", "b", false, null, null, "overlap")).toBe(false);
  });
});

describe("getStartLabel", () => {
  it("priority: overlap > missing > file > mode", () => {
    expect(getStartLabel("a", "b", null, null, "overlap!", "copy")).toBe("Copy Files");
    expect(getStartLabel("", "b", null, null, null, "copy")).toBe("Drop folders");
    expect(getStartLabel("a", "", null, null, null, "move")).toBe("Drop folders");
    expect(getStartLabel("a", "b", info("C:", true), null, null, "copy")).toBe(
      "Source must be a folder, not a file",
    );
    expect(getStartLabel("a", "b", null, info("D:", true), null, "copy")).toBe(
      "Destination must be a folder, not a file",
    );
    expect(getStartLabel("a", "b", null, null, null, "copy")).toBe("Copy Files");
    expect(getStartLabel("a", "b", null, null, null, "move")).toBe("Move Files");
    expect(getStartLabel("a", "b", null, null, null, "sync")).toBe("Sync Files");
  });
});

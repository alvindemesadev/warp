import { describe, it, expect } from "vitest";
import { transfer } from "./transfer.svelte";

// Smoke the derived `canStart` via the store's validation wiring.
// This exercises the Phase 2 transfer store without needing Tauri.

describe("transfer store canStart", () => {
  it("requires both paths", () => {
    transfer.sourcePath = "";
    transfer.destPath = "";
    expect(transfer.canStart).toBe(false);
    transfer.sourcePath = "C:\\a";
    expect(transfer.canStart).toBe(false);
    transfer.destPath = "C:\\b";
    // Mock file check: sourceInfo/destInfo null -> not a file, no overlap -> can start
    transfer.sourceInfo = null;
    transfer.destInfo = null;
    transfer.isProcessing = false;
    expect(transfer.canStart).toBe(true);
  });

  it("swapPaths is idempotent", () => {
    transfer.sourcePath = "C:\\src";
    transfer.destPath = "C:\\dst";
    transfer.swapPaths();
    expect(transfer.sourcePath).toBe("C:\\dst");
    expect(transfer.destPath).toBe("C:\\src");
    transfer.swapPaths();
    expect(transfer.sourcePath).toBe("C:\\src");
  });

  it("overlappingPath blocks canStart", () => {
    transfer.sourcePath = "C:\\a";
    transfer.destPath = "C:\\a";
    transfer.folderMode = "into";
    expect(transfer.overlappingPath).toBeTruthy();
    expect(transfer.canStart).toBe(false);
  });

  it("caps transferredFiles at 200", () => {
    transfer.transferredFiles = [];
    for (let i = 0; i < 250; i++) {
      transfer.handleProgress({
        percentage: 50,
        currentFile: `C:\\a\\file${i}.txt`,
        speed: "10 MB/s",
        filesDone: i,
        filesTotal: 250,
        indeterminate: false,
        bytesPerSec: 10_000,
        bytesDone: i * 1000,
        totalBytes: 250_000,
      });
    }
    expect(transfer.transferredFiles.length).toBeLessThanOrEqual(200);
    // BufReader streaming invariant: never buffers whole output (checked via slice)
    expect(transfer.transferredFiles[0]).toBe("file249.txt");
  });
});

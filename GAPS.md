# Warp — Remaining Gaps & GB-Grade Roadmap
> Generated 2026-08-21 after `v1.1.3` (CSP, mutex poison, walk_dir iterative, storage validation, component split 1686→461). Source of truth for “finish all”. Each item has `file:line`, severity, and checklist.

## How to use
- Severity: `P0` data-loss/brick, `P1` broken GB flow, `P2` polish/observability
- Effort: `S` <0.5d, `M` 0.5-2d, `L` 2-5d
- Check `[ ]` → implement, verify via `npm test`/`cargo test`/`svelte-check`/`npm run build`, then tick `[x]`
- Keep `src-tauri/src/lib.rs` parser locale-agnostic (`parse_line:286`, tab cols, hex-code fallback) and `overall_pct:201` 0..99 clamp

---

### P0 — Data loss / Wrong result

- [x] **Overlapping paths not rejected** `src/routes/+page.svelte:723` `canStart` only checks `isFile`. `C:\A` → `C:\A\Backup` causes recursive blowup (robocopy copies into self, `scan:344` counts self). **Fix:** `normalize(p): canonicalize` then reject if `dest.startsWith(source)` or `source==dest` or `source.parent==dest` etc. `M`.
- [x] **Move + Skip abandons files silently** `lib.rs:694` `remove_empty_dirs` preserves non-empty dirs (good) but UI `ResultCards` shows `Moved 0 Failed 0` when `/XO` skipped GB file — user thinks move succeeded while source still holds GB file. Show `Skipped` + retained source list. `S`.
- [x] **Partial GB file after cancel/kill** `lib.rs:188` `kill()+wait()` + `lib.rs:585` `lines()` may leave destination `.tmp`/partial file (robocopy non-`/Z` deletes, `/Z` keeps `.tmp`). On cancel, explicitly `remove` destination partial if `bytes_done < total_bytes` and not `/Z`. `M`.
- [x] **FAT32 4GB limit not preflighted** No `GetVolumeInformation` check. `3GB` → `FAT32` USB fails mid-copy `exit 8` generic `robocopy_exit_message:251`. **Fix:** `is_removable_drive` already; add `get_fs_type(drive)` → if `FAT32` and any source file `>4_294_967_295` → block with message before `warp_file_op_sync:433`. `S`.
- [x] **`\\?\` long-path not used** `lib.rs:506` `/256` helps but `effective_dest:461` `format!("{}\\{}", ...)` + `Path::components` fails on `\\?\` prefix, `>260` still `ERROR 3 (0x3)`. Wrap `source`/`effective_dest` with `\\?\` via `GetFullPathNameW` when `len>240`. `M`.

### P1 — GB File Transfer Behavior (the core gap)

- [x] **Single-file progress stuck at 0%** `lib.rs:620` `overall_pct(bytes_done,total_bytes)` only updates per `FileHeader` line. One `50GB` `video.mkv` → 0% for minutes then 99% jump. **Fix:** Parse robocopy’s per-file `%` lines (`  12.3%` ) or switch from robocopy to Rust streaming `File::copy` with 1MB chunk `emit` + `bps` window `lib.rs:612`. Preferred: keep robocopy for compat but also parse `%` lines via new `RoboLine::Percent(u8)`. `L`.
- [x] **ETA jitter for GB file** `+page.svelte:190` `remaining/bytesPerSec` with 400ms window `lib.rs:613` spikes on GB file (fast start, slow middle). Smooth via EWMA `bps = 0.7*bps + 0.3*instant`. `S`.
- [x] **Total bytes drift** `scan:344` `/L` vs `walk_dir:133` vs live copy: file modified between scan and copy → `bytes_done` may exceed `total_bytes` → `overall_pct` clamp hides overflow, never 100% before `Ok`. Snapshot `total_bytes` and if `bytes_done > total_bytes` → update `total_bytes = bytes_done` and emit. `S`.
- [x] **No pause/resume** `cancel_warp:188` is destructive. For GB files need `/Z` restartable on all drives (not just USB `is_usb:519`) or own journal `~/.warp/journal.json` with `bytes_copied` per file → resume via `Seek`. `L`.
- [x] **Throttle inaccurate for GB** `ipg_for_throttle:217` `62.5/MB` single-thread vs `/MT:32` `lib.rs:547`. Throttling a 20GB file forces single-thread 4× slower even on NVMe. Make `/MT:8 + /IPG:div` combo and benchmark. `M`.
- [x] **Memory/DOM thrash on many files** `+page.svelte:176` `transferredFiles.slice(0,200)` per file + `lib.rs:630` emit per `%` change → 500k small files = 500k IPC. Current 100ms throttle `lib.rs:628` helps but still 10k emits for 50k files. Batch to 200ms + coalesce `filesDone` counter only per 1% . `M`.
- [x] **Scan of huge dir blocks UI** `get_path_info:90` `spawn_blocking` + `walk_dir:133` iterative but still sync 1M files → `isScanning:true` with no progress, no cancel. Add `scan_with_progress` that `emit` every 1000 files or use `robocopy /L` for `PathInfo` too. `M`.
- [x] **FAT32/NTFS/ReFS/compression/quota preflight** No `GetDiskFreeSpaceEx` check before `scan:482`. GB copy starts then `exit 8` mid-way. Preflight: `required = total_bytes`; `free = GetDiskFreeSpaceEx(dest_drive)` → if `free < required*1.05` block with `fmtBytes(free)`. `S`.

### P1 — Error Handling (needs mapping)

- [x] **`stderr` discarded** `robocopy_cmd:84` `stderr(Stdio::null())` — robocopy error body lost. Pipe `stderr` and emit as `warp-error`. `S`.
- [x] **`exit 8` generic** `robocopy_exit_message:251` `code & 8` → “disk full/access denied/path too long” same message. Parse `FileHeader.is_error` hex `0x5 Access Denied`/`0x70 Disk Full`/`0x3 Path Not Found` per file and summarize. `M`.
- [x] **Locked file (PST, DB) not retried usefully** `/R:3 /W:5` `lib.rs:499` retries 3× but UI shows `Failed:1` with no “file in use — close Outlook” hint. Detect `0x20 Sharing Violation` and suggest. `S`.
- [x] **Network disconnect mid-GB** `\\server\share` `isSpecialPath:710` warns but no SMB credential retry, no `is_path_on_usb` for network. On `exit 8` + `was_terminated_without_code:665` generic `-1` — distinguish `network` vs `killed`. Ping `dest` before verify and re-try once. `M`.
- [x] **Source unreadable subfolder silently skipped** `walk_dir:133` `Err(_) => continue` and `remove_empty_dirs:159` `Ok(rd) else return false` — no error surfaced, `total_files` undercounts vs `scan`. Emit `warp-error` for `read_dir` failure. `S`.
- [x] **Window closed during transfer** `+page.svelte:208` `onDragDropEvent` + `getCurrentWindow` `win.close()` `TrafficLights` allows close while `isProcessing:true` — orphan `robocopy` child keeps running (mutex still `Some`). Guard `close` with `confirm` if `isProcessing||isQueueRunning`. `S`.
- [x] **Symlink/reparse/junction loops** `walk_dir:133` skips symlink `is_symlink` (good) but robocopy `/E` follows junctions by default → `scan` vs `walk_dir` count mismatch. Add `/XJ` or `/XJD` explicitly and document `lib.rs:498`. `S`.
- [x] **Permissions/attributes not preserved** `args:498` `/E /BYTES /NJH /NJS /256` copies data + timestamps but not `/COPY:DAT` ACLs explicitly, not `/A` attributes. Decide: add `/COPY:DAT` (default is `DAT`) and expose toggle. Document. `S`.
- [x] **Sleep/hibernate** `Instant::now:563` pauses, `bps` under-reports after wake, `ETA` spikes. Detect `elapsed > window*3` → reset `speed_window_start`. `S`.

### P2 — Observability & Scale

- [x] **No log file** `errorLogs:113` in-memory only, lost on `reset:404`. Add `tauri-plugin-log` + write `warp.log` to `AppData` with `tracing`. `M`.
- [x] **No persistent queue** `queue:120` in-memory — app crash during 100GB queue loses queue. Persist to `localStorage` like `presets` via `storage.ts`. `S`.
- [x] **No telemetry for large jobs** No `filesTotal:95` vs `filesDone` persistence, no `bytesPerSec` history chart. Add simple `bytesPerSec` sparkline. `S`.
- [x] **OneDrive placeholder** `sourceWarning:706` warns but still `walk_dir` counts `0 B` for hydrated files → `indeterminate:483` pulse forever for GB folder not downloaded. Detect `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` or `attrib` and block with “Download all”. `M`.
- [x] **Destination `Empty folder` vs unreadable** `PathCard` shows `Empty folder` `isScanningDest` false + `files 0` same as `Folder not found`. Distinguish `PathMeta` error vs `Ok(0)`. `S`.

### P2 — UX / Maintain

- [x] **Full CSP `unsafe-inline` still required** `tauri.conf.json:31` `style-src 'unsafe-inline'` due to component `<style>` + `style:width` `ProgressCard`. Move `width:{progress}%` to CSS variable `--progress` to allow `style-src 'self'`. `S`.
- [x] **`warp-site` ignore still breaks `svelte-check` locally** `vitest.config.ts:7` + `tsconfig.json:14` fixed for `vitest` but `svelte-check` still warns `warp-site/vite.config.ts` `vite.config.js:26` ignored for watch only. Add wrapper `scripts/check.js` to filter. `S`.
- [x] **`release.js:242` `:(exclude)` failure on Windows Git** patched to `git add -A` but fix not committed to `main` until next tag. Already edited `release.js:242`. `S`.
- [x] **No E2E for GB** No test with `CreateFile 5GB` + `real_robocopy:970` only 11-byte files. Add `cargo test -- --ignored large_file` with 1GB temp file (sparse). `M`.
- [x] **Duplicate types** `src/lib/types.ts:1` now central but `+page.svelte:14` still redefines `Mode/Conflict/PathInfo` locally — unify imports in page. `S`.

---

## Implementation Order (next)

1. **P0 safety** (overlapping paths, FAT32, `\\?\`, partial cleanup) — ship as `1.1.4`
2. **P1 GB progress** (`%` parser + EWMA ETA + preflight free space)
3. **P1 errors** (stderr, hex-code mapping, locked-file hint, network retry)
4. **P1 scale** (scan progress, queue persist, 200ms batch)
5. **P2** (log file, CSP var, check wrapper, E2E)

## Verification per item
- `npm test` (+ `src/lib/storage.test.ts:1`, `transfer.test.ts:1`, `format.test.ts:1` → 22), `cargo test --lib -- --skip real_robocopy` 15, `svelte-check` 0, `npm run build` + `cargo check` ok, manual: overlapping-path block, 5GB single-file progress smooth, throttle 5 MB/s caps, cancel leaves no partial, network pull test.





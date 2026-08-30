# Spec — Safety Net (Undo + Disk Health)

> **Status:** Draft — Windows-only

## What

**Undo for Move/Sync:** If you Move or Sync and realize it was the wrong folder, one click puts it back. Keeps a 7-day diary; no hunting `warp.log`.

**Disk health preflight:** If your destination is a slow SD card or old SMR drive that would crawl on 8 lanes, Warp shows `⚠ Slow drive — using 2 lanes` and automatically uses 2.

## User Stories

1. **Oops Sync:** `Sync` `D:\Photos` → `E:\Backup` but `E:\Backup` had `Vacation/` not in source, so Sync deleted it. `ResultCards` shows `Undo` for 7 days → click → `Vacation/` is back.
2. **Slow SD:** Copy `C:\Videos` (100 GB) → `F:\` (cheap SD). Warp tests `F:\` with 64 KB, sees 4 MB/s → `⚠ Slow SD — using 2 lanes for best speed`, finishes faster than 8-lane would.

## How (boring)

### Undo

- **Before** `Move`/`Sync` deletes, `remove_empty_dirs` and `Sync`'s `Extra` deletes are logged to `%TEMP%\.warp-trash\<date>.json`:
  ```json
  { "ts": 171..., "mode": "sync", "src": "C:\\Photos", "dst": "E:\\Backup", "deleted": ["E:\\Backup\\Vacation\\a.jpg"] }
  ```
- Hashes for privacy? No — need real paths to restore. File is `%TEMP%` user-only, 7-day prune on launch `remove_dir_all` for `.warp-trash\*.json` older than 7 days.
- **Undo:** Tauri command `undo_last` reads most recent trash log, does `robocopy dst src /E` reverse (or `move` back via `fs::rename` + `remove_empty_dirs` reverse), then deletes the log entry. `ResultCards` shows `Undo` button for 7 days if `summary.failed==0 && (mode=="move" || mode=="sync") && lastTrashExists`.
- If trash log >50 MB or >1000 entries, truncate oldest.

### Disk health

- **Test:** `health_check(dest)` in `preflight.rs` — `std::fs::OpenOptions::new().write(true).create(true).open(dest.join(".warp-health"))` then `write_all([0; 64*1024])` timed, `bytes / secs` → MB/s. Delete file after. If <10 MB/s and `workers==0 (Auto)` → force `2` and emit `warp-health-warning` event with `slow: true`.
- **Policy:** Already `resolve_workers_for` caps USB at 2, but this adds SMR/SD detection for non-USB slow drives (e.g., SMR HDD). Keeps total thread budget ≈ `/MT:32` (2 workers * 8 MT = 16, vs 6*8=48).
- **UI:** `OptionsPanel` shows `⚠ Slow drive — using 2 lanes` when `slow` event received; `ProgressCard` pool line shows `2 workers` already.

## Done When

- Sync deletes `E:\Backup\Vacation\a.jpg` → `ResultCards` shows `Undo` → click → `a.jpg` back in `E:\Backup\Vacation\`, `warp.log` has `undo` JSON
- Copy to `F:\` (4 MB/s) → `⚠ Slow SD` chip appears, `cargo test` shows `resolve_workers_for` with `slow=true` → 2, no crash on `health_check` failure (fallback to Auto)

## Out of Scope

- Cloud recycle bin, versioned trash, per-file undo
- SMART health, wear leveling

## Effort

3 days — 1 day trash log + Undo command, 1 day health check + auto 2-lane, 1 day UI + tests + prune

## References

- `src-tauri/src/lib.rs:376` `remove_empty_dirs`
- `src-tauri/src/pool.rs:334` `resolve_workers_for`
- `docs/FEATURES-NEXT.md:2`

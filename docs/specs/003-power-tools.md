# Spec — Power Tools (Filter + Compare)

> **Status:** Draft — Windows-only, no bloat

## What

**Filter:** One box to skip junk — `*.tmp; node_modules; .git` → `/XF *.tmp /XD node_modules .git`. Copy only what you need.

**Compare:** Dry-run button that shows **"3,204 files · 12.4 GB will copy, 45 will be skipped"** without copying anything.

## User Stories

1. **Dev:** Copy `C:\project` → `D:\backup` but skip `node_modules` and `*.log` — type `node_modules; *.log` → copy skips them, queue saves the filter.
2. **Check:** Before a big Sync, click **Compare** → see "12.4 GB to copy, 45 skipped, 0 extra to delete" — no surprises, no wait.

## How (boring)

### Filter

- `OptionsPanel` → `Exclude: [__________]` placeholder `*.tmp; node_modules; .git`, stored in `transfer.filter: string`, persisted in `Preset`/`QueueJob.filter?: string`
- Parse `;` or `,` → `["*.tmp","node_modules"]` trimmed, empty removed, max 20 entries, each ≤100 chars, no `..` or `\` escape
- Append to robocopy: `file patterns` (`*.tmp`) → `/XF`, `dir patterns` (no dot, no `*`? Actually `node_modules` is dir) → heuristic: if pattern contains `.` or `*` → `/XF`, else `/XD`. Simpler: put all in both `/XF` and `/XD`? No, robocopy `/XF` is files, `/XD` is dirs. For v1, split: `*.*` or `*.tmp` → `/XF`, else → `/XD` + also `/XF`? Safer to put every pattern in both `/XF` and `/XD` — robocopy will just ignore non-matching type, no harm. Max 20, so args stay short.
- Both engines: `warp_file_op_sync` and `pool::shard_args` add them

### Compare

- Button `Compare` next to `Copy Files` → calls `scan(source, effectiveDest, mode)` (`/L /E /BYTES`) already exists (`lib.rs:663` `verify_transfer` uses same), but we expose a new `compare` Tauri command that returns `(filesToCopy, bytesToCopy, skipped, extra)` by parsing `FileHeader`/`Extra`/`Same`
- Frontend `CompareModal` shows `fmtFiles` + `fmtBytes` + `skipped` + `extra` (for Sync) + `verify` note
- No copy, no side effects, re-uses existing scan logic + 100 ms debounce

## Done When

- Filter `*.tmp` → `Copy` → `dest` has no `*.tmp` files (`/XF` worked), `QueueJob` saved filter restores on load
- Compare `C:\a` (3,204 files) → modal `3,204 files · 12.4 GB will copy, 45 skipped` matches `robocopy /L` stdout

## Out of Scope

- Regex, include-only, per-file size filter
- Background compare

## Effort

1 day each — 1 day filter (`/XF`/`/XD` + UI + persist), 1 day compare (`compare` command + modal) + tests

## References

- `src-tauri/src/lib.rs:663` `verify_transfer` (re-uses scan)
- `src-tauri/src/pool.rs:265` `shard_args`
- `docs/FEATURES-NEXT.md:3`

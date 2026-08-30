# Warp — High-Speed File Transfer: Technical Whitepaper

**Version:** 1.2.4  
**Author:** Alvin  
**Date:** August 2026  
**Repo:** https://github.com/alvindemesadev/warp  
**License:** MIT

---

## Abstract

Warp is a minimal Windows desktop app that wraps the built-in `robocopy` engine in a modern Tauri + Rust + Svelte interface. It adds accurate byte-level progress, live speed/ETA, parallel sharded transfers, pause/resume, verify, throttling, and safe path handling — without reimplementing filesystem copying. This paper explains _how_ Warp works, _why_ those design choices were made, and where the limits are.

> Tagline: _We split your folders into 8 lanes, so it copies in parallel. One lane would crawl, eight just flies._

---

## 1. Introduction & Goals

Typical file copies on Windows (Explorer, `copy`/`xcopy`) lack:

- true total-bytes progress (they count files, not bytes)
- live throughput and ETA
- safe pause/resume and cancel without orphans
- multi-folder parallelism
- structural verification

Warp's goals:

1. **Stay native and tiny.** <10 MB installer vs ~150 MB for Electron.
2. **Reuse a battle-tested engine** instead of rewriting copy loops.
3. **Be honest** about progress, speed, and failures.
4. **Be safe** — never recurse into itself, never silently delete, never orphan `robocopy`.

Non-goals (v1.x): macOS/Linux support, hash-based verification, admin-elevated copies.

---

## 2. Why Robocopy?

`robocopy` ships with every Windows since Vista. Properties Warp relies on:

| Capability          | Flag used              | Why it matters                 |
| ------------------- | ---------------------- | ------------------------------ |
| List-only dry run   | `/L`                   | Scan pass without copying      |
| Byte-accurate sizes | `/BYTES`               | Progress from bytes, not files |
| Multi-threaded copy | `/MT:32` (or 4–8)      | Throughput                     |
| Long paths          | `/256` + `\\?\` prefix | Bypass `MAX_PATH` 260          |
| Junction exclusion  | `/XJ /XJD`             | Prevent symlink cycles         |
| Inter-packet gap    | `/IPG:n`               | Bandwidth throttling           |
| Restartable mode    | `/Z`                   | USB / large-file resilience    |
| Mirror              | `/MIR`                 | Sync mode                      |
| Skip existing       | `/XO /XN`              | Conflict = skip                |

`robocopy` has 20+ years of hardening for locked files, retries (`/R:3 /W:5`), and accurate exit codes (bitmask `0–16`).

**Alternative considered:** custom Rust `std::fs::copy` loop — rejected. Would need to re-solve buffering, long paths, ACLs, retries, and would be slower to harden.

---

## 3. Why Tauri + Svelte + Rust?

| Layer    | Choice                                     | Rationale                                       |
| -------- | ------------------------------------------ | ----------------------------------------------- |
| Shell    | Tauri 2 (`src-tauri/tauri.conf.json:4`)    | Native WebView2, <5 MB overhead, Rust IPC       |
| Frontend | SvelteKit 2 + Svelte 5 (`package.json:29`) | Compiler, no VDOM, <50 KB JS                    |
| Styling  | Custom CSS tokens `src/app.css`            | No framework, single theme source               |
| Backend  | Rust 2021 (`Cargo.toml:6`)                 | Handles `Child` processes, parsing, FS walks    |
| IPC      | `invoke` + `emit` events                   | `warp-progress`, `warp-error`, `warp-verifying` |

**Why not Electron?** `README.md:188` — Electron bundles Chromium (~150 MB). Warp is 4.7 MB setup / 6.3 MB MSI.

Frontend is a single page `src/routes/+page.svelte:1` — Svelte 5 runes (`$state`, `$derived`) drive progress, queue, presets, and modals.

---

## 4. System Architecture

```
┌─────────────────────────────────────────────────────┐
│  Svelte UI (+page.svelte) — drag-drop, pickers,     │
│  ModePicker, OptionsPanel, ProgressCard, QueueList   │
│  invoke("warp_file_op") ──►  Rust (lib.rs)          │
│  listen("warp-progress" / "warp-error") ◄── emit    │
└──────────────────────┬──────────────────────────────┘
                       │ Tauri IPC
┌──────────────────────▼──────────────────────────────┐
│  Rust backend (src-tauri/src/lib.rs)                │
│  • TransferControl — child registry + pause/cancel  │
│  • run_transfer — preflights → engine selection    │
│  • warp_file_op_sync — sequential engine           │
│  • transfer_parallel — parallel engine (pool.rs)   │
│  • shards.rs — partitioner                         │
│  • parse_line — locale-robust parser               │
│  • verify_transfer — /L re-compare                 │
└──────────────┬──────────────────┬───────────────────┘
               │ spawn            │ spawn N
        robocopy.exe (1)    robocopy.exe (N shards)
               │                  │
               ▼                  ▼
        NTFS / USB / Network destination
```

**Key invariants:**

- All long-running work runs on `spawn_blocking` (`lib.rs:714`) — never blocks Tokio IPC.
- Children are tracked in `TransferControl.children: Mutex<HashMap<u64, Child>>` (`lib.rs:25`) — cancel/close kills _all_.
- Frontend and backend share types `WarpProgress` / `WarpSummary` (`lib.rs:90,111`) via serde `camelCase`.

---

## 5. Transfer Pipeline (Sequential Engine)

### 5.1 Preflights (both engines)

Before any byte moves (`run_transfer` `lib.rs:872`):

1. **Resolve effective destination** `resolve_effective_dest` (`lib.rs:732`) — `into` appends source basename unless dest already ends with it (prevents `Photos/Photos`).
2. **Overlap guard** `check_overlap` (`lib.rs:761`) — blocks same-folder, dest-inside-source, source-inside-dest.
3. **Network preflight** `check_network_dest` (`lib.rs:785`) — `\\server\share` reachability via `metadata`.
4. **FAT32 preflight** `check_fat32_source` (`lib.rs:809`) — via `GetVolumeInformationW`; rejects >4 GiB file if dest is FAT32 (`max_file_size` `lib.rs:230` caps early).
5. **Scan** `scan` (`lib.rs:633`) — `robocopy /L /E /BYTES /NJH /NJS /NP` → `(total_bytes, total_files)`. Parser counts only non-error `FileHeader` rows.
6. **Free-space** `ensure_free_space` (`lib.rs:824`) — needs `total_bytes + 100 MB` via `GetDiskFreeSpaceExW` (`lib.rs:193`).

Empty or zero-byte-only jobs → `indeterminate = total_bytes == 0` (`lib.rs:957`) → UI pulses instead of %.

### 5.2 Spawn (Sequential)

Built in `warp_file_op_sync` (`lib.rs:944`):

```
args = [source, effective_dest, /E /NP /R:3 /W:5 /BYTES /NJH /NJS /256 /XJ /XJD /COPY:DAT]
     + mode:    /MOVE or /MIR or none
     + conflict skip: /XO /XN
     + throttle / USB / large-file MT/Z logic
```

`/MT` policy (`lib.rs:998`):

- `throttle >= 25 MB/s`: `/IPG:half` + `/MT:4` (NVMe-friendly cap)
- `throttle < 25`: `/IPG:n` single-thread (precise)
- USB (removable detection `is_removable_drive` `lib.rs:144` via `GetDriveTypeW`): `/MT:4 /Z`
- `>1 GiB` on internal: `/MT:8 /Z`
- default: `/MT:32`

`CREATE_NO_WINDOW` (`lib.rs:15,281`) hides the console. Stdout/stderr piped.

### 5.3 Progress Tracking

Scan total drives the denominator: `overall_pct = done/total *100 clamped 0..99` (`lib.rs:447`).

Streaming `BufReader::lines` (`lib.rs:1101`) parses each line:

- `FileHeader` (5-tab column, see §7) → one file. For `size >= 10 MB` transfers that aren't `Same`/error and not indeterminate, the file is **deferred** — bytes credited via `Percent` lines (§5.4) for smooth large-file progress (`lib.rs:1131,1084`).
- Small files: `bytes_done += size` immediately; expanding `total_bytes` if drifted (`lib.rs:1157`) — files mutated between scan and copy.
- Speed: EWMA over 400 ms window (`lib.rs:1163`): `bps = window_bytes / 0.4s`, smoothed `0.7*old + 0.3*new`, formatted `fmt_speed` (`lib.rs:452`) → `MB/s, KB/s`.
- ETA: frontend (`+page.svelte:115`) `remaining / bytesPerSec`.
- Throttle last emit: emit if `%` changed **or** `>=150 ms` elapsed (`lib.rs:1188`).

Sequential large-file deferral details (`lib.rs:1084,1134`): pending = `(size, before_bytes, name, last_percent)`; `Percent` lines credit `size * p/100`; regression ignored.

Emitter: `window.emit("warp-progress", WarpProgress{...})` (`lib.rs:1193`) — frontend throttles to 150 ms too.

---

## 6. Parallel Engine

### 6.1 When it runs

Cheap gate `should_attempt_parallel` (`lib.rs:852`):

- Hard **no** if `mode == "sync"` (`/MIR` concurrent deletes unsafe) or `throttle > 0` (`/IPG` per-process).
- If `workers > 1` explicit: **yes** (bypasses size heuristics, still respects hard gates).
- Else Auto: needs `>=400 files` **and** `>=256 MiB` **and** `>=2` top-level dirs (`shards::top_level_dir_count`).

Cheaper wins: quick `dir_stats` walk (`lib.rs:901`) doubles as metadata-cache warmup for `shards::partition`.

Hard gate re-checked inside `pool::resolve_workers_for` (`pool.rs:312`) with shard count.

### 6.2 Partitioning (`shards.rs`)

Invariant: **every file belongs to exactly one shard** — no overlapping source/dest.

Algorithm `partition` (`shards.rs:34`) → `split_dir` (`shards.rs:48`):

- `list_children` (`shards.rs:120`) skips symlinks, sorts by name.
- Loose files at any level → one `root_only=true` shard with `/LEV:1` (`shards.rs:59`).
- Each immediate child dir → one shard (`/E` recursive) **unless** dominant.
- Dominant check `should_split` (`shards.rs:93`): `bytes >=512 MiB` **and** `bytes >=40%` of total **and** `>=2` subdirs → recursively `split_dir` at `depth+1` (max depth 2, `MAX_SPLIT_DEPTH`).
- Destinations mirrored via `join_win` (`shards.rs:152`): `dest + "\" + child.name`.

IDs reassigned `1..N` after recursion (`shards.rs:42`).

Coverage/disjointness tested `shards.rs:223,267` — union equals full walk, no file in two shards.

### 6.3 Worker Pool (`pool.rs`)

`resolve_workers_for` (`pool.rs:312`) — correctness first:

- `sync`/`throttle`/`<2` shards → 1
- explicit `requested >1` → `min(requested,8)` (honored)
- Auto: `usb → 2`, `network → 3`, `local → available_parallelism()/2 clamp 2..=6` (e.g., 8-core → 4).

Thread budget: per-shard `/MT` drops to 4–8 (`pool::shard_args` `pool.rs:265`) so total stays ≈ `/MT:32` of sequential.

`shard_args` (`pool.rs:265`): `[src,dst] + /MOVE? + /XO/XN? + /E /NP /R:3 /W:5 /BYTES /NJH/NJS /256 /XJ/XJD /COPY:DAT + /LEV:1? + /MT:mt`.

Coordination (outline, `lib.rs:919` + `pool.rs`):

1. Build `N` children via `robocopy_cmd` + `shard_args`.
2. Bounded worker pool (semaphore) runs up to `W` children concurrently.
3. Shared `Tracker` (`pool.rs:33`, `Mutex<Tracker>`) merges byte deltas with **same** EWMA/throttle math as sequential (single source of truth for speed/%, `pool.rs:85,154`).
4. Coordinator stamps `active_workers / shards_done / shards_total` before emit.
5. Per-shard `LocalCounters` (`pool.rs:230`) + `ShardOutcome` (`pool.rs:239`) saved — **final summary is sum of outcomes, not live tracker** (tracker is display-only).

### 6.4 Aggregate Progress (Parallel)

`Tracker::ingest` (`pool.rs:154`) mirrors sequential logic:

- `FileHeader`: `files_seen++`, `transferred/skipped/failed`, `note_bytes` immediately (parallel **never defers** large files — concurrent large files would misattribute a single pending slot, comment `pool.rs:44`).
- `Percent`/`Speed` ignored when `defer_large==false` (`pool.rs:189`).
- `note_bytes` (`pool.rs:85`) drifts `total_bytes` upward if observed > scan.
- `revert_bytes` (`pool.rs:222`) undoes a failed shard's bytes before retry.

Emit throttle same `150 ms` / `%` change (`pool.rs:129`).

### 6.5 Retry & Pause

- **Retry** — failed shards collected by `exit_code & 8`; retried sequentially up to twice. `robocopy` skip logic means only missing files recopy. `retried_ok = prev_failed - new_failed` (`pool.rs:343`).
- **Pause** — dispatch gate (`pause_warp` `lib.rs:432`): sets `TransferControl.paused`. Coordinator stops dispatching new shards; in-flight `/LEV:1` shards finish. Resume clears the flag unless cancelled (`lib.rs:434`). Granularity: folder-level, not mid-file (documented `README.md:272`).

---

## 7. Robocopy Parser — Locale Robustness

`parse_line` (`lib.rs:546`) is the most subtle code.

**Problem:** robocopy status words (`New File`, `Same`, `ERROR`) are localized, but columns are not.

**Solution:** key off **tab-delimited column layout**, identical everywhere:

- File rows: `["", status, "", size, path]` — 5+ columns (`raw.split('\t')` `lib.rs:615`, must split `raw` not `trimmed` to keep leading empty col).
- Dir rows: 3 columns — skipped.
- `*EXTRA` rows: `status` starts with `*` — skipped (`lib.rs:620`).
- Size: `cols[3].parse::<u64>` — if parse fails, skip.
- `is_same = status == "Same"` (case-insensitive), `is_error = status == "ERROR"` — best-effort; unrecognized → treated as **regular copy** (safe direction; progress never misses a file).
- Error lines: locale-independent `"<dec> (0x<hex>)"` pair (`lib.rs:576`): scan whitespace tokens for `dec` + `(0x...)` hex; hints like `32 → file in use`, `5 → access denied`, `112 → disk full` (`lib.rs:591`). This catches errors even when `ERROR` word is translated.
- Percent lines: `tok.ends_with('%')` (`lib.rs:564`) — before error/file checks.
- Speed lines: `contains("bytes/sec")` (`lib.rs:553`) — best-effort, but live speed primarily from byte deltas so not critical.

File name extracted via `basename` (`lib.rs:478`) for clean UI; full paths keep `\\?\` long-path form internally.

Tests: `lib.rs` + `pool::tests` + `shards::tests` cover en_US and simulate non-English fallback.

---

## 8. Verify Pass

Optional checkbox (`verify` bool `lib.rs:707`).

`verify_transfer` (`lib.rs:663`):

```
robocopy source effective_dest /L /E /BYTES /NJH /NJS /NP → parse
any FileHeader with !is_same && !is_error → mismatches++
exit code 0 → 0 else max(mismatches,1)
```

- Structural check: existence + size + timestamp (what robocopy compares). **Not a hash.** Documented `README.md:256,269`.
- Non-English safe: if parser misses a localized `Same`, `exit_code !=0` forces `mismatches.max(1)` so verify **never false-passes** (`lib.rs:688`).

---

## 9. Cancel, Pause, and Lifecycle

`TransferControl` (`lib.rs:25`):

- `children: Mutex<HashMap<u64, Child>>`, `cancelled/paused: AtomicBool`.
- `kill_all` (`lib.rs:76`): `cancelled=true`, `drain`, `kill` + `wait` each.
- Sequential registers `SEQ_CHILD_ID=1` (`lib.rs:18`), parallel registers per shard.

Commands:

- `cancel_warp` (`lib.rs:422`): clears `paused`, `kill_all`.
- `pause_warp` (`lib.rs:432`): sets `paused` true; resume only if not cancelled.

Window/app exit handlers also call `kill_all` — no orphan robocopy.

Frontend `cancelTransfer` (`+page.svelte:242`) shows `Cancelling…` but leaves `isProcessing` until the killed `warp_file_op` resolves — prevents overlapping invocations (race comment `+page.svelte:224` + `+page.svelte:243`).

---

## 10. Frontend Details

- **Drag & drop** (`+page.svelte:128`, Tauri `dragDropEnabled` `tauri.conf.json:25`) — `over/drop` events; `PathCard` visuals; rejects files (`sourceInfo.isFile` guard `+page.svelte:396`).
- **Browse** — `plugin-dialog` `open({directory:true})` (`+page.svelte:203`).
- **Overlap warning** (`+page.svelte:382`) mirrors Rust guard, includes effective dest for `into` mode.
- **Cross-drive move warning** (`+page.svelte:394`, `PathInfo.drive` `lib.rs:137`).
- **Throttle** (`lib.rs:470`): `ipg = round(62.5 / MB/s)`, 64 KB blocks → `blocks/sec = MB/s *16`.
- **Queue** (`+page.svelte:270,278`) — persist via `loadQueue/saveQueue`; `Run Queue` sequentializes `warp_file_op` calls; combined summary.
- **Presets / Recent** — persisted 5 recent (`+page.svelte:318`), presets map by name.
- **Notifications** — `plugin-notification` `notifyDone` (`+page.svelte:320`).
- **Updater** (`+page.svelte:340, Updates`) — `plugin-updater` `check/downloadAndInstall`; `tauri.conf.json:59` pubkey + GitHub `latest.json` endpoint.

---

## 11. Safety & Reliability

| Guard                    | Where                                        | Effect                                |
| ------------------------ | -------------------------------------------- | ------------------------------------- |
| Long paths               | `to_long_path` `lib.rs:216`                  | `\\?\C:\` or `\\?\UNC\` if >240 chars |
| Symlink cycles           | `walk_dir` + `split_dir` skip `is_symlink`   | Never follow junctions                |
| Junction copy loops      | `/XJ /XJD`                                   | Robocopy skips them too               |
| Fat32 4 GiB              | `is_fat32_volume` + `max_file_size`          | Block with pretty bytes               |
| Disk full                | `free_bytes_available` `GetDiskFreeSpaceExW` | Block `need +100 MB` headroom         |
| Network offline          | `check_network_dest`                         | Block with share path                 |
| Concurrent mirror delete | hard gate single-process for `sync`          | No clobber                            |
| Inaccurate throttle cap  | hard gate single-process for `throttle`      | `/IPG` stays true                     |

Logging: `log_event` (`lib.rs:261`) appends `[epoch] msg` to `%TEMP%\warp.log` for post-mortem.

Error surfacing: `robocopy_exit_message` (`lib.rs:505`) decodes bitmask; per-file `warp-error` events stream to `errorLogs` → `ResultCards`.

---

## 12. Updates & Distribution

- **Unsigned installers** — NSIS `.exe` (4.7 MB) + MSI (6.3 MB) built `npm run build:win` (`scripts/build.js` finds `vcvars64`, picks `~/.tauri/warp.key`).
- **Signer** — `~/.tauri/warp.key` (private) + `tauri.conf.json:61` pubkey (minisign). `.env` ignored — must be env var. CI secrets `TAURI_SIGNING_PRIVATE_KEY`.
- **Artifacts** — `scripts/updater-manifest.js` → `latest.json`; release `scripts/release.js` bumps versions across both repos (`warp` + `warp-site`), rebuilds, syncs `docs/` + `public/`, tags, pushes.
- **In-app** — `check` on launch + 4s (`+page.svelte:165`); `embedBootstrapper` WebView2 (`tauri.conf.json:51`) for offline-first installs.

---

## 13. Known Limitations (v1.2.2)

_from `README.md:264` verbatim:_

- Windows only (robocopy).
- No admin elevation → `Program Files` fails.
- OneDrive virtual files copy as 0-byte placeholders.
- Verify is structural, not hash.
- Throttle is approximate (`/IPG`, single-thread).
- Pause is folder-granular (active folders finish).
- Parallel off for Sync & throttled by design.
- Non-English: `Same`/`ERROR` classification best-effort (progress/verify still correct via columns + exit code).
- Log at `%TEMP%\warp.log`.

Additional notes:

- Drift between scan and copy (files created during copy) is auto-corrected by expanding `total_bytes`.

---

## 14. Testing

- **Rust unit tests** — `cargo test` in `lib.rs`, `pool.rs:404`, `shards.rs:166`: parser, `Tracker` EWMA/emit, `resolve_workers_for`, `shard_args`, `recovered_from_retry`, partition coverage/dominant split/empty/loose.
- **FS isolation** — `tmp_root` `shards.rs:173` via `std::process::id()` + `remove_dir_all`.
- **Frontend** — `vitest` (`npm test`).

Run: `npm test` then `cargo test --manifest-path src-tauri/Cargo.toml` (or `npm run tauri dev`).

---

## 15. Future Work

- Hash-based verify option (SHA-256 streaming, opt-in for large sets).
- Content-defined chunking for single huge-file parallelism (today only large-dir fan-out).
- rsync backend stub for macOS/Linux (build-tagged).
- Elevation prompt for protected destinations.
- Per-shard `/Z` resume across app restarts.

---

## 16. Acknowledgements

Tauri, Svelte, Robocopy, `windows` crate (`0.58`), `minisign-verify`.

## References

- `src-tauri/src/lib.rs` — commands, parser, preflights, sequential engine
- `src-tauri/src/pool.rs` — Tracker, worker policy, consume_stream
- `src-tauri/src/shards.rs` — partitioner
- `src/routes/+page.svelte` — UI, queue, updater
- `src-tauri/tauri.conf.json` — window, bundle, updater
- `README.md` — user-facing feature table + architecture summary
- `scripts/build.js`, `scripts/release.js`, `scripts/updater-manifest.js`

---

_Warp wraps what Windows already does best, and gets out of the way. — If you found this useful, star the repo and share a transfer screenshot._

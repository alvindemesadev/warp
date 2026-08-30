# 004 — Balanced Sharding (Plan A + Conditional Plan B)

**Status:** Implemented — 2026-08-31 (`shards.rs:34 partition_balanced`, `lib.rs:2115` wired)  
**Related:** `shards.rs:34 partition`, `shards.rs:93 should_split`, `pool.rs:363 resolve_workers_for`, `lib.rs:2115` two-phase Sync

## Summary

Keep **Plan A** (byte-balanced bin-pack of top-level dirs) as default. Enable **Plan B** (split a flat large shard into file-chunks) only when `max_shard > 1.5 × avg_bucket`.

Goal: equalize worker load so `8 workers` means `8 × ~total/8 bytes` in parallel, without paying shard-spawn cost on small/balanced jobs.

## Motivation

Current sharding is **directory-based**:

* 1 shard per immediate child dir (`D:\src\Photos → D:\dst\Photos` `/E`), loose files → 1 `root_only /LEV:1` shard (`shards.rs:58`)
* Only dominant child (`>512 MiB` + `>40% total` + `≥2 subdirs`) is split recursively to depth 2 (`should_split` `shards.rs:93`)

Result for `20 / 60 / 10 / 400 MB` (flat `400 MB` folder with 1000 files, no subdirs):

* Shards = `[20, 60, 10, 400]` → workers = 4 (or 8 but 4 busy) → tail = `400 MB` single-thread time, other workers idle early. Queue `VecDeque<Shard>` `lib.rs:2138` helps only between shards, not inside a shard.

## Proposed: Hybrid

### Plan A — Byte-balanced grouping (always)

1. `dir_stats` every top-level dir (already done in `partition`; reuse metadata cache).
2. Sort dirs descending by `est_bytes`.
3. Greedy bin-pack into `W = resolve_workers_for(...)` buckets: repeatedly put next dir into bucket with smallest `bucket_bytes`.
4. Each bucket → 1 shard (still disjoint, still per-dir). Keeps total shards ≈ `W`, keeps `list_children` traversal cheap.

*Cost:* `O(n log n)` sort, no extra `read_dir` depth.

### Plan B — Split flat monster (conditional)

Trigger: after Plan A, compute `avg_bucket = total_bytes / W` and `max_shard = max(bucket_bytes)`.

If `max_shard > 1.5 × avg_bucket` **and** that max bucket holds a single dir shard (i.e., not already grouped):

1. Walk that dir's files (flat list, skip symlinks, respect `filter`), `stat` sizes.
2. Sort files descending, chunk into `k = ceil(max_shard / avg_bucket)` pieces of `~avg_bucket` bytes (target 128–256 MiB per chunk, clamp `k ≤ 6` to avoid shard explosion).
3. Replace that 1 shard with `k` file-chunk shards (still disjoint by file set, same `dst` prefix + file filter). For `Sync`, file-chunk respects `collect_extra_files` extra handling — chunking is copy-only.

If the monster dir has subdirs, prefer existing `should_split` path first; file-chunk is fallback for flat.

*Cost:* one extra `read_dir` + `stat` for that dir only, plus `k-1` extra `robocopy` spawns.

### Threshold

`1.5×` chosen: `400 / (490/4=122.5) = 3.26×` → triggers B → splits into `4×100 MB` → buckets become `~70 MB` each. Balanced case like `50/55/60/45` → `max 60 / avg 52.5 = 1.14×` → no B, no overhead.

Tune: `1.3×` too eager (splits on mild skew), `2.0×` too lazy (misses `180/90/90/90` where `180` still straggles). `1.5×` + `128–256 MiB` chunk clamp is sweet spot.

## Speed Analysis

| Workload | Plan A | Plan B | Winner |
|----------|--------|--------|--------|
| Many small/medium dirs (6–20 dirs, each 20–100 MB, balanced) | `~total/W` balanced already, 1 scan, `W` spawns | Same buckets, no trigger, same as A | **A** (tie, less code) |
| One flat monster `400 MB` in `490 MB` total, `W=4` | Tail `400` → wall `~400/MBps` | `4×100` → wall `~100/MBps` → **~3–4× faster** on NVMe | **B** |
| Tiny job `<400 files` or `<256 MiB` | Gated to `1` by `should_attempt_parallel` `lib.rs:1147` + `pool.rs:374` | Same gate | Tie (sequential wins) |
| USB (`W=2, /MT:4`) or Network (`W=3`) | Bus bottleneck, A already saturates | Extra shards don't increase bus → overhead | **A** |
| Large file single (5 GB ISO) | Not sharded (file-level, not dir) — out of scope | Would need chunked file copy (future) | N/A |

## Correctness

* Disjointness preserved: Plan A by dir, Plan B by file set within one dir prefix → still `shards::partition` invariant `union == all files, no overlap` (`shards.rs:223` test).
* Long paths: chunk uses `to_long_path` `lib.rs:315`.
* Symlinks/junctions: still skipped (`is_symlink`, `/XJ /XJD`).
* Filter (`*.tmp; node_modules`): chunk respects `parse_filter` + `matches_filter` `lib.rs:1947` — filtered names not counted in `est_bytes`, not chunked.
* Throttle: still hard gate `1` (`pool.rs:374`), B never triggers when `throttle>0`.

## Risks & Mitigations

* **Shard explosion:** Cap `k ≤ 6` and `min_chunk ≥ 64 MiB` → at most `W+5` shards.
* **Small-file overhead:** If monster is 10k × 4 KB files, chunking by bytes still creates many files per chunk but avoids per-file `stat` blow-up — sort by size is `O(f log f)` once.
* **Sync:** Phase 1 delete uses `collect_extra_files` (dest walk). Chunking only affects Phase 2 copy shards, not delete. No extra delete spawns.

## Alternatives Considered

* Always file-chunk every dir → accurate but `read_dir` entire tree + sort all files → slower for balanced case.
* Work-stealing inside shard (threads stealing files) → would require abandoning `robocopy` per-shard model, reimplementing copy loop.

## Implementation (done)

* `shards.rs:34` `partition_balanced(source, dest, workers)` — Plan A sort largest-first, Plan B file-chunk split for flat monster (`max > 1.5×avg`, `k=ceil(max/avg) clamp 2..6`, bin-pack files descending). New `Shard.chunk_files: Option<Vec<String>>` + `run_file_chunk_shard` `lib.rs:1907` (direct `std::fs::copy` per file, respects pause/cancel, `Tracker`, `skip_conflict`).
* `lib.rs:2193` `transfer_parallel` now tries `partition_balanced` when `workers>1`; falls back to `partition`.
* Tests: `cargo test` 44 passed, `shard_files` handles chunks.

## Next Steps

* Add dedicated `balanced_partition_covers_without_overlap` + `flat_monster_is_split` tests.
* Tune `1.5×` / `64 MiB` min chunk if needed from real `400 MB` flat perf run.

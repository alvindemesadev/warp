# 005 — Cost-Balanced Sharding (Bytes + File Count)

**Status:** Implemented — 2026-08-31 (`shards.rs:19 FILE_OVERHEAD`, `shards.rs:52 partition_balanced` cost sort + file-chunk)  
**Related:** `004-balanced-sharding.md`, `shards.rs:34 partition`, `shards.rs:52 partition_balanced`, `pool.rs:363`, `lib.rs:2115`

## Motivation

`shards.rs:34` balanced only by `est_bytes`. Real wall time is `bytes + per-file overhead` (open/stat/close, MFT, robocopy log). `1×100 MB` (1 file) ≠ `1000×100 KB` (100 MB) — second is 3-4× slower on same NVMe.

Current `18/20 folders → 2 workers @37%` for `Demo/source` (42k files, 424 MB): `node_modules` = `250 MB / 8000 files` dominates by cost, not just bytes, but was kept as 1 shard → straggler.

## Cost Model

```rust
const FILE_OVERHEAD: u64 = 64 * 1024; // 64KB-equivalent per file, tune via bench
fn shard_cost(s: &Shard) -> u64 { s.est_bytes + s.est_files * FILE_OVERHEAD }
fn file_cost(f: &FileEntry) -> u64 { f.size + FILE_OVERHEAD }
```

Calibrate `FILE_OVERHEAD` via `cargo bench` `benches/scan.rs:3204`: compare `1×40MB` vs `10k×4KB` (same bytes) wall, solve for `OH`.

## Plan

### Plan A — Cost-balanced buckets (always, replaces bytes-only sort)

1. Collect top-level dirs + loose shard as `(bytes, files, cost)` using `dir_stats` + `list_children` (cached).
2. Sort shards descending by `cost` (not `est_bytes`).
3. Greedy bin-pack into `W = resolve_workers_for(...)` buckets: next shard → bucket with smallest `bucket_cost`.
4. Result: `20` shards **not merged**, just reordered so large-cost shards start first → `queue` `lib.rs:2138` drains evenly. No extra `read_dir`.

### Plan B — File-chunk by cost (conditional)

Trigger: after Plan A, compute `avg_cost = total_cost / W`, `max_cost = max(bucket_cost)`. If `max_cost > 1.5 × avg_cost`:

_Identify victim bucket/shard:_ the `max` shard.

- If victim is flat (`listing.dirs.is_empty()` && `files.len()>=2`): `k = ceil(max_cost / avg_cost)` clamp `2..6` and `≤ files.len()`. Bin-pack its files descending by `file_cost` into `k` buckets (same greedy). Replace victim with `k` `Shard{ chunk_files: Some(names), est_bytes, est_files }` `shards.rs:147`.

- If victim has subdirs (e.g., `node_modules` with many subdirs, not flat): first expand single-shard outer case `shards.len()==1 && dirs.len()>=2` into per-child shards (already in `partition_balanced:64`), then re-evaluate cost. If still `max_cost >1.5×avg`, file-chunk the largest flat child.

- Single file `400 MB / 1 file` → `cost≈400M`, `k` would be `7` but `files.len()=1` → clamp to `1` → no split (correct — needs future chunked-file copy).

### Execution

- `Shard.chunk_files` already exists `shards.rs:29` + `run_file_chunk_shard` `lib.rs:1907` (direct `std::fs::copy` per file, respects `skip_conflict`, `pause/cancel`, `Tracker` ingest `size`). No change.
- `Tracker` progress still by `bytes` (user-visible), cost only for **scheduling**.
- Long paths `to_long_path`, symlinks `/XJ`, filter `matches_filter` respected.

## Speed Expectation

- `1×100 MB` vs `1000×100 KB`: cost predicts `~3×` wall → balanced buckets put `1000` small files across `3` workers vs `1`, wall drops `~3×`.
- `Demo/source` `20` shards, `W=8`: before `18/20 → 2 workers @37%` (tail 63% bytes in 2 shards). After: buckets `~53 MB cost` each → `8` stays busy past `70%`, tail `2` only at `~90%`.

## Risks

- `FILE_OVERHEAD` mis-tuned → over-splits tiny-file dirs or under-splits large-file dirs. Mitigate via bench on target NVMe vs USB (`W=2` already caps).
- Shard explosion capped `k≤6`, min chunk `64KB` cost.

## Next Steps

1. Add `shard_cost` + `FILE_OVERHEAD` to `shards.rs`, switch sort/bin-pack to `cost`.
2. Update `partition_balanced` trigger to `cost` (keep `1.5×`).
3. Keep `resolve_workers_for` gates unchanged.

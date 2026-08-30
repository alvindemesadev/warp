# ADR 003: Parallel gates — when to shard

**Date:** 2026-08-10
**Status:** Accepted

## Context

Parallel speeds large multi-folder jobs but concurrent `/MIR` or `/IPG` would corrupt.

## Decision

`should_attempt_parallel` (`lib.rs:852`) hard gates: `sync` or `throttle>0` → 1 worker. Auto needs `≥400 files && ≥256 MiB && ≥2 top dirs`. Explicit `workers>1` bypasses size heuristics but never hard gates. `resolve_workers_for` (`pool.rs:334`) caps: USB 2, network 3, local `available_parallelism/2` clamp 2..6, `shard_args` `/MT:4-8` keeps total ≈ `/MT:32`.

## Consequences

- Sync/throttled stay single-process (correctness over speed, `WHITEPAPER.md:6.1`)
- Dominant child `≥512 MiB && ≥40%` split recursively `MAX_SPLIT_DEPTH=2` (`shards.rs:14`)

## References

- `src-tauri/src/shards.rs:93`
- `src-tauri/src/pool.rs:334`

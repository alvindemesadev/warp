# Warp Architecture — 1.2.4

> One-page view of `docs/WHITEPAPER.md:4` — mermaid + file map. Version 1.2.4

```mermaid
flowchart TD
  UI[+page.svelte<br/>wiring only] --> T[stores/transfer.svelte.ts<br/>Progress + derived]
  UI --> Q[stores/queue.svelte.ts]
  UI --> P[stores/presets.svelte.ts]
  UI --> U[stores/updater.svelte.ts]
  UI --> V[services/validation.ts<br/>overlap/preflight]
  T --> S[services/warp.ts<br/>invoke/listen]
  S --> R[src-tauri/src/lib.rs<br/>run_transfer]
  R --> B[backend.rs<br/>TransferBackend trait]
  R --> Pa[parser.rs<br/>RoboLine]
  R --> Pr[progress.rs<br/>overall_pct]
  R --> Sh[shards.rs<br/>partition]
  R --> Po[pool.rs<br/>Tracker + resolve_workers_for]
  R -.-> C[preflight.rs<br/>stub]
  Po --> Sh
```

## File map

| Path                                | Role                                                                                        |
| ----------------------------------- | ------------------------------------------------------------------------------------------- |
| `src/routes/+page.svelte`           | 130 lines script, composes `PathCard`/`ModePicker`/`ProgressCard` etc., no business logic   |
| `src/lib/stores/transfer.svelte.ts` | `TransferStore` class `$state`/`$derived` (`canStart`/`overlappingPath`)                    |
| `src/lib/services/validation.ts`    | `getOverlappingPath` etc., pure                                                             |
| `src/lib/services/warp.ts`          | `warpFileOp`/`listenWarpProgress`                                                           |
| `src-tauri/src/lib.rs`              | `run_transfer` → `warp_file_op_sync` / `transfer_parallel`, `TransferControl` + `JobObject` |
| `src-tauri/src/parser.rs`           | `parse_line` tab-column locale robust                                                       |
| `src-tauri/src/progress.rs`         | `overall_pct`/`fmt_speed` parity with `format.ts`                                           |
| `src-tauri/src/backend.rs`          | `TransferBackend` trait (` RobocopyBackend` / `RsyncBackend` stub)                          |
| `src-tauri/src/pool.rs`             | `Tracker`, `resolve_workers_for`, `consume_stream`                                          |
| `src-tauri/src/shards.rs`           | `partition` disjoint shards                                                                 |

## Invariants

- `TransferControl` `kill_all` + `JobObject` `KILL_ON_JOB_CLOSE` — no orphan
- `shards::partition` disjoint + `Tracker` same EWMA as sequential
- `check_overlap` canonicalizes (`lib.rs:848`) to defeat `..\` and junctions
- `warp.log` JSON hashed paths, 5 MB rotate (`lib.rs:310`)

```

```

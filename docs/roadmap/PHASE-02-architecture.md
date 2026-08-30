# Phase 2 — Architecture & Maintainability (7.0 → 10)

> **Goal:** Break the monolith, kill duplication, make the codebase boring to change.  
> **Effort:** 1–2 weeks  
> **Depends:** Phase 1 (lint gates)  
> **Unlocks:** Phase 3, 4, 5, 6, 8

Scan found `src/routes/+page.svelte:1` is **569 lines, 54 `$state` vars** (`+page.svelte:33-84`) owning drag-drop, queue, presets, updater, and transfer orchestration. Business logic bleeds into the view. Duplicate formatting/progress math lives in 3 places.

---

## 1. Target Structure

```
src/
  lib/
    types.ts              ← canonical (keep)
    format.ts             ← pure display (keep, add tests)
    storage.ts            ← localStorage (keep, add zod)
    transfer.ts           ← throttle/worker helpers (keep)
    tauri.ts              ← thin invoke wrappers → import from types.ts only
    stores/
      transfer.svelte.ts  ← NEW: Svelte 5 runes store (source/dest/mode/progress)
      queue.svelte.ts     ← NEW: queue logic from +page.svelte:278
      presets.svelte.ts   ← NEW: presets CRUD from +page.svelte:303
      updater.svelte.ts   ← NEW: updater state from +page.svelte:332
      ui.svelte.ts        ← NEW: toasts, modals, drag state
    services/
      warp.ts             ← NEW: invoke("warp_file_op") + event wiring
      validation.ts       ← NEW: overlap/special-path/drive checks (extract from +page.svelte:382)
  routes/
    +page.svelte          ← 150 lines max: composes components + stores
    +layout.svelte        ← keep
```

Rust stays `lib.rs` / `pool.rs` / `shards.rs` but with traits (see 2.3).

---

## 2. Tasks

### 2.1 Frontend Decomposition (5–7 days) — highest ROI

- [ ] **Extract `transfer.svelte.ts` store**
  - Move from `+page.svelte:33-84`: `sourcePath`, `destPath`, `sourceInfo`, `destInfo`, `mode`, `conflict`, `folderMode`, `throttle`, `verify`, `workers`, `progress`, `speed`, `etaSeconds`, `transferredFiles`, `paused`.
  - Expose `setSource(p)` (`+page.svelte:182`), `setDest`, `swapPaths` (`+page.svelte:198`), `reset` (`+page.svelte:259`).
  - Keep `$derived` for `overlappingPath` (`+page.svelte:382`), `crossDriveMove` (`+page.svelte:394`), `canStart` (`+page.svelte:396`).

- [ ] **Extract `queue.svelte.ts`**
  - Move `queue`, `isQueueRunning`, `queueIndex`, `queueTotal`, `queueResults`, `addToQueue` (`+page.svelte:270`), `removeFromQueue`, `clearQueue`, `runQueue` (`+page.svelte:278`). Persist via `storage.ts:112`.

- [ ] **Extract `presets.svelte.ts` + `updater.svelte.ts` + `ui.svelte.ts`**
  - Presets: `openPresetModal`, `confirmSavePreset`, `loadPreset`, `deletePreset` (`+page.svelte:303-316`).
  - Updater: `checkForUpdates`, `installUpdate`, `_pendingUpdate`, `updateState` (`+page.svelte:332-372`).
  - UI: `showSyncWarning`, `dropConflict`, `_pendingDrop`, `toast` (`+page.svelte:65-66/174`).

- [ ] **Extract `services/warp.ts`**
  - Wrap `invoke("warp_file_op")` + `listen("warp-progress"/"warp-error"/"warp-verifying")` wiring now duplicated in `+page.svelte:93-172` and `lib.rs:101-120`. Provide `startWarp(config)` returning `Promise<WarpSummary>` with `onProgress` callback.

- [ ] **Shrink `+page.svelte` to 150 lines**
  - Only: imports stores/services, composes `PathCard`, `ModePicker`, `OptionsPanel`, `ProgressCard`, `QueueList`, `ResultCards`, `QueueSummary`, modals. No business logic. Verify no `invoke`/`listen` remains.

- [ ] **Add unit tests for stores**
  - `src/lib/stores/transfer.test.ts`, `queue.test.ts` — test `canStart` matrix, `swapPaths`, queue persistence (mock `localStorage` like `storage.test.ts:5`).

### 2.2 Deduplication (1–2 days)

- [ ] **Single source for formatting**
  - Today: `fmtBytes` in `format.ts:10`, `fmt_bytes_pretty` in `lib.rs:459`, `fmt_speed` in `lib.rs:452` + `pool.rs:85`. Create `src/lib/format.ts` as canonical for frontend; Rust keeps its own but add `// parity with format.ts:10 — update both` comment and a cross-language test that asserts same outputs for `0, 1023, 1024, 1_048_576`.

- [ ] **Single source for progress math**
  - `overall_pct` (`lib.rs:447` ↔ `pool.rs:85`), `ipg_for_throttle` (`lib.rs:470`), `THROTTLE_OPTIONS` (`transfer.ts:7` ↔ `lib.rs:470`). Keep Rust as source for `overall_pct`/`ipg`, expose as const for TS via `types.ts` comment + test.

- [ ] **Unify `PathInfo`/`WarpSummary`**
  - Remove duplicate in `tauri.ts:4`, re-export from `types.ts:9/17`.

### 2.3 Rust Structure (2–3 days)

- [ ] **Introduce `TransferBackend` trait (Phase 8 prep, no behavior change)**

  ```rust
  // src-tauri/src/backend.rs (new)
  trait TransferBackend { fn scan(&self) -> (u64,u32); fn copy(&self, shard: &Shard) -> ShardOutcome; }
  struct RobocopyBackend; // implements trait, wraps lib.rs:637 + pool.rs
  ```

  Move `robocopy_cmd` (`lib.rs:278`), `parse_line` (`lib.rs:547`), `scan`, `verify_transfer` behind trait. No new backend yet — just seam. Matches `docs/WHITEPAPER.md:4` "rsync stub" intention.

- [ ] **Split `lib.rs` (2,100+ lines)**
  - Extract: `preflight.rs` (`resolve_effective_dest` `lib.rs:738`, `check_overlap` `lib.rs:776`, `check_network_dest`, `check_fat32_source`, `ensure_free_space`), `parser.rs` (`parse_line` + `RoboLine` `lib.rs:528`), `progress.rs` (`overall_pct`, `fmt_speed`), `commands.rs` (`get_path_info`, `warp_file_op`, `cancel_warp`, `pause_warp`). `lib.rs` becomes wiring only.

- [ ] **Document `TransferControl` invariants**
  - Add module doc to `lib.rs:22` explaining `children` registry, `kill_all` (`lib.rs:76`), `lock_children` poison handling (`lib.rs:42`), and window-close handler (`lib.rs:1859`).

### 2.4 State & Persistence Hardening

- [ ] **Replace ad-hoc `localStorage` validation with `zod`**
  - `storage.ts:25` `isValidPreset`/`isValidRecentEntry` hand-rolled → `zod` schemas, inferred types. Keep `loadPresets`/`loadRecent` fallback semantics (`storage.ts:53`).

- [ ] **Add migration version to persisted JSON**
  - Store `{ v: 1, data: [...] }` so future schema bumps don't silently drop user presets.

---

## 3. Acceptance

- [ ] `+page.svelte` ≤ 150 lines, 0 `invoke`/`listen` calls
- [ ] 0 duplicate formatter/progress implementations (or parity test passes)
- [ ] `cargo test` still passes after trait extraction (no behavior change)
- [ ] New stores have ≥80% coverage (`vitest coverage`)
- [ ] `lib.rs` ≤ 400 lines (wiring only)
- [ ] `docs/WHITEPAPER.md:77` architecture diagram updated to show stores/services + backend trait

---

## 4. Verification

```bash
npm run check
npm test -- --coverage
cargo test --lib --manifest-path src-tauri/Cargo.toml
# Manual: open app, run one copy + one queue + one sync — same behavior as 1.2.4
```

---

## 5. Risks

| Risk                                  | Mitigation                                                |
| ------------------------------------- | --------------------------------------------------------- |
| Svelte 5 runes store API churn        | Pin `svelte@^5.0`, follow `svelte.dev/docs/svelte/$state` |
| Over-abstracting Rust trait too early | Trait has 3 methods max; no generics, no async            |

---

**Next:** `PHASE-03-testing.md` (can start after 2.1 merges)

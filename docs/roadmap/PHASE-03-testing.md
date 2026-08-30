# Phase 3 — Testing & Correctness (8.0 → 10)

> **Goal:** Confidence to refactor and ship with eyes closed.  
> **Effort:** 1–2 weeks  
> **Depends:** Phase 2 (stores exist to test)  
> **Unlocks:** Phase 4, 5, 6

Current: 25 Vitest tests (format/storage/transfer) + Rust unit + real-robocopy `#[cfg(windows)]` integration (`lib.rs:2183`) + ignored perf harness (`lib.rs:2342`). Good foundation, gaps in e2e, fuzz, and coverage enforcement.

---

## 1. Targets

| Metric                  | Now        | 10/10                                       |
| ----------------------- | ---------- | ------------------------------------------- |
| Vitest coverage (lines) | unmeasured | ≥95%                                        |
| Rust coverage (lines)   | unmeasured | ≥85% (`cargo tarpaulin` or `llvm-cov`)      |
| E2E                     | 0          | 10 Playwright Tauri scenarios passing in CI |
| Parser fuzz             | 0          | 1h fuzz clean                               |
| Mutation score          | —          | ≥80%                                        |

---

## 2. Tasks

### 3.1 Coverage & Gates (1 day)

- [ ] **Enable Vitest coverage**
  - `vitest.config.ts:6` add `coverage: { provider: 'v8', thresholds: { lines: 95, branches: 90 } }` + `npm run test:coverage`.
  - Exclude `warp-site/**`, `src-tauri/target/**` already done (`vitest.config.ts:8`).

- [ ] **Enable Rust coverage**
  - Add `cargo llvm-cov --summary-only` to `release.yml:32` (or `cargo tarpaulin`). Threshold 85% on `lib.rs`, `pool.rs:404`, `shards.rs:166`.

- [ ] **Gate CI on coverage**
  - Fail PR if coverage drops >1%.

### 3.2 Frontend Unit Expansion (2–3 days)

- [ ] **`format.ts` edge cases** (`src/lib/format.test.ts:1`)
  - Add: `fmtBytes(0)`, `1_073_741_823` boundary, `fmtDuration` 60s exact, `fmtEta` 3599/3600, `basename` with `\\?\` prefix (`lib.rs:216` parity).

- [ ] **`storage.ts` corruption matrix** (`storage.test.ts:66`)
  - Add: `localStorage` quota exceeded (`setItem` throws), `JSON.parse` with `__proto__` pollution, `loadQueue` with mixed valid/invalid (`storage.ts:112`), migration `v` field.

- [ ] **New: `validation.test.ts`**
  - Extract from `+page.svelte:382` `overlappingPath` + `transfer.ts:44` `isSpecialPath` + drive logic `lib.rs:144`. Cases: `C:\a` vs `C:\a\Photos` with `folderMode=="into"`, case-insensitive, trailing slash.

- [ ] **New: `stores/*.test.ts`** (after Phase 2)
  - `transfer.svelte.test.ts`: `canStart` permutations (file source `lib.rs:137` `is_file`, overlapping, empty), `swapPaths` idempotence.
  - `queue.svelte.test.ts`: `addToQueue` dedup, `runQueue` order, persistence.

### 3.3 Rust Unit Expansion (2–3 days)

- [ ] **Parser property / fuzz**
  - Add `cargo fuzz` (or `proptest`) for `parse_line` (`lib.rs:547`): generate random tab strings, assert never panics, `FileHeader` size ≤ `u64::MAX`, `name` not empty when `FileHeader`. Include non-English fixtures: German `Neue Datei`, French `Nouveau fichier`, Japanese.

- [ ] **`Tracker` invariants** (`pool.rs:33`)
  - Already has `small_files_count_bytes_and_emit_monotonically` (`pool.rs:443`). Add: concurrent `ingest` from 8 threads (stress `Mutex<Tracker>`), `revert_bytes` (`pool.rs:222`) never underflows.

- [ ] **`shards::partition` invariants** (`shards.rs:223`)
  - Already covers `partition_covers_everything_without_overlap` and `dominant_child_is_recursively_split`. Add: symlink ignored, empty source `[]`, `MAX_SPLIT_DEPTH=2` (`shards.rs:14`) depth cap, `MIN_SPLIT_BYTES=512MiB` (`shards.rs:18`) not split when small.

- [ ] **`preflight` unit tests** (after Phase 2 split)
  - `resolve_effective_dest` (`lib.rs:738`): `C:\Photos\Screenshots → C:\Backup\Screenshots` vs already-`Screenshots` no double-nest.
  - `check_overlap`, `ensure_free_space` with mocked `free_bytes_available` (`lib.rs:193`).

### 3.4 Integration & E2E (3–5 days)

- [ ] **Tauri E2E with `tauri-driver` + Playwright**
  - New folder `e2e/` + `e2e/tauri.spec.ts`. Scenarios (10):
    1. Copy small tree 3 files → 100% + summary `transferred==3`
    2. Move with empty dirs → source removed (`remove_empty_dirs` `lib.rs:376`)
    3. Sync with `*EXTRA` deletes progress moves (`lib.rs:1225`)
    4. Overlap blocked (`check_overlap` `lib.rs:776`) shows UI warning (`+page.svelte:382`)
    5. Cancel mid-transfer → no orphan `robocopy` (check `TransferControl.children` empty)
    6. Pause/resume parallel (2 shards) → `paused` gate (`lib.rs:636`)
    7. Throttle 5 MB/s → verify `ipg_for_throttle` (`lib.rs:470`) + `shard_args` with `/MT:4`
    8. Verify pass structural (`verify_transfer` `lib.rs:663`) detects deleted dest file
    9. Queue 3 jobs → sequential, combined summary (`+page.svelte:278`)
    10. Non-English fixture: feed German parser line → still counts bytes

- [ ] **Real robocopy suite always on**
  - Today `real_robocopy` (`lib.rs:2183`) is `#[cfg(windows)]` — ensure CI `windows-latest` runs it (add `cargo test --lib -- --include-ignored` toggle for `perf_*` only).

- [ ] **Perf regression harness**
  - Promote `lib.rs:2354` `perf_local` (`30×300×16KB`) to nightly CI, assert parallel ≥ sequential throughput within 10% (not necessarily faster — contention is real, see `WHITEPAPER.md:229`).

### 3.5 Contracts

- [ ] **Frontend↔Backend type snapshot**
  - Generate JSON schema from `WarpProgress`/`WarpSummary` (`lib.rs:90/111` + `types.ts:17/33`) and diff in CI — prevents drift.

---

## 3. Acceptance

- [ ] Coverage gates green (95% TS, 85% Rust)
- [ ] 10 e2e scenarios green on `windows-latest` in PR CI
- [ ] `cargo fuzz` 1h clean (no panic/OOM)
- [ ] Mutation score ≥80% on `parser.rs` + `format.ts`
- [ ] `npm run test:coverage` + `cargo llvm-cov` in `release.yml`

---

## 4. Verification

```bash
npm run test:coverage
cargo llvm-cov --summary-only --manifest-path src-tauri/Cargo.toml
cargo test --lib --manifest-path src-tauri/Cargo.toml
cargo fuzz run parser -- -max_total_time=60
npx playwright test e2e/
```

---

**Next:** `PHASE-04-security.md` (can overlap after 3.1)

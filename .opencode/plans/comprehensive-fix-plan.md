# Warp — Comprehensive Fix Plan (v1.1.2 → v1.1.3)

> Addresses all issues from codebase scan 2026-08-22. Ordered by risk/dependency. Each item cites `file:line`.

## 0. Guiding Principles
- No data loss (move/skip/mirror are destructive)
- Keep `robocopy` locale-independent parsing (`src-tauri/src/lib.rs:286`) and exit-code fallback (`lib.rs:399`)
- Small PRs, verified by `npm test` + `cargo test --lib` + `svelte-check --tsconfig ./tsconfig.json`
- Maintain version single-source truth: `src-tauri/tauri.conf.json:4` → propagated by `scripts/release.js:130`

## 1. Critical — Security & Correctness (Ship First)

### 1.1 CSP is disabled
- **Problem:** `src-tauri/tauri.conf.json:31` `"csp": null` disables Tauri CSP. Inline `onclick` in `src/routes/+page.svelte:750` + `src/app.html:13` shim rely on it.
- **Fix:** 
  1. Set `app.security.csp: "default-src 'self' ipc: http://ipc.localhost; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'"`
  2. Remove `src/app.html:13` TAURI_INTERNALS shim (dev-only, gated by `import.meta.env.DEV` or delete entirely — it masks failures in `vite preview`)
  3. Verify no remote `eval`/`innerHTML` remains (grep)
- **Verify:** `cargo test`, manual Tauri build loads, browser console no CSP violation.

### 1.2 Mutex poison panic
- **Problem:** `src-tauri/src/lib.rs:190, 547, 640` `Mutex::lock().unwrap()` — a prior panic poisons lock → second call panics, UI hangs on cancel/progress.
- **Fix:** Replace all 3 with `lock().unwrap_or_else(|e| e.into_inner())` + log poison. Extract helper `fn lock_process(state: &ActiveProcess) -> MutexGuard<...>`.
- **Verify:** Unit test that poisons mutex then calls `cancel_warp` still recovers.

### 1.3 Exit code `None` mishandled
- **Problem:** `src-tauri/src/lib.rs:644` `child.wait().ok().and_then(|s| s.code()).unwrap_or(0)` treats killed-by-signal as success `0`, masking cancel/failure. Same in `scan:351` and `verify_transfer:378`.
- **Fix:** `code: Option<i32>` → `None` = `cancelled=true` or `error_code = -1, errorMessage = "Process terminated"`. Do not map to `0`.
- **Verify:** Integration test: spawn child, kill, assert `cancelled`.

### 1.4 Unsafe manual FFI `GetDriveTypeW`
- **Problem:** `src-tauri/src/lib.rs:60` `extern "system" { fn winapi_GetDriveTypeW }` unsafe, no `windows` crate, brittle width calc `lib.rs:67`.
- **Fix:** Add `windows = { version="0.58", features=["Win32_Storage_FileSystem"] }` to `src-tauri/Cargo.toml:17`, replace `is_removable_drive` with `unsafe { GetDriveTypeW(PCWSTR(wide.as_ptr())) } == DRIVE_REMOVABLE`.
- **Verify:** `cargo check`, `is_removable_drive` unit test on `C:`/`D:`.

### 1.5 Recursive `walk_dir` stack overflow
- **Problem:** `src-tauri/src/lib.rs:133` recursive `walk_dir(&e.path().to_string_lossy(), ...)` — deep `node_modules` (>1000 depth) overflows.
- **Fix:** Iterative stack `Vec<PathBuf>` + `read_dir` loop, skip symlink loops via `is_symlink()`.
- **Verify:** Test with 1500-deep temp tree.

## 2. High — Build & Release Integrity

### 2.1 Stale `latest.json`
- **Problem:** `latest.json:1` is `1.0.1` while app is `1.1.2`; file is gitignored (`# Updater manifest` `.gitignore:24`) so never reviewed, updater serves wrong version.
- **Fix:** 
  1. After `scripts/build.js:78` succeeds, always run `node scripts/updater-manifest.js` (add to `build.js` tail)
  2. In `scripts/release.js:167` assert `latest.json` version == `version` before tag, fail otherwise
  3. Add CI check `release.yml:56` that fails if `latest.json` missing `*.sig`
- **File:** `scripts/build.js:78`, `scripts/updater-manifest.js:54`, `.github/workflows/release.yml:56`

### 2.2 Hardcoded versions
- **Problem:** `scripts/readme-download.js:20` hardcodes `Warp_1.1.2`, `src/routes/+page.svelte:696` `APP_VERSION = $state("1.1.2")` is fallback literal; drift risk.
- **Fix:** 
  1. `readme-download.js` read `src-tauri/tauri.conf.json` version dynamically (like `release.js:36` `CURRENT`)
  2. `+page.svelte:696` keep literal but add build-time replace via `release.js:134` already does — add CI grep that fails if literal != `tauri.conf.json`
  3. Optionally import version at build via `__APP_VERSION__` define in `vite.config.js:7` `define: { __APP_VERSION__: JSON.stringify(conf.version) }`
- **Verify:** `node scripts/readme-download.js` after bump prints correct name without manual edit.

### 2.3 Single source of truth enforcement
- **Problem:** 7 places bumped in `scripts/release.js:130` via regex; fragile (Cargo.lock `name="warp"` pattern `release.js:132` assumes exact formatting).
- **Fix:** Add `scripts/check-versions.js` run in CI: parse `tauri.conf.json`, `Cargo.toml`, `package.json`, `warp-site/package.json`, `+page.svelte` literal, assert equality. Fail PR otherwise.
- **Verify:** `npm run check:versions` in `release.yml:37`.

## 3. High — Frontend Architecture (Tech Debt)

### 3.1 Monolithic `+page.svelte` (1566 lines)
- **Problem:** Single file holds types `+page.svelte:13`, state `+page.svelte:79`, lifecycle `+page.svelte:156`, path helpers `+page.svelte:284`, transfer `+page.svelte:340`, queue `+page.svelte:429`, presets `+page.svelte:530`, recent `+page.svelte:570`, notifications `+page.svelte:585`, updater `+page.svelte:620`, constants `+page.svelte:692`, all markup 747-1566.
- **Fix (incremental, 3 PRs):**
  1. **Extract pure logic:** `src/lib/transfer.ts` (types `Mode/Conflict/QueueJob/Preset/RecentEntry`, `isSpecialPath`, `syncSpeedMode`, `currentJobConfig`), `src/lib/storage.ts` (wrap `localStorage` for `warp-recent`/`warp-presets` with validation — see 3.3), `src/lib/tauri.ts` (wrappers `getPathInfo`, `warpFileOp`, `cancelWarp`, event `listen` helpers)
  2. **Extract stores:** `src/lib/stores/queue.svelte.ts` (Svelte 5 `$state` for `queue/isQueueRunning/queueIndex/...` + `addToQueue/removeFromQueue/runQueue`), `src/lib/stores/ui.svelte.ts` (modals/toast/updateState)
  3. **Extract components:** `src/lib/components/DropZone.svelte`, `ModeSelector.svelte`, `ThrottleControl.svelte`, `ProgressCard.svelte`, `QueueList.svelte`, `PresetPanel.svelte`, `RecentPanel.svelte`, `UpdateModal.svelte`, `SyncWarningModal.svelte`, `TrafficLights.svelte` — `+page.svelte` becomes ~200 line orchestrator
- **Verify:** No visual regression (screenshot compare), `npm test`, `svelte-check` 0 errors.

### 3.2 Inline styles (750+ instances)
- **Problem:** Every element uses `style="..."` (e.g., `+page.svelte:793` svg noise, `+page.svelte:815` modal). No theming, large bundle, CSP requires `'unsafe-inline'`.
- **Fix:** Migrate to scoped `<style>` + `src/app.css:52` tokens. Create `src/lib/styles/tokens.css` already mostly done, add classes `.glass-card`, `.btn-primary`, `.modal-backdrop`, `.traffic-light`. Keep minimal inline for dynamic `width:{pct}%` only. Use `tailwind-v4-shadcn` skill pattern (`@theme inline`) if adopting utility layer — otherwise pure CSS.
- **Verify:** Visual diff, CSP can tighten to `style-src 'self'` after migration (Phase 3.2 second pass).

### 3.3 `localStorage` without validation
- **Problem:** `+page.svelte:162` `JSON.parse(localStorage.getItem("warp-recent"))` and `+page.svelte:169` `warp-presets` — corrupt JSON or old schema crashes `presets = JSON.parse(...)`.
- **Fix:** New `src/lib/storage.ts:1` with Zod-like guard:
  ```ts
  function load<T>(key:string, validate:(v:unknown)=>T|null):T|null
  ```
  Validate `RecentEntry` has `source/dest/mode/timestamp`, `Preset` has `name/source/dest/mode`. On failure, `localStorage.removeItem(key)` + return `[]`, log.
- **Verify:** Unit test corrupt JSON → `[]`, old schema → migration.

## 4. Medium — Performance & Resilience

### 4.1 Progress event flood
- **Problem:** `src-tauri/src/lib.rs:612` `if pct != last_emitted || !name.is_empty()` emits per file (10k small files = 10k IPC). Front `+page.svelte:176` prepends `transferredFiles.slice(0,200)` causes 10k Svelte updates.
- **Fix:** Backend throttle to 100ms or 1% delta: `if now - last_emit > 100ms || pct != last_emitted_at_emit` then emit. Frontend batch `transferredFiles` via `requestAnimationFrame` or Svelte `tick`.
- **Verify:** Copy 5000x1KB, Task Manager + devtools timeline shows < 10 emits/sec.

### 4.2 `get_path_info` duplicate scan
- **Problem:** `src/routes/+page.svelte:288` calls `get_path_info` (full `walk_dir`) for warnings, then `lib.rs:482` `scan()` re-does `robocopy /L` for total bytes — double I/O on large trees.
- **Fix (optional):** Keep both — `walk_dir` provides `removable`/`drive` for UX immediately, `scan` accurate for progress. Document, or unify by returning `totalBytes` from `get_path_info` and reuse if recent (<5s) to skip second scan.
- **Verify:** Benchmark 100k files.

### 4.3 Custom throttle validation
- **Problem:** `+page.svelte:140` `customSpeedValue = $state(50)` no clamp; user could set `0` or `NaN` → `ipg_for_throttle(0)=None` (unlimited) unexpectedly.
- **Fix:** `<input type="number" min="1" max="500">` + `on:input` clamp `Math.min(500, Math.max(1, parseInt(v)||1))`, shared helper `normalizeThrottle(n:number):number`.
- **Verify:** Unit test `normalizeThrottle`.

## 5. Medium — Testing & Quality Gates

### 5.1 No UI/integration tests
- **Problem:** Only `src/lib/format.test.ts:1` (9 tests) + `lib.rs:714` parser tests. No test for `+page.svelte:283` `applyDropToPending`, `+page.svelte:434` `addToQueue`, `+page.svelte:535` `confirmSavePreset`.
- **Fix:** Add `vitest` `environment: "jsdom"` + `@testing-library/svelte` for `format.ts` already covered, add `src/lib/storage.test.ts`, `src/lib/transfer.test.ts` (queue logic, `isSpecialPath`, `syncSpeedMode`), `src/routes/+page.test.ts` shallow render. Add Rust `#[test] is_removable_drive_mock`.
- **Verify:** `npm test` > 30 tests, `cargo test --lib` passes locally + CI.

### 5.2 No lint/format in CI
- **Problem:** No `eslint`, `prettier`, `svelte-check` in `release.yml:37` — only `cargo test`.
- **Fix:** Add `eslint` (flat config, `eslint-plugin-svelte`) + `prettier` + `prettier-plugin-svelte`, `package.json` scripts `lint`, `format`, `check`. Insert in `release.yml:37`:
  ```yaml
  - run: npm run check
  - run: npm run lint
  ```
  Configure `vitest.config.ts:4` to ignore `warp-site` via `exclude: ["warp-site/**", ".svelte-kit/**"]` (currently svelte-check errors on `warp-site/vite.config.ts`).
- **Verify:** CI fails on lint violation.

### 5.3 `warp-site` gitignored but present
- **Problem:** `.gitignore:27` `warp-site/` but directory exists with `.git` separate repo — `svelte-check` in `warp` errors `Error while loading config at warp-site/vite.config.ts` (`bash` output 2026-08-22), `npm install` at root doesn't install site deps, confusion for contributors.
- **Fix:** Document in `README.md:214` that `warp-site` is a git submodule / external checkout (or convert to `git submodule`). Add `svelte.config.js:6` to `exclude`? Actually `svelte-check` auto-discovers — add `warpsite` to `tsconfig.json` `exclude` and `vite.config.js` `server.watch.ignored` already `src-tauri` but not `warp-site` — add `warp-site/**` to `vitest` exclude and to `.vscode/settings.json` `svelte.enable-ts-plugin: false` for that folder.
- **Verify:** `npx svelte-check --tsconfig ./tsconfig.json` 0 errors without warp-site noise.

## 6. Low — DX & Polish

### 6.1 `src/app.html:13` shim cleanup
- Done in 1.1. Replace with conditional `if (import.meta.env.DEV) { /* mock */ }` or remove; add `src/lib/mockTauri.ts` for `vite preview` only.

### 6.2 `scripts/build.js:25` signing key log
- Currently warn only if `createUpdaterArtifacts` but no key (`build.js:35`). Make warning actionable: print `npm run tauri signer generate ...` and link to `README.md:317`.

### 6.3 Version badge drift
- `README.md:15` badge `Version-1.1.2-339dff` updated by `release.js:138` regex `badge/Version-...` — brittle if badge URL changes. Add `check-versions.js` coverage.

## Execution Order & Estimates

| Phase | Items | Risk | Effort | PR |
|-------|-------|------|--------|----|
| 1 | 1.1→1.5 Critical | High | 1d | PR #1 `fix/security-correctness` |
| 2 | 2.1→2.3 Release integrity | High | 0.5d | PR #2 `fix/release-versions` |
| 3 | 3.3 Storage validation | Med | 0.5d | PR #3 `fix/storage-validation` |
| 4 | 3.1 Split monolith | Med | 2-3d | PR #4-6 incremental |
| 5 | 3.2 Inline styles → classes | Med | 1-2d | PR #7 |
| 6 | 4.1 Throttle progress | Med | 0.5d | PR #8 |
| 7 | 5.x Tests + lint | Low | 1d | PR #9 |
| 8 | 6.x DX polish | Low | 0.5d | PR #10 |

**Total 7-9 days solo.** Parallelize 1+2, then 3-8.

## Verification Checklist (per PR)
- `npm test` (Vitest 9 → 30+ tests)
- `cargo test --lib` (Windows: includes `real_robocopy` integration)
- `npx svelte-check --tsconfig ./tsconfig.json` 0 errors/warnings
- `npm run lint && npm run format:check` (once added)
- Manual: drag-drop (`+page.svelte:208`), swap (`+page.svelte:309`), queue 3 jobs (`+page.svelte:450`), cancel mid-copy (`+page.svelte:405`), preset save/load (`+page.svelte:535`), OneDrive/network warnings (`+page.svelte:710`), throttle 5/25/100 (`+page.svelte:131`), verify toggle.
- Build: `node scripts/build.js` → `npx svelte-check` + `cargo test` + `latest.json` version matches.

## Risks & Mitigations
- **Split regression**: Keep `+page.svelte` snapshot tests before split; feature-flag behind `src/lib/featureFlags.ts`.
- **CSP tightening breaks UI**: Roll out `csp: "default-src 'self'"` with `reportOnly` first.
- **Windows FFI change**: Gate `windows` crate `cfg(windows)` only, keep `#[cfg(not(windows))] false`.

## Next Step
Approve this plan → execute Phase 1 `fix/security-correctness` first (no file moves, minimal blast radius). Say `proceed` to start.

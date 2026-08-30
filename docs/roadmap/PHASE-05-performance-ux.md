# Phase 5 — Performance & UX (8.5 → 10)

> **Goal:** Feels instant, looks perfect, works for everyone.  
> **Effort:** 1–2 weeks  
> **Depends:** Phase 2 (stores), Phase 3 (e2e to guard regressions)

---

## 1. Performance 9.0 → 10

### Why not 10

- No perf budgets — `perf_local` (`lib.rs:2453`) is ignored, not gated.
- Large-file `Percent` smoothing (`lib.rs:1292`) could be monotonic but untested under concurrent shards.
- `walk_dir` (`lib.rs:331`) double-walks for `dir_stats` (`lib.rs:901`) + `shards::partition` — cache warmup noted but not measured.
- No `WebView2` perf audit (startup, memory).

### Tasks

- [ ] **Budgets & harness (2 days)**
  - Promote `lib.rs:2390` `make_perf_fixture(30×300×16KB)` to `cargo bench` (Criterion). Budgets: scan 9k files <2s cold, sequential copy 144 MB NVMe <4s, parallel 6-worker <3s. Fail CI if >10% regression (nightly).
  - Add `measure` script: `npm run bench:rust` + GitHub artifact for flamegraph.

- [ ] **Startup budget**
  - `+page.svelte:93` `onMount` does `getVersion` + `listen` + `checkForUpdates(true):165` delayed 4s. Measure `window.onload` → first paint <400ms on `windows-latest`. Lazy-load `UpdateModal` (`+page.svelte:424`) via `{#await}`.

- [ ] **Walk optimization**
  - Single walk for `quick_bytes/quick_files/top_dirs` (`lib.rs:901/917`): teach `dir_stats` to also return `top_dirs` or memoize `list_children` (`shards.rs:120`) per run — avoid double `read_dir`.

- [ ] **Large-file concurrency polish**
  - Parallel `Tracker` disables deferral (`pool.rs:44`) — correct but means large concurrent files emit at file-header granularity only. Add `Percent` tracking per-shard (optional `PendingLarge` per `id`) if `overall_pct` stutter observed. Measure before building.

- [ ] **Memory**
  - Cap `transferredFiles` (`+page.svelte:112` `.slice(0,200)`) already good — add test assert ≤200. Ensure `BufReader::lines` streaming (`lib.rs:1116`) never buffers whole `robocopy` output.

---

## 2. UX 8.5 → 10

### Why not 10

- No `a11y` audit, keyboard traps unknown, `Esc` handling (`+page.svelte:153`) not tested with screen reader.
- No empty/error/loading skeletons beyond `indeterminate` pulse.
- Design tokens good (`src/app.css:55` `--glass-*`) but no dark/light beyond dark, no motion-reduce.
- `TrafficLights` (`TrafficLights.svelte:1`) custom chrome — may not match Windows 11 snap.

### Tasks

- [ ] **A11y audit (2 days)**
  - `axe-core` on `build/` (static adapter). Fix: `button` labels, `PathCard` dropzone `role="button"` + `aria-dropeffect`, `ProgressCard` live region `aria-live="polite"` for `currentFile`/`speed`. Respect `prefers-reduced-motion` (`src/app.css:84` animations gated).

- [ ] **Keyboard & focus**
  - Tab order: source → dest → mode → options → presets/queue → engage. `Ctrl+O`/`Ctrl+Shift+O` (`+page.svelte:160`) must not trap. `Esc` must close modals in stack order (`SyncWarningModal` > `DropConflictModal` > `PresetNameModal`).

- [ ] **Empty / error / loading states**
  - Empty `RecentPanel`/`PresetsPanel` (`RecentPanel.svelte:39`): illustrate with icon + CTA, not blank. `ResultCards` error: copy `errorLogs` (`+page.svelte:68`) button + open `warp.log` path (`README.md:274`). `ProgressCard` indeterminate (`lib.rs:957`) already pulses — add `%` tooltip "empty folder".

- [ ] **Polish & tokens**
  - Audit `src/app.css:60` tokens for contrast (WCAG AA). Add `prefers-color-scheme: light` alternative (even if gated behind flag). Add `motion-safe` wrappers for `animate-shimmer`/`pulse-soft`.

- [ ] **Window behavior**
  - Test `tauri.conf.json:13` `width 600/height 820` + `minWidth 520` + `maxWidth 820` + `resizable:true` on 125%/150% DPI. Ensure `shadow:true` + `transparent:true` not clipped on Windows 10.

- [ ] **Notifications & updater UX**
  - `notifyDone` (`+page.svelte:320`) already permission-gated — add "Don't ask again" toggle persisted via `storage.ts`. `UpdateModal` (`UpdateModal.svelte:103` lines) already has `Available/Downloading/Installing` — add offline fallback (show cached `latest.json` age).

---

## 3. Acceptance

- [ ] `cargo bench` budgets green, nightly perf artifact uploaded
- [ ] Startup first paint <400ms, `axe` 0 violations
- [ ] Keyboard-only copy→verify flow passes without mouse
- [ ] `prefers-reduced-motion` disables shimmer/pulse
- [ ] No visual regression on 100%/125%/150% DPI screenshots (Playwright)
- [ ] `transferredFiles` capped test passes

---

## 4. Verification

```bash
cargo bench --manifest-path src-tauri/Cargo.toml
npx playwright test e2e/a11y.spec.ts
axe build/index.html
# Manual: 150% DPI, keyboard-only, reduced-motion
```

---

**Next:** `PHASE-06-ci-cd.md`

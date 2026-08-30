# Phase 8 — Future-proofing & Platform — **DEFERRED (Windows-only)**

> **Status:** Deferred per 2026-08-30 decision — Warp stays Windows-only. Mac/Linux would not be faster than `rsync`/`cp`, only nicer UX, not worth the complexity.  
> **Goal (when active):** Windows excellence today, clean seams for tomorrow — no rewrite.  
> **Effort:** 2–4 weeks (track after Phase 2)  
> **Depends:** Phase 2 (backend trait)  
> **Not a 10/10 blocker** — 10 means "abstraction is ready", not "ship macOS/Linux"

---

## 1. What 10 Means Here

- `robocopy` remains the only **shipped** backend, but code no longer assumes it.
- Adding `rsync`/`rclone`/`custom Rust copy` is a new `impl TransferBackend`, not a rewrite of `lib.rs`.
- Single-huge-file parallel and optional hash verify are **opt-in** features, not regressions.

---

## 2. Tasks

### 8.1 Backend Abstraction (1 week, after Phase 2.3)

- [ ] **Trait + registry** (`src-tauri/src/backend.rs` new, see Phase 2.3)

  ```rust
  trait TransferBackend: Send + Sync {
      fn name(&self) -> &'static str; // "robocopy"
      fn scan(&self, source: &str, dest: &str, mode: &str) -> Result<(u64,u32), String>;
      fn copy_shard(&self, shard: &Shard, opts: &CopyOpts) -> ShardOutcome; // opts: mode, throttle, mt
      fn verify(&self, source: &str, dest: &str) -> u32;
      fn capabilities(&self) -> Caps; // supports_mirror, supports_ipg, max_path
  }
  ```

- [ ] **Wire `run_transfer` (`lib.rs:887`) to trait**
  - `fn run_transfer(backend: &dyn TransferBackend, ...)` — today always `RobocopyBackend`. No behavior change, just indirection. Gate `traits` behind `#[cfg(windows)]` for now.

- [ ] **Stub `RsyncBackend` (non-Windows, `#[cfg(not(windows))]`)**
  - Methods `unimplemented!("rsync backend — future")` but compiles on macOS/Linux. Proves `lib.rs:142` `#[cfg(not(windows))]` pattern scales.

### 8.2 Single-Huge-File Parallel (1 week, optional, high value)

- **Problem:** Today parallel only splits **dirs** (`shards.rs:48` `split_dir`). One 50 GB file → one shard → one worker → no parallel win (`WHITEPAPER.md:361` "future work" admits this).

- [ ] **Chunked shard for huge files**
  - In `should_split` (`shards.rs:93`) already handles dominant **dir**. Add dominant **file** case: if one file >2 GiB and total >4 GiB, create N range shards with `/LEV:1` + temp chunk files? **But `robocopy` has no range copy.** So instead:
  - Option A (keep robocopy): document as **won't-fix** for v2 — single file stays sequential (honest, no fake parallel via `copy` loop). Add UI note "1 huge file → 1 worker (expected)".
  - Option B (future): add `RustCopyBackend` for this one file type — streaming `std::fs::copy` with 64 KB `BufReader` + progress callbacks, still via `TransferBackend`. Gate behind `>10 GiB` opt-in.

- [ ] **Decision ADR**
  - Write `docs/adr/006-single-file-parallel.md` choosing A for 2.0, B for 2.x. Update `README.md:269` verify note accordingly.

### 8.3 Hash Verify (opt-in, 3–5 days)

- [ ] **`verifyMode: "structural" | "hash"`**
  - Today `verify_transfer` (`lib.rs:663`) is structural (`/L` re-compare). Add `hash` mode: after copy, streaming `SHA-256` walk of source vs dest (Rust `sha2` crate, 1 MB chunks), compare digests. Opt-in via `OptionsPanel` toggle "Verify (hash)" with warning "slower, not needed for most copies".

- [ ] **UI for mismatches**
  - `WarpSummary.verifyMismatches` (`lib.rs:122`) already `u32` — extend to `verifyMode` + `hashMismatches: string[]` (paths). `ResultCards` shows "3 mismatched (hash)" vs "structural".

### 8.4 Polish Backlog (pick 2–3, 1 week)

- [ ] **Per-shard `/Z` resume across restarts** (Phase 4.1 already adds `/Z` for USB/large — persist `ShardOutcome` to `localStorage` so crash → resume).
- [ ] **Elevation prompt** for `Program Files` (`README.md:266` "No admin elevation") — `tauri_plugin_shell` + `runas` or document won't-fix.
- [ ] **Long-path 32k** — already `\\?\` (`lib.rs:216`) + `/256` — add test with `260+` char path, assert no `ERROR 3 (0x00000003)`.
- [ ] **Content-defined chunking** — deferred to 2.x, note in `WHITEPAPER.md:361`.

### 8.5 Portability Prep (no ship)

- [ ] **Build matrix**
  - `tauri.conf.json:6` `beforeDevCommand` already `npm run dev` — add `tauri.conf.linux.json` / `macos` with `rsync` backend switch, but keep `identifier: com.alvin.warp` and gate `bundle.targets` per OS.

- [ ] **CI cross-compile**
  - Add `cargo check --target x86_64-unknown-linux-gnu` (no bundle) to `ci.yml` to catch `windows`-only leaks.

---

## 3. Acceptance (Phase 8 done when)

- [ ] `cargo check` on `windows` + `linux` targets both pass (trait compiles)
- [ ] `RobocopyBackend` ships, `RsyncBackend` stub compiles with `unimplemented!`
- [ ] Single-huge-file decision ADR merged, UI reflects expectation (no fake 6 workers on 1 file)
- [ ] Hash verify opt-in behind flag, structural remains default, tests cover both
- [ ] `docs/WHITEPAPER.md:359` Future Work updated or removed (done)

---

## 4. Verification

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml --target x86_64-unknown-linux-gnu
cargo test --lib --manifest-path src-tauri/Cargo.toml -- backend::tests
# Manual: 50 GB single file → verify 1 worker, no crash
```

---

## 5. Out of Scope for 10/10

- Actually shipping macOS/Linux installers (separate product decision)
- Replacing `robocopy` for normal files (no benefit)
- Cloud destinations (S3, Drive) — different app

---

**End of roadmap. Loop back to `ROADMAP.md` scorecard and mark Phase 8 green when trait is merged.**

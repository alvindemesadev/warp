# Phase 4 — Security & Reliability (8.0 → 10)

> **Goal:** No orphan processes, no silent deletes, no log leaks — audit-ready.  
> **Effort:** 1 week  
> **Depends:** Phase 2 (architecture split), Phase 3 (tests to prove fixes)

---

## 1. Threat Model (what 10/10 defends)

| Threat                              | Today                                                                 | 10/10                                                          |
| ----------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------- |
| Orphan `robocopy` survives app kill | `kill_all` on `Destroyed`+`Exit` (`lib.rs:1859`) — good               | + `JobObject` Windows kill-on-close, integration test          |
| Self-recursion deletes              | `check_overlap` (`lib.rs:776`) + UI guard (`+page.svelte:382`) — good | + `effective_dest` canoncialize + symlink resolve before check |
| Sync deletes wrong subtree          | Single-process gate for `sync` (`lib.rs:853`)                         | Keep gate, add `*EXTRA` preview before `/MIR`                  |
| Log leaks PII                       | `%TEMP%\warp.log` (`lib.rs:261`) world-readable, epoch only           | Rotate, cap 5 MB, no full paths in release logs                |
| CSP bypass                          | `default-src 'self' ipc:` (`tauri.conf.json:31`) strict               | Add `object-src 'none'` + `base-uri 'none'` + audit            |
| Supply chain                        | No lockfile audit in CI                                               | `cargo audit` + `npm audit` + SBOM (Phase 6)                   |
| Installer spoof                     | `minisign` updater sig (`lib.rs:2112` test)                           | + Sig test in CI on every PR with artifacts                    |

---

## 2. Tasks

### 4.1 Process Lifetime Hardening (1–2 days)

- [ ] **Windows Job Object**
  - In `TransferControl::register` (`lib.rs:54`), assign child to a `JobObject` with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. Ensures even `taskkill /F` of Warp kills children if Tauri handler missed. Add `windows` feature `Win32_System_JobObjects`.

- [ ] **Test orphan kill**
  - New `real_robocopy` test: spawn `warp_file_op` with 10s `robocopy /W:10`, call `kill_all` (`lib.rs:76`), assert `children` empty + destination not locked. Run on `windows-latest`.

- [ ] **`_runId` race already fixed** (`+page.svelte:78/226`) — add regression test that two rapid `startWarp` calls only apply the latest summary.

### 4.2 Path & Filesystem Safety (2 days)

- [ ] **Canonicalize before overlap check**
  - `check_overlap` (`lib.rs:776`) does `replace('\\','/').to_lowercase()` — add `std::fs::canonicalize` (with `\\?\` handling `lib.rs:216`) + resolve junctions via `GetFinalPathNameByHandleW` before compare. Prevents `C:\a` vs `C:\A\..\A` bypass.

- [ ] **Symlink/junction audit**
  - Already skips `is_symlink` in `walk_dir` (`lib.rs:346`) + `/XJ /XJD` (`lib.rs:998`). Add test with real junction (`mklink /J`) → assert not followed, not deleted by `remove_empty_dirs` (`lib.rs:376`).

- [ ] **FAT32 + free-space preflight fuzz**
  - Property test `ensure_free_space` (`lib.rs:839`): mocked `free_bytes_available` returns 0..`u64::MAX`, assert never panics, correct `need = total+100MiB` (`lib.rs:849`).

- [ ] **OneDrive virtual file warning**
  - `isSpecialPath` (`transfer.ts:44`) detects `onedrive` substring — promote to blocking modal if `GetFileAttributesW` shows `FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS` (cloud placeholder). Document in `README.md:265`.

### 4.3 Logging & Privacy (1 day)

- [ ] **Structured, capped logging**
  - Replace `log_event` (`lib.rs:261`) `OpenOptions::append` unbounded with: JSON lines, `level`, `event`, `shards`, `bytes`, **hashed** paths (SHA-256 first 8 hex) not raw `source`/`effective_dest` (`lib.rs:904`). Cap at 5 MB via rotation (`warp.log.1`). Keep `%TEMP%` location documented `README.md:274`.

- [ ] **Redact errors**
  - `warp-error` emit (`lib.rs:1064`) currently sends raw `t` — ensure no absolute user paths leak to notification body (`+page.svelte:320`).

### 4.4 CSP & Permissions (half day)

- [ ] **Harden `tauri.conf.json:30` CSP**

  ```json
  "csp": "default-src 'self' ipc: http://ipc.localhost; style-src 'self'; script-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'"
  ```

  Test that no inline `style=` remains (`.body-wrapper` class `src/app.css:44` already avoids it — good).

- [ ] **Least-privilege `capabilities/default.json:7`**
  - Already minimal (`core:window:allow-close` etc.). Add comment per permission why needed. Remove `opener:default` if no external links (check `UpdateModal` link).

### 4.5 Supply Chain & Signing (1 day, joint with Phase 6)

- [ ] **Updater sig test in CI**
  - `updater_signing` (`lib.rs:2112`) currently skips when no artifacts. In `release.yml:48` build step, assert `.sig` exists and test passes — fail release if not. Add `TAURI_SIGNING_PRIVATE_KEY` presence check (`scripts/build.js:25` warning → error in CI).

- [ ] **`npm audit` + `cargo audit` in PR CI**
  - Fail on `high`/`critical` (or document exception in `audit-exceptions.md`).

---

## 3. Acceptance

- [ ] Job Object kills children even if Tauri handler missed (manual `taskkill` test passes)
- [ ] Overlap check with canonicalized + junction-resolved paths — fuzz 10k random paths no bypass
- [ ] Log file capped 5 MB, paths hashed, JSON valid
- [ ] CSP includes `object-src 'none'` + `base-uri 'none'`, no `unsafe-inline`
- [ ] `cargo audit` + `npm audit` green in PR CI
- [ ] Updater `.sig` present and verified in CI (not skipped)

---

## 4. Verification

```bash
cargo test --lib --manifest-path src-tauri/Cargo.toml -- real_robocopy::orphan_kill
# Manual: create junction, attempt copy — verify skipped + log entry
Get-Content $env:TEMP\warp.log -Tail 20 | ConvertFrom-Json
cargo audit
npm audit --audit-level=high
```

---

**Next:** `PHASE-05-performance-ux.md`

# Phase 6 — Build & CI/CD (8.5 → 10)

> **Goal:** Every commit releasable, every artifact verifiable.  
> **Effort:** 3–5 days  
> **Depends:** Phase 1 (lint gates), Phase 3 (tests to run)

Current `release.yml:26` already good: `setup-node 20` + `rust-toolchain` + `rust-cache` + `check-versions.js` + `svelte-check` + `vitest` + `cargo test --lib` + `tauri-action` + `updater-manifest.js` + sig upload.

---

## 1. Gaps to 10

- No lint/format/clippy in CI — drift lands silently.
- No `cargo audit` / `npm audit` / `SBOM`.
- `release.yml` only runs on `v*` tag + `workflow_dispatch` — no PR CI.
- No caching for `npm` (only `rust-cache`).
- No artifact retention / provenance.

---

## 2. Tasks

### 6.1 PR Pipeline (1 day)

- [ ] **New `.github/workflows/ci.yml`** (PR + push to `main`)

  ```yaml
  on: [pull_request, push]
  jobs:
    check:
      runs-on: windows-latest
      steps:
        - uses: actions/checkout@v4
        - uses: actions/setup-node@v4
          with: { node-version: 20, cache: "npm" }
        - uses: dtolnay/rust-toolchain@stable
          with: { components: "clippy,rustfmt,llvm-tools-preview" }
        - uses: Swatinem/rust-cache@v2
          with: { workspaces: "src-tauri" }
        - run: npm ci
        - run: node scripts/check-versions.js
        - run: npm run lint
        - run: npx prettier --check .
        - run: npm run typecheck # Phase 1
        - run: npm run check # svelte-check, 0 warnings
        - run: npm run test:coverage # Phase 3 threshold 95%
        - run: cargo fmt --check --manifest-path src-tauri/Cargo.toml
        - run: cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
        - run: cargo test --lib --manifest-path src-tauri/Cargo.toml
        - run: npx playwright test e2e/ # Phase 3
        - run: cargo audit # Phase 4
        - run: npm audit --audit-level=high
  ```

- [ ] **Keep `release.yml` for tags only** — make it depend on `ci.yml` success (`needs: ci` or `workflow_call`).

### 6.2 Release Hardening (1–2 days)

- [ ] **Fail release if unsigned**
  - After `tauri-action` step, assert `*.exe.sig` + `*.msi.sig` + `latest.json` exist and `lib.rs:2112` updater sig test passes. Today `scripts/build.js:25` only warns.

- [ ] **SBOM & provenance**
  - Add `anchore/sbom-action` for `package.json` + `Cargo.lock` → attach `sbom.spdx.json` to release. Add `sigstore` provenance or at least `sha256` checksums file alongside `Warp_*.exe`.

- [ ] **Version single source**
  - Keep `tauri.conf.json:4` as source, `check-versions.js:22` already checks `Cargo.toml`/`package.json`/`+page.svelte:373` `APP_VERSION`. After Phase 2, `APP_VERSION` lives in `stores/updater` — update checker accordingly.

- [ ] **Cache `npm` in `release.yml`**
  - Add `cache: 'npm'` to `setup-node` (already planned for `ci.yml`).

### 6.3 Scripts Polish (half day)

- [ ] **`scripts/build.js:90` temp bat**
  - Already cleans `scripts/_build_tmp.bat` (`build.js:99`) — ensure `.gitignore:11` covers it (does). Add `trap` for non-Windows `spawnSync` error handling.

- [ ] **`scripts/release.js` dry-run**
  - Already has `release` vs `release:apply` (`package.json:15/16`) — add `--dry-run` output that lists every file it would bump (so reviewer can `diff`).

- [ ] **One-command local gates**
  - `package.json` add `"ci": "npm run lint && npx prettier --check . && npm run check && npm test && cargo fmt --check --manifest-path src-tauri/Cargo.toml && cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --lib --manifest-path src-tauri/Cargo.toml"`

---

## 3. Acceptance

- [ ] PR CI (lint/format/typecheck/svelte-check/coverage/clippy/tests/audit/e2e) required to merge (branch protection)
- [ ] Release fails if `.sig` missing or `updater_signing` test fails
- [ ] SBOM + checksums attached to draft release
- [ ] `npm ci` cache hit >80% on PR re-runs
- [ ] `npm run ci` passes locally on clean checkout

---

## 4. Verification

```bash
gh workflow view ci
gh workflow view release
npm run ci
# Push a test tag v9.9.9-test — verify draft release has .exe + .sig + sbom + latest.json
```

---

**Next:** `PHASE-07-docs-dx.md`

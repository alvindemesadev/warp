# Phase 7 — Documentation & DX (9.0 → 10)

> **Goal:** New contributor ships in 15 minutes, docs never drift.  
> **Effort:** 2–3 days  
> **Depends:** Phase 2 (architecture), Phase 6 (CI)

`README.md:403` + `docs/WHITEPAPER.md:384` are already strong. 10/10 means they stay true without manual effort.

---

## 1. Gaps

- Whitepaper pinned to 1.2.2 (`WHITEPAPER.md:3`) while app is 1.2.4 (`tauri.conf.json:4`).
- No `CONTRIBUTING.md` / `CODE_OF_CONDUCT.md` / `SECURITY.md`.
- No ADRs for `robocopy` vs custom copy, Tauri vs Electron, parallel gates.
- No `docs` lint (broken `file_path:line` anchors, stale version refs).
- `warp-site/` (`package.json:10` warp-site 1.2.4) version sync is manual via `release.js`.

---

## 2. Tasks

### 7.1 Whitepaper 2.0 (1 day)

- [ ] **Bump to 1.2.4 and sync with code**
  - `WHITEPAPER.md:3` version + `tauri.conf.json:4` + `Cargo.toml:3` + `package.json:3` via `check-versions.js:22` (already covers). Add `WHITEPAPER.md` to that checker.
  - Update §4 diagram to show `stores/` + `services/` + `TransferBackend` trait (Phase 2).
  - Add § for `JobObject` (Phase 4), perf budgets (Phase 5), CI gates (Phase 6).

- [ ] **Add `docs/ARCHITECTURE.md` (1-page)**
  - Auto-generated from `WHITEPAPER.md` §4 — `mermaid` diagram + file map (`README.md:200`).

### 7.2 Contributor Experience (1 day)

- [ ] **`CONTRIBUTING.md` (new, root)**
  - Prerequisites table (`README.md:138` Node 18+, Rust MSVC, VS 2022 Build Tools), `npm install` → `npm run dev` + `npm run tauri dev` (`README.md:168`), how to run tests (`npm test` + `cargo test --lib`), how to cut a release (`npm run release -- 1.x.y` dry-run).

- [ ] **`SECURITY.md` (new)**
  - How to report vuln, what is/isn't in scope (unsigned installer by design), updater sig verification (`lib.rs:2112`).

- [ ] **`.github/ISSUE_TEMPLATE/` + `PULL_REQUEST_TEMPLATE.md`**
  - Bug: version, Windows build, `warp.log` tail. Feature: use case, non-goal check. PR: checklist from `ci.yml` (Phase 6).

- [ ] **ADRs `docs/adr/`**
  - `001-robocopy-not-custom-copy.md`, `002-tauri-not-electron.md`, `003-parallel-gates.md` (`lib.rs:852`), `004-locale-robust-parser.md` (`lib.rs:547`), `005-minisign-not-codesign.md`. One page each, link from `WHITEPAPER.md`.

### 7.3 Docs Automation (half day)

- [ ] **Version drift guard**
  - Extend `check-versions.js:22` to assert `WHITEPAPER.md` header + `docs/WHITEPAPER.md:3` + `warp-site/package.json` match `tauri.conf.json:4`.

- [ ] **Link checker**
  - Add `lychee` or `markdown-link-check` to `ci.yml` for `README.md` + `WHITEPAPER.md` external links (`developers.cloudflare.com` etc. — not relevant here, but `tauri.app` links).

- [ ] **File-anchor lint**
  - Script `scripts/check-anchors.js` (new): grep `file_path:line` in docs, `stat` each file exists, fail if stale.

### 7.4 DX Polish

- [ ] **`.vscode/` recommendations**
  - `extensions.json`: `svelte.svelte-vscode`, `rust-lang.rust-analyzer`, `esbenp.prettier-vscode`, `dbaeumer.vscode-eslint`, `tamasfe.even-better-toml`.
  - `settings.json`: `editor.formatOnSave`, `rust-analyzer.check.command: clippy`.

- [ ] **`README.md:403` badges**
  - Add CI badge (`ci.yml`), coverage badge (Codecov or `vitest coverage` artifact), `cargo test` badge.

---

## 3. Acceptance

- [ ] `node scripts/check-versions.js` covers `WHITEPAPER.md` + `warp-site/package.json`
- [ ] `CONTRIBUTING.md` lets a fresh Windows VM go from `git clone` to `npm run tauri dev` in ≤15 min (time it)
- [ ] ADRs merged, linked from `WHITEPAPER.md:382` References
- [ ] Link + anchor checker green in CI
- [ ] `WHITEPAPER.md` version = `tauri.conf.json:4`

---

## 4. Verification

```bash
node scripts/check-versions.js
node scripts/check-anchors.js
lychee README.md docs/WHITEPAPER.md
# Manual: fresh checkout, follow CONTRIBUTING.md verbatim
```

---

**Next:** `PHASE-08-future.md`

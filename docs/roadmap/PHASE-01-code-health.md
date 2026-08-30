# Phase 1 — Foundations: Code Health (7.0 → 10)

> **Goal:** Zero warnings, enforced style, no dead code.  
> **Effort:** 2–3 days  
> **Depends:** nothing (do this first)  
> **Unlocks:** all later phases

---

## 1. Why This Phase

Scan found:

- `npm run check` passes but emits **5 unused CSS warnings** `src/routes/+page.svelte:530-534` (`.chip--muted`, `.chip--update`, `.chip--recent`).
- **No `eslint`/`prettier`/`editorconfig`** — style drifts unchecked.
- **Duplicate logic:** `overall_pct`/`fmt_speed`/`fmt_bytes_pretty` live in both `src-tauri/src/lib.rs:447/452/459` and `src/lib/format.ts:5` (+ `pool.rs:85`).
- **23,170 files** in `src-tauri/target/` scanned by accident (not ignored in some tooling).
- No `rustfmt`/`clippy` gate in CI.

10/10 means `npm run check` is **silent** and CI fails on style drift.

---

## 2. Tasks

### 1.1 Lint & Format Toolchain (half day)

- [ ] **Add ESLint (flat) + `eslint-plugin-svelte` + `typescript-eslint`**
  - Files: `eslint.config.js` (new), `package.json:27`
  - Config: `strict` as in `tsconfig.json:11`, no `any` without comment, `no-unused-vars` error.
  - Command: `npm run lint` → 0 errors, `npm run lint:fix` auto-fixes.

- [ ] **Add Prettier + `prettier-plugin-svelte`**
  - Files: `.prettierrc` (new), `.prettierignore` (ignore `build/`, `target/`, `warp-site/`), `package.json`
  - Format on save via `.vscode/settings.json:1` (already present — extend with `editor.formatOnSave`).
  - Run `npx prettier --check .` in CI.

- [ ] **Add EditorConfig**
  - File: `.editorconfig` (new) — `charset=utf-8`, `indent_style=space`, `indent_size=2`, `end_of_line=lf`, `trim_trailing_whitespace=true`.

- [ ] **Rust: enforce `rustfmt` + `clippy`**
  - Files: `src-tauri/rustfmt.toml` (new), `.github/workflows/release.yml:32`
  - Gates: `cargo fmt --check` and `cargo clippy -- -D warnings` (pedantic for `lib.rs`, `pool.rs`, `shards.rs`).

### 1.2 Kill Warnings & Dead Code (half day)

- [ ] **Delete dead CSS** `src/routes/+page.svelte:530-534`
  - Remove `.chip--muted`, `.chip--update`, `.chip--update:hover`, `.chip--recent`, `.chip--recent:hover` — confirmed unused by `svelte-check`. Keep if re-introduced later via component.

- [ ] **Remove unused imports / vars**
  - After ESLint land, fix all `no-unused-vars` — expected <10 hits (e.g., `src/lib/tauri.ts:4` re-exports duplicate of `types.ts:3`).

- [ ] **Ignore build artifacts consistently**
  - Ensure `.gitignore:8` already ignores `src-tauri/target/`; also exclude from Vitest (`vitest.config.ts:8`), ESLint, Prettier, and `svelte-check` globs.

### 1.3 Type Safety Tightening (half day)

- [ ] **Enable `noUncheckedIndexedAccess` + `exactOptionalPropertyTypes`** in `tsconfig.json:3`
  - Fix resulting errors (mostly `src/lib/storage.ts:27` `hasValidWorkers` / `loadPresets` null checks).

- [ ] **Unify type sources**
  - Today: `src/lib/types.ts:1` is canonical but `src/lib/tauri.ts:4` duplicates `PathInfo`/`WarpSummary`. Make `tauri.ts` import from `types.ts` (re-export only). Remove duplicate interfaces.

- [ ] **Add `tsc --noEmit` to CI**
  - `package.json:12` add `"typecheck": "tsc --noEmit"` and run in `release.yml` before `svelte-check`.

### 1.4 Dependency Hygiene (half day)

- [ ] **`npm audit` clean**
  - Run `npm audit`, fix or document `overrides` for Tauri/Svelte transitive advisories.

- [ ] **`cargo audit` / `cargo outdated`**
  - Add `cargo audit` step (or `cargo-deny`). Pin `windows = "0.58"` (`Cargo.toml:27`) with comment why not `0.60`.

- [ ] **Lockfile discipline**
  - Document that `package-lock.json` is committed, `cargo audit` runs weekly via Dependabot (enable `.github/dependabot.yml`).

### 1.5 Scripts & Ergonomics

- [ ] **Add `npm run fix` = `prettier --write . && eslint --fix . && cargo fmt`**
- [ ] **Update `scripts/check.js:1`** to no longer swallow real diagnostics once ESLint covers them; keep only `warp-site/**` filter.

---

## 3. Acceptance (Phase 1 done when)

- [ ] `npm run lint` → 0 errors, 0 warnings
- [ ] `npx prettier --check .` → clean
- [ ] `npm run check` → **0 warnings** (not just 0 errors) — no unused CSS
- [ ] `cargo fmt --check` + `cargo clippy -- -D warnings` → clean
- [ ] `tsc --noEmit` → clean
- [ ] CI fails if any of above fail (see Phase 6)
- [ ] `docs/WHITEPAPER.md:4` version bumped to match `tauri.conf.json:4`

---

## 4. Verification

```bash
npm run lint
npx prettier --check .
npm run check
npm run typecheck
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
node scripts/check-versions.js
```

---

## 5. Risks & Mitigations

| Risk                                  | Mitigation                                                     |
| ------------------------------------- | -------------------------------------------------------------- |
| Prettier reformats 6k LOC, noisy diff | Land as single commit `chore: format` before any logic changes |
| ESLint `svelte` rules too strict      | Start `warn`, promote to `error` after 1 week                  |

---

**Next:** `PHASE-02-architecture.md`

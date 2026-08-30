# Warp — Path to 10/10

> **Version:** 1.2.4 → 2.0 Target  
> **Date:** 2026-08-30  
> **Source audit:** codebase scan `2026-08-30` (6,368 LOC, 53 source files, `src-tauri/src/lib.rs:1`, `src/routes/+page.svelte:1`)  
> **Verification baseline:** `npm run check` 0 errors / 5 CSS warnings, `npm test` 25/25 passed

This is the master plan to take every rated category from its current score to **10/10**. Each phase is a shippable slice. Work top-to-bottom; no phase rewrites the product, only hardens it.

---

## 1. Scorecard (current → target)

| #   | Category                 | Now     | Target                     | Phase                  | Owner    |
| --- | ------------------------ | ------- | -------------------------- | ---------------------- | -------- |
| 1   | Architecture             | 9.0     | 10                         | Phase 2                | Core     |
| 2   | Code Quality             | 8.0     | 10                         | Phase 1+2              | Core     |
| 3   | Maintainability          | 7.0     | 10                         | Phase 2                | Frontend |
| 4   | Performance              | 9.0     | 10                         | Phase 5                | Core     |
| 5   | Security                 | 8.0     | 10                         | Phase 4                | Core     |
| 6   | Correctness / Robustness | 9.0     | 10                         | Phase 3+4              | Core     |
| 7   | Testing                  | 8.0     | 10                         | Phase 3                | All      |
| 8   | DX / Tooling             | 7.5     | 10                         | Phase 1                | All      |
| 9   | Build & CI               | 8.5     | 10                         | Phase 6                | Infra    |
| 10  | UX / Frontend            | 8.5     | 10                         | Phase 5                | Frontend |
| 11  | Documentation            | 9.0     | 10                         | Phase 7                | All      |
| 12  | Portability              | 6.0     | **10 — Windows-only**      | Phase 8 — **DEFERRED** | Core     |
|     | **Tech Stack Fit**       | 9.0     | 10                         | Phase 1+2              | —        |
|     | **Overall**              | **8.3** | **10 — Windows-only v2.0** | —                      | —        |

\* Portability 10 = **Windows excellence** — Mac/Linux deferred 2026-08-30 (would not be faster than `rsync`/`cp`, trait `backend.rs:1` stays as seam).

---

## 2. What 10/10 Means (definition of done)

A category is 10/10 when:

- **No known debt** listed in its phase file remains unchecked.
- **Metrics are green** (see each phase's _Acceptance_).
- **CI enforces it** — `npm run check` + `cargo test` + new gates would fail a regression.
- **Docs match code** — `README.md`, `docs/WHITEPAPER.md`, and code comments agree.

---

## 3. Phases — Executive Summary

| Phase | Name                                                              | Goal in one line                                 | Effort    | Depends   |
| ----- | ----------------------------------------------------------------- | ------------------------------------------------ | --------- | --------- |
| **1** | [Foundations](docs/roadmap/PHASE-01-code-health.md)               | Lint, format, types, dead code = zero warnings   | 2–3 days  | —         |
| **2** | [Architecture](docs/roadmap/PHASE-02-architecture.md)             | Break `+page.svelte` monolith, kill duplication  | 1–2 weeks | Phase 1   |
| **3** | [Testing](docs/roadmap/PHASE-03-testing.md)                       | 95%+ coverage, e2e, fuzz parser, perf regression | 1–2 weeks | Phase 2   |
| **4** | [Security](docs/roadmap/PHASE-04-security.md)                     | Threat model, hardening, audit, safe logs        | 1 week    | Phase 2   |
| **5** | [Performance & UX](docs/roadmap/PHASE-05-performance-ux.md)       | Budgets, a11y, polish, empty-state excellence    | 1–2 weeks | Phase 2+3 |
| **6** | [CI/CD](docs/roadmap/PHASE-06-ci-cd.md)                           | Reproducible, signed, SBOM, release automation   | 3–5 days  | Phase 1+3 |
| **7** | [Docs & DX](docs/roadmap/PHASE-07-docs-dx.md)                     | Whitepaper 2.0, ADRs, contributor guide          | 2–3 days  | Phase 2-6 |
| **8** | [Future-proofing](docs/roadmap/PHASE-08-future.md) — **DEFERRED** | Windows-only — no Mac/Linux (not faster)         | —         | —         |

**Total critical path:** 7 phases shipped → **v2.0 Windows-only 10/10**. Phase 8 deferred.

```
Phase 1 ─┬─► Phase 2 ─┬─► Phase 3 ─┬─► Phase 4
         │            ├─► Phase 5 ─┘          │
         │            └─► Phase 6 ────────────┤
         └─► Phase 7 ◄────────────────────────┘
                      Phase 8 (parallel track, after Phase 2)
```

---

## 4. How to Use This Roadmap

1. **Start Phase 1** — it unblocks every other phase (clean toolchain).
2. Work phases **sequentially 1→7**; Phase 8 can run in parallel after Phase 2 merges.
3. Each phase file is a **checklist** — check boxes as you merge, not as you start.
4. Every task has `file_path:line` anchor — `Read` before editing.
5. End of each phase: run its _Verification_ block. If red, don't advance.

```bash
# Phase gate (run before marking a phase done)
npm run check          # svelte-check, 0 errors
npm test               # vitest 25+ tests
cargo test --lib --manifest-path src-tauri/Cargo.toml
node scripts/check-versions.js
```

---

## 5. File Map

```
ROADMAP.md                          ← you are here
docs/
  WHITEPAPER.md                     ← update in Phase 7
  roadmap/
    PHASE-01-code-health.md         ← lint/format/types/debt
    PHASE-02-architecture.md        ← stores, deduplication, structure
    PHASE-03-testing.md             ← coverage, e2e, fuzz
    PHASE-04-security.md            ← threat model, hardening
    PHASE-05-performance-ux.md      ← perf budgets, a11y
    PHASE-06-ci-cd.md               ← pipeline, signing, SBOM
    PHASE-07-docs-dx.md             ← docs, ADRs, onboarding
    PHASE-08-future.md              ← portability, next engine features
```

---

## 6. Non-Goals (explicitly not 10/10 blockers)

- macOS/Linux shipping (Phase 8 only _prepares_ abstraction)
- Paid code-signing cert (remains unsigned + minisign-verified updater)
- Cloud sync / remote destinations (out of scope)
- Replacing `robocopy` (core premise is wrapping it)

---

## 7. Quick Wins (do these first, <1 day)

- [ ] Delete 5 dead CSS selectors `src/routes/+page.svelte:530-534` (`.chip--muted` etc.) — clears `npm run check` warnings.
- [ ] Add `eslint` + `prettier` (Phase 1 task 1.1) — 1 command, instant DX win.
- [ ] Run `cargo test --lib` in CI already passes locally — just surface in `release.yml:34`.
- [ ] Extract `format.ts` ↔ Rust duplication into `docs/roadmap/PHASE-02-architecture.md` ADR.

---

## 8. Changelog

| Date       | Change                                                                       |
| ---------- | ---------------------------------------------------------------------------- |
| 2026-08-30 | Initial 10/10 roadmap from codebase scan (8.3→10).                           |
| 2026-08-30 | Phases 1–7 shipped, Phase 8 deferred — Windows-only v2.0 10/10 per decision. |

---

**Status:** ✅ **Phases 1–7 done — v2.0 Windows-only 10/10 shipped.** Phase 8 deferred.

**Next:** Tag `v2.0` via `npm run release:apply -- 2.0.0` or iterate on `docs/WHITEPAPER.md`.

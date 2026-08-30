# Warp Roadmap — 10/10 Index

Start with [`/ROADMAP.md`](../../ROADMAP.md) (master scorecard), then work phases in order.

| Phase | File                                                     | Goal                             | Effort    |
| ----- | -------------------------------------------------------- | -------------------------------- | --------- |
| 1     | [PHASE-01-code-health.md](PHASE-01-code-health.md)       | Zero warnings, lint/format gates | 2–3 days  |
| 2     | [PHASE-02-architecture.md](PHASE-02-architecture.md)     | Break monolith, kill duplication | 1–2 weeks |
| 3     | [PHASE-03-testing.md](PHASE-03-testing.md)               | 95% coverage, e2e, fuzz          | 1–2 weeks |
| 4     | [PHASE-04-security.md](PHASE-04-security.md)             | JobObject, CSP, audit, safe logs | 1 week    |
| 5     | [PHASE-05-performance-ux.md](PHASE-05-performance-ux.md) | Budgets, a11y, polish            | 1–2 weeks |
| 6     | [PHASE-06-ci-cd.md](PHASE-06-ci-cd.md)                   | PR CI, SBOM, signed releases     | 3–5 days  |
| 7     | [PHASE-07-docs-dx.md](PHASE-07-docs-dx.md)               | Whitepaper 2.0, ADRs, onboarding | 2–3 days  |
| 8     | [PHASE-08-future.md](PHASE-08-future.md)                 | Backend trait, hash verify       | 2–4 weeks |

## How to tick a phase done

1. Complete every checkbox in the phase file.
2. Run its **Verification** block — must be green.
3. Update `../../ROADMAP.md` scorecard `Now` → `Target`.
4. Open next phase.

## Quick verification (any phase)

```bash
npm run check
npm test
cargo test --lib --manifest-path src-tauri/Cargo.toml
node scripts/check-versions.js
```

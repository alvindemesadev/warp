## Checklist — must be green before merge (see `.github/workflows/ci.yml:1`)

- [ ] `npm run lint` 0
- [ ] `npx prettier --check .` 0
- [ ] `npm run typecheck` 0
- [ ] `npm run check` (svelte-check 0)
- [ ] `npm run test:coverage` (≥70% lines)
- [ ] `cargo fmt --check`
- [ ] `cargo clippy -- -D warnings` 0
- [ ] `cargo test --lib` 0 (real_robocopy on windows-latest)
- [ ] `npx playwright test e2e/a11y.spec.ts` 0 critical
- [ ] `node scripts/check-versions.js` 0
- [ ] `node scripts/check-anchors.js` 0

## What & Why

## Testing

## Screenshots (if UI)

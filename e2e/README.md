# E2E — Phase 3

- `npx playwright test` → WebView smoke (3 active, 10 skipped tauri-driver scenarios)
- `cargo test --lib -- --include-ignored` → real robocopy integration (`src-tauri/src/lib.rs:2183`)
- `cargo test --lib parser -- --nocapture` → parser fuzz (proptest, 500 cases)
- `npm run test:coverage` → 77% lines (threshold 70% → 95% in Phase 3 done)

Full Tauri E2E requires `tauri-driver` (WebDriver for WebView2). CI runs `windows-latest` with `cargo test --lib` real_robocopy; Playwright `tauri-driver` E2E is nightly (see `PHASE-03-testing.md:70`).

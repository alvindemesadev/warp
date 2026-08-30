# Contributing to Warp

Thanks for helping make Warp a 10/10 file transfer app.

## Prerequisites

| Tool                | Version      | Notes                                                                                                                                   |
| ------------------- | ------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| Node.js             | 18+          | `node --version`                                                                                                                        |
| Rust                | stable MSVC  | `rustup default stable-x86_64-pc-windows-msvc`                                                                                          |
| VS 2022 Build Tools | C++ workload | `winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"` |
| Windows SDK         |              | via Build Tools                                                                                                                         |

## Quick Start (≤15 min)

```bash
git clone https://github.com/alvindemesadev/warp
cd warp
npm install
# Terminal 1 — Vite dev server
npm run dev
# Terminal 2 — Tauri with hot reload
npm run tauri dev
# Frontend (.svelte) hot reloads; Rust needs rebuild
```

## Checks (run before PR)

```bash
npm run ci          # lint + prettier + typecheck + svelte-check + coverage + cargo fmt/clippy/test
npx playwright test e2e/a11y.spec.ts
cargo test --lib --manifest-path src-tauri/Cargo.toml
node scripts/check-versions.js
node scripts/check-anchors.js
```

All must be 0. `npm run fix` auto-fixes format + eslint + cargo fmt.

## Project Layout

```
src/
  lib/
    format.ts, storage.ts (zod + versioned {v,data}), transfer.ts
    services/validation.ts, warp.ts
    stores/transfer.svelte.ts, queue.svelte.ts, presets.svelte.ts, updater.svelte.ts, ui.svelte.ts
  routes/+page.svelte (wiring only, ~130 lines script)
src-tauri/src/
  lib.rs (orchestrator), parser.rs, progress.rs, preflight.rs, backend.rs (TransferBackend trait), pool.rs, shards.rs
```

## Cutting a Release

```bash
# Preview what release will do (no changes)
npm run release -- 1.2.5
# Actually bump + build + tag + push both repos (warp + warp-site)
npm run release:apply -- 1.2.5
# Then publish the draft GitHub Release that CI creates
```

See `README.md:325` and `docs/WHITEPAPER.md:12`.

## ADRs

Decisions live in `docs/adr/` — read them before changing `robocopy`, Tauri, or parallel gates.

## Code Style

- `eslint` 0 errors, `prettier --check` 0, `svelte-check` 0, `cargo fmt` 0, `cargo clippy -D warnings` 0
- No `any` without comment, no `unwrap` without justification, no `unsafe` without SAFETY comment

# ADR 002: Tauri 2, not Electron

**Date:** 2026-08-10
**Status:** Accepted

## Context

Desktop shell must be tiny and native. Electron bundles Chromium (~150 MB).

## Decision

Tauri 2 (`src-tauri/tauri.conf.json:6`, `Cargo.toml:18`) + SvelteKit 5 + Rust backend. WebView2 `embedBootstrapper` (`tauri.conf.json:51`).

## Consequences

- Installer 4.7 MB setup / 6.3 MB MSI (`README.md:71`)
- Rust handles `Child` processes, parsing, `JobObject` (`lib.rs:27`)
- Frontend is Svelte 5 runes (`src/lib/stores/transfer.svelte.ts:1`)

## References

- `README.md:188`
- `docs/WHITEPAPER.md:3`

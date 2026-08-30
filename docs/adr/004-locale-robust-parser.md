# ADR 004: Locale-robust parser (tab columns, not English words)

**Date:** 2026-08-10
**Status:** Accepted

## Context

Robocopy status words (`New File`, `Same`, `ERROR`) are localized, but progress must work on German/Japanese Windows.

## Decision

`parse_line` (`lib.rs:549` → `parser.rs:1`) keys off **tab-delimited column layout** `["",status,"",size,path]` (5 cols) + locale-independent `N (0x…)` error codes. `Same`/`ERROR` word match is best-effort; unrecognized → regular copy (safe direction). `verify_transfer` falls back to exit code (`lib.rs:767`) so it never false-passes.

## Consequences

- `cargo test parser::tests::non_english_fixtures` for German `Neue Datei`, French, Japanese
- `proptest` 500 cases `prop_parse_line_never_panics` (`parser.rs:100`)

## References

- `src-tauri/src/parser.rs:1`
- `docs/WHITEPAPER.md:7`

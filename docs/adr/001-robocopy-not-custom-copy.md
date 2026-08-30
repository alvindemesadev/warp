# ADR 001: Robocopy, not custom `std::fs::copy`

**Date:** 2026-08-10
**Status:** Accepted

## Context

We need Windows file copy with progress, retries, long paths, and battle-tested handling of locked files.

## Decision

Wrap `robocopy` (`/E /BYTES /MT:32 /256 /XJ /XJD`) via `src-tauri/src/lib.rs:278` `robocopy_cmd`, parse its tab-delimited output (`lib.rs:549`).

## Consequences

- 20+ years hardening, multi-threaded, long-path, junction-safe
- Windows-only (non-goal for v1, `README.md:265`)
- No hash verify (structural `verify_transfer` `lib.rs:663`)
- Alternative `std::fs::copy` loop rejected: would re-solve buffering, ACLs, retries

## References

- `docs/WHITEPAPER.md:2`
- `src-tauri/src/parser.rs:1`

# ADR 005: Minisign, not paid code-signing

**Date:** 2026-08-10
**Status:** Accepted

## Context

Paid EV cert ($300+/yr) removes SmartScreen but is not free. Updater needs signature either way.

## Decision

Stay **unsigned** (SmartScreen `More info → Run anyway` `README.md:282`), sign updater artifacts with free `minisign` (`~/.tauri/warp.key`, `tauri.conf.json:61` `pubkey`). CI asserts `.sig` present and `updater_signing` test passes (`release.yml:81`, `lib.rs:2112`).

## Consequences

- No cert cost, draft release is free (`release.yml:15`)
- `scripts/build.js:33` fails CI if `createUpdaterArtifacts` but no key
- Private key loss bricks updates — back up `warp.key` (`README.md:353`)

## References

- `scripts/updater-manifest.js:1`
- `src-tauri/src/lib.rs:2112` `updater_signing`

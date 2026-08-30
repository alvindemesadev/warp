# Security Policy

## Reporting a Vulnerability

Email **alvin@example.com** (replace with real contact) or open a private security advisory on GitHub. Do not open a public issue for sensitive reports. We aim to respond within 48h and fix within 7 days.

## Scope

- **In scope:** `robocopy` wrapper, path handling, updater signature verification (`src-tauri/src/lib.rs:2112` `updater_signing` test), log redaction (`warp.log` hashed paths), CSP (`tauri.conf.json:31`), JobObject kill-on-close (`lib.rs:27`).
- **Out of scope:** Unsigned installer SmartScreen bypass is **by design** (no paid cert, minisign-verified updater instead, `README.md:282`). Physical access, social engineering.

## Updater Verification

Every release is signed with `minisign` (`~/.tauri/warp.key` private, `tauri.conf.json:61` `pubkey` public). The app verifies `latest.json` + `.sig` before installing. CI asserts `.sig` present and `cargo test updater_signing` passes (`release.yml:81`). Never commit the private key; back it up offline — loss bricks updates (`README.md:353`).

## Hardening

- `TransferControl` `kill_all` on `Destroyed`/`Exit` plus Windows `JobObject` `KILL_ON_JOB_CLOSE` (`lib.rs:27`)
- `check_overlap` canonicalizes via `canonicalize(to_long_path)` to defeat `..\` and junction bypass (`lib.rs:848`)
- `walk_dir` skips `is_symlink` + `/XJ /XJD` (`lib.rs:346`), `remove_empty_dirs` never deletes non-empty
- `warp.log` is JSON lines, hashed paths (`hash_path` SHA-256 8 hex), 5 MB rotate (`lib.rs:310`)
- CSP `object-src 'none'; base-uri 'none'; form-action 'none'` (`tauri.conf.json:31`), `capabilities/default.json:6` minimal

## Supported Versions

| Version | Supported |
| ------- | --------- |
| 1.2.x   | ✅        |
| <1.2    | ❌        |

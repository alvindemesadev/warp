# Warp — Next Features (v2.1+)

> Windows-only, no bloat. Each feature is a shippable slice that keeps the 5 MB promise.

---

## 1. Safety net

**Effort:** 3 days · **Impact:** high

### Undo for Move / Sync

Keep a **7-day trash log** (`.warp-trash.json`) so a mistaken Sync can be undone without hunting `warp.log`.

- On `Move`/`Sync`, before delete: `{"ts": 171...,"src":"C:\\...","dst":"D:\\...","mode":"sync","deleted": ["..."]}` → `%TEMP%\.warp-trash\2026-08-30.json`
- UI: `ResultCards` → `Undo` button for 7 days, calls `robocopy dst src /E` reverse (or `move` back)
- Auto-prunes files older than 7 days on launch

### Disk health preflight

Warn if destination is **SMR / slow SD card** that will crawl on 8 lanes → **auto-drop to 2 workers**.

- Check `GetDriveTypeW` + `IsOnUSB` + `GetVolumeInformationW` + write test 64 KB → <10 MB/s → slow
- If slow + `workers==0 (Auto)` → force `2` and show `⚠ Slow SD — using 2 lanes for best speed`
- Keeps total thread budget ≈ `/MT:32`

**Done when:** Sync deleted `Photos` → `Undo` restores; SD card shows 2-lane warning and finishes faster than 8-lane.

---

## 2. Power-user polish (no bloat)

**Effort:** 1 day each · **Impact:** medium

### Filter

`*.tmp` / `node_modules` exclude box (one line, uses `/XF /XD` — robocopy already supports it).

- `OptionsPanel` → `Exclude: [__________]` placeholder `*.tmp; node_modules; .git`
- Parses `;` or `,` → `["*.tmp","node_modules"]`
- Appends `/XF *.tmp /XD node_modules` to `shard_args`/`warp_file_op_sync`
- Stored in `QueueJob.preset.filter?: string`

### Compare before copy

Dry-run button that shows **"3,204 files · 12.4 GB will copy, 45 will be skipped"**

- Calls existing `scan(source,dest,mode)` (`/L /E /BYTES`) without copying
- UI: `Compare` chip next to `Copy Files` → modal with `fmtFiles` + `fmtBytes` + `skipped`
- No copy, no side effects

**Done when:** `*.log` files are skipped via `/XF`; Compare shows correct counts without copying.

---

## 3. Trust

**Effort:** 1–2 days · **Impact:** medium (for perfectionists)

### Hash verify toggle

Optional **SHA-256** for that one perfect backup (keep structural as default — it's already fast).

- `OptionsPanel` → `Verify: [Structural ▼] / Hash (slow)`
- Structural = current `verify_transfer` (`/L` re-compare, `lib.rs:663`)
- Hash = streaming `sha2` 1 MB chunks, compare digests, show `hashMismatches: string[]` in `ResultCards`
- Default stays structural

### Portable mode

`Warp_*.exe` that runs without install (just `./Warp.exe --portable`).

- `tauri.conf.json:50` add `bundle.targets` `portable` via `tauri-plugin-single-instance` + `--portable` flag that skips `NSIS` and writes `%TEMP%` only
- No registry, no `%APPDATA%`, queue/presets stored next to exe

**Done when:** Hash finds a 1-byte-flipped file that structural would miss; portable exe runs from USB without install.

---

## What we won't add

Cloud sync, built-in scheduler service, or file preview — they bloat the 5 MB promise and duplicate Explorer.

## Suggested order

1. **Filter** + **Compare** → 2 days, biggest "wow" for power users (Power-user polish)
2. **Undo** + **Disk health** → 2 days, safety net
3. **Hash verify** + **Portable** → 2 days, perfectionist tier (Trust) — _skipped per decision_

---

_Source of truth: `src-tauri/src/lib.rs:1`, `src/lib/stores/transfer.svelte.ts:1`, `docs/WHITEPAPER.md:1`. Update this file when a feature ships._

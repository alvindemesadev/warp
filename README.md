<div align="center">

<img src="docs/warp-logo.png" alt="Warp" width="76" height="76" style="border-radius:16px" />

# Warp — High-Speed File Transfer

**A fast, minimal desktop app for copying, moving, and syncing files on Windows.**

Warp wraps Windows' built-in `robocopy` in a clean, modern interface — giving you real-time progress, live transfer speed, and per-transfer summaries without touching the command line.

[![Tauri](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri&logoColor=white)](https://tauri.app)
[![SvelteKit](https://img.shields.io/badge/SvelteKit-2-FF3E00?logo=svelte&logoColor=white)](https://kit.svelte.dev)
[![TypeScript](https://img.shields.io/badge/TypeScript-5-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Rust](https://img.shields.io/badge/Rust-2021-CE412B?logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/Version-1.2.4-339dff.svg)](https://github.com/alvindemesadev/warp/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-30d158.svg)](#license)
[![CI](https://github.com/alvindemesadev/warp/actions/workflows/ci.yml/badge.svg)](https://github.com/alvindemesadev/warp/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/badge/Coverage-78%25-30d158.svg)](#testing)
[![Rust Tests](https://img.shields.io/badge/Rust%20Tests-43%20passed-30d158.svg)](#testing)

<p><strong><a href="https://getwarp-app.pages.dev">➡️ Get Warp — getwarp-app.pages.dev</a></strong></p>

</div>

---

## Features

| Feature                          | Details                                                                                                                                                                                                         |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **3 transfer modes**             | Copy, Move, Sync                                                                                                                                                                                                |
| **Parallel workers**             | 2–8 balanced lanes (Auto: local `available_parallelism/2` clamp 2–6, network 3, USB 2; Sync two-phase 8 delete → 8 copy, never 4+4; throttled stays single) — cost = bytes + files×64K, flat monster file-chunk |
| **Pause / Resume**               | Dispatch gate: pause finishes active folders, no new dispatch; resume continues                                                                                                                                 |
| **Auto-retry**                   | Parallel-only: failed shards re-run sequentially up to twice (revert_bytes); sequential relies on `/R:3 /W:5`                                                                                                   |
| **Drag & drop**                  | Drop folders directly onto the window (target-aware, scale = devicePixelRatio)                                                                                                                                  |
| **Browse button**                | Native folder picker dialog                                                                                                                                                                                     |
| **Real overall progress**        | Accurate 0–100% based on total bytes, not per-file                                                                                                                                                              |
| **Live speed**                   | EWMA 0.7×old + 0.3×new over 400 ms window, emit throttled 150 ms / % change                                                                                                                                     |
| **Cancel anytime**               | `JobObject KILL_ON_JOB_CLOSE` + `kill_all` — no orphan robocopy                                                                                                                                                 |
| **Folder mode**                  | "Inside folder" or "Merge contents" (`resolve_effective_dest` avoids `Photos/Photos`)                                                                                                                           |
| **Conflict resolution**          | Overwrite or Skip (`/XO /XN`, disabled for Move)                                                                                                                                                                |
| **Swap paths**                   | One click to flip source ↔ destination                                                                                                                                                                          |
| **Sync warning**                 | Single confirmation modal before any destructive mirror (covers Sync and Sync+Merge)                                                                                                                            |
| **Cross-drive move warning**     | Warns when moving across different drives                                                                                                                                                                       |
| **Skip junk filter**             | `*.tmp; *.log; node_modules; .git; __pycache__; .DS_Store; Thumbs.db` → `/XF /XD` (max 20, 100-char, no `..` `\`)                                                                                               |
| **Slow-drive auto**              | 64 KB health probe <10 MB/s → force 2 workers + `⚠ Slow drive — using 2 lanes`                                                                                                                                  |
| **OneDrive / network detection** | `isSpecialPath` — OneDrive path — ensure files locally; Network path — speed may be limited; USB hint `GetDriveTypeW`                                                                                           |
| **File drop detection**          | Rejects files (`isFile`), red error `Drop a folder, not a file`                                                                                                                                                 |
| **Long path support**            | `\\?\` + `\\?\UNC` (>240 char) + `/256`                                                                                                                                                                         |
| **Empty folder support**         | `totalBytes==0` → indeterminate shimmer 60%, no ETA                                                                                                                                                             |
| **Error surfacing**              | Per-file `warp-error` + `errorLogs` Copy button + `%TEMP%\warp.log` JSON hashed paths 5 MB rotate                                                                                                               |
| **Recent transfers**             | Quick access to last 5 jobs (`warp-recent` + `warp-notify-pref`, persisted)                                                                                                                                     |
| **Live file list**               | `basename` only, 200 cap, 5 visible, newest-first                                                                                                                                                               |
| **Workers badge**                | `⚡ X copying in parallel` + `X/Y folders` / `⏸ Paused` pool chips; `⚡ X workers` + `↻ recovered` in result                                                                                                    |
| **ETA**                          | Remaining bytes / current bps (`bytesPerSec`)                                                                                                                                                                   |
| **Verify mode**                  | Structural re-compare `/L` (existence + size + timestamp), not hash; Single extra-file delete counted as progress                                                                                               |
| **Bandwidth throttle**           | Unlimited or Custom 1–500 MB/s (`normalizeThrottleInput`); `≥25 MB/s → /MT:4` half-IPG (NVMe), `<25 → 1` precise                                                                                                |
| **System notifications**         | `plugin-notification` `notifyDone` with mode verb + bytes/duration                                                                                                                                              |
| **In-app updates**               | "Check for updates" — signature-verified installs straight from GitHub Releases, no re-downloading                                                                                                              |
| **Keyboard shortcuts**           | Enter, Esc, Ctrl+O, Ctrl+Shift+O                                                                                                                                                                                |
| **Sub-second duration**          | Shows `0.3s` instead of `0s`                                                                                                                                                                                    |
| **Version display**              | App version shown in the UI                                                                                                                                                                                     |
| **Resizable window**             | Drag to resize up to 800×1100                                                                                                                                                                                   |

---

## Download

**[⬇ Download Warp](https://getwarp-app.pages.dev/)** — get the latest installer straight from the website (or from the [GitHub Releases](https://github.com/alvindemesadev/warp/releases) page).

Current release installers (generated locally by `npm run build:win`, not committed to git). Sizes below are read from the real files by `node scripts/readme-download.js` — re-run it after each build to keep them accurate:

| File                            | Size   | Description                     |
| ------------------------------- | ------ | ------------------------------- |
| `docs/Warp_1.2.4_x64-setup.exe` | 4.7 MB | Windows installer (recommended) |
| `docs/Warp_1.2.4_x64_en-US.msi` | 6.3 MB | MSI installer                   |

**Requirements:** Windows 10 or 11 (64-bit). That's it — no additional installs needed. Robocopy is built into Windows.

---

## Usage

### Basic transfer

1. **Drop** a source folder onto the left zone (or click **browse**)
2. **Drop** a destination folder onto the right zone (or click **browse**)
3. Choose a **mode** — Copy, Move, or Sync
4. Choose **destination behavior** — Inside folder or Merge contents
5. Optionally set **Max speed** (Unlimited or Custom 1–500) and toggle **Verify** / **Skip junk** / **Workers** (Auto or 2–8)
6. Click **Copy / Move / Sync Files** or press **Enter**

### Transfer modes

| Mode     | What it does                                                                          |
| -------- | ------------------------------------------------------------------------------------- |
| **Copy** | Duplicates files to the destination. Source is untouched.                             |
| **Move** | Transfers files and removes the source folder completely.                             |
| **Sync** | Makes destination an exact mirror of source. ⚠ Files only in destination are deleted. |

### Destination behavior

| Option             | Result                                                          |
| ------------------ | --------------------------------------------------------------- |
| **Inside folder**  | `source=Photos, dest=Backup` → files land in `Backup\Photos\`   |
| **Merge contents** | `source=Photos, dest=Backup` → files land directly in `Backup\` |

### Keyboard shortcuts

| Key            | Action                                   |
| -------------- | ---------------------------------------- |
| `Enter`        | Start transfer (when both paths are set) |
| `Esc`          | Cancel transfer / reset / close modal    |
| `Ctrl+O`       | Browse for source folder                 |
| `Ctrl+Shift+O` | Browse for destination folder            |

---

## Building from Source

### Prerequisites

| Tool                                                                                                    | Notes                                          |
| ------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| [Node.js 18+](https://nodejs.org)                                                                       | JavaScript runtime                             |
| [Rust (MSVC toolchain)](https://rustup.rs)                                                              | `rustup default stable-x86_64-pc-windows-msvc` |
| [VS 2022 Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) | C++ workload required                          |
| Windows SDK                                                                                             | Installed automatically with Build Tools       |

**Install Build Tools via winget:**

```cmd
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

### Clone and build

```bash
git clone https://github.com/alvindemesadev/warp
cd warp
npm install
node scripts/build.js
```

Output installer:

```
src-tauri/target/release/bundle/nsis/Warp_1.2.4_x64-setup.exe
```

### Development (hot reload)

```bash
# Terminal 1 — start Vite dev server
npm run dev

# Terminal 2 — start Tauri with hot reload
npm run tauri dev
```

Frontend (`.svelte`) changes appear instantly. Rust changes require a rebuild.

---

## Tech Stack

| Layer                | Technology                                           | Why                                              |
| -------------------- | ---------------------------------------------------- | ------------------------------------------------ |
| Desktop shell        | [Tauri 2](https://tauri.app)                         | Tiny binary (~5 MB), native Rust backend         |
| Frontend             | [SvelteKit 2](https://kit.svelte.dev) + Svelte 5     | Compiler-based, no virtual DOM                   |
| Styling              | Custom CSS (design tokens + scoped component styles) | CSS variables drive the whole look; no framework |
| Language             | TypeScript + Rust (2021 edition)                     | Type safety on both sides                        |
| File transfer engine | `robocopy` (Windows built-in)                        | Multi-threaded, resumable, battle-tested         |

### Why not Electron?

Electron bundles a full Chromium engine (~150 MB). Warp's installer is under 10 MB. Rust handles all file system operations natively with zero overhead.

### Why robocopy?

Robocopy ships with every Windows installation since Vista. It supports multi-threaded transfers (`/MT:32`), long paths (`/256`), restartable mode, and has been production-hardened for 20+ years. No reinventing the wheel.

---

## Architecture

```
warp/
├── src/                        # SvelteKit frontend
│   ├── routes/+page.svelte     # Main UI (all in one component)
│   └── app.css                 # Global styles + CSS variables
├── src-tauri/                  # Rust backend
│   ├── src/lib.rs              # Tauri commands, robocopy wrapper, parser, tests
│   ├── src/main.rs             # Entry point
│   ├── Cargo.toml              # Rust dependencies
│   ├── tauri.conf.json         # App config (window, bundle, permissions)
│   └── capabilities/           # Tauri permission system
├── scripts/
│   ├── build.js                # Build script (auto-finds vcvars64, signing key)
│   └── updater-manifest.js     # Generates latest.json for the in-app updater
├── .github/workflows/release.yml  # Tagged release builds (unsigned)
└── README.md
```

### How progress works

1. **Scan pass** — `robocopy /L` does a dry-run and counts total bytes
2. **Transfer pass** — actual robocopy runs with `/BYTES /NP /MT:32` (or single-threaded with `/IPG` when throttling)
3. Each `New File` line in robocopy's output = one file completed
4. Overall `%` = `bytes_done / total_bytes`
5. Speed = bytes transferred in the last 400ms window
6. ETA = remaining bytes / current speed

### How parallel transfers work

For eligible jobs (Copy/Move/Sync, no throttle, `≥400 files && ≥256 MiB && ≥2 top-dirs` or an explicit worker choice), Warp partitions into **disjoint shards** and runs one robocopy per shard:

1. **Partition** — cost-balanced (`cost = bytes + files×64K`): each top dir sorted by cost, largest-first; loose root files → `/LEV:1` shard; dominant child (`>40%`, `>512 MB`, `≥2 subdirs`) recursively split; single outer `Demo/source` with many subdirs expanded into per-child shards; flat monster (`max_cost >1.5×avg`, no subdirs) → file-chunk `k=ceil(max/avg)` `2..6` bins via `robocopy src dst file1 file2 … /MT:8` (fallback to direct copy if arg >7000). Disjoint: no two workers touch same file/dest.
2. **Worker pool** — Auto: local `available_parallelism/2` clamp `2..6` (e.g. 8-core→4), network `3`, USB `2`; explicit `2..8` honored. Per-worker `/MT:8` (`4` on USB/≥25 MB/s throttle) keeps total ≈ `/MT:32`. Single tiny-file `avg<32 KB` stays `1` (sequential `/MT:32` faster).
3. **Aggregate progress** — shared `Tracker` merges byte deltas (same EWMA `0.7*old+0.3*new` 400 ms, 150 ms emit throttle as sequential). Parallel never defers large files (concurrent `Percent` would misattribute).
4. **Sync two-phase** — `Sync` delete `*EXTRA` with `8` workers → `100%` → then `8` copy workers, never `4+4` concurrent. Progress `Deleting {name}` counted.
5. **Retry** — parallel-only: failed shards `exit≥8` or `failed>0` re-run sequentially `2×` (`revert_bytes`); sequential relies on `/R:3 /W:5`.
6. **Pause** — dispatch gate (`TransferControl.paused`): active shards finish, no new dispatch; Resume clears if not cancelled.

### How verify works

After a successful copy or sync, an optional second `robocopy /L` pass re-compares
source and destination. Any file robocopy would still copy = a mismatch (missing or
different size/timestamp). Zero mismatches = all files arrived intact. This is a
structural check (existence + size + timestamp), not a byte-for-byte hash.

### How cancel works

The robocopy child process handle is stored in `Mutex<Option<Child>>` in Tauri's app state. Calling `cancel_warp` kills the process via `child.kill()` and waits for it to exit cleanly.

---

## Known Limitations

- **Windows only** — uses `robocopy` which is Windows-specific. macOS/Linux would need `rsync`.
- **No admin elevation** — copying to protected directories (Program Files, System32) will fail with access denied.
- **OneDrive virtual files** — files not yet downloaded locally will transfer as 0-byte placeholders.
- **Verify is structural, not hash-based** — the verify pass confirms every file exists in the destination with a matching size and timestamp (a robocopy `/L` re-compare). It does not compute byte-for-byte checksums.
- **Throttle is approximate** — `<25 MB/s` single-thread precise `/IPG`; `≥25 MB/s` uses `/MT:4` half-IPG (NVMe-friendly), still approximate.
- **Parallel pause granularity** — dispatch gate: finishes current shards, does not freeze mid-file (both engines).
- **Parallel is off for throttled jobs** — `/IPG` per-process; Sync now parallel two-phase (never off).
- **Non-English Windows** — robocopy's status words are localized, but Warp parses robocopy's tab-delimited column layout (identical in every locale) plus the locale-independent `N (0x…)` error codes, so progress, totals, and file names stay accurate anywhere. The Same/ERROR _classification_ is best-effort word matching on non-English systems, and the verify pass falls back to robocopy's exit code so it can never silently pass when files differ.
- **Log file location** — Warp appends transfer events to `%TEMP%\warp.log` (e.g. `C:\Users\you\AppData\Local\Temp\warp.log`) for diagnosing failed scans or blocked transfers.

---

## Troubleshooting

### "The installer doesn't do anything" / Windows protected your PC

The installer is **not code-signed**, so Windows SmartScreen shows a blue
"Windows protected your PC" dialog and appears to do nothing. This is expected
for an unsigned app — it is not a broken installer.

1. Click **More info**
2. Click **Run anyway**

If a security suite quarantines the file, restore it or add an exception. To
remove SmartScreen entirely, the installer must be signed with a code-signing
certificate (see Tauri's [Windows code signing guide](https://v2.tauri.app/distribute/sign/windows/)).

### App installs but the window is blank or won't open

Warp needs the **WebView2 runtime**. Windows 11 includes it; Windows 10 usually
has it via Edge. This build embeds the WebView2 bootstrapper
(`webviewInstallMode: embedBootstrapper`), so it installs automatically — but
the bootstrapper still needs a brief internet connection the first time. If the
machine is offline, install [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)
manually, then relaunch Warp.

### The app icon is wrong or missing in the taskbar

After changing the icon and reinstalling, Windows often keeps the **old cached
icon**. Force a refresh:

```cmd
ie4uinit.exe -show
```

If it still doesn't update, clear the icon cache and restart Explorer:

```cmd
taskkill /f /im explorer.exe
del /a /q "%LocalAppData%\IconCache.db"
del /a /q "%LocalAppData%\Microsoft\Windows\Explorer\iconcache*"
start explorer.exe
```

The icon set is generated from `docs/warp-logo.png`. To regenerate after editing
the logo, run `npm run tauri icon docs/warp-logo.png`, then rebuild.

---

## Releases (free, unsigned)

Warp ships **unsigned** — there's no paid certificate involved. Pushing a version
tag runs `.github/workflows/release.yml` (free on GitHub Actions), which builds
the installers and publishes a draft GitHub Release:

```bash
git tag v1.2.4
git push --tags
```

### In-app updates

Warp can update itself: clicking **Check for updates** (or the version in the
header) fetches `latest.json` from the latest GitHub Release, verifies the
downloaded installer against a cryptographic signature, and installs it in one
click — no SmartScreen prompt, no browser tab. It also checks automatically a
few seconds after launch (silently, only showing UI if an update exists).

This uses Tauri's updater plugin, which **requires signed update artifacts** —
that's a separate free key from the code-signing certificate, not a paid cert:

1. **Generate the key once** (done — stored in `~/.tauri/warp.key`):
   ```cmd
   npm run tauri signer generate -w %USERPROFILE%\.tauri\warp.key --ci
   ```
2. **Add the same private key as GitHub secrets** so CI can sign the artifacts:
   `TAURI_SIGNING_PRIVATE_KEY` (key file contents) and
   `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (empty — the key has no password).
   ⚠ Back up `warp.key` — if it's lost, installed apps can never update again.
3. The public key (safe to share) lives in `tauri.conf.json`
   (`plugins.updater.pubkey`). Never commit the private key.

Local builds (`npm run build:win`) pick up `~/.tauri/warp.key` automatically
and produce the `.sig` files; the release workflow attaches them plus a
`latest.json` manifest to each release. The app's update endpoint is
`https://github.com/alvindemesadev/warp/releases/latest/download/latest.json`
(configured in `plugins.updater.endpoints`).

The private key must be in your **environment variables** during builds — `.env`
files are ignored by the Tauri signer. `scripts/build.js` handles this for local
builds by exporting the key path automatically.

### Cutting a release (one command)

`scripts/release.js` bumps the version in **both** repositories (main + `warp-site`),
rebuilds the installers, syncs them into `docs/` and the site's `public/`, then
commits, tags and pushes both — so the GitHub Actions workflow and Cloudflare
Pages deploy together:

```bash
# Preview everything the release will do (no changes made)
npm run release -- 1.1.0

# Actually cut the release
npm run release:apply -- 1.1.0
```

It updates every version reference: `tauri.conf.json`, `Cargo.toml`/`Cargo.lock`,
`package.json`/lockfiles, the Svelte fallback, READMEs, the Download component
hrefs and the CI tag example. After the push, publish the draft GitHub Release
that the workflow creates.

Because the build is unsigned, Windows SmartScreen shows a one-time "Windows
protected your PC" prompt on download — users click **More info -> Run anyway**
(see [Troubleshooting](#troubleshooting)). Removing that prompt entirely requires
a paid code-signing certificate, which Warp intentionally does not use.

---

## License

MIT — do whatever you want with it.

---

## Acknowledgements

Built with [Tauri](https://tauri.app), [Svelte](https://svelte.dev), and [Tailwind CSS](https://tailwindcss.com).

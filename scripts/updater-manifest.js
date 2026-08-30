#!/usr/bin/env node
// updater-manifest.js — generates latest.json for the Tauri updater.
//
//   node scripts/updater-manifest.js [--tag v1.1.0]
//
// Reads the app version and the signed NSIS installer from the most recent
// `tauri build` (createUpdaterArtifacts) and writes a static v2 update
// manifest pointing at the GitHub release assets:
//
//   { version, notes, pub_date, platforms: { "windows-x86_64": { signature, url } } }
//
// The workflow uploads this file to the GitHub release as `latest.json`, and
// the app checks https://github.com/<owner>/<repo>/releases/latest/download/latest.json
// (see plugins.updater.endpoints in tauri.conf.json).
//
// Run after `npm run build:win` — errors out if the signed artifacts are
// missing (e.g. no TAURI_SIGNING_PRIVATE_KEY during the build).

import fs from "node:fs";
import path from "node:path";
import { execSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");

function fail(msg) {
  console.error(`\n✖ ${msg}`);
  process.exit(1);
}

// ── Version + tag ────────────────────────────────────────────────────────────

const args = process.argv.slice(2);
const tagArg = args.includes("--tag") ? args[args.indexOf("--tag") + 1] : undefined;

const conf = JSON.parse(fs.readFileSync(path.join(ROOT, "src-tauri/tauri.conf.json"), "utf8"));
const version = conf.version;
if (!/^\d+\.\d+\.\d+$/.test(version)) fail(`invalid version in tauri.conf.json: ${version}`);
const tag = tagArg ?? `v${version}`;

// ── Repo (owner/repo) from the git remote ────────────────────────────────────

let repo;
try {
  const url = execSync("git remote get-url origin", { cwd: ROOT, encoding: "utf8" }).trim();
  const m = url.match(/github\.com[:\/]([^\/]+\/[^\/]+?)(\.git)?$/);
  repo = m ? m[1] : undefined;
} catch {}
if (!repo) {
  fail(
    "could not determine the GitHub repo — set origin: `git remote add origin https://github.com/OWNER/REPO.git`",
  );
}

// ── Signed artifact ──────────────────────────────────────────────────────────

const exeName = `Warp_${version}_x64-setup.exe`;
const nsisDir = path.join(ROOT, "src-tauri/target/release/bundle/nsis");
const sigPath = path.join(nsisDir, `${exeName}.sig`);
if (!fs.existsSync(sigPath)) {
  fail(
    `no updater signature at ${path.relative(ROOT, sigPath)}\n` +
      "The build must run with TAURI_SIGNING_PRIVATE_KEY set and createUpdaterArtifacts enabled.",
  );
}

const signature = fs.readFileSync(sigPath, "utf8").trim();

// ── Write the manifest ───────────────────────────────────────────────────────

const manifest = {
  version,
  notes: `https://github.com/${repo}/releases/tag/${tag}`,
  pub_date: new Date().toISOString(),
  platforms: {
    "windows-x86_64": {
      signature,
      url: `https://github.com/${repo}/releases/download/${tag}/${exeName}`,
    },
  },
};

const outPath = path.join(ROOT, "latest.json");
fs.writeFileSync(outPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`+ latest.json  (v${version} → ${repo}/releases/download/${tag}/${exeName})`);

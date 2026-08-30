#!/usr/bin/env node
// check-versions.js — fails if any version reference drifts from tauri.conf.json (single source of truth).
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const SITE = path.join(ROOT, "warp-site");

function readJson(p) {
  return JSON.parse(fs.readFileSync(p, "utf8"));
}

const conf = readJson(path.join(ROOT, "src-tauri/tauri.conf.json"));
const version = conf.version;
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`Invalid version in tauri.conf.json: ${version}`);
  process.exit(1);
}

let failed = false;
function check(file, expected, label) {
  const p = path.join(ROOT, file);
  if (!fs.existsSync(p)) {
    console.log(`  ? ${file} — not found, skipping (${label})`);
    return;
  }
  const content = fs.readFileSync(p, "utf8");
  let found;
  try {
    if (file.endsWith(".json")) {
      const j = JSON.parse(content);
      found = j.version;
    } else if (file.endsWith(".toml")) {
      const m = content.match(/version\s*=\s*"(\d+\.\d+\.\d+)"/);
      found = m ? m[1] : null;
    } else if (file.endsWith(".svelte")) {
      const m = content.match(/APP_VERSION\s*=\s*\$state\("(\d+\.\d+\.\d+)"\)/);
      found = m ? m[1] : null;
    } else {
      // generic: look for version string
      found = content.includes(version) ? version : null;
    }
  } catch {
    found = null;
  }
  if (found !== version) {
    console.error(`  ✖ ${file} — expected ${version}, found ${found ?? "no match"} (${label})`);
    failed = true;
  } else {
    console.log(`  ✔ ${file} — ${version} (${label})`);
  }
}

console.log(`Checking version consistency (source: tauri.conf.json ${version})`);
check("src-tauri/Cargo.toml", version, "Cargo.toml");
check("package.json", version, "package.json");
check("src/lib/stores/updater.svelte.ts", version, "fallback literal");
check("docs/WHITEPAPER.md", version, "whitepaper");
check("docs/ARCHITECTURE.md", version, "architecture");
if (fs.existsSync(path.join(SITE, "package.json"))) {
  check("warp-site/package.json", version, "warp-site");
}
if (fs.existsSync(path.join(ROOT, "latest.json"))) {
  try {
    const lj = readJson(path.join(ROOT, "latest.json"));
    const sigExists = fs.existsSync(
      path.join(ROOT, `src-tauri/target/release/bundle/nsis/Warp_${version}_x64-setup.exe.sig`),
    );
    if (lj.version !== version) {
      if (!sigExists) {
        console.warn(
          `  ! latest.json — expected ${version}, found ${lj.version} (stale, no .sig — run build:win to regenerate)`,
        );
      } else {
        console.error(`  ✖ latest.json — expected ${version}, found ${lj.version}`);
        failed = true;
      }
    } else {
      console.log(`  ✔ latest.json — ${version}`);
    }
  } catch (e) {
    console.error(`  ✖ latest.json — parse error: ${e.message}`);
    failed = true;
  }
}
if (failed) {
  console.error("\nVersion mismatch — run node scripts/release.js <version> or fix manually.");
  process.exit(1);
}
console.log("\nAll versions match.");

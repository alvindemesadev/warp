// Warp release script — bumps the version everywhere, rebuilds the
// installers, syncs the assets the website serves, and (when applied)
// commits, tags and pushes both repositories.
//
//   node scripts/release.js <version>            # dry run (default)
//   node scripts/release.js <version> --apply    # actually do it
//
// A "dry run" prints every file it would touch and every command it
// would run, without changing anything on disk or in git.
//
// Version must be plain semver (e.g. 1.1.0 — no "v" prefix).
//
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

// ---------------------------------------------------------------------------
// Setup
// ---------------------------------------------------------------------------

const args = process.argv.slice(2);
const version = args.find((a) => !a.startsWith("--"));
const APPLY = args.includes("--apply");

const ROOT = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const SITE = path.join(ROOT, "warp-site");
const BUNDLE_NSIS = path.join(ROOT, "src-tauri/target/release/bundle/nsis");
const BUNDLE_MSI = path.join(ROOT, "src-tauri/target/release/bundle/msi");
const DOCS = path.join(ROOT, "docs");
const SITE_PUBLIC = path.join(SITE, "public");

// Current version, read from tauri.conf.json (the source of truth that
// release.js bumps). Every "from" pattern below uses this instead of a
// hardcoded version, so successive releases keep working.
const CURRENT = JSON.parse(fs.readFileSync(path.join(ROOT, "src-tauri/tauri.conf.json"), "utf8")).version;

const exeName = (v) => `Warp_${v}_x64-setup.exe`;
const msiName = (v) => `Warp_${v}_x64_en-US.msi`;

function fail(msg) {
  console.error(`\n✖ ${msg}`);
  process.exit(1);
}

// ---------------------------------------------------------------------------
// Argument checks
// ---------------------------------------------------------------------------

if (!version) fail("Usage: node scripts/release.js <version> [--apply]");
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  fail(`"${version}" is not a valid semver — use e.g. 1.1.0 (no "v" prefix).`);
}
if (version === CURRENT) {
  fail(`version ${version} is already the current version — bump to something newer.`);
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

function read(p) {
  return fs.readFileSync(p, "utf8");
}
function write(p, content) {
  if (APPLY) fs.writeFileSync(p, content);
  console.log(`  ~ ${rel(p)}`);
}
function rel(p) {
  return path.relative(ROOT, p) || p;
}
/** Replace every occurrence of `from` with `to` in a file (and save). */
function replaceIn(file, from, to, { all = true, note } = {}) {
  const p = path.resolve(ROOT, file);
  if (!fs.existsSync(p)) return;
  const s = read(p);
  const re = typeof from === "string" ? new RegExp(from.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), all ? "g" : "") : from;
  const next = s.replace(re, to);
  if (next === s) {
    if (note) console.log(`  ! ${file} — no match for ${note}`);
    return;
  }
  write(p, next);
}
function run(cmd, args, opts = {}) {
  if (APPLY) {
    // Windows: npm is a .cmd shim that must run under cmd.exe; everything
    // else (git, node) spawns directly so argument quoting is preserved.
    const r = process.platform === "win32" && cmd === "npm"
      ? spawnSync("cmd.exe", ["/d", "/s", "/c", cmd, ...args], { stdio: "inherit", cwd: opts.cwd || ROOT })
      : spawnSync(cmd, args, { stdio: "inherit", cwd: opts.cwd || ROOT });
    if (r.status !== 0) fail(`command failed: ${cmd} ${args.join(" ")}`);
  } else {
    console.log(`  $ ${cmd} ${args.join(" ")}${opts.cwd && opts.cwd !== ROOT ? `  (cwd: ${rel(opts.cwd)})` : ""}`);
  }
}
/** Run a command and return its stdout (only ever executed in apply mode). */
function capture(cmd, args, opts = {}) {
  if (!APPLY) {
    console.log(`  $ ${cmd} ${args.join(" ")}  (captured)`);
    return "";
  }
  const r = spawnSync(cmd, args, { cwd: opts.cwd || ROOT, encoding: "utf8" });
  if (r.status !== 0) fail(`command failed: ${cmd} ${args.join(" ")}`);
  return r.stdout;
}
/** Remove installer files for any version other than the new one. */
function pruneOldInstallers(dir) {
  if (!fs.existsSync(dir)) return;
  for (const f of fs.readdirSync(dir)) {
    // Matches Warp_1.2.3_x64-setup.exe and Warp_1.2.3_x64_en-US.msi.
    if (!/Warp_\d+\.\d+\.\d+_x64(-setup)?(_en-US)?\.(exe|msi)$/.test(f)) continue;
    if (f === exeName(version) || f === msiName(version)) continue;
    console.log(`  - ${rel(path.join(dir, f))}`);
    if (APPLY) fs.unlinkSync(path.join(dir, f));
  }
}
function copy(src, dest) {
  console.log(`  + ${rel(dest)}`);
  if (APPLY) fs.copyFileSync(src, dest);
}

// ---------------------------------------------------------------------------
// 1. Bump versions
// ---------------------------------------------------------------------------

console.log(`\n=== 1/6 Bump version to ${version} ===`);

// Main repo
replaceIn("src-tauri/tauri.conf.json", `"version": "${CURRENT}"`, `"version": "${version}"`, { note: "tauri.conf.json version" });
replaceIn("src-tauri/Cargo.toml", `version = "${CURRENT}"`, `version = "${version}"`);
replaceIn("src-tauri/Cargo.lock", new RegExp(`name = "warp"\\r?\\nversion = "${CURRENT}"`), `name = "warp"\nversion = "${version}"`, { all: false });
replaceIn("package.json", `"version": "${CURRENT}"`, `"version": "${version}"`);
replaceIn("src/routes/+page.svelte", `let APP_VERSION = $state("${CURRENT}")`, `let APP_VERSION = $state("${version}")`);
replaceIn("README.md", `Warp_${CURRENT}_x64`, `Warp_${version}_x64`, { all: true });
replaceIn("README.md", `git tag v${CURRENT}`, `git tag v${version}`);
// Version badge (shields.io) — keep it in sync with the release.
replaceIn("README.md", new RegExp(`badge/Version-${CURRENT.replace(/\./g, "\\.")}-339dff`), `badge/Version-${version}-339dff`, { all: true });
replaceIn("scripts/readme-download.js", `Warp_${CURRENT}_x64`, `Warp_${version}_x64`, { all: true });
replaceIn(".github/workflows/release.yml", `git tag v${CURRENT}`, `git tag v${version}`);
replaceIn("package-lock.json", new RegExp(`"name": "warp",\\r?\\n  "version": "${CURRENT}"`), `"name": "warp",\n  "version": "${version}"`);

// warp-site repo (its own git root)
if (fs.existsSync(path.join(SITE, ".git"))) {
  replaceIn("warp-site/package.json", `"version": "${CURRENT}"`, `"version": "${version}"`);
  replaceIn("warp-site/package-lock.json", new RegExp(`"name": "warp-site",\\r?\\n  "version": "${CURRENT}"`), `"name": "warp-site",\n  "version": "${version}"`);
  replaceIn("warp-site/package-lock.json", new RegExp(`"name": "warp-site",\\r?\\n      "version": "${CURRENT}"`), `"name": "warp-site",\n      "version": "${version}"`); // packages[""] entry
  replaceIn("warp-site/src/components/Download.tsx", `Warp_${CURRENT}_x64`, `Warp_${version}_x64`, { all: true });
  replaceIn("warp-site/README.md", `Warp_${CURRENT}_x64`, `Warp_${version}_x64`, { all: true });
} else {
  console.log(`  ! warp-site/.git not found — skipping site version bump`);
}

// Refuse to run if the tag already exists in either repo.
function ensureTagIsFree(repoDir) {
  const cwd = repoDir === ROOT ? ROOT : SITE;
  const r = spawnSync("git", ["tag", "-l", `v${version}`], { cwd, encoding: "utf8" });
  if ((r.stdout || "").trim()) fail(`tag v${version} already exists in ${rel(cwd)} — delete it or bump the version`);
}
ensureTagIsFree(ROOT);
if (fs.existsSync(path.join(SITE, ".git"))) ensureTagIsFree(SITE);

// ---------------------------------------------------------------------------
// 2. Rebuild the installers
// ---------------------------------------------------------------------------

console.log(`\n=== 2/6 Rebuild installers (npm run build:win) ===`);
run("npm", ["run", "build:win"], { cwd: ROOT });
if (fs.existsSync(path.join(SITE, ".git"))) {
  run("npm", ["install", "--package-lock-only"], { cwd: SITE });
}

// ---------------------------------------------------------------------------
// 3. Sync installers into docs/ and the site's public/
// ---------------------------------------------------------------------------

console.log(`\n=== 3/6 Sync installers ===`);

pruneOldInstallers(DOCS);
const exePath = path.join(BUNDLE_NSIS, exeName(version));
const msiPath = path.join(BUNDLE_MSI, msiName(version));
for (const src of [exePath, msiPath]) {
  if (!fs.existsSync(src)) {
    // In a dry run the build hasn't run yet — the real check happens after
    // the apply-mode build. Verify the old installer exists instead, so we
    // know the build toolchain has produced artifacts before.
    const old = path.join(path.dirname(src), path.basename(src).replace(version, CURRENT));
    if (!fs.existsSync(old)) fail(`no installer found in ${rel(path.dirname(src))} (run build:win first)`);
    continue;
  }
  copy(src, path.join(DOCS, path.basename(src)));
}

if (fs.existsSync(path.join(SITE, ".git"))) {
  pruneOldInstallers(SITE_PUBLIC);
  for (const src of [exePath, msiPath]) {
    if (!fs.existsSync(src)) continue; // dry run — build hasn't run yet
    copy(src, path.join(SITE_PUBLIC, path.basename(src)));
  }
}

// ---------------------------------------------------------------------------
// 4. Regenerate the README size table (sizes only exist after the build)
// ---------------------------------------------------------------------------

console.log(`\n=== 4/6 Regenerate README download sizes ===`);
const tableOut = capture("node", ["scripts/readme-download.js"]);
if (tableOut.trim()) {
  const readmePath = path.join(ROOT, "README.md");
  const readme = read(readmePath);
  // Replace the whole markdown table block (header + separator + rows).
  // README uses CRLF, so match both line-ending styles.
  const re = /\| File \| Size \| Description \|\r?\n\|[-|]*\|\r?\n(?:\|[^\r\n]*\r?\n)+/;
  const header = "| File | Size | Description |\n|---|---|---|\n";
  const body = tableOut
    .split("\n")
    .filter((l) => l.startsWith("| `docs/")) // data rows only — readme-download.js prints the header too
    .join("\n");
  const next = readme.replace(re, header + body + "\n");
  if (next === readme) fail("could not locate the README download-size table");
  write(readmePath, next);
}

// ---------------------------------------------------------------------------
// 5. Commit, tag, push (both repos)
// ---------------------------------------------------------------------------

const commitMsg = `Release v${version}`;

function releaseRepo(repoDir) {
  const cwd = repoDir === ROOT ? ROOT : SITE;
  const inCwd = (args) => run("git", args, { cwd });

  // Never stage session tooling or other junk that may be lying around.
  const pathspec = cwd === ROOT ? [".", ":(exclude).freebuff"] : ["."];
  inCwd(["add", "-A", "--", ...pathspec]);
  inCwd(["commit", "-m", commitMsg, "--allow-empty"]);
  inCwd(["tag", "-a", `v${version}`, "-m", `Warp ${version}`]);
  inCwd(["push", "origin", "HEAD"]);
  inCwd(["push", "origin", `v${version}`]);
}

console.log(`\n=== 5/6 Tag & push main repo ===`);
releaseRepo(ROOT);

console.log(`\n=== 6/6 Tag & push warp-site repo ===`);
if (fs.existsSync(path.join(SITE, ".git"))) {
  releaseRepo(SITE);
}

console.log(`\n✔ Release ${version} ${APPLY ? "released" : "DRY-RUN — run with --apply to execute"}.`);

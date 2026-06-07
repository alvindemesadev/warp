#!/usr/bin/env node
/**
 * build.js — Cross-platform build script for Warp
 * Automatically locates and sources vcvars64.bat on Windows before running tauri build.
 * Run with: node scripts/build.js
 */

const { execSync, spawnSync } = require("child_process");
const path = require("path");
const fs = require("fs");

const isWindows = process.platform === "win32";

if (!isWindows) {
  // macOS / Linux — just run tauri build directly
  const result = spawnSync("npm", ["run", "tauri", "--", "build"], {
    stdio: "inherit",
    shell: true,
  });
  process.exit(result.status ?? 1);
}

// ── Windows: find vcvars64.bat ────────────────────────────────────────────────
const vcSearchPaths = [
  "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Auxiliary\\Build\\vcvars64.bat",
  "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat",
  "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\Professional\\VC\\Auxiliary\\Build\\vcvars64.bat",
  "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\Build\\vcvars64.bat",
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Auxiliary\\Build\\vcvars64.bat",
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat",
];

const vcvars = vcSearchPaths.find(p => fs.existsSync(p));

if (!vcvars) {
  console.error("\nERROR: Could not find vcvars64.bat");
  console.error("Install Visual Studio 2022 Build Tools with the C++ workload:");
  console.error("  winget install Microsoft.VisualStudio.2022.BuildTools --override \"--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended\"");
  process.exit(1);
}

console.log(`Found MSVC: ${vcvars}`);

// Write a temp batch file that sources vcvars then calls tauri build
const batContent = `@echo off\r\ncall "${vcvars}"\r\nnpm run tauri -- build\r\n`;
const batPath = path.join(__dirname, "_build_tmp.bat");
fs.writeFileSync(batPath, batContent);

const result = spawnSync("cmd", ["/c", batPath], { stdio: "inherit", cwd: path.join(__dirname, "..") });

// Cleanup
try { fs.unlinkSync(batPath); } catch {}

process.exit(result.status ?? 1);

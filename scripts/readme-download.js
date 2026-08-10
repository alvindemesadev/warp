// Prints the README "Download" section table with the real, on-disk
// installer sizes so the documented numbers can never drift from the
// shipped binaries. Run after `npm run build:win`:
//
//   node scripts/readme-download.js
//
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.join(path.dirname(fileURLToPath(import.meta.url)), "..");
const docsDir = path.join(root, "docs");

function fmtBytes(bytes) {
  const mb = bytes / (1024 * 1024);
  return mb >= 10 ? `${Math.round(mb)} MB` : `${mb.toFixed(1)} MB`;
}

const installers = [
  { file: "Warp_1.1.1_x64-setup.exe", note: "Windows installer (recommended)" },
  { file: "Warp_1.1.1_x64_en-US.msi", note: "MSI installer" },
];

console.log("Current release installers (generated locally by `npm run build:win`, not committed to git):");
console.log("");
console.log("| File | Size | Description |");
console.log("|---|---|---|");
for (const { file, note } of installers) {
  const p = path.join(docsDir, file);
  let size = "n/a";
  try {
    size = fmtBytes(fs.statSync(p).size);
  } catch {
    // file missing (fresh clone, no local build yet)
  }
  console.log(`| \`docs/${file}\` | ${size} | ${note} |`);
}

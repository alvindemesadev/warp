#!/usr/bin/env node
// Generates site-preview.html next to the preview bundle so the marketing
// site can embed the real app UI in an iframe with one file reference.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const dist = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "site-preview-dist");
const html = `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<link rel="stylesheet" href="./site-preview.css" />
</head>
<body style="margin:0;background:#000;">
<script src="./site-preview.js"></script>
</body>
</html>
`;
fs.writeFileSync(path.join(dist, "site-preview.html"), html);
console.log(`+ ${path.relative(process.cwd(), path.join(dist, "site-preview.html"))}`);

// Site preview entry — mounts the REAL Warp app component (+page.svelte) into
// a plain browser page so the marketing site can embed the actual UI in an
// iframe. The Tauri APIs are replaced with preview shims (see preview-shims/),
// and a simulated drag-drop fills the source/destination slots the way a real
// window would. Built by `npm run preview:build` → site-preview-dist/.
import { mount } from "svelte";
import App from "./routes/+page.svelte";
import { simulateDrop } from "./preview-shims/window";
import "./app.css";

mount(App, { target: document.body });

// Replay two drops like a real window session: source, then destination.
setTimeout(() => simulateDrop(["C:\\Users\\alvin\\Pictures\\Screenshots"]), 400);
setTimeout(() => simulateDrop(["D:\\Backup"]), 900);

import { getCurrentWindow, currentMonitor, LogicalSize, LogicalPosition } from "@tauri-apps/api/window";

export class UiStore {
  showSyncWarning = $state(false);
  dropConflict = $state(false);
  showRecent = $state(false);
  showPresets = $state(false);
  showPresetModal = $state(false);
  dragTarget = $state<"source" | "dest" | null>(null);
  isDragging = $state(false);
  _pendingDrop = $state("");

  theme = $state<"dark" | "light">("dark");
  scale = $state(1.0);

  applyDropToPending(
    slot: "source" | "dest",
    setSource: (p: string) => void,
    setDest: (p: string) => void,
  ) {
    if (slot === "source") setSource(this._pendingDrop);
    else setDest(this._pendingDrop);
    this.dropConflict = false;
    this._pendingDrop = "";
  }

  resetUi() {
    this.showSyncWarning = false;
    this.dropConflict = false;
    this.showRecent = false;
    this.showPresets = false;
    this.showPresetModal = false;
    this.dragTarget = null;
    this.isDragging = false;
    this._pendingDrop = "";
  }

  loadThemeAndScale() {
    try {
      const t = localStorage.getItem("warp-theme") as "dark" | "light" | null;
      if (t === "dark" || t === "light") this.theme = t;
      const s = parseFloat(localStorage.getItem("warp-scale") ?? "");
      if (!Number.isNaN(s) && s >= 1.0 && s <= 1.3) this.scale = Math.round(s * 100) / 100;
      else this.scale = 1.0;
    } catch {
      this.scale = 1.0;
    }
    this.applyThemeAndScale(true);
  }

  setTheme(v: "dark" | "light") {
    this.theme = v;
    try {
      localStorage.setItem("warp-theme", v);
    } catch {}
    this.applyThemeAndScale(false);
  }

  setScale(v: number) {
    const clamped = Math.min(1.3, Math.max(1.0, Math.round(v * 100) / 100));
    this.scale = clamped;
    try {
      localStorage.setItem("warp-scale", String(clamped));
    } catch {}
    this.applyThemeAndScale(true);
  }

  zoomIn() {
    if (this.scale < 1.14) this.setScale(1.15);
    else if (this.scale < 1.29) this.setScale(1.3);
  }

  zoomOut() {
    if (this.scale > 1.16) this.setScale(1.15);
    else if (this.scale > 1.01) this.setScale(1.0);
  }

  async resizeWindow() {
    try {
      const win = getCurrentWindow();
      const baseWidth = 600;
      const baseHeight = 820;
      const w = Math.round(baseWidth * this.scale);
      const h = Math.round(baseHeight * this.scale);
      await win.setSize(new LogicalSize(w, h));
    } catch {}
  }

  applyThemeAndScale(resizeWin = false) {
    if (typeof document === "undefined") return;
    const root = document.documentElement;
    root.setAttribute("data-theme", this.theme);
    // CSS variable and zoom for proportional UI scaling
    root.style.setProperty("--scale", String(this.scale));
    root.style.fontSize = `calc(16px * ${this.scale})`;
    root.style.zoom = String(this.scale);
    if (resizeWin) {
      void this.resizeWindow();
    }
  }
}

export const ui = new UiStore();

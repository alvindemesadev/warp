/// <reference types="svelte" />
// Updater store — check/download/install flow.

import { check, type Update } from "@tauri-apps/plugin-updater";

export class UpdaterStore {
  updateState = $state<"idle" | "checking" | "available" | "downloading" | "installing">("idle");
  updateInfo = $state<{ version: string; body: string } | null>(null);
  showUpdateModal = $state(false);
  updateProgress = $state(0);
  toast = $state("");
  APP_VERSION = $state("1.2.4");
  _pendingUpdate: Update | null = null;
  _toastTimer: ReturnType<typeof setTimeout> | undefined = undefined;

  showToast(msg: string) {
    this.toast = msg;
    clearTimeout(this._toastTimer);
    this._toastTimer = setTimeout(() => (this.toast = ""), 3500);
  }

  async checkForUpdates(auto = false) {
    if (
      this.updateState === "checking" ||
      this.updateState === "downloading" ||
      this.updateState === "installing"
    )
      return;
    this.updateState = "checking";
    try {
      const update = await check();
      if (update) {
        this._pendingUpdate = update;
        this.updateInfo = { version: update.version, body: update.body ?? "" };
        this.updateState = "available";
        this.showUpdateModal = true;
      } else {
        this.updateState = "idle";
        if (!auto) this.showToast(`You're up to date (v${this.APP_VERSION})`);
      }
    } catch {
      this.updateState = "idle";
      if (!auto) this.showToast("Couldn't check for updates — check your connection");
    }
  }

  async installUpdate() {
    if (
      !this._pendingUpdate ||
      this.updateState === "downloading" ||
      this.updateState === "installing"
    )
      return;
    this.updateState = "downloading";
    this.updateProgress = 0;
    let downloaded = 0;
    let contentLength = 0;
    const dlStart = Date.now();
    const MIN_DL_MS = 1600;
    try {
      await this._pendingUpdate.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            contentLength = event.data.contentLength ?? 0;
            break;
          case "Progress":
            downloaded += event.data.chunkLength;
            if (contentLength > 0)
              this.updateProgress = Math.min(100, Math.round((downloaded / contentLength) * 100));
            break;
          case "Finished":
            this.updateProgress = 100;
            break;
        }
      });
      const elapsed = Date.now() - dlStart;
      if (elapsed < MIN_DL_MS) await new Promise((r) => setTimeout(r, MIN_DL_MS - elapsed));
      this.updateState = "installing";
      await new Promise((r) => setTimeout(r, 900));
    } catch {
      const elapsed = Date.now() - dlStart;
      if (elapsed < MIN_DL_MS) await new Promise((r) => setTimeout(r, MIN_DL_MS - elapsed));
      this.updateState = "idle";
      this.showToast("Update download failed — check your connection and try again");
    }
  }

  initUpdater(version: string) {
    this.APP_VERSION = version;
  }
}

export const updater = new UpdaterStore();

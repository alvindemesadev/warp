// @ts-nocheck
// UI transient state — modals, drag.

export class UiStore {
  showSyncWarning = $state(false);
  dropConflict = $state(false);
  showRecent = $state(false);
  showPresets = $state(false);
  showPresetModal = $state(false);
  dragTarget = $state<"source" | "dest" | null>(null);
  isDragging = $state(false);
  _pendingDrop = $state("");

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
}

export const ui = new UiStore();

// Browser shim for @tauri-apps/api/window — site-preview build only.
// Captures the drag-drop handler the app registers so site-preview.ts can
// replay a "drop" to fill the source/destination slots like a real window.

type DropEvent = { payload: { type: string; paths?: string[] } };

let dropHandler: ((e: DropEvent) => void) | null = null;

export function getCurrentWindow() {
  return {
    close: () => {},
    minimize: () => {},
    onDragDropEvent: (cb: (e: DropEvent) => void) => {
      dropHandler = cb;
      return Promise.resolve(() => {
        dropHandler = null;
      });
    },
  };
}

export function simulateDrop(paths: string[]) {
  if (dropHandler) dropHandler({ payload: { type: "drop", paths } });
}

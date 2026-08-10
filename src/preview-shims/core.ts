// Browser shim for @tauri-apps/api/core — used only by the site-preview build
// (vite.preview.config.ts). Keeps the REAL app component rendering in a
// browser by answering the calls it makes at rest with realistic mock data.

export async function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>
): Promise<T> {
  if (cmd === "get_path_info") {
    const p: string = (args?.path as string) ?? "";
    const isDest = p.toLowerCase().includes("backup");
    return {
      files: isDest ? 1208 : 3204,
      bytes: isDest ? 7.9e9 : 12.5e9,
      isFile: false,
      drive: isDest ? "D:" : "C:",
      removable: !isDest,
    } as T;
  }
  throw new Error(`Site preview: Tauri command "${cmd}" is not available in the browser.`);
}

export function isTauri(): boolean {
  return false;
}

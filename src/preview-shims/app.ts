// Browser shim for @tauri-apps/api/app — site-preview build only.
// Returns the real app version, injected at build time from tauri.conf.json.

declare const __PREVIEW_VERSION__: string;

export async function getVersion(): Promise<string> {
  return __PREVIEW_VERSION__;
}

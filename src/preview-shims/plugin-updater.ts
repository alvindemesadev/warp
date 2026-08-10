// Browser shim for @tauri-apps/plugin-updater — site-preview build only.
// No updates in a browser preview; check() always reports up to date.

export type Update = { version: string; body: string | null };

export async function check(): Promise<Update | null> {
  return null;
}

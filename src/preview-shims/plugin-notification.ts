// Browser shim for @tauri-apps/plugin-notification — site-preview build only.

export async function isPermissionGranted(): Promise<boolean> {
  return false;
}

export async function requestPermission(): Promise<string> {
  return "denied";
}

export function sendNotification(_options: unknown): void {}

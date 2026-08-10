// Browser shim for @tauri-apps/api/event — site-preview build only.

export async function listen<T>(
  _event: string,
  _handler: (e: { payload: T }) => void
): Promise<() => void> {
  return () => {};
}

// Window operations with explicit diagnostics.
//
// Every operation logs three distinguishable outcomes so a dead title bar is
// attributable to exactly one failure stage:
//
//   A. "requested" logged, but neither "completed" nor "failed" follows
//      → the Tauri IPC / main-thread event loop is blocked (the promise
//      never settles). This is the symptom of a wedged main thread.
//   B. "failed" logged → the OS rejected the operation (permission denied,
//      window not found, …); the rejection is logged and swallowed.
//   C. "completed" logged but the OS window did not change
//      → the API succeeded and something else (an OS quirk) is at fault.
//
// Rejections never propagate as unhandled promise rejections: every
// operation resolves with `undefined` after logging the failure, so callers
// can `void` them safely. `console.debug` keeps the happy path quiet in
// production bundles while the dev build still shows every transition.

import { getCurrentWindow } from "@tauri-apps/api/window";

const WINDOW = getCurrentWindow();

async function withDiagnostics<T>(
  operation: string,
  run: () => Promise<T>,
): Promise<T | undefined> {
  console.debug(`[window] ${operation} requested`);
  try {
    const result = await run();
    console.debug(`[window] ${operation} completed`);
    return result;
  } catch (error) {
    console.error(`[window] ${operation} failed`, error);
    return undefined;
  }
}

export const windowService = {
  minimize: () => withDiagnostics("minimize", () => WINDOW.minimize()),
  toggleMaximize: () => withDiagnostics("toggleMaximize", () => WINDOW.toggleMaximize()),
  close: () => withDiagnostics("close", () => WINDOW.close()),
  hide: () => withDiagnostics("hide", () => WINDOW.hide()),
  isMaximized: () => withDiagnostics("isMaximized", () => WINDOW.isMaximized()),
};

// Single source of truth for the main-window maximized state.
//
// Exactly one instance exists (created by `MainWindow` and passed down), so
// resize/move events produce exactly one IPC query. Native resize/move event
// streams are high-frequency (Win+Arrow snaps, Aero drags), so the query is
// debounced: many events collapse into one `isMaximized` call per burst.

import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { windowService } from "../services/window";

const WINDOW = getCurrentWindow();

const REFRESH_DEBOUNCE_MS = 80;

export function useWindowState() {
  const [maximized, setMaximized] = useState<boolean | null>(null);

  useEffect(() => {
    let disposed = false;
    let debounce: number | undefined;
    const unlisteners: (() => void)[] = [];

    const refresh = () => {
      window.clearTimeout(debounce);
      debounce = window.setTimeout(() => {
        void windowService.isMaximized().then((value) => {
          if (!disposed) setMaximized(value ?? null);
        });
      }, REFRESH_DEBOUNCE_MS);
    };

    // Initial state, then keep in sync with snap / Win+Arrow / external
    // window management — all of which land here via resize/move events.
    refresh();
    void WINDOW.onResized(refresh).then((fn) => unlisteners.push(fn));
    void WINDOW.onMoved(refresh).then((fn) => unlisteners.push(fn));

    return () => {
      disposed = true;
      window.clearTimeout(debounce);
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  return { maximized };
}

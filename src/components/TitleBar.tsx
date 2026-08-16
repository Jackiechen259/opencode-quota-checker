// Application-drawn title bar of the borderless main window.
//
// Layout, from left to right: app icon, app name, a draggable spacer, then
// the minimize / maximize-restore / close window controls — the same
// dimensions as a native Windows caption (32 px strip, 46 px buttons).

import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useState } from "react";
import { AppLogo } from "./common";
import { Icons } from "./icons";

const WINDOW = getCurrentWindow();

export function TitleBar() {
  const [maximized, setMaximized] = useState<boolean | null>(null);

  const refreshMaximized = useCallback(() => {
    void WINDOW.isMaximized().then(setMaximized).catch(() => setMaximized(null));
  }, []);

  useEffect(() => {
    refreshMaximized();
    const unlisteners: (() => void)[] = [];
    // Snap, Win+Arrow and external window management all land here, so the
    // glyph always reflects the real state.
    void WINDOW.onResized(refreshMaximized).then((fn) => unlisteners.push(fn));
    void WINDOW.onMoved(refreshMaximized).then((fn) => unlisteners.push(fn));
    return () => unlisteners.forEach((fn) => fn());
  }, [refreshMaximized]);

  const onDragDoubleClick = () => {
    void WINDOW.toggleMaximize();
    // The toggle is async; re-query the resulting state afterwards.
    window.setTimeout(refreshMaximized, 120);
  };

  return (
    <div className="titlebar" data-tauri-drag-region>
      <div
        className="titlebar-brand"
        data-tauri-drag-region
        onDoubleClick={onDragDoubleClick}
      >
        <AppLogo size={16} />
        <span className="titlebar-title">OpenCode Quota Checker</span>
      </div>
      <div className="titlebar-spacer" data-tauri-drag-region onDoubleClick={onDragDoubleClick} />
      <div className="titlebar-controls">
        <button
          type="button"
          className="titlebar-control"
          title="最小化"
          aria-label="最小化"
          onClick={() => void WINDOW.minimize()}
        >
          <Icons.WinMinimize size={11} />
        </button>
        <button
          type="button"
          className="titlebar-control"
          title={maximized ? "还原" : "最大化"}
          aria-label={maximized ? "还原" : "最大化"}
          onClick={() => {
            void WINDOW.toggleMaximize();
            window.setTimeout(refreshMaximized, 120);
          }}
        >
          {maximized ? <Icons.WinRestore size={11} /> : <Icons.WinMaximize size={11} />}
        </button>
        <button
          type="button"
          className="titlebar-control titlebar-control-close"
          title="关闭"
          aria-label="关闭"
          // Routes through the Rust close handler, which always terminates
          // the process. Hiding to the tray is an explicit action only
          // (header menu "隐藏主窗口" / tray menu).
          onClick={() => void WINDOW.close()}
        >
          <Icons.WinClose size={11} />
        </button>
      </div>
    </div>
  );
}

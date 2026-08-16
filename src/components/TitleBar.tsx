// Application-drawn title bar of the borderless main window.
//
// Layout, from left to right: app icon, app name, a draggable spacer, then
// the minimize / maximize-restore / close window controls — the same
// dimensions as a native Windows caption (32 px strip, 46 px buttons).
//
// Drag regions: only `titlebar-brand` and `titlebar-spacer` are
// `data-tauri-drag-region`. The controls container is deliberately NOT a
// drag region so window-control hit testing is structurally impossible to
// confuse with dragging.
//
// The maximized glyph comes from `useWindowState` (one listener in the main
// window shell); this component performs no window API calls itself except
// the button actions, which go through `windowService` for diagnostics.

import { useCallback } from "react";
import { AppLogo } from "./common";
import { Icons } from "./icons";
import { windowService } from "../services/window";

interface Props {
  maximized: boolean | null;
}

export function TitleBar({ maximized }: Props) {
  const toggleMaximize = useCallback(() => {
    void windowService.toggleMaximize();
    // The real state arrives through the shared resize/move listener
    // (debounced in `useWindowState`); no local re-query is needed.
  }, []);

  return (
    <div className="titlebar">
      <div
        className="titlebar-brand"
        data-tauri-drag-region
        onDoubleClick={toggleMaximize}
      >
        <AppLogo size={16} />
        <span className="titlebar-title">OpenCode Quota Checker</span>
      </div>
      <div className="titlebar-spacer" data-tauri-drag-region onDoubleClick={toggleMaximize} />
      <div className="titlebar-controls">
        <button
          type="button"
          className="titlebar-control"
          title="最小化"
          aria-label="最小化"
          onClick={() => void windowService.minimize()}
        >
          <Icons.WinMinimize size={11} />
        </button>
        <button
          type="button"
          className="titlebar-control"
          title={maximized ? "还原" : "最大化"}
          aria-label={maximized ? "还原" : "最大化"}
          onClick={toggleMaximize}
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
          onClick={() => void windowService.close()}
        >
          <Icons.WinClose size={11} />
        </button>
      </div>
    </div>
  );
}

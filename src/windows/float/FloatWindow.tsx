// Floating window shell.
//
// State architecture (the fix for the mode-desync bug):
//
//   AppConfig.float_mode  ── persisted (full | compact)
//   FloatState.top_docked ── transient (native drag / dock command)
//   presentationMode      ── derived in the BACKEND, delivered in the DTO
//   native window size    ── set by the backend from the same derivation
//
// This component keeps NO local mode state: it renders exclusively from
// `floatState.presentationMode`. Until the first `get_float_state` resolves
// it shows a lightweight boot shell — it never guesses "full" and flashes
// the wrong layout inside a Compact native window.

import { useCallback } from "react";
import { useQuota } from "../../hooks/useApp";
import { api } from "../../services/tauri";
import type { PersistedFloatMode } from "../../types/models";
import { CompactFloatView } from "./CompactFloatView";
import { DockedFloatView } from "./DockedFloatView";
import { FloatHeader } from "./FloatHeader";
import { FullFloatView } from "./FullFloatView";
import { highestWindow } from "./floatLayout";
import { useFloatState } from "./useFloatState";

function BootShell() {
  return (
    <div className="float-window">
      <div className="float-inner">
        <div className="float-boot">
          <span className="float-logo">OC</span>
          <span className="float-boot-text">正在同步悬浮窗状态…</span>
        </div>
      </div>
    </div>
  );
}

export function FloatWindow() {
  const { report, loading, error } = useQuota();
  const floatState = useFloatState();

  const refresh = useCallback(() => {
    void api.refreshUsage();
  }, []);

  const changeMode = useCallback((next: PersistedFloatMode) => {
    void api.setFloatMode(next);
  }, []);

  const close = useCallback(() => {
    void api.closeFloatWindow();
  }, []);

  const presentationMode = floatState?.presentationMode ?? null;

  if (presentationMode === null) {
    return <BootShell />;
  }

  const status = {
    loading,
    error: error?.user ?? null,
    hasData: report !== null,
  };

  if (presentationMode === "docked") {
    return (
      <div className="float-window float-window--docked">
        <DockedFloatView
          report={report}
          loading={loading}
          error={error?.user ?? null}
          onExpand={() => changeMode("full")}
          onClose={close}
        />
      </div>
    );
  }

  const compact = presentationMode === "compact";
  const headerTitle = compact
    ? highestWindow(report)?.label ?? "OpenCode Quota"
    : "OpenCode Quota Checker";

  return (
    <div className={`float-window float-window--${presentationMode}`}>
      <div className="float-inner">
        <FloatHeader
          variant={compact ? "compact" : "full"}
          title={headerTitle}
          status={status}
          onRefresh={refresh}
          onToggleMode={() => changeMode(compact ? "full" : "compact")}
          onClose={close}
        />
        {compact ? (
          <CompactFloatView
            report={report}
            loading={loading}
            error={error?.user ?? null}
            onRetry={refresh}
          />
        ) : (
          <FullFloatView
            report={report}
            loading={loading}
            error={error?.user ?? null}
            onRetry={refresh}
          />
        )}
      </div>
    </div>
  );
}

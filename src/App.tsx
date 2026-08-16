// Main window shell: title bar, header, body router, footer, overlays.
// One rounded frame around the whole page (like the archived Iced frame);
// maximized windows go square and flush with the screen edge.
//
// Boot contract: the shell (TitleBar + AppHeader + window controls) renders
// immediately and is never gated on backend state. Only the main body waits
// on the boot status, and only for a bounded time: `get_boot_status` must
// answer within STARTUP_TIMEOUT_MS, otherwise the loading screen becomes a
// StartupError with an explicit retry. A failed/hanging IPC is never
// silently reinterpreted as "still loading".

import { useCallback, useEffect, useState, type ReactNode } from "react";
import { AppHeader, type HeaderPage } from "./components/AppHeader";
import { StartupError } from "./components/StartupError";
import { Toast } from "./components/common";
import { TitleBar } from "./components/TitleBar";
import { UpdateBanner } from "./components/UpdateBanner";
import { useAppStatus, useConfig, useQuota, useUpdater } from "./hooks/useApp";
import { useNow } from "./hooks/useTauriEvents";
import { useWindowState } from "./hooks/useWindowState";
import { Credentials } from "./pages/Credentials";
import { Dashboard } from "./pages/Dashboard";
import { Debug } from "./pages/Debug";
import { Settings } from "./pages/Settings";
import { api } from "./services/tauri";
import { windowService } from "./services/window";
import { timestamp } from "./lib/format";

/** How long the initial `get_boot_status` may take before the loading
 * screen turns into an explicit startup error. */
const STARTUP_TIMEOUT_MS = 8_000;

export function MainWindow() {
  const { boot, status, reload: reloadStatus, retry: retryBoot } = useAppStatus();
  const { report, loading, error } = useQuota();
  const { config, setConfig } = useConfig();
  const { update } = useUpdater();
  const nowMs = useNow();
  const { maximized } = useWindowState();

  const [page, setPage] = useState<HeaderPage>("dashboard");
  const [toast, setToast] = useState<string | null>(null);
  const [startupTimedOut, setStartupTimedOut] = useState(false);

  // Watchdog: the boot command has a hard deadline. If it neither resolves
  // nor rejects in time, the UI must stop showing an infinite loading state.
  useEffect(() => {
    if (boot.phase !== "loading") return;
    setStartupTimedOut(false);
    const timer = window.setTimeout(() => setStartupTimedOut(true), STARTUP_TIMEOUT_MS);
    return () => window.clearTimeout(timer);
  }, [boot.phase]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 3_000);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const refresh = useCallback(() => {
    void api.refreshUsage();
  }, []);

  const toggleFloat = useCallback(() => {
    void api.openFloatWindow().catch((error) => {
      console.error("[float] open_float_window failed", error);
    });
  }, []);

  const hideMain = useCallback(() => {
    void windowService.hide();
  }, []);

  const exitApp = useCallback(() => {
    void api.quitApp();
  }, []);

  const onKeyDown = useCallback((event: KeyboardEvent) => {
    if (event.key === "Escape") {
      setPage((current) => (current === "dashboard" ? current : "dashboard"));
    }
  }, []);

  useEffect(() => {
    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [onKeyDown]);

  // The boot-ready status is the only source for boot gating; the undefined
  // fallback pattern (`status?.x ?? false`) is banned here because it hides
  // schema mismatches behind a "false" that looks like real data.
  const booted = boot.phase === "ready" ? boot.status : null;
  const configured = booted?.configured ?? false;
  const credentialPhase = booted?.credentials.phase ?? "checking";
  const checking = credentialPhase === "checking";
  const configLoaded = booted?.configLoaded ?? false;
  const dashboardOpen = page === "dashboard" && configLoaded && configured;

  let body: ReactNode;
  if (page === "debug") {
    body = <Debug onClose={() => setPage("dashboard")} onToast={setToast} />;
  } else if (page === "settings") {
    body = (
      <Settings
        status={status}
        config={config}
        update={update}
        onClose={() => setPage("dashboard")}
        onConfigSaved={(saved) => {
          setConfig(saved);
          reloadStatus();
        }}
        onRefreshStatus={reloadStatus}
      />
    );
  } else if (boot.phase === "error") {
    body = (
      <StartupError
        title="应用初始化失败"
        message="无法从后端获取启动状态。请检查日志后重试。"
        detail={boot.error}
        onRetry={retryBoot}
      />
    );
  } else if (boot.phase === "loading") {
    if (startupTimedOut) {
      body = (
        <StartupError
          title="应用初始化超时"
          message={`后端未在 ${STARTUP_TIMEOUT_MS / 1_000} 秒内响应启动状态请求。`}
          detail="get_boot_status 未在期限内返回。可能原因：Tauri IPC 主线程阻塞、命令 panic 或命令不存在。"
          onRetry={retryBoot}
        />
      );
    } else {
      body = <div className="checking-state">正在检查系统钥匙串…</div>;
    }
  } else if (!configLoaded) {
    // Ready but the config never loaded: a backend contract violation, not
    // something to hide behind a loading screen.
    console.error("[ipc-contract] BootStatusDto missing configLoaded", booted);
    body = (
      <StartupError
        title="应用初始化失败"
        message="后端报告配置未加载，无法继续。"
        detail="configLoaded 为 false。请检查配置文件后重试。"
        onRetry={retryBoot}
      />
    );
  } else if (checking) {
    // A recheck is in flight (initial boot never reaches here: while the
    // boot command itself is pending the shell shows the watchdog-guarded
    // loading state above). The shell stays fully interactive.
    body = <div className="checking-state">正在检查系统钥匙串…</div>;
  } else if (!configured) {
    body = (
      <Credentials
        configured={false}
        phase={credentialPhase}
        statusError={booted?.credentials.error ?? null}
        onRecheck={() => {
          void api.recheckCredentials();
          reloadStatus();
        }}
        onSaved={reloadStatus}
      />
    );
  } else {
    body = (
      <Dashboard
        report={report}
        loading={loading}
        error={error}
        nowMs={nowMs}
        onRefresh={refresh}
      />
    );
  }

  const footer = report ? `最后更新：${timestamp(report.fetched_at)}` : "最后更新：暂无数据";

  return (
    <div className={`main-window ${maximized ? "main-window-maximized" : ""}`}>
      <div className="main-frame">
        <TitleBar maximized={maximized} />
        <AppHeader
          status={status}
          report={report}
          nowMs={nowMs}
          loading={loading}
          page={page}
          onNavigate={setPage}
          onRefresh={refresh}
          onToggleFloat={toggleFloat}
          onHide={hideMain}
          onExit={exitApp}
        />
        {update &&
        !update.bannerDismissed &&
        (update.status === "available" ||
          update.status === "downloading" ||
          update.status === "ready_to_install") ? (
          <UpdateBanner update={update} />
        ) : null}
        <div className={`main-body ${dashboardOpen ? "main-body-scroll" : ""}`}>{body}</div>
        {dashboardOpen ? <div className="main-footer">{footer}</div> : null}
      </div>
      {toast ? <Toast message={toast} onDismiss={() => setToast(null)} /> : null}
    </div>
  );
}

// Main window shell: title bar, header, body router, footer, overlays.
// One rounded frame around the whole page (like the archived Iced frame);
// maximized windows go square and flush with the screen edge.

import { useCallback, useEffect, useState, type ReactNode } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { AppHeader, type HeaderPage } from "./components/AppHeader";
import { Toast } from "./components/common";
import { TitleBar } from "./components/TitleBar";
import { UpdateBanner } from "./components/UpdateBanner";
import { useAppStatus, useConfig, useQuota, useUpdater } from "./hooks/useApp";
import { useNow } from "./hooks/useTauriEvents";
import { Credentials } from "./pages/Credentials";
import { Dashboard } from "./pages/Dashboard";
import { Debug } from "./pages/Debug";
import { Settings } from "./pages/Settings";
import { api } from "./services/tauri";
import { timestamp } from "./lib/format";

const WINDOW = getCurrentWindow();

export function MainWindow() {
  const { status, reload: reloadStatus } = useAppStatus();
  const { report, loading, error } = useQuota();
  const { config, setConfig } = useConfig();
  const { update } = useUpdater();
  const nowMs = useNow();

  const [page, setPage] = useState<HeaderPage>("dashboard");
  const [toast, setToast] = useState<string | null>(null);
  const [maximized, setMaximized] = useState<boolean | null>(null);

  const refreshMaximized = useCallback(() => {
    void WINDOW.isMaximized().then(setMaximized).catch(() => setMaximized(null));
  }, []);

  useEffect(() => {
    refreshMaximized();
    const unlisteners: (() => void)[] = [];
    void WINDOW.onResized(refreshMaximized).then((fn) => unlisteners.push(fn));
    void WINDOW.onMoved(refreshMaximized).then((fn) => unlisteners.push(fn));
    return () => unlisteners.forEach((fn) => fn());
  }, [refreshMaximized]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 3_000);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const refresh = useCallback(() => {
    void api.refreshUsage();
  }, []);

  const toggleFloat = useCallback(() => {
    void api.openFloatWindow().catch(() => {});
  }, []);

  const hideMain = useCallback(() => {
    void WINDOW.hide();
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

  const configured = status?.configured ?? false;
  const credentialPhase = status?.credentials.phase ?? "checking";
  const checking = credentialPhase === "checking";
  const configLoaded = status?.config_loaded ?? false;
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
  } else if (checking || !configLoaded) {
    body = <div className="checking-state">正在检查系统钥匙串…</div>;
  } else if (!configured) {
    body = (
      <Credentials
        configured={false}
        phase={credentialPhase}
        statusError={status?.credentials.error ?? null}
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
        <TitleBar />
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
        !update.banner_dismissed &&
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

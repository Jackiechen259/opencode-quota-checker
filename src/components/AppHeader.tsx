// 48px control bar: status badges, last-update label, and the refresh /
// settings / float / more actions (the "more" menu carries Details, Hide,
// Exit — matching the archived Iced header).

import { useEffect, useRef, useState } from "react";
import { relative } from "../lib/format";
import type { AppStatusDto, UsageReport } from "../types/models";
import { quotaHealth } from "../types/models";
import { IconButton, StatusBadge, healthColor } from "./common";
import { Icons } from "./icons";

export type HeaderPage = "dashboard" | "settings" | "debug";

interface Props {
  status: AppStatusDto | null;
  report: UsageReport | null;
  nowMs: number;
  loading: boolean;
  page: HeaderPage;
  onNavigate: (page: HeaderPage) => void;
  onRefresh: () => void;
  onToggleFloat: () => void;
  onHide: () => void;
  onExit: () => void;
}

function highestWindow(report: UsageReport | null) {
  if (!report) return null;
  let highest = null as (typeof report.windows)[number] | null;
  for (const window of report.windows) {
    if (!highest || window.percent > highest.percent) highest = window;
  }
  return highest;
}

function LoadBadge({ report }: { report: UsageReport | null }) {
  const highest = highestWindow(report);
  if (!highest) {
    return <StatusBadge>最高 —</StatusBadge>;
  }
  const health = quotaHealth(highest.percent);
  const tone =
    health === "critical" ? "danger" : health === "warning" ? "warning" : "success";
  return (
    <StatusBadge tone={tone}>
      <span
        className="dot"
        style={{
          background: healthColor(health),
          width: 8,
          height: 8,
          display: "inline-block",
          borderRadius: "50%",
        }}
      />
      最高 {highest.percent.toFixed(1)}%
    </StatusBadge>
  );
}

export function AppHeader({
  status,
  report,
  nowMs,
  loading,
  page,
  onNavigate,
  onRefresh,
  onToggleFloat,
  onHide,
  onExit,
}: Props) {
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!menuOpen) return;
    const close = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        setMenuOpen(false);
      }
    };
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") setMenuOpen(false);
    };
    document.addEventListener("mousedown", close);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", close);
      document.removeEventListener("keydown", onKey);
    };
  }, [menuOpen]);

  const updatedLabel =
    report && loading ? (
      <span className="header-updated header-updated-loading">正在刷新…</span>
    ) : report ? (
      <span className="header-updated">{relative(report.fetched_at, nowMs)}更新</span>
    ) : null;

  return (
    <div className="app-header">
      <div className="app-header-status">
        <LoadBadge report={report} />
        {status && !status.trayAvailable ? (
          <StatusBadge tone="danger">托盘不可用</StatusBadge>
        ) : null}
      </div>
      <div className="app-header-actions">
        {updatedLabel}
        <IconButton
          icon={<Icons.Refresh size={16} />}
          label="刷新"
          onClick={onRefresh}
          loading={loading}
        />
        <IconButton
          icon={<Icons.Settings size={16} />}
          label="设置"
          onClick={() => onNavigate("settings")}
          focused={page === "settings"}
        />
        <IconButton
          icon={<Icons.Float size={16} />}
          label="悬浮窗"
          onClick={onToggleFloat}
        />
        <div className="header-menu-anchor" ref={menuRef}>
          <IconButton
            icon={<Icons.More size={16} />}
            label="更多"
            onClick={() => setMenuOpen((open) => !open)}
            focused={menuOpen}
          />
          {menuOpen ? (
            <div className="header-menu">
              <button
                type="button"
                className="header-menu-item"
                onClick={() => {
                  setMenuOpen(false);
                  onNavigate("debug");
                }}
              >
                原始响应
              </button>
              <button
                type="button"
                className="header-menu-item"
                onClick={() => {
                  setMenuOpen(false);
                  onHide();
                }}
              >
                隐藏主窗口
              </button>
              <button
                type="button"
                className="header-menu-item header-menu-item-danger"
                onClick={() => {
                  setMenuOpen(false);
                  onExit();
                }}
              >
                退出
              </button>
            </div>
          ) : null}
        </div>
      </div>
    </div>
  );
}

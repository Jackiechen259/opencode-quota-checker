// Small shared UI atoms.

import type { ReactNode } from "react";
import type { QuotaHealth } from "../types/models";
import { Icons } from "./icons";

export function Dot({ color, size = 9 }: { color: string; size?: number }) {
  return <span className="dot" style={{ background: color, width: size, height: size }} />;
}

const healthColors: Record<QuotaHealth, string> = {
  healthy: "var(--color-success)",
  warning: "var(--color-warning)",
  critical: "var(--color-danger)",
};

export function healthColor(health: QuotaHealth): string {
  return healthColors[health];
}

export function progressColor(health: QuotaHealth): string {
  return health === "healthy" ? "var(--color-primary)" : healthColors[health];
}

type Tone = "neutral" | "primary" | "success" | "warning" | "danger";

const toneClass: Record<Tone, string> = {
  neutral: "badge-neutral",
  primary: "badge-primary",
  success: "badge-success",
  warning: "badge-warning",
  danger: "badge-danger",
};

export function StatusBadge({ children, tone }: { children: ReactNode; tone?: Tone }) {
  return <span className={`badge ${toneClass[tone ?? "neutral"]}`}>{children}</span>;
}

type NoticeKind = "error" | "warning" | "success";

export function Notice({
  kind,
  children,
}: {
  kind: NoticeKind;
  children: ReactNode;
}) {
  return <div className={`notice notice-${kind}`}>{children}</div>;
}

export function Card({ children, className }: { children: ReactNode; className?: string }) {
  return <div className={`card ${className ?? ""}`}>{children}</div>;
}

export function SectionHeader({
  icon,
  title,
  subtitle,
  trailing,
}: {
  icon?: ReactNode;
  title: string;
  subtitle?: string;
  trailing?: ReactNode;
}) {
  return (
    <div className="section-header">
      {icon ? <span className="section-header-icon">{icon}</span> : null}
      <div className="section-header-text">
        <div className="section-header-title">{title}</div>
        {subtitle ? <div className="section-header-subtitle">{subtitle}</div> : null}
      </div>
      {trailing ? <div className="section-header-trailing">{trailing}</div> : null}
    </div>
  );
}

export function Spinner({ size = 48 }: { size?: number }) {
  return (
    <svg
      className="spinner"
      width={size}
      height={size}
      viewBox="0 0 48 48"
      aria-label="加载中"
    >
      <circle cx="24" cy="24" r="18" fill="none" stroke="var(--color-track)" strokeWidth="4" />
      <path
        d="M42 24a18 18 0 0 0-18-18"
        fill="none"
        stroke="var(--color-primary)"
        strokeWidth="4"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function IconButton({
  icon,
  label,
  onClick,
  disabled,
  focused,
  loading,
}: {
  icon: ReactNode;
  label: string;
  onClick?: () => void;
  disabled?: boolean;
  focused?: boolean;
  loading?: boolean;
}) {
  return (
    <button
      type="button"
      className={`icon-button ${focused ? "icon-button-focused" : ""}`}
      title={label}
      aria-label={label}
      onClick={onClick}
      disabled={disabled}
    >
      {loading ? <span className="icon-button-spinner" /> : icon}
    </button>
  );
}

export function Divider({ height = 40 }: { height?: number }) {
  return <span className="divider" style={{ height }} />;
}

export function SectionDivider() {
  return <hr className="section-divider" />;
}

export function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="metric">
      <div className="metric-label">{label}</div>
      <div className="metric-value">{value}</div>
    </div>
  );
}

export function ProgressBar({ percent, color }: { percent: number; color: string }) {
  return (
    <div className="progress">
      <div
        className="progress-fill"
        style={{ width: `${Math.min(100, Math.max(0, percent))}%`, background: color }}
      />
    </div>
  );
}

export function Toast({ message, onDismiss }: { message: string; onDismiss: () => void }) {
  return (
    <div className="toast" role="status" onClick={onDismiss}>
      {message}
    </div>
  );
}

export function ConfirmDialog({
  title,
  body,
  confirmLabel,
  cancelLabel,
  onConfirm,
  onCancel,
}: {
  title: string;
  body: string;
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: () => void;
  onCancel: () => void;
}) {
  return (
    <div className="dialog-backdrop" role="dialog" aria-modal="true">
      <div className="dialog">
        <div className="dialog-title">{title}</div>
        <div className="dialog-body">{body}</div>
        <div className="dialog-actions">
          <button type="button" className="btn btn-secondary" onClick={onCancel}>
            {cancelLabel}
          </button>
          <button type="button" className="btn btn-primary" onClick={onConfirm}>
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

export function EmptyState({
  icon,
  title,
  detail,
  action,
}: {
  icon?: ReactNode;
  title: string;
  detail: string;
  action?: ReactNode;
}) {
  return (
    <div className="empty-state">
      {icon ? <span className="empty-state-icon">{icon}</span> : null}
      <div className="empty-state-title">{title}</div>
      <div className="empty-state-detail">{detail}</div>
      {action}
    </div>
  );
}

export function AppLogo({ size = 16 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <defs>
        <linearGradient id="app-logo-gradient" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0%" stopColor="#3b82f6" />
          <stop offset="100%" stopColor="#2563eb" />
        </linearGradient>
      </defs>
      <rect x="1" y="1" width="30" height="30" rx="8" fill="url(#app-logo-gradient)" />
      <text
        x="16"
        y="21.5"
        textAnchor="middle"
        fontSize="13"
        fontWeight="700"
        fill="#ffffff"
        fontFamily="var(--font-ui)"
      >
        OC
      </text>
    </svg>
  );
}

export { Icons };

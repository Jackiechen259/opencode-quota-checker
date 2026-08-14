// Settings page: connection, monitoring rules, updater, danger zone.
//
// Data flow: React edits local form state → `save_config` command → Rust
// `AppConfig::validate` (the final authority) → canonical config returned.

import { useEffect, useState } from "react";
import { ConfirmDialog, Dot, Notice, StatusBadge } from "../components/common";
import { Icons } from "../components/icons";
import { bytes, lastCheckedText } from "../lib/format";
import { api } from "../services/tauri";
import type {
  AppConfig,
  AppError,
  AppStatusDto,
  UpdateStateDto,
} from "../types/models";

interface Props {
  status: AppStatusDto | null;
  config: AppConfig | null;
  update: UpdateStateDto | null;
  onClose: () => void;
  onConfigSaved: (config: AppConfig) => void;
  onRefreshStatus: () => void;
}

function SettingsCard({
  icon,
  title,
  subtitle,
  trailing,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  trailing?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="settings-card">
      <div className="settings-card-header">
        <span className="settings-card-icon">{icon}</span>
        <div>
          <div className="settings-card-title">{title}</div>
          <div className="settings-card-subtitle">{subtitle}</div>
        </div>
        {trailing ? <div className="settings-card-trailing">{trailing}</div> : null}
      </div>
      {children}
    </section>
  );
}

function FormField({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="settings-field">
      <div className="settings-field-label">{label}</div>
      {children}
      {hint ? <div className="settings-field-hint">{hint}</div> : null}
    </div>
  );
}

function ToggleRow({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="settings-toggle-row">
      <span className="settings-toggle-label">{label}</span>
      <span style={{ flex: 1 }} />
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        className={`toggle ${checked ? "toggle-on" : ""}`}
        onClick={() => onChange(!checked)}
      >
        <span className="toggle-knob" />
      </button>
    </div>
  );
}

function UpdateStatusBlock({ update }: { update: UpdateStateDto }) {
  switch (update.status) {
    case "checking":
      return <div className="settings-update-status">正在检查更新…</div>;
    case "available": {
      const tag = update.available?.tag ?? "新版本";
      return (
        <div className="settings-update-status settings-update-row">
          <span>新版本 {tag} 可用</span>
          <span style={{ flex: 1 }} />
          <button
            type="button"
            className="btn btn-soft"
            onClick={() => void api.openReleaseNotes()}
          >
            查看更新说明
          </button>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => void api.downloadUpdate()}
          >
            下载更新
          </button>
        </div>
      );
    }
    case "downloading": {
      const tag = update.available?.tag ?? "更新";
      const progress = update.progress;
      const ratio = progress?.total ? progress.downloaded / progress.total : 0;
      return (
        <div className="settings-update-status">
          <div>正在下载 {tag}…</div>
          <div className="progress settings-update-progress">
            <div
              className="progress-fill"
              style={{ width: `${Math.min(100, ratio * 100)}%`, background: "var(--color-primary)" }}
            />
          </div>
          {progress ? (
            <div className="settings-update-progress-label">
              {bytes(progress.downloaded)}
              {progress.total ? ` / ${bytes(progress.total)}` : ""}
            </div>
          ) : null}
        </div>
      );
    }
    case "ready_to_install": {
      const version = update.downloaded_version ?? "";
      const installLabel = "安装并重启";
      return (
        <div className="settings-update-status settings-update-row">
          <span>v{version} 已准备好</span>
          <span style={{ flex: 1 }} />
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => void api.installUpdate()}
          >
            {installLabel}
          </button>
        </div>
      );
    }
    case "error":
      return update.error ? <Notice kind="error">{update.error.user}</Notice> : null;
    default:
      return null;
  }
}

export function Settings({
  status,
  config,
  update,
  onClose,
  onConfigSaved,
  onRefreshStatus,
}: Props) {
  const [interval, setInterval] = useState("");
  const [fiveHour, setFiveHour] = useState("");
  const [weekly, setWeekly] = useState("");
  const [monthly, setMonthly] = useState("");
  const [workspace, setWorkspace] = useState("");
  const [cookie, setCookie] = useState("");
  const [saving, setSaving] = useState(false);
  const [mutatingCredentials, setMutatingCredentials] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);

  useEffect(() => {
    if (!config) return;
    setInterval(String(config.monitor_interval_secs));
    setFiveHour(String(config.thresholds.five_hour));
    setWeekly(String(config.thresholds.weekly));
    setMonthly(String(config.thresholds.monthly));
    setWorkspace(config.opencode_workspace_id ?? "");
  }, [config]);

  const persist = async (mutation: (config: AppConfig) => AppConfig) => {
    if (!config) return;
    setSaving(true);
    setError(null);
    setNotice(null);
    try {
      const saved = await api.saveConfig(mutation(config));
      onConfigSaved(saved);
      return saved;
    } catch (caught) {
      setError(caught as AppError);
      return null;
    } finally {
      setSaving(false);
    }
  };

  const startMonitor = async () => {
    const saved = await persist((config) => ({
      ...config,
      monitor_enabled: true,
      monitor_interval_secs: Number(interval),
      thresholds: {
        five_hour: Number(fiveHour),
        weekly: Number(weekly),
        monthly: Number(monthly),
      },
    }));
    if (saved) {
      await api.setMonitor(true).catch(() => {});
      setNotice("监控配置已保存并启动。");
      await api.refreshUsage().catch(() => {});
    }
  };

  const stopMonitor = async () => {
    await api.setMonitor(false).catch(() => {});
    const saved = await persist((config) => ({ ...config, monitor_enabled: false }));
    if (saved) setNotice("监控已停止。");
  };

  const saveConnection = async () => {
    if (!workspace.trim() || !cookie.trim() || mutatingCredentials) return;
    setMutatingCredentials(true);
    setError(null);
    setNotice(null);
    try {
      await api.saveCredentials(workspace.trim(), cookie);
      setCookie("");
      setNotice("连接已保存到系统钥匙串。");
      onRefreshStatus();
    } catch (caught) {
      setError(caught as AppError);
    } finally {
      setMutatingCredentials(false);
    }
  };

  const clearConnection = async () => {
    setConfirmClear(false);
    setMutatingCredentials(true);
    setError(null);
    setNotice(null);
    try {
      await api.clearCredentials();
      setNotice("连接已清除，监控已停止。");
      onRefreshStatus();
    } catch (caught) {
      setError(caught as AppError);
    } finally {
      setMutatingCredentials(false);
    }
  };

  const toggleUpdateChecks = async (enabled: boolean) => {
    const saved = await persist((config) => ({ ...config, update_checks_enabled: enabled }));
    if (saved && enabled) void api.checkForUpdate();
  };

  const toggleAutoDownload = async (enabled: boolean) => {
    await persist((config) => ({ ...config, auto_download_updates: enabled }));
  };

  if (!config) return null;
  const credentials = status?.credentials;
  const monitorEnabled = status?.monitor.enabled ?? config.monitor_enabled;

  return (
    <div className="settings-page">
      <div className="settings-header">
        <div>
          <div className="settings-title">设置</div>
          <div className="settings-subtitle">管理 OpenCode 连接和额度监控规则</div>
        </div>
        <span style={{ flex: 1 }} />
        <button type="button" className="icon-button" title="关闭" aria-label="关闭" onClick={onClose}>
          <Icons.Close size={16} />
        </button>
      </div>

      {error ? <Notice kind="error">{error.user}</Notice> : null}
      {notice ? <Notice kind="success">{notice}</Notice> : null}
      {status?.config_error ? (
        <Notice kind="warning">配置文件读取失败，已使用默认配置：{status.config_error.user}</Notice>
      ) : null}

      <SettingsCard
        icon={<Icons.Globe size={16} />}
        title="OpenCode Go"
        subtitle="OpenCode 账户连接"
        trailing={
          credentials ? (
            <StatusBadge tone={credentials.available ? "success" : "neutral"}>
              {credentials.available ? "已连接" : "未配置"}
            </StatusBadge>
          ) : undefined
        }
      >
        <FormField label="Workspace ID">
          <input
            className="text-input"
            placeholder="ws_xxxxxxxxxxxxx"
            value={workspace}
            onChange={(event) => setWorkspace(event.target.value)}
            autoComplete="off"
          />
        </FormField>
        <FormField label="Auth Cookie">
          <input
            className="text-input"
            placeholder="粘贴 auth Cookie"
            type="password"
            value={cookie}
            onChange={(event) => setCookie(event.target.value)}
            autoComplete="off"
            spellCheck={false}
          />
        </FormField>
        <div className="settings-security-hint">
          <div>🔒 凭证安全保存在系统钥匙串中</div>
          <div className="settings-security-hint-sub">仅在请求 OpenCode API 时使用。</div>
        </div>
        <div className="settings-row-right">
          <button
            type="button"
            className="btn btn-primary"
            disabled={mutatingCredentials || !workspace.trim() || !cookie.trim()}
            onClick={saveConnection}
          >
            {mutatingCredentials ? "保存中…" : "保存连接"}
          </button>
        </div>
      </SettingsCard>

      <SettingsCard
        icon={<Icons.Activity size={16} />}
        title="额度监控"
        subtitle="配置自动检查频率和通知阈值"
        trailing={
          <span className="settings-monitor-badge">
            <Dot color={monitorEnabled ? "var(--color-success)" : "var(--color-text-muted)"} />
            {monitorEnabled ? "运行中" : "已停止"}
          </span>
        }
      >
        <FormField label="检查间隔" hint="允许范围 30–3600 秒">
          <div className="settings-interval-row">
            <span>每</span>
            <input
              className="text-input settings-interval-input"
              value={interval}
              onChange={(event) => setInterval(event.target.value)}
              inputMode="numeric"
            />
            <span>秒</span>
          </div>
        </FormField>
        <FormField label="通知阈值">
          <div className="settings-thresholds">
            {[
              ["5 小时", fiveHour, setFiveHour],
              ["近一周", weekly, setWeekly],
              ["近一月", monthly, setMonthly],
            ].map(([label, value, setter]) => (
              <div className="settings-threshold" key={label as string}>
                <span className="settings-threshold-label">{label as string}</span>
                <input
                  className="text-input settings-threshold-input"
                  value={value as string}
                  onChange={(event) => (setter as (value: string) => void)(event.target.value)}
                  inputMode="decimal"
                />
              </div>
            ))}
          </div>
        </FormField>
        <div className="settings-row-right">
          {monitorEnabled ? (
            <button
              type="button"
              className="btn btn-secondary"
              disabled={saving}
              onClick={stopMonitor}
            >
              {saving ? "保存中…" : "停止监控"}
            </button>
          ) : (
            <button
              type="button"
              className="btn btn-primary"
              disabled={saving}
              onClick={startMonitor}
            >
              {saving ? "保存中…" : "保存并启动"}
            </button>
          )}
        </div>
      </SettingsCard>

      <SettingsCard
        icon={<Icons.Refresh size={16} />}
        title="应用更新"
        subtitle="从 GitHub Releases 自动发现并安装新版本"
        trailing={
          update ? (
            <StatusBadge
              tone={
                update.status === "up_to_date"
                  ? "success"
                  : update.status === "error"
                    ? "danger"
                    : update.status === "checking" || update.status === "downloading"
                      ? "primary"
                      : "neutral"
              }
            >
              {update.status === "checking"
                ? "检查中…"
                : update.status === "downloading"
                  ? "下载中…"
                  : update.status === "up_to_date"
                    ? "已是最新"
                    : update.status === "error"
                      ? "检查失败"
                      : "未检查"}
            </StatusBadge>
          ) : undefined
        }
      >
        <FormField label="当前版本">
          <span className="settings-version">v{status?.version ?? "0.1.2"}</span>
        </FormField>
        <ToggleRow
          label="自动检查更新"
          checked={config.update_checks_enabled}
          onChange={toggleUpdateChecks}
        />
        <ToggleRow
          label="自动下载更新"
          checked={config.auto_download_updates}
          onChange={toggleAutoDownload}
        />
        <FormField label="上次检查">
          <span className="settings-last-checked">{lastCheckedText(update?.last_checked_ms ?? null)}</span>
        </FormField>
        <div className="settings-row-right">
          <button
            type="button"
            className="btn btn-secondary"
            disabled={update?.status === "checking" || update?.status === "downloading" || update?.status === "installing"}
            onClick={() => void api.checkForUpdate()}
          >
            检查更新
          </button>
        </div>
        {update ? <UpdateStatusBlock update={update} /> : null}
      </SettingsCard>

      <section className="settings-card settings-danger-zone">
        <div className="settings-danger-title">危险操作</div>
        <div className="settings-danger-row">
          <div>
            <div className="settings-danger-label">清除所有连接凭证</div>
            <div className="settings-danger-hint">删除钥匙串中的 Auth Cookie 并停止监控。</div>
          </div>
          <span style={{ flex: 1 }} />
          <button
            type="button"
            className="btn btn-danger-outline"
            disabled={mutatingCredentials}
            onClick={() => setConfirmClear(true)}
          >
            清除凭证
          </button>
        </div>
      </section>

      {confirmClear ? (
        <ConfirmDialog
          title="清除连接凭证"
          body="将删除系统钥匙串中的 Auth Cookie 并停止监控。此操作无法撤销。"
          confirmLabel="确认清除"
          cancelLabel="取消"
          onConfirm={clearConnection}
          onCancel={() => setConfirmClear(false)}
        />
      ) : null}
    </div>
  );
}

// Settings page: connection, monitoring rules, updater, danger zone.
//
// Button contract:
//
// * Every action has its own pending key. Clicking one button only changes
//   that button's label and only disables the controls it would conflict
//   with — the old shared `saving` flag made a monitor save look like a
//   connection save and vice versa.
// * Config writes are serialized and always start from the backend's latest
//   config (`get_config`), so a stale frontend snapshot can never overwrite a
//   field that another action just persisted (workspace id, monitor_enabled).
// * Progress, stop and apply are three separate operations: "保存更改" keeps
//   monitoring in its current state, "启动监控" enables it, "停止监控" only
//   disables it. Stopping never validates or applies the draft form, so a
//   half-edited interval cannot block it.
// * Every invoke rejection is surfaced as a notice. No settings button is
//   fire-and-forget anymore.

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import { ConfirmDialog, Dot, Notice, StatusBadge } from "../components/common";
import { Icons } from "../components/icons";
import { bytes, lastCheckedText } from "../lib/format";
import { api } from "../services/tauri";
import type {
  AppConfig,
  AppError,
  AppStatusDto,
  UpdateStateDto,
  UpdateStatus,
} from "../types/models";

interface Props {
  status: AppStatusDto | null;
  config: AppConfig | null;
  update: UpdateStateDto | null;
  onClose: () => void;
  onConfigSaved: (config: AppConfig) => void;
  onRefreshStatus: () => void;
}

type PendingKey =
  | "open-console"
  | "save-connection"
  | "test-connection"
  | "clear-credentials"
  | "apply-monitor"
  | "start-monitor"
  | "stop-monitor"
  | "toggle-update-checks"
  | "toggle-auto-download"
  | "check-update"
  | "download-update"
  | "install-update"
  | "open-release-notes";

type MonitorAction = "apply" | "start" | "stop";

function usePendingActions() {
  const [pending, setPending] = useState<ReadonlySet<PendingKey>>(() => new Set());
  const active = useRef(new Set<PendingKey>());

  const begin = useCallback((key: PendingKey) => {
    // The ref is the synchronous guard: two clicks in the same tick must not
    // start the same action twice, before React has re-rendered the button.
    if (active.current.has(key)) return false;
    active.current.add(key);
    setPending(new Set(active.current));
    return true;
  }, []);

  const end = useCallback((key: PendingKey) => {
    active.current.delete(key);
    setPending(new Set(active.current));
  }, []);

  const has = useCallback((key: PendingKey) => pending.has(key), [pending]);

  return { begin, end, has };
}

/** Normalizes anything thrown by IPC into the `AppError` shape the notices use. */
function toAppError(caught: unknown): AppError {
  if (typeof caught === "object" && caught !== null) {
    const candidate = caught as Partial<AppError>;
    if (typeof candidate.user === "string" && candidate.user.length > 0) {
      return {
        code: candidate.code ?? "unexpected_error",
        user: candidate.user,
        detail: candidate.detail ?? candidate.user,
      };
    }
  }
  const detail = caught instanceof Error ? caught.message : String(caught);
  console.error("[settings] unexpected action failure", caught);
  return {
    code: "unexpected_error",
    user: detail.length > 0 ? detail : "操作失败，请重试。",
    detail,
  };
}

function SettingsCard({
  icon,
  title,
  subtitle,
  trailing,
  children,
}: {
  icon: ReactNode;
  title: string;
  subtitle: string;
  trailing?: ReactNode;
  children: ReactNode;
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
  children: ReactNode;
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
  hint,
  checked,
  disabled,
  busy,
  onChange,
}: {
  label: string;
  hint?: string;
  checked: boolean;
  disabled?: boolean;
  busy?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className={`settings-toggle-row ${disabled ? "settings-toggle-row-disabled" : ""}`}>
      <div className="settings-toggle-copy">
        <span className="settings-toggle-label">{label}</span>
        {hint ? <span className="settings-toggle-hint">{hint}</span> : null}
      </div>
      <span style={{ flex: 1 }} />
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={label}
        aria-busy={busy ? true : undefined}
        className={`toggle ${checked ? "toggle-on" : ""}`}
        disabled={disabled}
        onClick={() => onChange(!checked)}
      >
        <span className="toggle-knob" />
      </button>
    </div>
  );
}

type UpdateBadge = {
  label: string;
  tone: "neutral" | "primary" | "success" | "warning" | "danger";
};

function updateBadge(update: UpdateStateDto | null): UpdateBadge {
  if (!update) return { label: "未检查", tone: "neutral" };
  switch (update.status) {
    case "checking":
      return { label: "检查中…", tone: "primary" };
    case "available":
      return { label: "有可用更新", tone: "primary" };
    case "downloading":
      return { label: "下载中…", tone: "primary" };
    case "ready_to_install":
      return { label: "待安装", tone: "warning" };
    case "installing":
      return { label: "安装中…", tone: "primary" };
    case "up_to_date":
      return { label: "已是最新", tone: "success" };
    case "error":
      return { label: "检查失败", tone: "danger" };
    default:
      return { label: "未检查", tone: "neutral" };
  }
}

const UPDATE_BUSY_STATUSES: readonly UpdateStatus[] = [
  "checking",
  "downloading",
  "installing",
];

function UpdateStatusBlock({
  update,
  busy,
  onOpenNotes,
  onDownload,
  onInstall,
  onRetryCheck,
}: {
  update: UpdateStateDto;
  busy: boolean;
  onOpenNotes: () => void;
  onDownload: () => void;
  onInstall: () => void;
  onRetryCheck: () => void;
}) {
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
            disabled={busy}
            onClick={onOpenNotes}
          >
            查看更新说明
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={busy}
            onClick={onDownload}
          >
            下载更新
          </button>
        </div>
      );
    }
    case "downloading": {
      const tag = update.available?.tag ?? "更新";
      const progress = update.progress;
      const ratio = progress?.total
        ? Math.min(1, Math.max(0, progress.downloaded / progress.total))
        : 0;
      const percent = Math.round(ratio * 100);
      return (
        <div className="settings-update-status">
          <div>正在下载 {tag}…</div>
          <div
            className="progress settings-update-progress"
            role="progressbar"
            aria-valuenow={percent}
            aria-valuemin={0}
            aria-valuemax={100}
          >
            <div
              className="progress-fill"
              style={{ width: `${percent}%`, background: "var(--color-primary)" }}
            />
          </div>
          {progress ? (
            <div className="settings-update-progress-label">
              {progress.total ? `${percent}% · ` : ""}
              {bytes(progress.downloaded)}
              {progress.total ? ` / ${bytes(progress.total)}` : ""}
            </div>
          ) : null}
        </div>
      );
    }
    case "ready_to_install": {
      const version = update.downloadedVersion ?? update.available?.version ?? "";
      return (
        <div className="settings-update-status settings-update-row">
          <span>{version ? `v${version} 已准备好` : "更新已准备好"}</span>
          <span style={{ flex: 1 }} />
          <button
            type="button"
            className="btn btn-primary"
            disabled={busy}
            onClick={onInstall}
          >
            安装并重启
          </button>
        </div>
      );
    }
    case "installing":
      return <div className="settings-update-status">正在安装更新，应用即将重启…</div>;
    case "error":
      return (
        <div className="settings-update-status">
          <Notice kind="error">{update.error?.user ?? "更新失败，请重试。"}</Notice>
          <div className="settings-row-right">
            <button
              type="button"
              className="btn btn-soft"
              disabled={busy}
              onClick={onRetryCheck}
            >
              重试检查
            </button>
          </div>
        </div>
      );
    case "up_to_date":
      return <div className="settings-update-status">已是最新版本。</div>;
    default:
      return <div className="settings-update-status">尚未检查更新。</div>;
  }
}

/** Frontend UX validation; the Rust `AppConfig::validate` remains the final
 * authority and runs again on every save. */
export function validateMonitorForm(
  interval: string,
  fiveHour: string,
  weekly: string,
  monthly: string,
): string | null {
  const intervalNumber = Number(interval.trim());
  if (!Number.isInteger(intervalNumber) || intervalNumber < 30 || intervalNumber > 3600) {
    return "检查间隔必须是 30–3600 之间的整数。";
  }
  for (const [label, value] of [
    ["5 小时阈值", fiveHour],
    ["近一周阈值", weekly],
    ["近一月阈值", monthly],
  ] as const) {
    const parsed = Number(value.trim());
    if (!Number.isFinite(parsed) || parsed < 0 || parsed > 100) {
      return `${label}必须是 0–100 之间的数字。`;
    }
  }
  return null;
}

export function Settings({
  status,
  config,
  update,
  onClose,
  onConfigSaved,
  onRefreshStatus,
}: Props) {
  const [interval, setIntervalText] = useState("");
  const [fiveHour, setFiveHour] = useState("");
  const [weekly, setWeekly] = useState("");
  const [monthly, setMonthly] = useState("");
  const [workspace, setWorkspace] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [error, setError] = useState<AppError | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);
  const [confirmInstall, setConfirmInstall] = useState(false);

  const { begin, end, has: isPending } = usePendingActions();

  // Config mutations run one at a time, in click order. Each one re-reads the
  // backend config first, so two cards can never overwrite each other's fields
  // with a stale snapshot.
  const configWriteChain = useRef<Promise<unknown>>(Promise.resolve());
  const serializeConfigWrite = useCallback(function serialized<T>(
    task: () => Promise<T>,
  ): Promise<T> {
    const next = configWriteChain.current.then(task, task);
    configWriteChain.current = next.then(
      () => undefined,
      () => undefined,
    );
    return next;
  }, []);

  const applyMonitorForm = useCallback((next: AppConfig) => {
    setIntervalText(String(next.monitor_interval_secs));
    setFiveHour(String(next.thresholds.five_hour));
    setWeekly(String(next.thresholds.weekly));
    setMonthly(String(next.thresholds.monthly));
  }, []);

  const applyConnectionForm = useCallback((next: AppConfig) => {
    setWorkspace(next.opencode_workspace_id ?? "");
  }, []);

  // Initialise the draft form once. Later config updates caused by unrelated
  // cards (e.g. toggling update checks) must not discard monitor edits that
  // are still being typed; save actions reset their own fields explicitly.
  const formInitialized = useRef(false);
  useEffect(() => {
    if (!config || formInitialized.current) return;
    formInitialized.current = true;
    applyMonitorForm(config);
    applyConnectionForm(config);
  }, [config, applyMonitorForm, applyConnectionForm]);

  const run = useCallback(
    async (key: PendingKey, task: () => Promise<void>): Promise<boolean> => {
      if (!begin(key)) return false;
      setError(null);
      setNotice(null);
      try {
        await task();
        return true;
      } catch (caught) {
        setError(toAppError(caught));
        return false;
      } finally {
        end(key);
      }
    },
    [begin, end],
  );

  const persist = useCallback(
    async (
      mutation: (current: AppConfig) => AppConfig,
      applySavedToForm: "monitor" | "connection" | null = null,
    ) => {
      return serializeConfigWrite(async () => {
        const current = await api.getConfig();
        const saved = await api.saveConfig(mutation(current));
        if (applySavedToForm === "monitor") applyMonitorForm(saved);
        if (applySavedToForm === "connection") applyConnectionForm(saved);
        onConfigSaved(saved);
        return saved;
      });
    },
    [serializeConfigWrite, applyMonitorForm, applyConnectionForm, onConfigSaved],
  );

  if (!config) return null;

  const credentials = status?.credentials;
  const credentialsAvailable = credentials?.available ?? false;
  const credentialsMissing = credentials?.phase === "missing";
  const monitorEnabled = config.monitor_enabled;

  const workspaceValue = workspace.trim();
  const storedWorkspace = config.opencode_workspace_id ?? "";
  const workspaceDirty = workspaceValue !== storedWorkspace;
  const apiKeyPresent = apiKey.trim().length > 0;
  const canSaveWorkspaceOnly = credentialsAvailable && workspaceDirty && !apiKeyPresent;
  const canSaveConnection = apiKeyPresent || canSaveWorkspaceOnly;
  const canClearCredentials =
    credentials !== undefined &&
    !credentialsMissing &&
    credentials.phase !== "checking" &&
    !isPending("clear-credentials") &&
    !isPending("save-connection") &&
    !isPending("test-connection");

  const monitorDirty =
    interval !== String(config.monitor_interval_secs) ||
    fiveHour !== String(config.thresholds.five_hour) ||
    weekly !== String(config.thresholds.weekly) ||
    monthly !== String(config.thresholds.monthly);

  const connectionBusy =
    isPending("save-connection") ||
    isPending("test-connection") ||
    isPending("clear-credentials");
  const monitorBusy =
    isPending("apply-monitor") ||
    isPending("start-monitor") ||
    isPending("stop-monitor");
  const updateToggleBusy =
    isPending("toggle-update-checks") || isPending("toggle-auto-download");
  const updateActionBusy =
    isPending("check-update") ||
    isPending("download-update") ||
    isPending("install-update") ||
    isPending("open-release-notes");

  const updateView = update ?? status?.update ?? null;
  const updateStatusBusy = updateView
    ? UPDATE_BUSY_STATUSES.includes(updateView.status)
    : false;
  const checkUpdateDisabled =
    updateActionBusy || updateStatusBusy || updateView?.status === "ready_to_install";
  const badge = updateBadge(updateView);

  const validateMonitorDraft = (): boolean => {
    const validationError = validateMonitorForm(interval, fiveHour, weekly, monthly);
    if (!validationError) return true;
    setError({ code: "settings_invalid", user: validationError, detail: validationError });
    setNotice(null);
    return false;
  };

  const saveMonitorSettings = (action: MonitorAction) => {
    if (action === "start" && !credentialsAvailable) return;
    if (action === "stop") {
      void run("stop-monitor", async () => {
        // Stop from the persisted config, never from the draft: an invalid
        // draft interval must not keep the monitor running.
        await persist((current) => ({ ...current, monitor_enabled: false }));
        setNotice("监控已停止。");
      });
      return;
    }

    const applyDraft = action === "apply" || monitorDirty;
    if (applyDraft && !validateMonitorDraft()) return;

    if (action === "apply") {
      void run("apply-monitor", async () => {
        await persist(
          (current) => ({
            ...current,
            monitor_interval_secs: Number(interval),
            thresholds: {
              five_hour: Number(fiveHour),
              weekly: Number(weekly),
              monthly: Number(monthly),
            },
          }),
          "monitor",
        );
        setNotice("监控设置已保存。");
      });
      return;
    }

    void run("start-monitor", async () => {
      await persist(
        (current) => {
          const next: AppConfig = {
            ...current,
            monitor_enabled: true,
            ...(monitorDirty
              ? {
                  monitor_interval_secs: Number(interval),
                  thresholds: {
                    five_hour: Number(fiveHour),
                    weekly: Number(weekly),
                    monthly: Number(monthly),
                  },
                }
              : {}),
          };
          return next;
        },
        monitorDirty ? "monitor" : null,
      );
      setNotice(monitorDirty ? "监控配置已保存，监控已启动。" : "监控已启动。");
      // The monitor loop waits one interval before its first fetch; refresh
      // immediately so the dashboard is not empty after starting.
      void api.refreshUsage().catch((caught) => {
        console.error("[settings] refresh_usage failed", caught);
      });
    });
  };

  const saveConnection = () => {
    if (connectionBusy || !canSaveConnection) return;

    if (!apiKeyPresent) {
      // Workspace ID lives in the plain config, not the keyring, so it can be
      // saved without re-entering the API key.
      void run("save-connection", async () => {
        await persist(
          (current) => ({
            ...current,
            opencode_workspace_id: workspaceValue.length > 0 ? workspaceValue : null,
          }),
          "connection",
        );
        setNotice("Workspace ID 已保存。");
        void api.refreshUsage().catch((caught) => {
          console.error("[settings] refresh_usage failed", caught);
        });
      });
      return;
    }

    const key = apiKey.trim();
    void run("save-connection", async () => {
      await serializeConfigWrite(async () => {
        let keyStored = false;
        try {
          await api.saveCredentials(workspaceValue.length > 0 ? workspaceValue : null, key);
          keyStored = true;
          setApiKey("");
          setNotice("连接已校验并保存到系统钥匙串。");
        } catch (caught) {
          // The key is stored even when the probe fails; keep the input so a
          // keyring failure can be retried, and surface the probe error. The
          // draft workspace is also kept: resetting the field could discard
          // input from a save that failed before persisting it.
          setError(toAppError(caught));
        }

        // `save_credentials` persists the workspace before probing. Adopt the
        // canonical config so a later save cannot resurrect the previous id.
        try {
          const canonical = await api.getConfig();
          onConfigSaved(canonical);
          if (keyStored) applyConnectionForm(canonical);
        } catch (caught) {
          console.error("[settings] get_config after save_credentials failed", caught);
        }

        onRefreshStatus();
      });
    });
  };

  const testConnection = () => {
    if (!credentialsAvailable || apiKeyPresent) return;
    void run("test-connection", async () => {
      await api.testConnection();
      setNotice("连接正常，已读取到最新用量。");
      onRefreshStatus();
    });
  };

  const openConsole = () => {
    void run("open-console", async () => {
      await api.openConsolePage();
      setNotice("已在浏览器中打开 OpenCode Console。");
    });
  };

  const clearConnection = () => {
    setConfirmClear(false);
    void run("clear-credentials", async () => {
      await serializeConfigWrite(async () => {
        await api.clearCredentials();
        setApiKey("");
        setWorkspace("");
        setNotice("连接已清除，监控已停止。");
        try {
          const canonical = await api.getConfig();
          onConfigSaved(canonical);
          applyConnectionForm(canonical);
        } catch (caught) {
          console.error("[settings] get_config after clear_credentials failed", caught);
        }
        onRefreshStatus();
      });
    });
  };

  const checkForUpdate = () => {
    if (checkUpdateDisabled) return;
    void run("check-update", async () => {
      await api.checkForUpdate();
    });
  };

  const downloadUpdate = () => {
    if (updateActionBusy || updateView?.status !== "available") return;
    void run("download-update", async () => {
      await api.downloadUpdate();
    });
  };

  const installUpdate = () => {
    setConfirmInstall(false);
    void run("install-update", async () => {
      await api.installUpdate();
    });
  };

  const openReleaseNotes = () => {
    void run("open-release-notes", async () => {
      await api.openReleaseNotes();
    });
  };

  const toggleUpdateChecks = async (enabled: boolean) => {
    const didSave = await run("toggle-update-checks", async () => {
      await persist((current) => ({ ...current, update_checks_enabled: enabled }));
    });
    // Only an explicit enable triggers an immediate check; disabling simply
    // stops the next scheduled one. Auto-download guards keep a ready package
    // from being re-checked (and potentially invalidated).
    if (didSave && enabled) checkForUpdate();
  };

  const toggleAutoDownload = async (enabled: boolean) => {
    await run("toggle-auto-download", async () => {
      await persist((current) => ({ ...current, auto_download_updates: enabled }));
    });
  };

  const saveConnectionLabel = isPending("save-connection")
    ? "保存中…"
    : apiKeyPresent
      ? "保存连接"
      : "保存 Workspace ID";
  const saveConnectionTitle = canSaveConnection
    ? undefined
    : !credentialsAvailable
      ? "请先输入 API Key 并保存连接"
      : "修改 Workspace ID 或输入新的 API Key 后可保存";
  const testConnectionTitle = apiKeyPresent
    ? "请先保存新的 API Key，测试连接只校验已保存的凭证"
    : !credentialsAvailable
      ? "请先保存 API Key 后才能测试连接"
      : undefined;

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
      {status?.configError ? (
        <Notice kind="warning">配置文件读取失败，已使用默认配置：{status.configError.user}</Notice>
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
        <FormField
          label="Workspace ID"
          hint="可选。API Key 已绑定创建工作区，仅在需要覆盖时才填写。"
        >
          <input
            className="text-input"
            placeholder="ws_xxxxxxxxxxxxx"
            aria-label="Workspace ID"
            value={workspace}
            disabled={connectionBusy}
            onChange={(event) => setWorkspace(event.target.value)}
            autoComplete="off"
          />
        </FormField>
        <FormField
          label="API Key"
          hint="在 OpenCode Console → Account → Personal API keys 创建，建议勾选 Usage › Read 权限。"
        >
          <input
            className="text-input"
            placeholder="oc_sk_..."
            aria-label="API Key"
            type="password"
            value={apiKey}
            disabled={connectionBusy}
            onChange={(event) => setApiKey(event.target.value)}
            autoComplete="off"
            spellCheck={false}
          />
        </FormField>
        <div className="settings-security-hint">
          <div>🔒 凭证安全保存在系统钥匙串中</div>
          <div className="settings-security-hint-sub">
            仅作为请求头发送到 opencode.ai，不会写入配置或日志。
          </div>
        </div>
        <div className="settings-row-right">
          <button
            type="button"
            className="btn btn-soft"
            disabled={isPending("open-console")}
            onClick={openConsole}
          >
            {isPending("open-console") ? "正在打开…" : "打开 Console"}
          </button>
          <button
            type="button"
            className="btn btn-soft"
            disabled={connectionBusy || !credentialsAvailable || apiKeyPresent}
            title={testConnectionTitle}
            onClick={testConnection}
          >
            {isPending("test-connection") ? "测试中…" : "测试连接"}
          </button>
          <button
            type="button"
            className="btn btn-primary"
            disabled={connectionBusy || !canSaveConnection}
            title={saveConnectionTitle}
            onClick={saveConnection}
          >
            {saveConnectionLabel}
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
              aria-label="检查间隔（秒）"
              value={interval}
              disabled={monitorBusy}
              onChange={(event) => setIntervalText(event.target.value)}
              inputMode="numeric"
            />
            <span>秒</span>
          </div>
        </FormField>
        <FormField label="通知阈值">
          <div className="settings-thresholds">
            {[
              { key: "five_hour", label: "5 小时", value: fiveHour, set: setFiveHour },
              { key: "weekly", label: "近一周", value: weekly, set: setWeekly },
              { key: "monthly", label: "近一月", value: monthly, set: setMonthly },
            ].map((item) => (
              <div className="settings-threshold" key={item.key}>
                <span className="settings-threshold-label">{item.label}</span>
                <input
                  className="text-input settings-threshold-input"
                  aria-label={`${item.label}阈值`}
                  value={item.value}
                  disabled={monitorBusy}
                  onChange={(event) => item.set(event.target.value)}
                  inputMode="decimal"
                />
              </div>
            ))}
          </div>
        </FormField>
        <div className="settings-row-right">
          {!monitorEnabled && !credentialsAvailable ? (
            <span className="settings-action-hint">保存 API Key 后才能启动监控</span>
          ) : null}
          <button
            type="button"
            className="btn btn-secondary"
            disabled={monitorBusy || !monitorDirty}
            title={!monitorDirty ? "修改检查间隔或阈值后可保存" : undefined}
            onClick={() => saveMonitorSettings("apply")}
          >
            {isPending("apply-monitor") ? "保存中…" : "保存更改"}
          </button>
          {monitorEnabled ? (
            <button
              type="button"
              className="btn btn-secondary"
              disabled={monitorBusy}
              onClick={() => saveMonitorSettings("stop")}
            >
              {isPending("stop-monitor") ? "停止中…" : "停止监控"}
            </button>
          ) : (
            <button
              type="button"
              className="btn btn-primary"
              disabled={monitorBusy || !credentialsAvailable}
              title={!credentialsAvailable ? "请先在连接卡片中保存 API Key" : undefined}
              onClick={() => saveMonitorSettings("start")}
            >
              {isPending("start-monitor")
                ? "启动中…"
                : monitorDirty
                  ? "保存并启动"
                  : "启动监控"}
            </button>
          )}
        </div>
      </SettingsCard>

      <SettingsCard
        icon={<Icons.Refresh size={16} />}
        title="应用更新"
        subtitle="从 GitHub Releases 自动发现并安装新版本"
        trailing={<StatusBadge tone={badge.tone}>{badge.label}</StatusBadge>}
      >
        <FormField label="当前版本">
          <span className="settings-version">
            {status?.version ? `v${status.version}` : "未知"}
          </span>
        </FormField>
        <ToggleRow
          label="自动检查更新"
          checked={config.update_checks_enabled}
          disabled={updateToggleBusy}
          busy={isPending("toggle-update-checks")}
          onChange={(enabled) => void toggleUpdateChecks(enabled)}
        />
        <ToggleRow
          label="自动下载更新"
          hint={
            config.auto_download_updates
              ? "手动检查发现新版本时也会自动下载"
              : undefined
          }
          checked={config.auto_download_updates}
          disabled={updateToggleBusy}
          busy={isPending("toggle-auto-download")}
          onChange={(enabled) => void toggleAutoDownload(enabled)}
        />
        <FormField label="上次检查">
          <span className="settings-last-checked">
            {lastCheckedText(updateView?.lastCheckedMs ?? null)}
          </span>
        </FormField>
        <div className="settings-row-right">
          <button
            type="button"
            className="btn btn-secondary"
            disabled={checkUpdateDisabled}
            title={
              updateView?.status === "ready_to_install"
                ? "请先安装当前更新"
                : updateStatusBusy
                  ? "更新任务进行中，请稍候"
                  : undefined
            }
            onClick={checkForUpdate}
          >
            {isPending("check-update") || updateView?.status === "checking"
              ? "检查中…"
              : "检查更新"}
          </button>
        </div>
        {updateView ? (
          <UpdateStatusBlock
            update={updateView}
            busy={updateActionBusy}
            onOpenNotes={openReleaseNotes}
            onDownload={downloadUpdate}
            onInstall={() => {
              setConfirmClear(false);
              setConfirmInstall(true);
            }}
            onRetryCheck={checkForUpdate}
          />
        ) : null}
      </SettingsCard>

      <section className="settings-card settings-danger-zone">
        <div className="settings-danger-title">危险操作</div>
        <div className="settings-danger-row">
          <div>
            <div className="settings-danger-label">清除所有连接凭证</div>
            <div className="settings-danger-hint">删除钥匙串中的 API Key 并停止监控。</div>
          </div>
          <span style={{ flex: 1 }} />
          <button
            type="button"
            className="btn btn-danger-outline"
            disabled={!canClearCredentials}
            title={
              credentialsMissing
                ? "当前没有已保存的凭证"
                : credentials === undefined || credentials.phase === "checking"
                  ? "正在检查已保存的凭证"
                  : connectionBusy
                    ? "连接操作进行中，请稍候"
                    : undefined
            }
            onClick={() => {
              setConfirmInstall(false);
              setConfirmClear(true);
            }}
          >
            {isPending("clear-credentials") ? "清除中…" : "清除凭证"}
          </button>
        </div>
      </section>

      {confirmClear ? (
        <ConfirmDialog
          title="清除连接凭证"
          body="将删除系统钥匙串中的 API Key 并停止监控。此操作无法撤销。"
          confirmLabel="确认清除"
          cancelLabel="取消"
          onConfirm={clearConnection}
          onCancel={() => setConfirmClear(false)}
        />
      ) : null}

      {confirmInstall && updateView?.status === "ready_to_install" ? (
        <ConfirmDialog
          title="安装更新"
          body={`将安装 ${
            updateView.downloadedVersion
              ? `v${updateView.downloadedVersion}`
              : updateView.available?.tag ?? "更新"
          } 并重启应用。安装过程中请勿关闭应用。`}
          confirmLabel="安装并重启"
          cancelLabel="取消"
          onConfirm={installUpdate}
          onCancel={() => setConfirmInstall(false)}
        />
      ) : null}
    </div>
  );
}

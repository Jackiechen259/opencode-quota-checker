// Credentials page: OpenCode Console API key.
//
// The key is a secret: it lives in the password input, travels to the Rust
// command once, and is cleared from React state immediately after a successful
// save. It is never stored in the webview or written to logs.

import { useState } from "react";
import { AppLogo, Notice } from "../components/common";
import { api } from "../services/tauri";
import type { AppError, CredentialPhase } from "../types/models";

interface Props {
  configured: boolean;
  /** Keyring availability phase reported by the backend. */
  phase: CredentialPhase;
  /** Error attached to the keyring check (timeout / keyring failure). */
  statusError: AppError | null;
  /** Re-runs the keyring availability check. */
  onRecheck: () => void;
  onSaved: () => void;
}

export function Credentials({
  configured,
  phase,
  statusError,
  onRecheck,
  onSaved,
}: Props) {
  const [workspace, setWorkspace] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [mutating, setMutating] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [consoleNotice, setConsoleNotice] = useState<string | null>(null);

  // Only the API key is required: a personal key already targets one workspace.
  const canSave = !mutating && apiKey.trim().length > 0;

  const keyringFailed = phase === "error" || phase === "timeout";

  const save = async () => {
    if (!canSave) return;
    setMutating(true);
    setError(null);
    try {
      const trimmedWorkspace = workspace.trim();
      await api.saveCredentials(trimmedWorkspace.length > 0 ? trimmedWorkspace : null, apiKey);
      // The secret is cleared from the webview immediately after saving.
      setApiKey("");
      setWorkspace(trimmedWorkspace);
      onSaved();
    } catch (caught) {
      setError(caught as AppError);
    } finally {
      setMutating(false);
    }
  };

  const openConsole = async () => {
    setError(null);
    try {
      await api.openConsolePage();
      setConsoleNotice(
        "已在浏览器中打开 OpenCode Console。请进入 Account › Personal API keys 创建一个 Key（建议勾选 Usage › Read 权限），复制后粘贴到下方再保存。",
      );
    } catch (caught) {
      setError(caught as AppError);
    }
  };

  return (
    <div className="credentials-page">
      <div className="credentials-card">
        <span className="credentials-logo">
          <AppLogo size={44} />
        </span>
        <div className="credentials-title">尚未配置 OpenCode API Key</div>
        <div className="credentials-detail">
          OpenCode 提供结构化的用量接口，应用用一个 Console API Key 读取 5 小时、近一周与近一月的
          配额。Workspace ID 保存在普通配置中且可以留空，API Key 仅保存到系统钥匙串。
        </div>
        <button
          type="button"
          className="btn btn-primary credentials-login"
          onClick={openConsole}
        >
          在浏览器中打开 Console
        </button>
        <div className="credentials-or">或手动填写：</div>
        <input
          className="text-input"
          placeholder="Workspace ID（可选）"
          value={workspace}
          onChange={(event) => setWorkspace(event.target.value)}
          autoComplete="off"
        />
        <input
          className="text-input"
          placeholder="API Key（oc_sk_...）"
          type="password"
          value={apiKey}
          onChange={(event) => setApiKey(event.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
        <div className="credentials-warning">
          请将 API Key 视为密码保管。它会随请求发送到 opencode.ai，不会写入配置或日志。
        </div>
        <button
          type="button"
          className="btn btn-primary credentials-save"
          disabled={!canSave}
          onClick={save}
        >
          {mutating ? "校验并保存中…" : "保存到系统钥匙串"}
        </button>
        <div className="credentials-help">
          手动获取方式：登录 opencode.ai → 打开 Console → Account → Personal API keys →
          新建 Key（建议勾选 Usage › Read）→ 复制以 oc_sk_ 开头的值。
          如果你之前填写的是 auth Cookie，它已随旧版工作区面板一起停用，需要改用 API Key。
        </div>
        {consoleNotice ? <div className="credentials-login-notice">{consoleNotice}</div> : null}
        {error ? <Notice kind="error">{error.user}</Notice> : null}
        {keyringFailed && statusError ? <Notice kind="error">{statusError.user}</Notice> : null}
        {keyringFailed ? (
          <button
            type="button"
            className="btn credentials-recheck"
            onClick={onRecheck}
          >
            重新检查系统钥匙串
          </button>
        ) : null}
      </div>
      {configured ? (
        <div className="credentials-configured-hint">
          连接已配置。如需更换账户，请前往「设置」中的连接卡片。
        </div>
      ) : null}
    </div>
  );
}

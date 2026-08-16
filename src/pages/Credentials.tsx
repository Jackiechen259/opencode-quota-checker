// Credentials page: OpenCode Go Workspace ID + auth cookie.
//
// The cookie is a session secret: it lives in the password input, travels to
// the Rust command once, and is cleared from React state immediately after a
// successful save. It is never stored in the webview or written to logs.

import { useState } from "react";
import { Notice } from "../components/common";
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
  const [cookie, setCookie] = useState("");
  const [mutating, setMutating] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [loginNotice, setLoginNotice] = useState<string | null>(null);

  const canSave = !mutating && workspace.trim().length > 0 && cookie.trim().length > 0;

  const keyringFailed = phase === "error" || phase === "timeout";

  const save = async () => {
    if (!canSave) return;
    setMutating(true);
    setError(null);
    try {
      await api.saveCredentials(workspace.trim(), cookie);
      // The secret is cleared from the webview immediately after saving.
      setCookie("");
      setWorkspace((value) => value.trim());
      onSaved();
    } catch (caught) {
      setError(caught as AppError);
    } finally {
      setMutating(false);
    }
  };

  const openLogin = async () => {
    setError(null);
    try {
      await api.openLoginPage();
      setLoginNotice(
        "已在浏览器中打开 opencode.ai 登录页。请完成 GitHub / Google 登录，打开你的工作区，将地址栏的 Workspace ID 填入上方，并在浏览器开发者工具中复制 auth Cookie 填入下方，再点击保存。",
      );
    } catch (caught) {
      setError(caught as AppError);
    }
  };

  return (
    <div className="credentials-page">
      <div className="credentials-card">
        <div className="credentials-title">尚未配置 OpenCode Go</div>
        <div className="credentials-detail">
          OpenCode Go 尚无公开的配额 API，配额数据来自登录后的工作区面板。Workspace
          ID 保存在普通配置中，Auth Cookie 仅保存到系统钥匙串。
        </div>
        <button type="button" className="btn btn-primary credentials-login" onClick={openLogin}>
          在浏览器中登录
        </button>
        <div className="credentials-or">或手动填写：</div>
        <input
          className="text-input"
          placeholder="Workspace ID"
          value={workspace}
          onChange={(event) => setWorkspace(event.target.value)}
          autoComplete="off"
        />
        <input
          className="text-input"
          placeholder="Auth Cookie"
          type="password"
          value={cookie}
          onChange={(event) => setCookie(event.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
        <div className="credentials-warning">
          请将 Auth Cookie 视为密码保管。它会随请求发送到 opencode.ai，不会写入配置或日志。
        </div>
        <button
          type="button"
          className="btn btn-primary credentials-save"
          disabled={!canSave}
          onClick={save}
        >
          {mutating ? "保存中…" : "保存到系统钥匙串"}
        </button>
        <div className="credentials-help">
          手动获取方式：登录 opencode.ai → 打开 OpenCode Go 工作区 → 从地址栏复制
          Workspace ID → 在浏览器开发者工具中找到 opencode.ai 的 auth Cookie 值。
        </div>
        {loginNotice ? <div className="credentials-login-notice">{loginNotice}</div> : null}
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

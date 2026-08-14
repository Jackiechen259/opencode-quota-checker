// Raw-response debug overlay: view / copy / close.
//
// The raw dashboard HTML may contain server-side data, so a warning stays
// visible; the `auth` cookie and any request secrets are never part of it
// (opencode-core strips them before returning).

import { useState } from "react";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { Notice } from "../components/common";
import { Icons } from "../components/icons";
import { api } from "../services/tauri";
import type { AppError } from "../types/models";

interface Props {
  onClose: () => void;
  onToast: (message: string) => void;
}

export function Debug({ onClose, onToast }: Props) {
  const [raw, setRaw] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<AppError | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const body = await api.getRawDashboard();
      setRaw(body);
    } catch (caught) {
      setError(caught as AppError);
    } finally {
      setLoading(false);
    }
  };

  const copy = async () => {
    if (!raw) return;
    try {
      await writeText(raw);
      onToast("原始 JSON 已复制到剪贴板。");
    } catch {
      onToast("复制失败。");
    }
  };

  return (
    <div className="debug-page">
      <div className="debug-header">
        <span className="debug-title">原始 API JSON</span>
        <span style={{ flex: 1 }} />
        <button type="button" className="btn btn-primary" disabled={!raw} onClick={copy}>
          复制 JSON
        </button>
        <button type="button" className="btn btn-soft" onClick={onClose}>
          关闭（Esc）
        </button>
      </div>
      <Notice kind="warning">原始响应可能包含服务器数据，请勿对外分享。</Notice>
      <div className="debug-body">
        {raw !== null ? (
          <pre className="debug-pre">{raw}</pre>
        ) : loading ? (
          <div className="debug-placeholder">正在读取原始响应…</div>
        ) : error ? (
          <div className="debug-error">
            <div>{error.user}</div>
            <div className="debug-error-detail">{error.detail}</div>
          </div>
        ) : (
          <div className="debug-placeholder">
            <button type="button" className="btn btn-soft" onClick={load}>
              <Icons.Download size={14} /> 加载原始响应
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

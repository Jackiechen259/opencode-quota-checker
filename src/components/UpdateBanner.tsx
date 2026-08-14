// Non-blocking notification strip pinned above the dashboard when an update
// is available. Never interrupts interaction; dismissible for this run.

import { api } from "../services/tauri";
import type { UpdateStateDto } from "../types/models";
import { Icons } from "./icons";

export function UpdateBanner({ update }: { update: UpdateStateDto }) {
  let label: string;
  let action: { label: string; onClick: () => void } | null = null;

  switch (update.status) {
    case "ready_to_install": {
      const version = update.downloaded_version ?? "";
      label = `新版本 v${version} 已准备好`;
      action = { label: "安装并重启", onClick: () => void api.installUpdate() };
      break;
    }
    case "downloading":
      label = `正在下载 ${update.available?.tag ?? "更新"}…`;
      break;
    default:
      label = `新版本 ${update.available?.tag ?? "更新"} 可用`;
      action = { label: "更新", onClick: () => void api.downloadUpdate() };
      break;
  }

  return (
    <div className="update-banner">
      <span className="update-banner-spark">✨</span>
      <span className="update-banner-label">{label}</span>
      <span style={{ flex: 1 }} />
      <button
        type="button"
        className="btn btn-soft update-banner-action"
        onClick={() => void api.openReleaseNotes()}
      >
        查看
      </button>
      {action ? (
        <button type="button" className="btn btn-primary update-banner-action" onClick={action.onClick}>
          {action.label}
        </button>
      ) : null}
      <button
        type="button"
        className="icon-button"
        title="关闭"
        aria-label="关闭"
        onClick={() => void api.dismissUpdate()}
      >
        <Icons.Close size={14} />
      </button>
    </div>
  );
}

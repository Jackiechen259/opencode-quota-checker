// Startup failure screen: shown instead of an infinite loading state when
// the backend does not answer `get_boot_status` (IPC rejection, command
// failure, or the startup watchdog deadline). Never retries automatically —
// the user decides.

interface Props {
  title: string;
  message: string;
  detail?: unknown;
  onRetry: () => void;
}

function describeDetail(detail: unknown): string {
  if (detail === undefined || detail === null) return "";
  if (detail instanceof Error) return detail.message;
  if (typeof detail === "string") return detail;
  try {
    return JSON.stringify(detail, null, 2);
  } catch {
    return String(detail);
  }
}

export function StartupError({ title, message, detail, onRetry }: Props) {
  const detailText = describeDetail(detail);
  return (
    <div className="checking-state startup-error">
      <div className="startup-error-card">
        <div className="startup-error-title">{title}</div>
        <div className="startup-error-message">{message}</div>
        {detailText ? (
          <details className="startup-error-details">
            <summary>查看错误详情</summary>
            <pre className="startup-error-detail">{detailText}</pre>
          </details>
        ) : null}
        <div className="startup-error-actions">
          <button type="button" className="btn btn-primary" onClick={onRetry}>
            重新尝试
          </button>
        </div>
      </div>
    </div>
  );
}

// Compact floating view: highest-risk quota hero, not a squashed Full view.
//
// Hero percent + remaining, a single progress bar, and one status line.
// The only ticking piece is the reset countdown on the bottom row.

import { Dot } from "../../components/common";
import { quotaHealth, type UsageReport } from "../../types/models";
import { clampPercent, compactNumber, healthColors, healthLabel, highestWindow, progressWidth } from "./floatLayout";
import { FloatStateView } from "./FloatStateView";
import { ResetCountdown } from "./tickers";

export function CompactFloatView({
  report,
  loading,
  error,
  onRetry,
}: {
  report: UsageReport | null;
  loading: boolean;
  error: string | null;
  onRetry: () => void;
}) {
  const window = highestWindow(report);
  if (!window) {
    return (
      <div className="float-compact">
        <FloatStateView loading={loading} error={error} onRetry={onRetry} variant="compact" />
      </div>
    );
  }
  const health = quotaHealth(window.percent);
  const accent = healthColors[health];
  const percent = clampPercent(window.percent);
  return (
    <div className="float-compact">
      <div className="float-compact-hero">
        <div className="float-compact-percent-block">
          <span className="float-compact-percent" style={{ color: accent }}>
            {window.percent.toFixed(1)}%
          </span>
          <span className="float-compact-label" title={window.label}>
            {window.label}
          </span>
        </div>
        <div className="float-compact-remaining">
          <span className="float-compact-remaining-label">剩余额度</span>
          <span className="float-compact-remaining-value">
            {compactNumber(window.remaining)}
          </span>
        </div>
      </div>
      <div
        className="float-card-progress"
        role="progressbar"
        aria-valuenow={Math.round(percent)}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-label={`${window.label} 已用 ${percent.toFixed(1)}%`}
      >
        <div
          className="float-card-progress-fill"
          style={{ width: `${progressWidth(window.percent)}%`, background: accent }}
        />
      </div>
      <div className="float-compact-bottom">
        <span className="float-compact-health" style={{ color: accent }}>
          <Dot color={accent} size={7} />
          {healthLabel(health)}
        </span>
        <span className="float-spacer" />
        <ResetCountdown resetMs={window.reset_time} className="float-compact-reset" />
      </div>
    </div>
  );
}

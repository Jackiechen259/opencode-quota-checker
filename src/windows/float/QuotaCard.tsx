// Single quota card for the Full view.
//
// Only the dot, the percentage and the progress fill take the health color;
// the card surface stays neutral so several cards never shout at once.

import type { CSSProperties } from "react";
import { Dot } from "../../components/common";
import { quotaHealth, type WindowReport } from "../../types/models";
import { clampPercent, compactNumber, healthColors, progressWidth } from "./floatLayout";
import { ResetCountdown } from "./tickers";

export function QuotaCard({ window }: { window: WindowReport }) {
  const health = quotaHealth(window.percent);
  const accent = healthColors[health];
  const percent = clampPercent(window.percent);
  return (
    <div
      className="float-card"
      style={{ "--quota-accent": accent } as CSSProperties}
    >
      <div className="float-card-top">
        <span className="float-card-label">
          <Dot color={accent} size={8} />
          <span className="float-card-label-text" title={window.label}>
            {window.label}
          </span>
        </span>
        <span className="float-card-percent">{window.percent.toFixed(1)}%</span>
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
          style={{ width: `${progressWidth(window.percent)}%` }}
        />
      </div>
      <div className="float-card-bottom">
        <span className="float-card-muted">剩余 {compactNumber(window.remaining)}</span>
        <span className="float-spacer" />
        <ResetCountdown resetMs={window.reset_time} className="float-card-muted" />
      </div>
    </div>
  );
}

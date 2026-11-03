// SVG cumulative Go usage chart: usage line, linear forecast, 100% monthly
// limit, hover detail, and the daily-average footer.
//
// No chart library is used; the geometry is simple enough to keep the bundle
// small and the math is tested separately in `lib/goUsage.ts`.

import { useState, type MouseEvent } from "react";
import { formatAxisDate, formatGoDate, type GoSeries } from "../lib/goUsage";

const WIDTH = 960;
const HEIGHT = 300;
const LEFT = 54;
const RIGHT = 46;
const TOP = 24;
const BOTTOM = 34;
const PLOT_WIDTH = WIDTH - LEFT - RIGHT;
const PLOT_HEIGHT = HEIGHT - TOP - BOTTOM;

interface Props {
  series: GoSeries;
  startsAt: number;
  endsAt: number;
  percent: number;
}

function axisDate(ms: number): string {
  return formatAxisDate(new Date(ms).toISOString().slice(0, 10));
}

export function GoUsageChart({ series, startsAt, endsAt, percent }: Props) {
  const [active, setActive] = useState<number | null>(null);
  const yMax = Math.max(
    150,
    Math.ceil(
      Math.max(series.forecast ?? 0, percent, 100) / 50,
    ) * 50,
  );
  const x = (normalized: number) => LEFT + normalized * PLOT_WIDTH;
  const y = (value: number) => TOP + (1 - value / yMax) * PLOT_HEIGHT;

  const points = series.points;
  const line = points
    .map((point, index) => `${index === 0 ? "M" : "L"}${x(point.x).toFixed(2)},${y(point.value).toFixed(2)}`)
    .join(" ");
  const first = points[0] ?? { x: 0, value: 0 };
  const last = points[points.length - 1] ?? first;
  const area = `${line} L${x(last.x).toFixed(2)},${y(0).toFixed(2)} L${x(first.x).toFixed(2)},${y(0).toFixed(2)} Z`;
  const forecast =
    series.forecast !== null && series.forecast > last.value
      ? `M${x(last.x).toFixed(2)},${y(last.value).toFixed(2)} L${x(1).toFixed(2)},${y(series.forecast).toFixed(2)}`
      : null;

  const ticks = [0, 50, 100, 150].filter((tick) => tick <= yMax);
  if (yMax > 150) ticks.push(yMax);

  const activePoint =
    active !== null && active > 0 ? points[active] : null;

  // eslint-disable-next-line no-undef -- SVGSVGElement comes from lib.dom
  const onMove = (event: MouseEvent<SVGSVGElement>) => {
    if (points.length < 2) return;
    const rect = event.currentTarget.getBoundingClientRect();
    if (rect.width <= 0) return;
    const svgX = ((event.clientX - rect.left) / rect.width) * WIDTH;
    let nearest = 0;
    for (let index = 1; index < points.length; index += 1) {
      if (Math.abs(x(points[index].x) - svgX) < Math.abs(x(points[nearest].x) - svgX)) {
        nearest = index;
      }
    }
    setActive(nearest);
  };

  return (
    <div className="go-chart">
      <svg
        className="go-chart-svg"
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        role="img"
        aria-label="Go 月度累计用量与预测"
        onMouseMove={onMove}
        onMouseLeave={() => setActive(null)}
      >
        <defs>
          <linearGradient id="go-usage-fill" x1="0" y1="0" x2="0" y2="1">
            <stop offset="0%" stopColor="var(--color-success)" stopOpacity="0.16" />
            <stop offset="100%" stopColor="var(--color-success)" stopOpacity="0.01" />
          </linearGradient>
        </defs>

        {ticks.map((tick) => (
          <g key={tick}>
            <line
              x1={LEFT}
              x2={WIDTH - RIGHT}
              y1={y(tick)}
              y2={y(tick)}
              className="go-chart-grid"
            />
            <text x={LEFT - 10} y={y(tick) + 3} textAnchor="end" className="go-chart-axis">
              {tick}%
            </text>
          </g>
        ))}

        <line
          x1={LEFT}
          x2={WIDTH - RIGHT}
          y1={y(100)}
          y2={y(100)}
          className="go-chart-limit"
        />
        <text x={LEFT + 4} y={y(100) - 7} textAnchor="start" className="go-chart-axis">
          月度上限
        </text>

        <path d={area} fill="url(#go-usage-fill)" />
        <path d={line} className="go-chart-usage" />
        {forecast ? <path d={forecast} className="go-chart-forecast" /> : null}

        <circle cx={x(last.x)} cy={y(last.value)} r={4} className="go-chart-current" />
        {series.forecast !== null ? (
          <text
            x={WIDTH - RIGHT - 4}
            y={y(series.forecast) - 8}
            textAnchor="end"
            className="go-chart-forecast-label"
          >
            {Math.round(series.forecast)}%
          </text>
        ) : null}

        {[0, 0.5, 1].map((tick, index) => {
          const ms = startsAt + (endsAt - startsAt) * tick;
          return (
            <text
              key={tick}
              x={x(tick)}
              y={HEIGHT - 10}
              textAnchor={index === 0 ? "start" : index === 1 ? "middle" : "end"}
              className="go-chart-axis"
            >
              {axisDate(ms)}
            </text>
          );
        })}

        {activePoint ? (
          <line
            x1={x(activePoint.x)}
            x2={x(activePoint.x)}
            y1={TOP}
            y2={HEIGHT - BOTTOM}
            className="go-chart-cursor"
          />
        ) : null}
      </svg>

      <div className="go-chart-footer">
        <div className="go-chart-detail">
          {activePoint ? (
            <>
              <span>
                {activePoint.date
                  ? formatGoDate(Date.parse(`${activePoint.date}T00:00:00Z`))
                  : "当前"}
              </span>
              <span>累计 {activePoint.value.toFixed(1)}%</span>
              {activePoint.dailyValue !== undefined ? (
                <span>
                  当日 +{activePoint.dailyValue.toFixed(1)}%
                  {activePoint.requests !== undefined
                    ? ` · 请求 ${activePoint.requests.toLocaleString("en-US")}`
                    : ""}
                </span>
              ) : null}
            </>
          ) : series.perDay !== null ? (
            <>
              <span>平均每天 {series.perDay.toFixed(1)}%</span>
              {series.estimated ? <span>按当前进度估算</span> : null}
            </>
          ) : (
            <span>{series.empty ? "本期暂无用量" : "数据积累中"}</span>
          )}
        </div>
        <div className="go-chart-legend">
          <span>
            <i className="go-key go-key-usage" />
            已用
          </span>
          <span>
            <i className="go-key go-key-forecast" />
            预测
          </span>
          <span>
            <i className="go-key go-key-limit" />
            月度上限
          </span>
        </div>
      </div>
    </div>
  );
}

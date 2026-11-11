import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { GoCalendar } from "./GoCalendar";
import type { GoActivityDay } from "../types/models";

/** Wednesday 2026-11-11, 05:00 UTC. */
const NOW = Date.UTC(2026, 10, 11, 5);

const history: GoActivityDay[] = [
  { date: "2026-11-09", cost_microcents: 60_000_000, requests: 12, tokens: 1_200_000 },
  { date: "2026-11-10", cost_microcents: 30_000_000, requests: 6, tokens: 600_000 },
];

describe("GoCalendar", () => {
  it("renders the year grid, the ranked cells, and the headline stats", () => {
    const { container } = render(<GoCalendar days={history} nowMs={NOW} />);

    expect(screen.getByRole("img", { name: /2 天有用量/ })).toBeInTheDocument();
    expect(container.querySelectorAll(".go-calendar-cell")).toHaveLength(53 * 7);
    expect(screen.getByText("活跃天数")).toBeInTheDocument();
    expect(screen.getAllByText("2 天")).toHaveLength(2); // active days + streak
    expect(screen.getByText("180万")).toBeInTheDocument(); // 1.2M + 0.6M

    const busiest = container.querySelector('.go-calendar-cell[title*="2026-11-09"]');
    expect(busiest).toHaveAttribute("data-level", "4");
    const quietest = container.querySelector('.go-calendar-cell[title*="2026-11-10"]');
    expect(quietest).toHaveAttribute("data-level", "1");
  });

  it("reports the hovered day above the legend", () => {
    const { container } = render(<GoCalendar days={history} nowMs={NOW} />);
    expect(screen.getByText("悬停格子查看当天明细")).toBeInTheDocument();

    const cell = container.querySelector('.go-calendar-cell[title*="2026-11-09"]');
    expect(cell).not.toBeNull();
    fireEvent.mouseEnter(cell as Element);

    expect(screen.getByText("11 月 9 日", { selector: "span" })).toBeInTheDocument();
    expect(screen.getByText("12 请求")).toBeInTheDocument();
    expect(screen.getByText("$0.60")).toBeInTheDocument();
  });

  it("explains why the first part of the grid is empty", () => {
    render(<GoCalendar days={history} nowMs={NOW} />);
    expect(
      screen.getByText(/本地记录从 11 月 9 日 开始，更早的格子显示为空/),
    ).toBeInTheDocument();
  });

  it("shows an empty state before the first recorded day", () => {
    render(<GoCalendar days={[]} nowMs={NOW} />);

    expect(screen.getByRole("img", { name: /0 天有用量/ })).toBeInTheDocument();
    expect(screen.getByText(/还没有记录到 Go 用量/)).toBeInTheDocument();
    expect(screen.queryByText(/本地记录从/)).not.toBeInTheDocument();
  });
});

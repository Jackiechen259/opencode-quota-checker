// Self-contained ticking labels.
//
// Each component owns its own `useNow` interval, so the one-second tick
// re-renders only the label — never the quota list, header, or buttons
// around it (the float window is always-on-top and must stay cheap).

import { useNow } from "../../hooks/useTauriEvents";
import { countdownShort, relative } from "../../lib/format";
import { resetSeconds } from "./floatLayout";

/** "3h 21m 后重置" / "即将重置" for a quota window's reset timestamp. */
export function ResetCountdown({
  resetMs,
  className,
}: {
  resetMs: number;
  className?: string;
}) {
  const nowMs = useNow();
  const seconds = resetSeconds(resetMs, nowMs);
  const label = seconds <= 0 ? "即将重置" : `${countdownShort(seconds)} 后重置`;
  return <span className={className}>{label}</span>;
}

/** "更新于 刚刚" / "更新于 3 分钟前" for a fetch timestamp. */
export function RelativeUpdateTime({
  fetchedMs,
  className,
}: {
  fetchedMs: number;
  className?: string;
}) {
  const nowMs = useNow();
  return <span className={className}>更新于 {relative(fetchedMs, nowMs)}</span>;
}

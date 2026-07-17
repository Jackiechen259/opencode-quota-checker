<script lang="ts">
  import type { WindowReport } from "../types";

  let { window }: { window: WindowReport } = $props();
  let now = $state(Date.now());

  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  let liveResetInSecs = $derived(Math.max(0, Math.floor((window.reset_time - now) / 1000)));
  let colorClass = $derived(window.percent >= 90 ? "danger" : window.percent >= 70 ? "warning" : "ok");
  let ringOffset = $derived(276 - Math.min(window.percent, 100) * 2.76);

  function formatDuration(secs: number): string {
    if (secs <= 0) return "已重置";
    const h = Math.floor(secs / 3600), m = Math.floor((secs % 3600) / 60);
    return h > 0 ? `${h}时${m}分` : `${m}分`;
  }
  function statusLabel() { return colorClass === "danger" ? "超限" : colorClass === "warning" ? "预警" : "正常"; }
</script>

<article class="quota-card {colorClass}">
  <header class="card-head">
    <h4>{window.label}</h4>
    <span class="state"><i></i>{statusLabel()}</span>
  </header>
  <div class="card-content">
    <div class="ring-wrap" aria-label={`已用 ${window.percent.toFixed(1)}%`}>
      <svg viewBox="0 0 100 100" role="img">
        <circle class="ring-track" cx="50" cy="50" r="44" />
        <circle class="ring-value" cx="50" cy="50" r="44" pathLength="100" style={`stroke-dasharray: ${Math.min(window.percent, 100)} 100`} />
      </svg>
      <span>{window.percent.toFixed(1)}%</span>
    </div>
    <dl class="card-metrics">
      <div><dt>已用</dt><dd>{window.used.toFixed(1)} / {window.quota.toFixed(1)}</dd></div>
      <div><dt>剩余可用</dt><dd class="remaining">{window.remaining.toFixed(1)}</dd></div>
    </dl>
  </div>
  <footer>下次重置 <strong>{formatDuration(liveResetInSecs)}</strong></footer>
</article>

<style>
  .quota-card {
    min-height: 262px;
    display: flex;
    flex-direction: column;
    padding: 24px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: var(--shadow-sm);
    transition: transform .2s ease, border-color .2s ease, box-shadow .2s ease;
  }
  .quota-card:hover { transform: translateY(-2px); border-color: var(--border-strong); box-shadow: var(--shadow-md); }
  .card-head { display: flex; align-items: center; justify-content: space-between; }
  h4 { font-size: 16px; letter-spacing: -.01em; color: var(--text); }
  .state { display: inline-flex; align-items: center; gap: 7px; font-size: 13px; font-weight: 700; }
  .state i { width: 9px; height: 9px; border-radius: 50%; background: currentColor; }
  .ok .state, .ok .remaining { color: var(--green); }
  .warning .state, .warning .remaining { color: var(--yellow); }
  .danger .state, .danger .remaining { color: var(--red); }
  .card-content { display: grid; grid-template-columns: 124px 1fr; align-items: center; gap: 20px; flex: 1; padding: 20px 0 18px; }
  .ring-wrap { position: relative; width: 120px; height: 120px; }
  svg { width: 100%; height: 100%; transform: rotate(-90deg); }
  circle { fill: none; stroke-width: 7; }
  .ring-track { stroke: var(--ring-track); }
  .ring-value { stroke: var(--accent); stroke-linecap: round; transition: stroke-dasharray .7s ease; }
  .ok .ring-value { stroke: var(--green); }
  .warning .ring-value { stroke: var(--yellow); }
  .danger .ring-value { stroke: var(--red); }
  .ring-wrap span { position: absolute; inset: 0; display: grid; place-items: center; color: var(--text); font-size: 20px; font-weight: 700; letter-spacing: -.04em; font-variant-numeric: tabular-nums; }
  .card-metrics { display: grid; gap: 16px; }
  dt { margin-bottom: 5px; color: var(--text-muted); font-size: 12px; font-weight: 500; }
  dd { color: var(--text); font-size: 15px; font-weight: 650; font-variant-numeric: tabular-nums; letter-spacing: -.02em; }
  footer { padding-top: 16px; border-top: 1px solid var(--border); color: var(--text-dim); font-size: 13px; }
  footer strong { margin-left: 8px; color: var(--text-muted); font-weight: 650; font-variant-numeric: tabular-nums; }
  @media (max-width: 520px) { .quota-card { min-height: 230px; padding: 20px; } .card-content { grid-template-columns: 102px 1fr; gap: 14px; } .ring-wrap { width: 102px; height: 102px; } }
</style>

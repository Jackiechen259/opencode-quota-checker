<script lang="ts">
  import { onMount } from "svelte";
  import { listen, emit } from "@tauri-apps/api/event";
  import { getCurrentWindow, LogicalSize, PhysicalPosition, currentMonitor } from "@tauri-apps/api/window";
  import { fetchUsage, hasCredentials } from "../api";
  import type { UsageReport, WindowReport } from "../types";
  import { FLOAT_SIZES, getCompactPref, setCompactPref } from "../floatWindow";

  let report = $state<UsageReport | null>(null);
  let loading = $state(false);
  let error = $state("");
  let hasCreds = $state(false);
  let now = $state(Date.now());
  let compact = $state(false);
  let topDocked = $state(false);

  const TOP_SNAP_DISTANCE = 18;
  const TOP_RELEASE_DISTANCE = 24;

  $effect(() => {
    const timer = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(timer);
  });

  let worst = $derived(
    report?.windows.length
      ? report.windows.reduce((prev, curr) => (prev.percent > curr.percent ? prev : curr))
      : null
  );

  let status = $derived(worst ? colorClass(worst.percent) : "ok");
  let fiveHour = $derived(report?.windows.find((window) => window.key === "five_hour") ?? worst);
  let ringLength = 326.7;
  let ringOffset = $derived(worst ? ringLength - (Math.min(worst.percent, 100) / 100) * ringLength : ringLength);

  // 序列化窗口尺寸副作用,避免拖动期间多次 onMoved 的 setSize 叠加乱序;
  // moveToken 用于丢弃被更新的移动事件取代的过期结果;selfPos 用于吞掉自身 setPosition 的回声。
  let sizeChain: Promise<void> = Promise.resolve();
  let moveToken = 0;
  let selfPos: { x: number; y: number } | null = null;

  // 序列化 setSize 副作用:链式 Promise 保证执行顺序 = 调用顺序,避免拖动期间多次 onMoved 的 setSize 乱序完成。
  // 必须在调用时捕获尺寸值 s,不能在执行函数里读 topDocked/compact --
  // 否则延迟执行时状态已被后续事件改变,吸附时会用释放后的状态(没缩小)。
  // 显式捕获错误并读取实际 innerSize,确认 setSize 是否被平台最小尺寸钳制;
  // innerSize() 返回 PhysicalSize,必须经 scaleFactor 换算为逻辑像素再与请求的 LogicalSize 比较,
  // 否则高 DPI(如 150%)下 344x52 -> 516x78 会被误报为 clamp。
  async function setSizeTo(s: { width: number; height: number }) {
    const w = getCurrentWindow();
    try {
      await w.setSize(new LogicalSize(s.width, s.height));
      const factor = await w.scaleFactor();
      const actual = (await w.innerSize()).toLogical(factor);
      if (actual.width !== s.width || actual.height !== s.height) {
        console.warn(`[float] setSize(${s.width}x${s.height}) clamped to ${actual.width}x${actual.height} logical -- 最小尺寸约束`);
      }
    } catch (e) {
      console.error("[float] setSize failed", e);
    }
  }
  function applySize(): Promise<void> {
    const s = topDocked ? FLOAT_SIZES.docked : compact ? FLOAT_SIZES.compact : FLOAT_SIZES.full;
    const p = sizeChain.then(() => setSizeTo(s)).catch(() => {});
    sizeChain = p;
    return p;
  }

  async function toggleCompact() {
    topDocked = false;
    compact = !compact;
    await applySize();
    await setCompactPref(compact);
  }

  async function load() {
    loading = true;
    error = "";
    try { report = await fetchUsage(); } catch (e) { error = String(e); } finally { loading = false; }
  }

  async function close() {
    try { await emit("float-closed", null); } catch { /* ignore */ }
    await getCurrentWindow().close();
  }

  function planColor(pt: string): string {
    return { Small: "#91a4c7", Medium: "#72a7ff", Large: "#b397ff", Max: "#ffb866" }[pt] || "#91a4c7";
  }

  function colorClass(percent: number): string {
    return percent >= 90 ? "danger" : percent >= 70 ? "warning" : "ok";
  }

  function statusLabel(value: string): string {
    return value === "danger" ? "已超限" : value === "warning" ? "接近上限" : "运行正常";
  }

  function dockWindows(): WindowReport[] {
    if (!report) return [];
    const fiveHour = report.windows.find((window) => window.key === "five_hour");
    return fiveHour ? [fiveHour] : [];
  }

  // onMoved 在拖动时高频触发,handler 为 async fire-and-forget,多次调用会重叠乱序完成。
  // 用 moveToken 丢弃被后续事件取代的过期分支结果(避免过期 setPosition 把窗口拉回顶部);
  // selfPos 记录程序化 setPosition 的精确目标,在下一次 onMoved 命中该目标时吞掉回声,
  // 防止自身移动再次触发吸附判定 —— 仅在匹配事件到达时清除,不依赖 await 时序。
  async function handleWindowMoved(position: PhysicalPosition) {
    // 吞掉自身 setPosition 的回声:命中目标即清除并跳过吸附逻辑。
    // 必须先于 moveToken 递增,否则回声会取消正在 await currentMonitor 的真实拖动事件,
    // 导致该拖动事件被丢弃而窗口卡在吸附态。
    if (selfPos && Math.abs(position.x - selfPos.x) <= 1 && Math.abs(position.y - selfPos.y) <= 1) {
      selfPos = null;
      return;
    }

    const token = ++moveToken;

    const monitor = await currentMonitor();
    if (token !== moveToken) return;
    const monitorTop = monitor?.workArea.position.y ?? 0;
    const relativeY = position.y - monitorTop;

    if (!topDocked && relativeY <= TOP_SNAP_DISTANCE) {
      topDocked = true;
      await applySize();
      if (token !== moveToken) return;
      // 记录精确目标,等待对应的 onMoved 回声到来时由上方守卫吞掉。
      selfPos = { x: position.x, y: monitorTop };
      try { await getCurrentWindow().setPosition(new PhysicalPosition(position.x, monitorTop)); } catch { /* ignore */ }
      return;
    }

    if (topDocked && relativeY > TOP_RELEASE_DISTANCE) {
      topDocked = false;
      await applySize();
    }
  }

  function formatDuration(secs: number): string {
    if (secs <= 0) return "即将重置";
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return h > 0 ? `${h}时${m}分` : m > 0 ? `${m}分钟` : `${secs}秒`;
  }

  function resetInSecs(w: WindowReport): number { return Math.max(0, Math.floor((w.reset_time - now) / 1000)); }
  function formatTime(ms: number): string {
    return new Date(ms).toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" });
  }

  onMount(() => {
    let unlistenMoved: (() => void) | undefined;

    void (async () => {
      compact = await getCompactPref();
      if (compact) await applySize();
      hasCreds = await hasCredentials();
      if (hasCreds) await load();
      unlistenMoved = await getCurrentWindow().onMoved(({ payload }) => { void handleWindowMoved(payload); });
      listen<UsageReport>("usage-updated", (event) => { report = event.payload; error = ""; });
      listen<string>("usage-error", (event) => { error = event.payload; });
    })();

    return () => unlistenMoved?.();
  });
</script>

<div class="float" class:compact class:top-docked={topDocked} class:danger={status === "danger"} class:warning={status === "warning"}>
  {#if topDocked && report}
    <section class="dock-rails" data-tauri-drag-region aria-label="顶部吸附 5 小时配额">
      {#each dockWindows() as window (window.key)}
        <div class="dock-row {colorClass(window.percent)}" data-tauri-drag-region>
          <span data-tauri-drag-region>{window.label}</span>
          <div class="dock-track" data-tauri-drag-region><i style="width:{Math.min(window.percent, 100)}%"></i></div>
          <b data-tauri-drag-region>{window.percent.toFixed(0)}%</b>
        </div>
      {/each}
    </section>
  {:else}
  <header class="title-bar" data-tauri-drag-region>
    <div class="identity" data-tauri-drag-region>
      <span class="brand-dot" aria-hidden="true"></span>
      <div data-tauri-drag-region>
        <strong>方舟配额</strong>
        <span class="live"><i></i>实时监控</span>
      </div>
    </div>
    <div class="window-actions">
      <button class="icon-btn refresh" onclick={load} disabled={loading} title="刷新" aria-label="刷新">
        <svg class:spinning={loading} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 11a8.1 8.1 0 0 0-15.5-2M4 5v4h4"/><path d="M4 13a8.1 8.1 0 0 0 15.5 2M20 19v-4h-4"/></svg>
      </button>
      <button class="icon-btn" onclick={toggleCompact} title={compact ? "展开悬浮窗" : "收起悬浮窗"} aria-label="切换显示模式">
        {#if compact}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M7 3v4M17 3v4M7 21v-4M17 21v-4"/><rect x="5" y="7" width="14" height="10" rx="2"/></svg>
        {:else}
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 12h16"/><path d="M16 8l4 4-4 4"/></svg>
        {/if}
      </button>
      <button class="icon-btn close" onclick={close} title="关闭" aria-label="关闭"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"/></svg></button>
    </div>
  </header>

  <main class="body">
    {#if !hasCreds}
      <div class="placeholder"><span class="placeholder-mark">!</span><p>请先在主窗口配置凭证</p></div>
    {:else if error}
      <div class="placeholder error"><span class="placeholder-mark">!</span><p>更新失败</p><button onclick={load}>重新尝试</button></div>
    {:else if report && worst}
      {#if compact}
        <section class="compact-summary {colorClass(fiveHour!.percent)}">
          <div class="compact-window"><span class="eyebrow">5h 配额窗口</span><strong>{fiveHour!.label}</strong></div>
          <div class="compact-value"><span class="eyebrow">配额使用率</span><b>{fiveHour!.percent.toFixed(1)}%</b></div>
          <div class="compact-remaining"><span class="eyebrow">剩余 AFP</span><b>{fiveHour!.remaining.toFixed(1)}</b></div>
          <span class="compact-status"><i></i>{statusLabel(colorClass(fiveHour!.percent))}</span>
          <div class="compact-bar"><i style="width:{Math.min(fiveHour!.percent, 100)}%"></i></div>
        </section>
      {:else}
        <section class="overview">
          <div class="ring {status}" style="--ring-offset:{ringOffset}">
            <svg viewBox="0 0 120 120" aria-hidden="true"><circle class="ring-track" cx="60" cy="60" r="52"/><circle class="ring-value" cx="60" cy="60" r="52"/></svg>
            <div class="ring-copy"><span>最高负载</span><strong>{worst.percent.toFixed(1)}<em>%</em></strong><b>{worst.label}</b></div>
          </div>
          <div class="overview-info">
            <div class="plan-line"><span>当前套餐</span><b style="color:{planColor(report.plan_type)}">{report.plan_type}</b></div>
            <div class="metrics"><div><span>剩余 AFP</span><strong>{worst.remaining.toFixed(1)}</strong></div><div><span>重置倒计时</span><strong>{formatDuration(resetInSecs(worst))}</strong></div></div>
            <div class="usage-line"><span>已使用 {worst.used.toFixed(1)} / {worst.quota.toFixed(1)}</span><span>{worst.percent.toFixed(0)}%</span></div>
            <div class="usage-bar"><i style="width:{Math.min(worst.percent, 100)}%"></i></div>
          </div>
        </section>

        <section class="windows" aria-label="配额窗口明细">
          <div class="section-title"><span>全部窗口</span><small>{report.windows.length} 项</small></div>
          {#each report.windows as w (w.key)}
            <div class="window-row {colorClass(w.percent)}">
              <div class="window-heading"><span>{w.label}</span><b>{w.percent.toFixed(1)}%</b></div>
              <div class="mini-track"><i style="width:{Math.min(w.percent, 100)}%"></i></div>
            </div>
          {/each}
        </section>
      {/if}
    {:else}
      <div class="placeholder"><span class="loading-orb"></span><p>正在获取用量</p></div>
    {/if}
  </main>

  {#if report && !compact}<footer><span>更新于 {formatTime(report.fetched_at)}</span><span class="footer-status"><i></i>数据已同步</span></footer>{/if}
  {/if}
</div>

<style>
  .float {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    user-select: none;
    color: var(--text);
    background: var(--surface);
    border: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  .title-bar {
    position: relative;
    z-index: 1;
    min-height: 50px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 9px 0 14px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
    cursor: grab;
  }
  .title-bar:active { cursor: grabbing; }
  .identity { display: flex; align-items: center; gap: 9px; pointer-events: none; }
  .brand-dot {
    width: 10px; height: 10px;
    border-radius: 3px;
    background: var(--accent);
    transform: rotate(45deg);
  }
  .identity strong { display: block; font-size: 13px; letter-spacing: .02em; line-height: 1.05; color: var(--text); }
  .live { display: flex; align-items: center; gap: 4px; margin-top: 4px; color: var(--text-muted); font-size: 10px; line-height: 1; }
  .live i, .footer-status i {
    display: inline-block; width: 5px; height: 5px;
    border-radius: 50%; background: var(--green);
  }
  .window-actions { display: flex; gap: 2px; }
  .icon-btn {
    display: grid; place-items: center;
    width: 29px; height: 29px; padding: 0;
    color: var(--text-muted); background: transparent;
    border: 1px solid transparent; border-radius: 6px;
    transition: background .18s, color .18s, border-color .18s;
  }
  .icon-btn svg { width: 15px; height: 15px; }
  .icon-btn:hover:not(:disabled) { color: var(--text); background: rgba(15, 23, 42, .05); border-color: var(--border); }
  .icon-btn.close:hover:not(:disabled) { color: var(--red); background: rgba(239, 68, 68, .08); border-color: rgba(239, 68, 68, .2); }
  .spinning { animation: spin .8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .body { position: relative; z-index: 1; flex: 1; min-height: 0; display: flex; flex-direction: column; padding: 15px 16px 12px; overflow: auto; }

  .overview { position: relative; display: grid; grid-template-columns: 120px 1fr; align-items: center; gap: 16px; padding: 4px 0 17px; border-bottom: 1px solid var(--border); }
  .ring { position: relative; width: 120px; height: 120px; }
  .ring svg { position: relative; z-index: 1; width: 100%; height: 100%; transform: rotate(-90deg); }
  .ring circle { fill: none; stroke-width: 7; }
  .ring-track { stroke: var(--ring-track); }
  .ring-value { stroke: var(--accent); stroke-linecap: round; stroke-dasharray: 326.7; stroke-dashoffset: var(--ring-offset); transition: stroke-dashoffset .8s ease; }
  .ring.warning .ring-value { stroke: var(--yellow); }
  .ring.danger .ring-value { stroke: var(--red); }
  .ring-copy { position: absolute; z-index: 2; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; }
  .ring-copy span { color: var(--text-muted); font-size: 10px; }
  .ring-copy strong { margin: 2px 0 0; color: var(--text); font-size: 25px; letter-spacing: -.06em; line-height: 1; }
  .ring-copy em { font-size: 12px; font-style: normal; letter-spacing: 0; margin-left: 1px; }
  .ring-copy b { margin-top: 5px; color: var(--accent); font-size: 10px; font-weight: 600; }

  .overview-info { min-width: 0; }
  .plan-line, .usage-line { display: flex; justify-content: space-between; align-items: center; font-size: 10px; color: var(--text-muted); }
  .plan-line b { font-size: 11px; letter-spacing: .04em; }
  .metrics { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; margin: 13px 0; }
  .metrics div + div { padding-left: 10px; border-left: 1px solid var(--border); }
  .metrics span { display: block; color: var(--text-dim); font-size: 10px; white-space: nowrap; }
  .metrics strong { display: block; margin-top: 3px; color: var(--text); font-size: 14px; letter-spacing: -.03em; white-space: nowrap; }

  .usage-bar, .mini-track, .compact-bar { height: 4px; overflow: hidden; background: var(--ring-track); border-radius: 99px; }
  .usage-bar i, .mini-track i, .compact-bar i { display: block; height: 100%; border-radius: inherit; background: var(--accent); transition: width .8s ease; }
  .warning .usage-bar i, .window-row.warning i, .compact-summary.warning i { background: var(--yellow); }
  .danger .usage-bar i, .window-row.danger i, .compact-summary.danger i { background: var(--red); }

  .windows { margin-top: 10px; padding: 10px 11px 7px; border: 1px solid var(--border); border-radius: 10px; background: var(--surface-soft); }
  .section-title { display: flex; justify-content: space-between; margin-bottom: 7px; color: var(--text-muted); font-size: 10px; font-weight: 600; letter-spacing: .07em; text-transform: uppercase; }
  .section-title small { color: var(--text-dim); font-size: 10px; font-weight: 500; text-transform: none; }
  .window-row { padding: 5px 0; }
  .window-heading { display: flex; justify-content: space-between; margin-bottom: 5px; font-size: 11px; }
  .window-heading span { color: var(--text-muted); }
  .window-heading b { color: var(--accent); font-weight: 650; }
  .window-row.warning b { color: var(--yellow); }
  .window-row.danger b { color: var(--red); }
  .mini-track { height: 3px; }

  footer { position: relative; z-index: 1; display: flex; justify-content: space-between; padding: 9px 16px 10px; border-top: 1px solid var(--border); background: var(--surface-soft); color: var(--text-dim); font-size: 10px; }
  .footer-status { display: flex; align-items: center; gap: 5px; color: var(--text-muted); }

  .compact-summary { position: relative; display: grid; grid-template-columns: minmax(0, 1.22fr) .88fr .9fr; gap: 0; align-items: center; height: 100%; padding: 1px 2px 11px; }
  .compact-window, .compact-value, .compact-remaining { position: relative; z-index: 1; min-width: 0; padding: 0 11px; }
  .compact-window { padding-left: 12px; }
  .compact-value, .compact-remaining { border-left: 1px solid var(--border); }
  .compact-summary .eyebrow { display: block; margin-bottom: 5px; color: var(--text-dim); font-size: 9px; letter-spacing: .07em; white-space: nowrap; }
  .compact-summary strong { display: block; overflow: hidden; color: var(--text); font-size: 13px; text-overflow: ellipsis; white-space: nowrap; }
  .compact-value b, .compact-remaining b { display: block; color: var(--accent); font-size: 20px; letter-spacing: -.05em; line-height: 1; }
  .compact-remaining b { color: var(--text); font-size: 14px; letter-spacing: -.03em; }
  .compact-status { position: absolute; z-index: 2; right: 10px; bottom: 15px; display: inline-flex; align-items: center; gap: 4px; color: var(--accent); font-size: 9px; font-weight: 650; }
  .compact-status i { width: 5px; height: 5px; border-radius: 50%; background: currentColor; }
  .compact-summary.warning .compact-status { color: var(--yellow); }
  .compact-summary.danger .compact-status { color: var(--red); }
  .compact-bar { position: absolute; right: 7px; bottom: 0; left: 7px; z-index: 2; height: 4px; }

  .placeholder { margin: auto; display: flex; flex-direction: column; align-items: center; gap: 9px; color: var(--text-muted); font-size: 12px; text-align: center; }
  .placeholder-mark { display: grid; place-items: center; width: 26px; height: 26px; border: 1px solid var(--border); border-radius: 50%; color: var(--yellow); font-weight: 700; }
  .placeholder.error .placeholder-mark { color: var(--red); }
  .placeholder button { margin-top: 2px; padding: 6px 10px; background: var(--surface-soft); color: var(--text); border: 1px solid var(--border); box-shadow: none; font-size: 11px; }
  .loading-orb { width: 17px; height: 17px; border: 2px solid var(--ring-track); border-top-color: var(--accent); border-radius: 50%; animation: spin .8s linear infinite; }

  .float.compact .body { padding: 10px 14px 9px; }
  .float.compact .title-bar { min-height: 43px; }
  .float.compact .live { display: none; }
  .float.compact .brand-dot { width: 8px; height: 8px; }
  .float.compact footer { display: none; }

  .float.top-docked { border-color: var(--accent); }
  .dock-rails { display: grid; align-content: center; gap: 10px; height: 100%; padding: 9px 17px; cursor: grab; background: var(--surface-soft); }
  .dock-rails:active { cursor: grabbing; }
  .dock-row { display: grid; grid-template-columns: 44px 1fr 35px; align-items: center; gap: 9px; color: var(--text-muted); font-size: 11px; font-weight: 650; letter-spacing: .03em; }
  .dock-track { height: 5px; overflow: hidden; border: 1px solid var(--border); border-radius: 999px; background: var(--ring-track); }
  .dock-track i { display: block; height: 100%; border-radius: inherit; background: var(--accent); }
  .dock-row b { color: var(--accent); font-size: 12px; text-align: right; font-variant-numeric: tabular-nums; }
  .dock-row.warning .dock-track i { background: var(--yellow); } .dock-row.warning b { color: var(--yellow); }
  .dock-row.danger .dock-track i { background: var(--red); } .dock-row.danger b { color: var(--red); }

  @media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation-duration: .01ms !important; transition-duration: .01ms !important; } }
</style>

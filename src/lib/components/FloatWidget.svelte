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

  async function applySize() {
    const s = topDocked ? FLOAT_SIZES.docked : compact ? FLOAT_SIZES.compact : FLOAT_SIZES.full;
    try { await getCurrentWindow().setSize(new LogicalSize(s.width, s.height)); } catch { /* ignore */ }
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

  async function handleWindowMoved(position: PhysicalPosition) {
    const monitor = await currentMonitor();
    const monitorTop = monitor?.workArea.position.y ?? 0;
    const relativeY = position.y - monitorTop;

    if (!topDocked && relativeY <= TOP_SNAP_DISTANCE) {
      topDocked = true;
      await applySize();
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
  .float { --panel:#060d18; --surface:#0b1727; --line:rgba(85,159,255,.23); --text:#f2f7ff; --muted:#98abc5; --dim:#58708f; --accent:#2e9cff; --warn:#ffc04f; --danger:#ff6a63; position:relative; display:flex; flex-direction:column; height:100vh; overflow:hidden; user-select:none; color:var(--text); background:radial-gradient(ellipse at 50% -20%, rgba(24,108,212,.25), transparent 56%), linear-gradient(145deg,#08111f 0%,#050b15 100%); border:1px solid rgba(69,155,255,.64); box-shadow:inset 0 0 0 1px rgba(100,183,255,.08), inset 0 0 36px rgba(24,126,255,.08); font-variant-numeric:tabular-nums; }
  .float::before { position:absolute; inset:50px 0 auto; z-index:0; height:164px; background:linear-gradient(90deg, transparent 0 5%, rgba(46,156,255,.07) 5.3% 26%, transparent 26.3% 74%, rgba(46,156,255,.07) 74.3% 95%, transparent 95%), repeating-linear-gradient(90deg, transparent 0 23px, rgba(46,156,255,.1) 24px 25px, transparent 26px 49px); opacity:.7; pointer-events:none; content:""; mask-image:linear-gradient(to bottom,transparent,black 22%,black 62%,transparent); }
  .title-bar { position:relative; z-index:1; min-height:50px; display:flex; align-items:center; justify-content:space-between; padding:0 9px 0 14px; border-bottom:1px solid rgba(75,159,255,.34); background:rgba(7,16,29,.82); box-shadow:0 7px 18px rgba(0,0,0,.16); cursor:grab; }
  .title-bar::after { position:absolute; right:76px; bottom:-1px; left:42px; height:2px; background:linear-gradient(90deg,transparent,rgba(46,156,255,.95) 17% 72%,transparent); box-shadow:0 0 9px rgba(46,156,255,.55); content:""; }
  .title-bar:active { cursor:grabbing; }
  .identity { display:flex; align-items:center; gap:9px; pointer-events:none; }
  .brand-dot { position:relative; width:11px; height:11px; border:2px solid #b9e0ff; border-radius:3px; background:linear-gradient(135deg,#228cff,#6fc8ff); box-shadow:0 0 0 4px rgba(46,156,255,.1),0 0 12px rgba(46,156,255,.74); transform:rotate(45deg); }
  .identity strong { display:block; font-size:13px; letter-spacing:.04em; line-height:1.05; }
  .live { display:flex; align-items:center; gap:4px; margin-top:4px; color:var(--muted); font-size:10px; line-height:1; }
  .live i,.footer-status i { display:inline-block; width:5px; height:5px; border-radius:50%; background:#65e78d; box-shadow:0 0 8px rgba(101,231,141,.8); }
  .window-actions { display:flex; gap:2px; }
  .icon-btn { display:grid; place-items:center; width:29px; height:29px; padding:0; color:var(--muted); background:transparent; border:1px solid transparent; border-radius:6px; box-shadow:none; transition:background .18s,color .18s,border-color .18s; }
  .icon-btn svg { width:15px; height:15px; } .icon-btn:hover:not(:disabled) { color:#cbe9ff; background:rgba(46,156,255,.12); border-color:rgba(70,161,255,.35); } .icon-btn.close:hover:not(:disabled) { color:#ff917f; background:rgba(255,106,99,.13); border-color:rgba(255,106,99,.25); } .spinning { animation:spin .8s linear infinite; } @keyframes spin { to { transform:rotate(360deg); } }
  .body { position:relative; z-index:1; flex:1; min-height:0; display:flex; flex-direction:column; padding:15px 16px 12px; overflow:auto; }
  .overview { position:relative; display:grid; grid-template-columns:120px 1fr; align-items:center; gap:16px; padding:4px 0 17px; border-bottom:1px solid var(--line); }
  .ring { position:relative; width:120px; height:120px; filter:drop-shadow(0 0 9px rgba(46,156,255,.14)); } .ring::before,.ring::after { position:absolute; inset:3px; border:1px dashed rgba(75,167,255,.32); border-radius:50%; content:""; } .ring::after { inset:-4px; border-color:rgba(75,167,255,.18); transform:rotate(22deg); } .ring svg { position:relative; z-index:1; width:100%; height:100%; transform:rotate(-90deg); } .ring circle { fill:none; stroke-width:7; } .ring-track { stroke:#1a3049; } .ring-value { stroke:var(--accent); stroke-linecap:round; stroke-dasharray:326.7; stroke-dashoffset:var(--ring-offset); filter:drop-shadow(0 0 4px rgba(46,156,255,.95)); transition:stroke-dashoffset .8s ease; } .ring.warning .ring-value{stroke:var(--warn); filter:drop-shadow(0 0 4px rgba(255,192,79,.8))} .ring.danger .ring-value{stroke:var(--danger);filter:drop-shadow(0 0 4px rgba(255,106,99,.8))}
  .ring-copy { position:absolute; z-index:2; inset:0; display:flex; flex-direction:column; align-items:center; justify-content:center; } .ring-copy span { color:var(--muted); font-size:10px; } .ring-copy strong { margin:2px 0 0; color:var(--text); font-size:25px; letter-spacing:-.06em; line-height:1; text-shadow:0 0 16px rgba(220,241,255,.18); } .ring-copy em { font-size:12px; font-style:normal; letter-spacing:0; margin-left:1px; } .ring-copy b { margin-top:5px; color:#7fc5ff; font-size:10px; font-weight:600; }
  .overview-info { min-width:0; } .plan-line,.usage-line { display:flex; justify-content:space-between; align-items:center; font-size:10px; color:var(--muted); } .plan-line b { font-size:11px; letter-spacing:.04em; } .metrics { display:grid; grid-template-columns:1fr 1fr; gap:10px; margin:13px 0; } .metrics div+div { padding-left:10px; border-left:1px solid var(--line); } .metrics span { display:block; color:var(--dim); font-size:10px; white-space:nowrap; } .metrics strong { display:block; margin-top:3px; color:var(--text); font-size:14px; letter-spacing:-.03em; white-space:nowrap; }
  .usage-bar,.mini-track,.compact-bar { height:4px; overflow:hidden; background:#14263b; border-radius:99px; box-shadow:inset 0 1px 2px rgba(0,0,0,.42); } .usage-bar i,.mini-track i,.compact-bar i { display:block; height:100%; border-radius:inherit; background:linear-gradient(90deg,#1479ee,#4ab8ff); box-shadow:0 0 9px rgba(46,156,255,.9); transition:width .8s ease; } .warning .usage-bar i,.window-row.warning i,.compact-summary.warning i { background:var(--warn); box-shadow:0 0 9px rgba(255,192,79,.6); } .danger .usage-bar i,.window-row.danger i,.compact-summary.danger i { background:var(--danger); box-shadow:0 0 9px rgba(255,106,99,.6); }
  .windows { margin-top:10px; padding:10px 11px 7px; border:1px solid rgba(75,159,255,.16); border-radius:10px; background:rgba(7,17,30,.48); } .section-title { display:flex; justify-content:space-between; margin-bottom:7px; color:var(--muted); font-size:10px; font-weight:600; letter-spacing:.07em; text-transform:uppercase; } .section-title small { color:var(--dim); font-size:10px; font-weight:500; text-transform:none; } .window-row { padding:5px 0; } .window-heading { display:flex; justify-content:space-between; margin-bottom:5px; font-size:11px; } .window-heading span { color:#c8d8e9; } .window-heading b { color:#72c1ff; font-weight:650; } .window-row.warning b{color:var(--warn)} .window-row.danger b{color:var(--danger)} .mini-track { height:3px; }
  footer { position:relative; z-index:1; display:flex; justify-content:space-between; padding:9px 16px 10px; border-top:1px solid var(--line); background:rgba(5,12,22,.68); color:var(--dim); font-size:10px; } .footer-status { display:flex; align-items:center; gap:5px; color:var(--muted); }
  .compact-summary { position:relative; display:grid; grid-template-columns:minmax(0,1.22fr) .88fr .9fr; gap:0; align-items:center; height:100%; padding:1px 2px 11px; } .compact-summary::before { position:absolute; inset:1px 0 9px; border:1px solid rgba(69,155,255,.14); border-radius:8px; background:linear-gradient(90deg,rgba(46,156,255,.055),transparent 54%); content:""; } .compact-window,.compact-value,.compact-remaining { position:relative; z-index:1; min-width:0; padding:0 11px; } .compact-window { padding-left:12px; } .compact-value,.compact-remaining { border-left:1px solid rgba(103,160,221,.22); } .compact-summary .eyebrow { display:block; margin-bottom:5px; color:var(--dim); font-size:9px; letter-spacing:.07em; white-space:nowrap; } .compact-summary strong { display:block; overflow:hidden; color:#e1f1ff; font-size:13px; text-overflow:ellipsis; white-space:nowrap; } .compact-value b,.compact-remaining b { display:block; color:#69bdff; font-size:20px; letter-spacing:-.05em; line-height:1; text-shadow:0 0 12px rgba(46,156,255,.34); } .compact-remaining b { color:#dceeff; font-size:14px; letter-spacing:-.03em; text-shadow:none; } .compact-status { position:absolute; z-index:2; right:10px; bottom:15px; display:inline-flex; align-items:center; gap:4px; color:#83c9ff; font-size:9px; font-weight:650; } .compact-status i { width:5px; height:5px; border-radius:50%; background:currentColor; box-shadow:0 0 7px currentColor; } .compact-summary.warning .compact-status { color:var(--warn); } .compact-summary.danger .compact-status { color:var(--danger); } .compact-bar { position:absolute; right:7px; bottom:0; left:7px; z-index:2; height:4px; }
  .placeholder { margin:auto; display:flex; flex-direction:column; align-items:center; gap:9px; color:var(--muted); font-size:12px; text-align:center; } .placeholder-mark { display:grid; place-items:center; width:26px; height:26px; border:1px solid var(--line); border-radius:50%; color:var(--warn); font-weight:700; } .placeholder.error .placeholder-mark { color:var(--danger); } .placeholder button { margin-top:2px; padding:6px 10px; background:#263143; color:var(--text); border:0; box-shadow:none; font-size:11px; } .loading-orb { width:17px; height:17px; border:2px solid #354154; border-top-color:var(--accent); border-radius:50%; animation:spin .8s linear infinite; }
  .float.compact .body { padding:10px 14px 9px; } .float.compact .title-bar { min-height:43px; } .float.compact .live { display:none; } .float.compact .brand-dot { width:8px; height:8px; } .float.compact footer { display:none; }
  .float.top-docked { border-color:rgba(71,162,255,.78); box-shadow:inset 0 -1px 0 rgba(99,183,255,.34),0 5px 18px rgba(0,0,0,.3); }
  .dock-rails { display:grid; align-content:center; gap:10px; height:100%; padding:9px 17px; cursor:grab; background:linear-gradient(90deg,rgba(20,92,178,.15),transparent 50%,rgba(20,92,178,.1)); }
  .dock-rails:active { cursor:grabbing; }
  .dock-row { display:grid; grid-template-columns:44px 1fr 35px; align-items:center; gap:9px; color:var(--muted); font-size:11px; font-weight:650; letter-spacing:.03em; }
  .dock-track { height:5px; overflow:hidden; border:1px solid rgba(79,153,233,.25); border-radius:999px; background:#0d1d30; box-shadow:inset 0 1px 2px rgba(0,0,0,.45); }
  .dock-track i { display:block; height:100%; border-radius:inherit; background:linear-gradient(90deg,#1c80f4,#58c0ff); box-shadow:0 0 9px rgba(46,156,255,.8); }
  .dock-row b { color:#8ed2ff; font-size:12px; text-align:right; font-variant-numeric:tabular-nums; }
  .dock-row.warning .dock-track i { background:var(--warn); box-shadow:0 0 9px rgba(255,192,79,.6); } .dock-row.warning b { color:var(--warn); }
  .dock-row.danger .dock-track i { background:var(--danger); box-shadow:0 0 9px rgba(255,106,99,.6); } .dock-row.danger b { color:var(--danger); }
  @media (prefers-reduced-motion:reduce) { *,*::before,*::after { animation-duration:.01ms!important; transition-duration:.01ms!important; } }
</style>

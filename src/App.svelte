<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    fetchUsage,
    hasCredentials,
  } from "./lib/api";
  import type { UsageReport } from "./lib/types";
  import QuotaCard from "./lib/components/QuotaCard.svelte";
  import CredentialForm from "./lib/components/CredentialForm.svelte";
  import SettingsPanel from "./lib/components/SettingsPanel.svelte";
  import DebugRaw from "./lib/components/DebugRaw.svelte";
  import { load as loadStore } from "@tauri-apps/plugin-store";
  import { createFloatWindow, getFloatWindow } from "./lib/floatWindow";

  const STORE_PATH = "settings.json";
  const KEY_FLOAT_OPEN = "float_open";

  let report = $state<UsageReport | null>(null);
  let loading = $state(false);
  let error = $state("");
  let hasCreds = $state(false);
  let drawer = $state<null | "settings" | "debug">(null);
  let floatOpen = $state(false);

  // 最差窗口决定整体健康度
  let worst = $derived(
    report
      ? report.windows.reduce((prev, curr) =>
          prev.percent > curr.percent ? prev : curr
        )
      : null
  );
  let overallStatus = $derived(
    worst
      ? worst.percent >= 90
        ? "danger"
        : worst.percent >= 70
          ? "warning"
          : "ok"
      : "idle"
  );

  // 汇总指标，header 与 main 共用
  let highest = $derived(
    report
      ? report.windows.reduce((prev, curr) => (prev.percent > curr.percent) ? prev : curr)
      : null
  );
  let nextReset = $derived(
    report
      ? report.windows.reduce((prev, curr) => (prev.reset_time < curr.reset_time) ? prev : curr)
      : null
  );
  // 窗口健康概览:按使用率阈值统计各状态窗口数量(计数,而非跨桶求和)
  let health = $derived(
    report
      ? report.windows.reduce(
          (acc, w) => {
            if (w.percent >= 90) acc.danger++;
            else if (w.percent >= 70) acc.warning++;
            else acc.ok++;
            return acc;
          },
          { ok: 0, warning: 0, danger: 0 }
        )
      : { ok: 0, warning: 0, danger: 0 }
  );

  async function loadUsage() {
    loading = true;
    error = "";
    try {
      report = await fetchUsage();
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function checkCreds() {
    hasCreds = await hasCredentials();
  }

  // 凭证保存成功后:标记已配置、关闭抽屉、立即拉取用量进入主页。
  // 空状态表单与设置抽屉表单共用此回调,确保任一入口保存后都能跳转主页。
  async function onCredentialsSaved() {
    hasCreds = true;
    drawer = null;
    await loadUsage();
  }

  async function setFloatOpen(value: boolean) {
    floatOpen = value;
    const store = await loadStore(STORE_PATH);
    await store.set(KEY_FLOAT_OPEN, value);
    await store.save();
  }

  async function toggleFloat() {
    const existing = await getFloatWindow();
    if (existing) {
      await existing.close();
      await setFloatOpen(false);
    } else {
      try {
        await createFloatWindow();
        await setFloatOpen(true);
      } catch (e) {
        error = String(e);
      }
    }
  }

  onMount(async () => {
    listen("float-closed", () => {
      setFloatOpen(false);
    });

    const store = await loadStore(STORE_PATH);
    const restoreFloat = await store.get<boolean>(KEY_FLOAT_OPEN);
    if (restoreFloat) {
      try {
        await createFloatWindow();
        floatOpen = true;
      } catch (e) {
        error = String(e);
      }
    }

    await checkCreds();
    if (hasCreds) {
      await loadUsage();
    }
    listen<UsageReport>("usage-updated", (event) => {
      report = event.payload;
    });
    listen<string>("usage-error", (event) => {
      error = event.payload;
    });
  });

  function formatTime(ms: number): string {
    return new Date(ms).toLocaleTimeString("zh-CN", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  }

  function planColor(pt: string): string {
    const map: Record<string, string> = {
      Small: "#64748b",
      Medium: "#3b82f6",
      Large: "#8b5cf6",
      Max: "#f59e0b",
    };
    return map[pt] || "#64748b";
  }

  function severityClass(p: number): string {
    return p >= 90 ? "danger" : p >= 70 ? "warning" : "ok";
  }

  function statusLabel(s: string): string {
    return s === "danger" ? "危险" : s === "warning" ? "告警" : s === "ok" ? "正常" : "—";
  }
</script>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape" && drawer) drawer = null;
  }}
/>

<header>
  <div class="header-left">
    <div class="brand-mark">方</div>
    <div class="brand-text">
      <h1>方舟配额监控</h1>
      <span class="brand-sub">Agent Plan · AFP 配额</span>
    </div>
  </div>
  {#if report}
    <div class="header-center">
      <div class="hc-stat">
        <span class="hc-label">最高负载</span>
        <span class="hc-value {severityClass(highest!.percent)}">{highest!.percent.toFixed(1)}%</span>
      </div>
      <span class="hc-sep"></span>
      <div class="hc-stat">
        <span class="hc-label">窗口健康</span>
        <span class="hc-value {overallStatus}">正常 {health.ok}/{report.windows.length}</span>
      </div>
      <span class="hc-sep"></span>
      <div class="hc-stat">
        <span class="hc-label">下次重置 · {nextReset!.label}</span>
        <span class="hc-value">{new Date(nextReset!.reset_time).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })}</span>
      </div>
    </div>
  {/if}
  <div class="header-right">
    {#if report}
      <span class="status-pill {overallStatus}">
        <span class="dot"></span>
        <span class="pill-text">{statusLabel(overallStatus)}</span>
      </span>
    {/if}
    <button class="btn-primary" onclick={loadUsage} disabled={loading}>
      {#if loading}
        <span class="spinner"></span>
      {:else}
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="23 4 23 10 17 10" />
          <path d="M20.49 15a9 9 0 1 1-2.12-9.36L23 10" />
        </svg>
      {/if}
      <span class="btn-label">{loading ? "刷新中" : "刷新"}</span>
    </button>
    <button
      class="icon-btn"
      onclick={toggleFloat}
      title={floatOpen ? "关闭悬浮窗" : "打开悬浮窗"}
      aria-label="悬浮窗"
    >
      <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <rect x="3" y="3" width="13" height="13" rx="2" />
        <rect x="8" y="8" width="13" height="13" rx="2" />
      </svg>
    </button>
    <button class="icon-btn" onclick={() => (drawer = "settings")} title="设置" aria-label="设置">
      <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3" />
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
      </svg>
    </button>
    <button class="icon-btn" onclick={() => (drawer = "debug")} title="调试" aria-label="调试">
      <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <polyline points="16 18 22 12 16 6" />
        <polyline points="8 6 2 12 8 18" />
      </svg>
    </button>
  </div>
</header>

<main>
  {#if !hasCreds}
    <div class="empty-state">
      <div class="state-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="3" y="11" width="18" height="11" rx="2" />
          <path d="M7 11V7a5 5 0 0 1 10 0v4" />
        </svg>
      </div>
      <h2>请先配置凭证</h2>
      <p class="hint">需要火山引擎 Access Key ID 和 Secret Access Key</p>
      <div class="cred-wrap">
        <CredentialForm onsaved={onCredentialsSaved} />
      </div>
    </div>
  {:else if error}
    <div class="error-state">
      <div class="state-icon danger">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
      </div>
      <h2>请求出错</h2>
      <pre>{error}</pre>
      <div class="actions">
        <button class="btn-primary" onclick={loadUsage} disabled={loading}>重试</button>
        <button class="btn-ghost" onclick={checkCreds}>检查凭证</button>
      </div>
    </div>
  {:else if report}
    <div class="hero severity-{overallStatus}">
      <div class="hero-accent"></div>
      <div class="hero-head">
        <div class="hero-title">
          <h2>用量概览</h2>
          <span class="badge plan" style="background:{planColor(report.plan_type)}">{report.plan_type} Plan</span>
        </div>
        <span class="updated-time">最后更新 {formatTime(report.fetched_at)}</span>
      </div>

      <div class="hero-body">
        <div class="spotlight">
          <span class="spot-label">最高负载 · {highest!.label}</span>
          <span class="spot-percent {severityClass(highest!.percent)}">{highest!.percent.toFixed(1)}%</span>
          <div class="bar-track lg">
            <div class="bar-fill {severityClass(highest!.percent)}" style="width: {highest!.percent}%"></div>
          </div>
          <div class="spot-stats">
            <span class="ss-item">已用 <b>{highest!.used.toFixed(1)}</b></span>
            <span class="ss-item">总额 <b>{highest!.quota.toFixed(1)}</b></span>
            <span class="ss-item">剩余 <b>{highest!.remaining.toFixed(1)}</b></span>
          </div>
        </div>

        <div class="side-stats">
          <div class="mini-stat">
            <span class="mini-label">最近重置 · {nextReset!.label}</span>
            <span class="mini-value">
              {new Date(nextReset!.reset_time).toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })}
            </span>
            <span class="mini-desc">
              {new Date(nextReset!.reset_time).toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })} 重置
            </span>
          </div>
          <div class="mini-stat">
            <span class="mini-label">窗口健康</span>
            <div class="health-row">
              <span class="hr-tag ok {health.ok ? 'on' : 'off'}">正常 <b>{health.ok}</b></span>
              <span class="hr-sep">·</span>
              <span class="hr-tag warning {health.warning ? 'on' : 'off'}">告警 <b>{health.warning}</b></span>
              <span class="hr-sep">·</span>
              <span class="hr-tag danger {health.danger ? 'on' : 'off'}">危险 <b>{health.danger}</b></span>
            </div>
            <span class="mini-desc">{report.windows.length} 个窗口</span>
          </div>
        </div>
      </div>
    </div>

    <div class="section-header">
      <h3>详细指标</h3>
      <span class="section-count">{report.windows.length} 个窗口</span>
    </div>
    <div class="cards">
      {#each report.windows as w (w.key)}
        <QuotaCard window={w} />
      {/each}
    </div>
  {:else}
    <div class="loading-state">
      <span class="spinner dark"></span>
      <p>加载中...</p>
    </div>
  {/if}
</main>

{#if drawer}
  <button
    type="button"
    class="drawer-backdrop"
    onclick={() => (drawer = null)}
    aria-label="关闭面板"
  ></button>
  <div class="drawer" role="dialog" aria-modal="true">
    <div class="drawer-header">
      <h3>{drawer === "settings" ? "设置" : "调试 · 原始响应"}</h3>
      <button type="button" class="icon-btn" onclick={() => (drawer = null)} aria-label="关闭">
        <svg class="ico" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18" />
          <line x1="6" y1="6" x2="18" y2="18" />
        </svg>
      </button>
    </div>
    <div class="drawer-body">
      {#if drawer === "settings"}
        <h4 class="drawer-sub">监控配置</h4>
        <SettingsPanel />
        <div class="drawer-divider"></div>
        <h4 class="drawer-sub">凭证管理</h4>
        <CredentialForm onsaved={onCredentialsSaved} />
      {:else}
        <DebugRaw />
      {/if}
    </div>
  </div>
{/if}

<style>
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 15px 32px;
    background: rgba(7, 17, 31, 0.78);
    backdrop-filter: blur(20px) saturate(140%);
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    z-index: 10;
  }
  .header-left {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-shrink: 0;
  }
  .header-center {
    display: flex;
    align-items: center;
    gap: 18px;
    flex: 1;
    justify-content: center;
    min-width: 0;
  }
  .hc-stat {
    display: flex;
    flex-direction: column;
    gap: 2px;
    line-height: 1.2;
    white-space: nowrap;
  }
  .hc-label {
    font-size: 11px;
    color: var(--text-dim);
    font-weight: 500;
  }
  .hc-value {
    font-size: 15px;
    font-weight: 700;
    color: var(--text);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }
  .hc-value.ok { color: var(--green); }
  .hc-value.warning { color: var(--yellow); }
  .hc-value.danger { color: var(--red); }
  .hc-sep {
    width: 1px;
    height: 28px;
    background: var(--border);
    flex-shrink: 0;
  }
  .brand-mark {
    width: 36px;
    height: 36px;
    border-radius: 11px;
    background: linear-gradient(145deg, #57a2ff, #1b63dd);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 17px;
    font-weight: 700;
    letter-spacing: -0.04em;
  }
  .brand-text {
    display: flex;
    flex-direction: column;
    line-height: 1.2;
  }
  h1 {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.01em;
    color: var(--text);
  }
  .brand-sub {
    font-size: 11px;
    color: var(--text-dim);
    font-weight: 500;
  }
  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .btn-primary .ico {
    width: 14px;
    height: 14px;
  }
  .icon-btn .ico {
    width: 16px;
    height: 16px;
  }
  main {
    width: min(1440px, 100%);
    margin: 0 auto;
    padding: 34px 32px 42px;
  }

  /* ---- Hero ---- */
  .hero {
    position: relative;
    background: var(--bg-card);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    padding: 28px 30px 30px;
    margin-bottom: 34px;
    box-shadow: var(--shadow-md);
    background: linear-gradient(135deg, rgba(19, 40, 65, 0.94), rgba(9, 23, 40, 0.9));
    overflow: hidden;
  }
  .hero-accent {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: rgba(117, 153, 193, 0.45);
  }
  .hero.severity-ok .hero-accent { background: var(--green); }
  .hero.severity-warning .hero-accent { background: var(--yellow); }
  .hero.severity-danger .hero-accent { background: var(--red); }

  .hero-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 28px;
  }
  .hero-title {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .hero-title h2 {
    font-size: 21px;
    font-weight: 650;
    margin: 0;
  }
  .badge.plan {
    color: #fff;
    font-size: 11px;
    font-weight: 700;
    padding: 5px 10px;
    border-radius: 20px;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }
  .updated-time {
    font-size: 13px;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .hero-body {
    display: grid;
    grid-template-columns: 1.25fr 0.75fr;
    gap: 30px;
    align-items: stretch;
  }
  .spotlight {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 8px 30px 8px 4px;
    border-right: 1px solid var(--border);
  }
  .spot-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-muted);
  }
  .spot-percent {
    font-size: clamp(46px, 5vw, 62px);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.03em;
    line-height: 1.1;
  }
  .spot-percent.ok { color: var(--green); }
  .spot-percent.warning { color: var(--yellow); }
  .spot-percent.danger { color: var(--red); }

  .bar-track {
    height: 6px;
    background: var(--ring-track);
    border-radius: 6px;
    overflow: hidden;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.05);
  }
  .bar-track.lg {
    height: 12px;
    border-radius: 10px;
    margin: 10px 0 6px;
  }
  .bar-fill {
    height: 100%;
    border-radius: 6px;
    transition: width 0.8s cubic-bezier(0.4, 0, 0.2, 1);
  }
  .bar-track.lg .bar-fill {
    border-radius: 10px;
  }
  .bar-fill.ok { background: var(--green); }
  .bar-fill.warning { background: var(--yellow); }
  .bar-fill.danger { background: var(--red); }

  .spot-stats {
    display: flex;
    gap: 20px;
    margin-top: 6px;
  }
  .ss-item {
    font-size: 13px;
    color: var(--text-muted);
  }
  .ss-item b {
    color: var(--text);
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .side-stats {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .mini-stat {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 17px 18px;
    background: rgba(4, 14, 28, 0.42);
    border-radius: 14px;
    border: 1px solid var(--border);
  }
  .mini-label {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-muted);
  }
  .mini-value {
    font-size: 25px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.02em;
    color: var(--text);
    line-height: 1.2;
  }
  .mini-value.ok { color: var(--green); }
  .mini-desc {
    font-size: 12px;
    color: var(--text-dim);
  }
  .health-row {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: 5px 8px;
    margin-top: 1px;
    font-size: 18px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.01em;
  }
  .health-row .hr-tag b {
    font-weight: 700;
  }
  .health-row .hr-tag.on.ok { color: var(--green); }
  .health-row .hr-tag.on.warning { color: var(--yellow); }
  .health-row .hr-tag.on.danger { color: var(--red); }
  .health-row .hr-tag.off {
    color: var(--text-dim);
    font-weight: 500;
  }
  .health-row .hr-sep {
    color: var(--border-hover);
    font-weight: 400;
  }

  /* ---- Section ---- */
  .section-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: 18px;
  }
  .section-header h3 {
    font-size: 18px;
    font-weight: 650;
    color: var(--text);
  }
  .section-count {
    font-size: 12px;
    color: var(--text-dim);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 300px), 1fr));
    gap: 22px;
  }

  /* ---- States ---- */
  .empty-state,
  .error-state,
  .loading-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 64px 24px;
    text-align: center;
    background: rgba(13, 27, 45, 0.78);
    border: 1px dashed var(--border-hover);
    border-radius: var(--radius-lg);
    margin: 24px 0;
  }
  .state-icon {
    width: 48px;
    height: 48px;
    color: var(--text-dim);
    margin-bottom: 16px;
  }
  .state-icon svg {
    width: 100%;
    height: 100%;
  }
  .state-icon.danger {
    color: var(--red);
  }
  .empty-state h2,
  .error-state h2 {
    font-size: 20px;
    margin-bottom: 8px;
    font-weight: 600;
  }
  .hint {
    color: var(--text-muted);
    font-size: 14px;
    margin-bottom: 24px;
  }
  .cred-wrap {
    width: 100%;
    max-width: 440px;
    text-align: left;
    background: rgba(4, 14, 28, 0.5);
    padding: 24px;
    border-radius: 12px;
    border: 1px solid var(--border);
  }
  .error-state pre {
    background: rgba(239, 68, 68, 0.05);
    border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 8px;
    padding: 16px;
    font-size: 13px;
    color: var(--red);
    max-width: 600px;
    max-height: 200px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-all;
    margin-bottom: 24px;
    text-align: left;
  }
  .error-state .actions {
    display: flex;
    gap: 12px;
  }
  .loading-state {
    flex-direction: row;
    gap: 12px;
  }
  .spinner.dark {
    border-color: rgba(255, 255, 255, 0.18);
    border-top-color: var(--text);
  }

  /* ---- Drawer ---- */
  .drawer-divider {
    height: 1px;
    background: var(--border);
    margin: 22px 0 18px;
  }
  .drawer-sub {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-muted);
    margin-bottom: 14px;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  /* ---- Responsive: adapt to window size ---- */

  /* 中等窗口:收紧间距,header center 只保留最高负载 */
  @media (max-width: 980px) {
    main {
      padding: 26px 22px;
    }
    .hero {
      padding: 20px;
    }
    .header-center {
      gap: 12px;
    }
    .header-center .hc-stat:nth-child(n+4),
    .header-center .hc-sep:nth-child(n+3) {
      display: none;
    }
  }

  /* 窄窗口:Hero 堆叠为单列,隐藏 header center */
  @media (max-width: 760px) {
    main {
      padding: 20px 18px 30px;
    }
    header {
      padding: 10px 18px;
    }
    .header-center {
      display: none;
    }
    .hero-body {
      grid-template-columns: 1fr;
    }
    .spotlight {
      border-right: none;
      border-bottom: 1px solid var(--border);
      padding-right: 0;
      padding-bottom: 20px;
    }
    .hero-head {
      flex-wrap: wrap;
      gap: 8px;
    }
    .spot-stats {
      flex-wrap: wrap;
      gap: 16px 20px;
    }
    .brand-sub {
      display: none;
    }
  }

  /* 很窄的窗口:header 收敛为图标,内边距最小化 */
  @media (max-width: 560px) {
    main {
      padding: 14px;
    }
    header {
      padding: 10px 14px;
    }
    .header-right {
      gap: 4px;
    }
    /* 刷新按钮只保留图标 */
    .btn-primary .btn-label {
      display: none;
    }
    /* 状态药丸只保留圆点 */
    .status-pill {
      padding: 5px 8px;
    }
    .status-pill .pill-text {
      display: none;
    }
    .hero {
      padding: 16px;
    }
    .hero-title h2 {
      font-size: 16px;
    }
    .spot-percent {
      font-size: 36px;
    }
    .mini-value {
      font-size: 20px;
    }
    .cards {
      gap: 14px;
    }
    .empty-state,
    .error-state,
    .loading-state {
      padding: 40px 18px;
    }
  }

  /* ---- Command center redesign ---- */
  header {
    min-height: 104px;
    padding: 20px 42px;
    background: rgba(7, 16, 29, 0.82);
    border-bottom-color: rgba(148, 174, 210, 0.16);
    backdrop-filter: blur(22px) saturate(125%);
  }
  .brand-mark {
    width: 54px;
    height: 54px;
    border-radius: 14px;
    background: linear-gradient(145deg, #303a8d, #172456);
    box-shadow: inset 0 1px 1px rgba(255,255,255,.13), 0 10px 20px rgba(28, 66, 174, .22);
    font-size: 24px;
  }
  h1 { font-size: 20px; letter-spacing: -.035em; }
  .brand-sub { margin-top: 3px; font-size: 13px; }
  .header-center { gap: 30px; }
  .hc-stat { min-width: 112px; gap: 5px; }
  .hc-label { font-size: 13px; color: var(--text-muted); }
  .hc-value { font-size: 18px; letter-spacing: -.025em; }
  .hc-sep { height: 52px; background: rgba(148,174,210,.18); }
  .header-right { gap: 12px; }
  .header-right .btn-primary { min-height: 42px; padding: 10px 19px; font-size: 15px; }
  .header-right .icon-btn { width: 42px; height: 42px; }
  .status-pill { display: none; }
  main { width: min(1510px, 100%); padding: 34px 40px 52px; }
  .hero {
    margin-bottom: 30px;
    padding: 0;
    background: transparent;
    border: 0;
    border-radius: 0;
    box-shadow: none;
    overflow: visible;
  }
  .hero-accent { display: none; }
  .hero-head { align-items: baseline; gap: 18px; margin-bottom: 22px; }
  .hero-title h2 { font-size: 34px; font-weight: 750; letter-spacing: -.045em; }
  .badge.plan { padding: 5px 9px; border-radius: 7px; font-size: 11px; opacity: .9; }
  .updated-time { font-size: 13px; color: var(--text-muted); }
  .hero-body { grid-template-columns: minmax(0, 1.55fr) minmax(310px, .9fr); gap: 26px; }
  .spotlight {
    min-height: 320px;
    padding: 31px 32px 26px;
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: linear-gradient(135deg, rgba(29, 48, 73, .92), rgba(17, 30, 47, .87));
    box-shadow: var(--shadow-sm);
  }
  .spot-label { font-size: 17px; color: var(--text); font-weight: 650; }
  .spot-percent { font-size: clamp(64px, 6.5vw, 88px); color: var(--text) !important; margin-top: 10px; }
  .bar-track.lg { height: 11px; margin: 10px 0 19px; background: #2c3b4e; }
  .bar-fill.ok { background: var(--accent); }
  .spot-stats { justify-content: space-between; gap: 0; margin-top: 2px; }
  .ss-item { flex: 1; display: flex; flex-direction: column; gap: 7px; font-size: 13px; color: var(--text-muted); }
  .ss-item + .ss-item { padding-left: 26px; border-left: 1px solid var(--border); }
  .ss-item b { font-size: 21px; color: var(--text); letter-spacing: -.025em; }
  .side-stats { gap: 20px; }
  .mini-stat { position: relative; min-height: 150px; justify-content: center; padding: 24px 28px 23px 116px; background: linear-gradient(135deg, rgba(29,48,73,.88), rgba(17,30,47,.82)); border-radius: var(--radius-lg); box-shadow: var(--shadow-sm); }
  .mini-stat::before { position: absolute; left: 31px; top: 50%; width: 48px; height: 48px; border: 4px solid var(--accent); border-radius: 50%; transform: translateY(-50%); content: ''; opacity: .95; }
  .mini-stat:nth-child(2)::before { border-color: var(--green); border-radius: 16px 16px 16px 5px; transform: translateY(-50%) rotate(-45deg); }
  .mini-label { font-size: 14px; color: var(--text); font-weight: 650; }
  .mini-value { margin: 5px 0; font-size: 35px; }
  .mini-desc { font-size: 14px; }
  .health-row { margin: 6px 0 2px; font-size: 19px; }
  .section-header { margin: 4px 0 20px; justify-content: flex-start; gap: 10px; }
  .section-header h3 { font-size: 25px; letter-spacing: -.035em; }
  .section-count { font-size: 16px; color: var(--text-muted); }
  .cards { grid-template-columns: repeat(auto-fit, minmax(278px, 1fr)); gap: 20px; }
  .empty-state, .error-state, .loading-state { max-width: 720px; margin: 64px auto; background: var(--surface-soft); border-style: solid; }
  .drawer-sub { color: var(--text-muted); }

  @media (max-width: 980px) {
    header { padding: 16px 24px; min-height: 82px; }
    .brand-mark { width: 43px; height: 43px; font-size: 19px; }
    main { padding: 28px 24px 42px; }
    .header-center { gap: 20px; }
    .hero-body { grid-template-columns: 1fr; }
    .side-stats { display: grid; grid-template-columns: 1fr 1fr; }
    .mini-stat { min-height: 136px; }
  }
  @media (max-width: 760px) {
    header { padding: 12px 18px; min-height: 70px; }
    .brand-mark { width: 38px; height: 38px; border-radius: 11px; font-size: 17px; }
    h1 { font-size: 16px; }
    main { padding: 28px 18px 36px; }
    .hero-title h2 { font-size: 28px; }
    .hero-head { align-items: flex-start; flex-direction: column; gap: 6px; }
    .spotlight { min-height: auto; padding: 26px 24px; }
    .side-stats { grid-template-columns: 1fr; }
    .mini-stat { min-height: 132px; }
  }
  @media (max-width: 560px) {
    .header-right .btn-primary { min-height: 38px; padding: 10px; }
    .header-right .icon-btn { width: 38px; height: 38px; }
    .header-right .icon-btn:last-child { display: none; }
    main { padding: 24px 14px 32px; }
    .hero-title h2 { font-size: 25px; }
    .spot-percent { font-size: 62px; }
    .spot-stats { gap: 16px; flex-wrap: wrap; }
    .ss-item { flex: 0 0 calc(50% - 10px); }
    .ss-item + .ss-item { padding-left: 0; border-left: 0; }
    .section-header h3 { font-size: 21px; }
    .section-count { font-size: 14px; }
  }
</style>

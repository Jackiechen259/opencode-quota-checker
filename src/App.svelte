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
  import { autoStartMonitor } from "./lib/monitorConfig";

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
    // 凭证就绪后,若已开启自动更新则启动后台轮询
    await autoStartMonitor();
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
      // 启动时若已开启自动更新则启动后台轮询(主窗口触发,悬浮窗不重复触发)
      await autoStartMonitor();
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
      Small: "#475569",
      Medium: "#1d4ed8",
      Large: "#6d28d9",
      Max: "#92400e",
    };
    return map[pt] || "#475569";
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
          <span class="badge plan" style="--plan-color:{planColor(report.plan_type)}">{report.plan_type} Plan</span>
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
  /* ---- Header ---- */
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 18px 32px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    z-index: 10;
  }
  .header-left { display: flex; align-items: center; gap: 12px; flex-shrink: 0; }
  .header-center { display: flex; align-items: center; gap: 28px; flex: 1; justify-content: center; min-width: 0; }
  .hc-stat { display: flex; flex-direction: column; gap: 4px; min-width: 112px; line-height: 1.2; white-space: nowrap; }
  .hc-label { font-size: 12px; color: var(--text-muted); font-weight: 500; }
  .hc-value { font-size: 17px; font-weight: 700; color: var(--text); font-variant-numeric: tabular-nums; letter-spacing: -0.01em; }
  .hc-value.ok { color: var(--green); }
  .hc-value.warning { color: var(--yellow); }
  .hc-value.danger { color: var(--red); }
  .hc-sep { width: 1px; height: 36px; background: var(--border); flex-shrink: 0; }

  .brand-mark {
    width: 44px; height: 44px; border-radius: 12px;
    background: var(--accent); color: #fff;
    display: flex; align-items: center; justify-content: center;
    font-size: 19px; font-weight: 700; letter-spacing: -0.04em;
  }
  .brand-text { display: flex; flex-direction: column; line-height: 1.2; }
  h1 { font-size: 18px; font-weight: 700; letter-spacing: -0.01em; color: var(--text); }
  .brand-sub { margin-top: 2px; font-size: 12px; color: var(--text-dim); font-weight: 500; }

  .header-right { display: flex; align-items: center; gap: 10px; flex-shrink: 0; }
  .header-right .btn-primary { min-height: 40px; padding: 10px 18px; font-size: 14px; }
  .header-right .icon-btn { width: 40px; height: 40px; }
  .btn-primary .ico { width: 14px; height: 14px; }
  .icon-btn .ico { width: 16px; height: 16px; }
  .status-pill { display: none; }

  main { width: min(1440px, 100%); margin: 0 auto; padding: 32px 32px 48px; }

  /* ---- Hero ---- */
  .hero {
    position: relative;
    border: 0;
    border-radius: 0;
    padding: 0;
    margin-bottom: 20px;
    background: transparent;
    box-shadow: none;
    overflow: visible;
  }
  .hero-accent { display: none; }
  .hero-head { display: flex; justify-content: space-between; align-items: baseline; gap: 16px; margin-bottom: 14px; }
  .hero-title { display: flex; align-items: center; gap: 10px; }
  .hero-title h2 { font-size: 22px; font-weight: 700; letter-spacing: -0.035em; color: var(--text); }
  .badge.plan {
    --plan-color: var(--accent);
    color: var(--plan-color);
    background: color-mix(in srgb, var(--plan-color) 14%, transparent);
    border: 1px solid color-mix(in srgb, var(--plan-color) 28%, transparent);
    font-size: 11px; font-weight: 700; padding: 5px 10px;
    border-radius: 8px; text-transform: uppercase; letter-spacing: 0.02em;
  }
  .updated-time { font-size: 13px; color: var(--text-muted); font-variant-numeric: tabular-nums; }

  .hero-body { display: grid; grid-template-columns: minmax(0, 1.5fr) minmax(280px, 0.9fr); gap: 18px; align-items: stretch; }

  .spotlight {
    display: flex; flex-direction: column; gap: 6px;
    min-height: 200px; padding: 20px 24px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
  }
  .spot-label { font-size: 14px; color: var(--text-muted); font-weight: 600; }
  .spot-percent {
    font-size: clamp(40px, 4vw, 52px);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.03em;
    line-height: 1.1;
    color: var(--text);
    margin-top: 4px;
  }
  .spot-percent.ok { color: var(--green); }
  .spot-percent.warning { color: var(--yellow); }
  .spot-percent.danger { color: var(--red); }

  .bar-track {
    height: 6px; background: var(--ring-track); border-radius: 6px; overflow: hidden;
  }
  .bar-track.lg { height: 8px; border-radius: 8px; margin: 8px 0 12px; }
  .bar-fill { height: 100%; border-radius: inherit; transition: width 0.8s cubic-bezier(0.4, 0, 0.2, 1); }
  .bar-fill.ok { background: var(--green); }
  .bar-fill.warning { background: var(--yellow); }
  .bar-fill.danger { background: var(--red); }

  .spot-stats { display: flex; justify-content: space-between; gap: 0; margin-top: 2px; }
  .ss-item { flex: 1; display: flex; flex-direction: column; gap: 6px; font-size: 12px; color: var(--text-muted); }
  .ss-item + .ss-item { padding-left: 22px; border-left: 1px solid var(--border); }
  .ss-item b { font-size: 15px; color: var(--text); font-weight: 700; font-variant-numeric: tabular-nums; letter-spacing: -0.02em; }

  .side-stats { display: flex; flex-direction: column; gap: 12px; }
  .mini-stat {
    display: flex; flex-direction: column; gap: 4px;
    min-height: 92px; padding: 16px 20px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-sm);
    justify-content: center;
  }
  .mini-label { font-size: 13px; color: var(--text-muted); font-weight: 600; }
  .mini-value { font-size: 22px; font-weight: 700; font-variant-numeric: tabular-nums; letter-spacing: -0.02em; color: var(--text); line-height: 1.2; margin: 2px 0; }
  .mini-desc { font-size: 13px; color: var(--text-dim); }

  .health-row { display: flex; align-items: baseline; flex-wrap: wrap; gap: 5px 10px; margin: 4px 0 2px; font-size: 15px; font-weight: 700; font-variant-numeric: tabular-nums; letter-spacing: -0.01em; }
  .health-row .hr-tag b { font-weight: 700; }
  .health-row .hr-tag.on.ok { color: var(--green); }
  .health-row .hr-tag.on.warning { color: var(--yellow); }
  .health-row .hr-tag.on.danger { color: var(--red); }
  .health-row .hr-tag.off { color: var(--text-dim); font-weight: 500; }
  .health-row .hr-sep { color: var(--text-dim); font-weight: 400; }

  /* ---- Section ---- */
  .section-header { display: flex; align-items: baseline; justify-content: space-between; gap: 10px; margin: 4px 0 18px; }
  .section-header h3 { font-size: 22px; font-weight: 700; letter-spacing: -0.03em; color: var(--text); }
  .section-count { font-size: 14px; color: var(--text-muted); }
  .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(278px, 1fr)); gap: 18px; }

  /* ---- States ---- */
  .empty-state, .error-state, .loading-state {
    display: flex; flex-direction: column; align-items: center; justify-content: center;
    padding: 56px 24px; text-align: center;
    background: var(--surface); border: 1px dashed var(--border-strong);
    border-radius: var(--radius-lg); margin: 40px auto; max-width: 680px;
  }
  .state-icon { width: 44px; height: 44px; color: var(--text-dim); margin-bottom: 14px; }
  .state-icon svg { width: 100%; height: 100%; }
  .state-icon.danger { color: var(--red); }
  .empty-state h2, .error-state h2 { font-size: 18px; margin-bottom: 6px; font-weight: 600; color: var(--text); }
  .hint { color: var(--text-muted); font-size: 13px; margin-bottom: 22px; }
  .cred-wrap { width: 100%; max-width: 420px; text-align: left; background: var(--surface-soft); padding: 22px; border-radius: 12px; border: 1px solid var(--border); }
  .error-state pre {
    background: rgba(239, 68, 68, 0.05); border: 1px solid rgba(239, 68, 68, 0.2);
    border-radius: 8px; padding: 14px; font-size: 12px; color: var(--red);
    max-width: 600px; max-height: 200px; overflow: auto;
    white-space: pre-wrap; word-break: break-all; margin-bottom: 22px; text-align: left;
  }
  .error-state .actions { display: flex; gap: 12px; }
  .loading-state { flex-direction: row; gap: 12px; }
  .spinner.dark { border-color: rgba(15, 23, 42, 0.12); border-top-color: var(--accent); }

  /* ---- Drawer ---- */
  .drawer-divider { height: 1px; background: var(--border); margin: 22px 0 18px; }
  .drawer-sub { font-size: 12px; font-weight: 700; color: var(--text-muted); margin-bottom: 14px; text-transform: uppercase; letter-spacing: 0.05em; }

  /* ---- Responsive ---- */
  @media (max-width: 980px) {
    main { padding: 26px 22px; }
    header { padding: 16px 22px; }
    .hero-body { grid-template-columns: 1fr; }
    .side-stats { display: grid; grid-template-columns: 1fr 1fr; }
    .header-center { gap: 18px; }
    .header-center .hc-stat:nth-child(n+4),
    .header-center .hc-sep:nth-child(n+3) { display: none; }
  }
  @media (max-width: 760px) {
    main { padding: 22px 18px 32px; }
    header { padding: 12px 18px; }
    .header-center { display: none; }
    .hero-head { flex-wrap: wrap; gap: 8px; }
    .hero-title h2 { font-size: 20px; }
    .spotlight { min-height: auto; padding: 18px 18px; }
    .side-stats { grid-template-columns: 1fr; }
    .spot-stats { flex-wrap: wrap; gap: 16px 20px; }
    .brand-sub { display: none; }
  }
  @media (max-width: 560px) {
    main { padding: 18px 14px 28px; }
    header { padding: 10px 14px; }
    .header-right { gap: 6px; }
    .header-right .btn-primary { min-height: 36px; padding: 8px 10px; }
    .header-right .icon-btn { width: 36px; height: 36px; }
    .header-right .icon-btn:last-child { display: none; }
    .btn-primary .btn-label { display: none; }
    .hero-title h2 { font-size: 19px; }
    .spot-percent { font-size: 36px; }
    .mini-value { font-size: 20px; }
    .cards { gap: 14px; }
    .ss-item { flex: 0 0 calc(50% - 10px); }
    .ss-item + .ss-item { padding-left: 0; border-left: 0; }
    .empty-state, .error-state, .loading-state { padding: 36px 16px; }
  }
</style>

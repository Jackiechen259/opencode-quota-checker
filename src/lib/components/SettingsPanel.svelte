<script lang="ts">
  import type { Thresholds } from "../types";
  import { startMonitor, stopMonitor, getMonitorStatus } from "../api";
  import { loadMonitorConfig, saveMonitorConfig } from "../monitorConfig";

  let intervalSec = $state(300);
  let thresholds = $state<Thresholds>({
    five_hour: 80,
    weekly: 85,
    monthly: 85,
  });
  let running = $state(false);
  let message = $state("");

  async function handleStart() {
    message = "";
    try {
      await startMonitor(intervalSec, thresholds);
      running = true;
      // 持久化配置,下次启动自动恢复并自动开启后台轮询
      await saveMonitorConfig({ enabled: true, intervalSec, thresholds });
      message = "监控已启动";
    } catch (e) {
      message = String(e);
    }
  }

  async function handleStop() {
    message = "";
    try {
      await stopMonitor();
      running = false;
      await saveMonitorConfig({ enabled: false });
      message = "监控已停止";
    } catch (e) {
      message = String(e);
    }
  }

  async function loadStatus() {
    try {
      const cfg = await loadMonitorConfig();
      intervalSec = cfg.intervalSec;
      thresholds = cfg.thresholds;
      const s = await getMonitorStatus();
      running = s.running;
      // 运行中以真实间隔为准(可能在别处已修改)
      if (s.running) intervalSec = s.interval_sec;
    } catch {
      /* ignore */
    }
  }

  $effect(() => {
    loadStatus();
  });
</script>

<div class="settings">
  <div class="field">
    <label for="interval">轮询间隔(秒, 最小 30)</label>
    <input
      id="interval"
      type="number"
      min="30"
      max="3600"
      bind:value={intervalSec}
      disabled={running}
    />
  </div>

  <div class="thresholds">
    <div class="th-label">告警阈值(使用率 %)</div>
    <div class="th-grid">
      <div class="th-item">
        <span>5 小时</span>
        <input type="number" min="0" max="100" bind:value={thresholds.five_hour} disabled={running} />
      </div>
      <div class="th-item">
        <span>近一周</span>
        <input type="number" min="0" max="100" bind:value={thresholds.weekly} disabled={running} />
      </div>
      <div class="th-item">
        <span>近一月</span>
        <input type="number" min="0" max="100" bind:value={thresholds.monthly} disabled={running} />
      </div>
    </div>
  </div>

  <div class="actions">
    {#if running}
      <button class="btn-primary danger" onclick={handleStop}>停止监控</button>
    {:else}
      <button class="btn-primary" onclick={handleStart}>启动监控</button>
    {/if}
  </div>
  {#if message}
    <div class="message">{message}</div>
  {/if}
</div>

<style>
  .thresholds {
    margin: 12px 0;
  }
  .th-label {
    font-size: 12px;
    color: var(--text-muted);
    margin-bottom: 8px;
  }
  .th-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .th-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .th-item span {
    font-size: 11px;
    color: var(--text-dim);
  }
  .th-item input {
    padding: 6px 10px;
    font-size: 12px;
  }
  .actions {
    margin-top: 8px;
  }
  .btn-primary.danger {
    background: var(--red);
  }
  .btn-primary.danger:hover {
    background: #dc2626;
  }
  .message {
    margin-top: 8px;
    font-size: 12px;
    color: var(--accent);
  }
</style>

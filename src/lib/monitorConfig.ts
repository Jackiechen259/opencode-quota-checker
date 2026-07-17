import { load as loadStore } from "@tauri-apps/plugin-store";
import type { Thresholds } from "./types";
import { startMonitor } from "./api";

const STORE_PATH = "settings.json";
const KEY_ENABLED = "monitor_enabled";
const KEY_INTERVAL = "monitor_interval";
const KEY_THRESHOLDS = "monitor_thresholds";

/** 默认轮询间隔(秒) */
export const DEFAULT_INTERVAL = 300;
/** 默认告警阈值 */
export const DEFAULT_THRESHOLDS: Thresholds = {
  five_hour: 80,
  weekly: 85,
  monthly: 85,
};

export interface MonitorConfig {
  enabled: boolean;
  intervalSec: number;
  thresholds: Thresholds;
}

/** 从持久化存储读取监控配置;首次启动默认开启自动更新 */
export async function loadMonitorConfig(): Promise<MonitorConfig> {
  const store = await loadStore(STORE_PATH);
  const enabled = (await store.get<boolean>(KEY_ENABLED)) ?? true;
  const intervalSec =
    (await store.get<number>(KEY_INTERVAL)) ?? DEFAULT_INTERVAL;
  const thresholds =
    (await store.get<Thresholds>(KEY_THRESHOLDS)) ?? DEFAULT_THRESHOLDS;
  return { enabled, intervalSec, thresholds };
}

/** 增量持久化监控配置(仅写入提供的字段) */
export async function saveMonitorConfig(
  cfg: Partial<MonitorConfig>
): Promise<void> {
  const store = await loadStore(STORE_PATH);
  if (cfg.enabled !== undefined) await store.set(KEY_ENABLED, cfg.enabled);
  if (cfg.intervalSec !== undefined)
    await store.set(KEY_INTERVAL, cfg.intervalSec);
  if (cfg.thresholds !== undefined)
    await store.set(KEY_THRESHOLDS, cfg.thresholds);
  await store.save();
}

/**
 * 应用启动时的自动额度更新入口。
 * 当配置开启且已存在凭证时,以持久化的间隔与阈值启动后台轮询。
 * 无凭证或配置关闭时静默跳过。调用方需自行保证凭证已配置。
 */
export async function autoStartMonitor(): Promise<boolean> {
  const { enabled, intervalSec, thresholds } = await loadMonitorConfig();
  if (!enabled) return false;
  try {
    await startMonitor(intervalSec, thresholds);
    return true;
  } catch {
    return false;
  }
}

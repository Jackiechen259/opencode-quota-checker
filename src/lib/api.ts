import { invoke } from "@tauri-apps/api/core";
import type { UsageReport, Thresholds, MonitorStatus } from "./types";

export async function setCredentials(ak: string, sk: string): Promise<void> {
  await invoke("set_credentials", { ak, sk });
}

export async function hasCredentials(): Promise<boolean> {
  return await invoke("has_credentials");
}

export async function clearCredentials(): Promise<void> {
  await invoke("clear_credentials");
}

export async function fetchUsage(): Promise<UsageReport> {
  return await invoke("fetch_usage");
}

export async function fetchUsageRaw(): Promise<string> {
  return await invoke("fetch_usage_raw");
}

export async function startMonitor(
  intervalSec: number,
  thresholds: Thresholds
): Promise<void> {
  await invoke("start_monitor", {
    intervalSec,
    thresholds,
  });
}

export async function stopMonitor(): Promise<void> {
  await invoke("stop_monitor");
}

export async function getMonitorStatus(): Promise<MonitorStatus> {
  return await invoke("get_monitor_status");
}

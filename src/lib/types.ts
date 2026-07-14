export interface WindowReport {
  key: string;
  label: string;
  quota: number;
  used: number;
  remaining: number;
  percent: number;
  subscribe_time: number;
  reset_time: number;
  reset_in_secs: number;
}

export interface UsageReport {
  plan_type: string;
  windows: WindowReport[];
  fetched_at: number;
}

export interface Thresholds {
  five_hour: number;
  weekly: number;
  monthly: number;
}

export interface MonitorStatus {
  running: boolean;
  interval_sec: number;
}

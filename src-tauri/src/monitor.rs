use crate::client;
use crate::models::UsageReport;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// 各窗口的告警阈值(百分比 0-100)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thresholds {
    pub five_hour: f64,
    pub weekly: f64,
    pub monthly: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            five_hour: 80.0,
            weekly: 85.0,
            monthly: 85.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MonitorStatus {
    pub running: bool,
    pub interval_sec: u64,
}

pub struct Monitor {
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl Clone for Monitor {
    fn clone(&self) -> Self {
        Self {
            handle: Arc::clone(&self.handle),
        }
    }
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            handle: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(
        &self,
        app: AppHandle,
        ak: String,
        sk: String,
        interval_sec: u64,
        thresholds: Thresholds,
    ) -> Result<(), String> {
        let mut guard = self.handle.lock().await;
        if guard.is_some() {
            let _ = guard.take();
        }

        let handle = tokio::spawn(async move {
            run_loop(app, ak, sk, interval_sec, thresholds).await;
        });
        *guard = Some(handle);
        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        let mut guard = self.handle.lock().await;
        if let Some(h) = guard.take() {
            h.abort();
        }
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        let guard = self.handle.lock().await;
        guard.is_some()
    }
}

async fn run_loop(
    app: AppHandle,
    ak: String,
    sk: String,
    interval_sec: u64,
    thresholds: Thresholds,
) {
    let mut last_alerted: std::collections::HashMap<String, i64> =
        std::collections::HashMap::new();
    let interval = std::time::Duration::from_secs(interval_sec.max(30));

    loop {
        match client::fetch_report(&ak, &sk).await {
            Ok(report) => {
                check_and_notify(&app, &report, &thresholds, &mut last_alerted);
                let _ = app.emit("usage-updated", &report);
            }
            Err(e) => {
                log::error!("监控轮询失败: {}", e);
                let _ = app.emit("usage-error", e.to_string());
            }
        }
        tokio::time::sleep(interval).await;
    }
}

fn check_and_notify(
    app: &AppHandle,
    report: &UsageReport,
    thresholds: &Thresholds,
    last_alerted: &mut std::collections::HashMap<String, i64>,
) {
    let threshold_map = [
        ("five_hour", thresholds.five_hour),
        ("weekly", thresholds.weekly),
        ("monthly", thresholds.monthly),
    ];

    for w in &report.windows {
        let threshold = threshold_map
            .iter()
            .find(|(k, _)| *k == w.key)
            .map(|(_, t)| *t)
            .unwrap_or(80.0);

        if w.percent >= threshold {
            let already = last_alerted.get(&w.key).copied().unwrap_or(0) == w.subscribe_time;
            if !already {
                notify(
                    app,
                    &format!("配额告警: {}", w.label),
                    &format!(
                        "已用 {:.1} / {:.1} ({:.0}%), 剩余 {:.1}",
                        w.used, w.quota, w.percent, w.remaining
                    ),
                );
                last_alerted.insert(w.key.clone(), w.subscribe_time);
            }
        } else {
            if last_alerted.contains_key(&w.key) {
                last_alerted.remove(&w.key);
            }
        }
    }
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    use tauri_plugin_notification::NotificationExt;
    let _ = app.notification().builder()
        .title(title)
        .body(body)
        .show();
}
